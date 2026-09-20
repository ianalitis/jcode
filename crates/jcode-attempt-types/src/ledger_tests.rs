use super::*;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static NEXT_SCRATCH_ID: AtomicU64 = AtomicU64::new(0);

struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new(label: &str) -> Self {
        let root = std::env::var_os("JCODE_SCRATCH_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/test-scratch")
            });
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let id = NEXT_SCRATCH_ID.fetch_add(1, Ordering::Relaxed);
        let path = root.join(format!(
            "jcode-attempt-ledger-{label}-{}-{nonce}-{id}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("create scratch directory");
        Self(path)
    }

    fn ledger_path(&self) -> PathBuf {
        self.0.join("ledger.json")
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn wait_for_path(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !path.exists() {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {}",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn durable_reopen_preserves_held_ambiguous_settled_and_reconciled_amounts() {
    let scratch = ScratchDir::new("states");
    let path = scratch.ledger_path();
    {
        let ledger = LocalLedger::open(&path, 500).unwrap();
        ledger.reserve("held", 100).unwrap();
        ledger.reserve("ambiguous", 80).unwrap();
        ledger.mark_ambiguous("ambiguous").unwrap();
        ledger.reserve("settled", 70).unwrap();
        ledger.settle("settled", 20).unwrap();
        ledger.reserve("reconciled", 60).unwrap();
        ledger.reconcile("reconciled", 35).unwrap();
        assert_eq!(ledger.reserve_remaining("remaining").unwrap(), 265);
        assert_eq!(ledger.exposure_micro_usd(), 500);
    }

    let reopened = LocalLedger::open(&path, 500).unwrap();
    assert_eq!(reopened.exposure_micro_usd(), 500);
    assert_eq!(reopened.get("held").unwrap().state, ReservationState::Held);
    assert_eq!(
        reopened.get("ambiguous").unwrap().state,
        ReservationState::Ambiguous
    );
    assert_eq!(reopened.get("settled").unwrap().settled_micro_usd, Some(20));
    assert_eq!(
        reopened.get("reconciled").unwrap().settled_micro_usd,
        Some(35)
    );
    assert_eq!(reopened.get("remaining").unwrap().reserved_micro_usd, 265);
}

#[test]
fn durable_reopen_denies_duplicate_and_cap_mismatch() {
    let scratch = ScratchDir::new("duplicate-cap");
    let path = scratch.ledger_path();
    {
        let ledger = LocalLedger::open(&path, 100).unwrap();
        ledger.reserve("attempt", 100).unwrap();
    }

    let reopened = LocalLedger::open(&path, 100).unwrap();
    assert_eq!(
        reopened.reserve("attempt", 1),
        Err(LedgerError::DuplicateAttempt("attempt".into()))
    );
    drop(reopened);
    let mismatch = LocalLedger::open(&path, 101).map(|_| ());
    assert!(
        matches!(
            mismatch,
            Err(LedgerError::CapMismatch {
                persisted: 100,
                requested: 101
            })
        ),
        "{mismatch:?}"
    );
}

#[test]
fn durable_lock_child() {
    if std::env::var_os("JCODE_LEDGER_LOCK_CHILD").is_none() {
        return;
    }
    let path = PathBuf::from(std::env::var_os("JCODE_LEDGER_PATH").unwrap());
    let ready = PathBuf::from(std::env::var_os("JCODE_LEDGER_READY").unwrap());
    let release = PathBuf::from(std::env::var_os("JCODE_LEDGER_RELEASE").unwrap());
    let _ledger = LocalLedger::open(path, 100).expect("child acquires durable lock");
    std::fs::write(&ready, b"ready").unwrap();
    wait_for_path(&release);
}

#[test]
fn competing_process_is_refused_until_owner_exits() {
    let scratch = ScratchDir::new("process-lock");
    let path = scratch.ledger_path();
    let ready = scratch.0.join("ready");
    let release = scratch.0.join("release");
    let child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "ledger_tests::durable_lock_child", "--nocapture"])
        .env("JCODE_LEDGER_LOCK_CHILD", "1")
        .env("JCODE_LEDGER_PATH", &path)
        .env("JCODE_LEDGER_READY", &ready)
        .env("JCODE_LEDGER_RELEASE", &release)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn lock holder");
    wait_for_path(&ready);

    assert!(matches!(
        LocalLedger::open(&path, 100),
        Err(LedgerError::LockUnavailable(_))
    ));
    std::fs::write(&release, b"release").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "child failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    LocalLedger::open(&path, 100).expect("lock released when owner exits");
}

#[test]
fn durable_clone_keeps_sidecar_lock_until_last_owner_drops() {
    let scratch = ScratchDir::new("clone-lock");
    let path = scratch.ledger_path();
    let ledger = LocalLedger::open(&path, 100).unwrap();
    let clone = ledger.clone();
    drop(ledger);
    assert!(matches!(
        LocalLedger::open(&path, 100),
        Err(LedgerError::LockUnavailable(_))
    ));
    drop(clone);
    LocalLedger::open(&path, 100).expect("last clone releases sidecar lock");
}

#[test]
fn corrupt_or_unsupported_state_never_resets_to_empty() {
    let scratch = ScratchDir::new("corrupt");
    let path = scratch.ledger_path();
    std::fs::write(&path, b"not json").unwrap();
    assert!(matches!(
        LocalLedger::open(&path, 100),
        Err(LedgerError::CorruptState(_))
    ));
    assert_eq!(std::fs::read(&path).unwrap(), b"not json");

    std::fs::write(
        &path,
        br#"{"version":999,"cap_micro_usd":100,"reservations":{}}"#,
    )
    .unwrap();
    let unsupported = LocalLedger::open(&path, 100).map(|_| ());
    assert!(
        matches!(unsupported, Err(LedgerError::UnsupportedVersion(999))),
        "{unsupported:?}"
    );

    std::fs::write(
        &path,
        br#"{"version":1,"cap_micro_usd":100,"reservations":{"bad":{"attempt_id":"bad","reserved_micro_usd":50,"settled_micro_usd":25,"state":"held"}}}"#,
    )
    .unwrap();
    assert!(matches!(
        LocalLedger::open(&path, 100),
        Err(LedgerError::CorruptState(_))
    ));
}

#[test]
fn persistence_uncertainty_poisons_handle_and_preserves_admission_denial() {
    let scratch = ScratchDir::new("poison");
    let path = scratch.ledger_path();
    let ledger = LocalLedger::open(&path, 100).unwrap();
    ledger.fail_after_rename_once_for_tests();
    assert!(matches!(
        ledger.reserve("uncertain", 60),
        Err(LedgerError::Persistence(_))
    ));
    assert_eq!(ledger.exposure_micro_usd(), u64::MAX);
    assert!(matches!(
        ledger.reserve("later", 1),
        Err(LedgerError::Poisoned(_))
    ));
    drop(ledger);

    let reopened = LocalLedger::open(&path, 100).unwrap();
    assert_eq!(
        reopened.get("uncertain").unwrap().state,
        ReservationState::Held
    );
    assert_eq!(
        reopened.reserve("uncertain", 1),
        Err(LedgerError::DuplicateAttempt("uncertain".into()))
    );
}

#[test]
fn reconciled_exposure_overflow_survives_reopen_and_saturates_fail_closed() {
    let scratch = ScratchDir::new("reconciled-overflow");
    let path = scratch.ledger_path();
    {
        let ledger = LocalLedger::open(&path, u64::MAX).unwrap();
        ledger.reserve("first", 1).unwrap();
        ledger.reserve("second", 1).unwrap();
        ledger.reconcile("first", u64::MAX).unwrap();
        ledger.reconcile("second", u64::MAX).unwrap();
        assert_eq!(ledger.exposure_micro_usd(), u64::MAX);
    }

    let reopened = LocalLedger::open(&path, u64::MAX).unwrap();
    assert_eq!(reopened.exposure_micro_usd(), u64::MAX);
    assert_eq!(
        reopened.get("first").unwrap().settled_micro_usd,
        Some(u64::MAX)
    );
    assert_eq!(
        reopened.get("second").unwrap().settled_micro_usd,
        Some(u64::MAX)
    );
    assert!(matches!(
        reopened.reserve("blocked", 1),
        Err(LedgerError::CapExceeded { .. })
    ));
}

#[test]
fn overage_and_overflow_denials_survive_reopen() {
    let scratch = ScratchDir::new("overage-overflow");
    let path = scratch.ledger_path();
    {
        let ledger = LocalLedger::open(&path, u64::MAX).unwrap();
        ledger.reserve("overbilled", 60).unwrap();
        assert!(matches!(
            ledger.settle("overbilled", 75),
            Err(LedgerError::SettlementExceedsReservation { .. })
        ));
        ledger.reserve("near-max", u64::MAX - 75).unwrap();
        assert!(matches!(
            ledger.reserve("overflow", 1),
            Err(LedgerError::CapExceeded { .. })
        ));
    }

    let reopened = LocalLedger::open(&path, u64::MAX).unwrap();
    assert_eq!(
        reopened.get("overbilled").unwrap().settled_micro_usd,
        Some(75)
    );
    assert_eq!(
        reopened.get("overbilled").unwrap().state,
        ReservationState::Ambiguous
    );
    assert_eq!(reopened.exposure_micro_usd(), u64::MAX);
    assert!(matches!(
        reopened.reserve("still-full", 1),
        Err(LedgerError::CapExceeded { .. })
    ));
}

#[test]
fn preexisting_temp_sentinel_is_never_overwritten() {
    let scratch = ScratchDir::new("temp-sentinel");
    let path = scratch.ledger_path();
    {
        LocalLedger::open(&path, 100).unwrap();
    }
    let temp = PathBuf::from(format!("{}.tmp", path.display()));
    std::fs::write(&temp, b"sentinel").unwrap();
    let ledger = LocalLedger::open(&path, 100).unwrap();
    ledger.reserve("safe", 10).unwrap();
    assert_eq!(std::fs::read(&temp).unwrap(), b"sentinel");
}

#[cfg(unix)]
#[test]
fn preexisting_temp_symlink_sentinel_and_target_are_unchanged() {
    use std::os::unix::fs::symlink;

    let scratch = ScratchDir::new("temp-symlink");
    let path = scratch.ledger_path();
    {
        LocalLedger::open(&path, 100).unwrap();
    }
    let victim = scratch.0.join("victim");
    std::fs::write(&victim, b"do-not-touch").unwrap();
    let temp = PathBuf::from(format!("{}.tmp", path.display()));
    symlink(&victim, &temp).unwrap();

    let ledger = LocalLedger::open(&path, 100).unwrap();
    ledger.reserve("safe", 10).unwrap();
    assert_eq!(std::fs::read(&victim).unwrap(), b"do-not-touch");
    assert!(
        std::fs::symlink_metadata(&temp)
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

#[cfg(unix)]
#[test]
fn dangling_ledger_symlink_is_rejected_without_reset() {
    use std::os::unix::fs::symlink;

    let scratch = ScratchDir::new("dangling-ledger");
    let path = scratch.ledger_path();
    let missing = scratch.0.join("missing-target");
    symlink(&missing, &path).unwrap();
    assert!(matches!(
        LocalLedger::open(&path, 100),
        Err(LedgerError::CorruptState(_))
    ));
    assert!(
        std::fs::symlink_metadata(&path)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(!missing.exists());
}

#[cfg(unix)]
#[test]
fn lock_symlink_is_rejected_without_touching_target() {
    use std::os::unix::fs::symlink;

    let scratch = ScratchDir::new("lock-symlink");
    let path = scratch.ledger_path();
    let lock = PathBuf::from(format!("{}.lock", path.display()));
    let victim = scratch.0.join("lock-victim");
    std::fs::write(&victim, b"do-not-touch").unwrap();
    symlink(&victim, &lock).unwrap();
    assert!(matches!(
        LocalLedger::open(&path, 100),
        Err(LedgerError::CorruptState(_))
    ));
    assert_eq!(std::fs::read(&victim).unwrap(), b"do-not-touch");
}

#[test]
fn stable_relative_path_child() {
    if std::env::var_os("JCODE_LEDGER_RELATIVE_CHILD").is_none() {
        return;
    }
    let first = PathBuf::from(std::env::var_os("JCODE_LEDGER_FIRST_CWD").unwrap());
    let second = PathBuf::from(std::env::var_os("JCODE_LEDGER_SECOND_CWD").unwrap());
    std::env::set_current_dir(&first).unwrap();
    let ledger = LocalLedger::open("ledger.json", 100).unwrap();
    std::env::set_current_dir(&second).unwrap();
    ledger.reserve("after-cwd-change", 40).unwrap();
}

#[test]
fn relative_path_remains_bound_after_cwd_change() {
    let scratch = ScratchDir::new("relative-cwd");
    let first = scratch.0.join("first");
    let second = scratch.0.join("second");
    std::fs::create_dir_all(&first).unwrap();
    std::fs::create_dir_all(&second).unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "ledger_tests::stable_relative_path_child",
            "--nocapture",
        ])
        .env("JCODE_LEDGER_RELATIVE_CHILD", "1")
        .env("JCODE_LEDGER_FIRST_CWD", &first)
        .env("JCODE_LEDGER_SECOND_CWD", &second)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "child failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!second.join("ledger.json").exists());
    let ledger = LocalLedger::open(first.join("ledger.json"), 100).unwrap();
    assert_eq!(
        ledger.get("after-cwd-change").unwrap().reserved_micro_usd,
        40
    );
}

#[test]
fn duplicate_persisted_reservation_keys_are_rejected() {
    let scratch = ScratchDir::new("duplicate-json-key");
    let path = scratch.ledger_path();
    std::fs::write(
        &path,
        br#"{"version":1,"cap_micro_usd":100,"reservations":{"dup":{"attempt_id":"dup","reserved_micro_usd":40,"settled_micro_usd":null,"state":"held"},"dup":{"attempt_id":"dup","reserved_micro_usd":90,"settled_micro_usd":null,"state":"held"}}}"#,
    )
    .unwrap();
    assert!(matches!(
        LocalLedger::open(&path, 100),
        Err(LedgerError::CorruptState(message)) if message.contains("duplicate reservation key")
    ));
}

#[test]
fn durable_empty_attempt_id_is_denied_without_breaking_in_memory_compatibility() {
    let memory = LocalLedger::new(100);
    memory.reserve("", 10).unwrap();

    let scratch = ScratchDir::new("empty-id");
    let path = scratch.ledger_path();
    let durable = LocalLedger::open(&path, 100).unwrap();
    assert_eq!(durable.reserve("", 10), Err(LedgerError::InvalidAttemptId));
    assert_eq!(
        durable.reserve_remaining(""),
        Err(LedgerError::InvalidAttemptId)
    );
    assert_eq!(durable.settle("", 1), Err(LedgerError::InvalidAttemptId));
    assert_eq!(
        durable.mark_ambiguous(""),
        Err(LedgerError::InvalidAttemptId)
    );
    assert_eq!(durable.reconcile("", 1), Err(LedgerError::InvalidAttemptId));
    drop(durable);
    assert!(LocalLedger::open(&path, 100).unwrap().get("").is_none());
}

#[test]
fn poisoned_handle_cannot_race_to_report_reusable_capacity() {
    let scratch = ScratchDir::new("poison-race");
    let path = scratch.ledger_path();
    let ledger = LocalLedger::open(&path, 100).unwrap();
    ledger.pause_and_fail_after_rename_for_tests();
    let writer = {
        let ledger = ledger.clone();
        std::thread::spawn(move || ledger.reserve("uncertain", 60))
    };
    ledger.wait_until_rename_for_tests();
    let reader = {
        let ledger = ledger.clone();
        std::thread::spawn(move || ledger.exposure_micro_usd())
    };
    std::thread::sleep(Duration::from_millis(25));
    ledger.release_after_rename_for_tests();
    assert!(matches!(
        writer.join().unwrap(),
        Err(LedgerError::Persistence(_))
    ));
    assert_eq!(reader.join().unwrap(), u64::MAX);
}
