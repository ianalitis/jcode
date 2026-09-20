use super::*;
use crate::ReceiptKind;
use chrono::{TimeZone, Utc};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

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
            "jcode-attempt-receipts-{label}-{}-{nonce}-{id}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("create scratch directory");
        Self(path)
    }

    fn log_path(&self) -> PathBuf {
        self.0.join("receipts.jsonl")
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn receipt(attempt_id: &str) -> Receipt {
    let started = Utc.with_ymd_and_hms(2026, 9, 20, 3, 0, 0).unwrap();
    Receipt {
        attempt_id: attempt_id.into(),
        kind: ReceiptKind::ModelCall,
        cmd: "single-send openrouter approved/model".into(),
        argv_hash: "b".repeat(64),
        cwd: "http://127.0.0.1:1/v1/chat/completions".into(),
        exit_code: Some(0),
        stdout_sha256: "c".repeat(64),
        stderr_sha256: "d".repeat(64),
        started,
        finished: started + chrono::Duration::seconds(5),
        binary_id: "openrouter:approved/model".into(),
        usage: None,
        effective_telemetry: BTreeMap::new(),
        task_type: None,
    }
}

#[test]
fn appends_survive_reopen_in_order_and_duplicates_are_refused() {
    let scratch = ScratchDir::new("reopen");
    let log = ReceiptLog::open(scratch.log_path()).unwrap();
    log.append(&receipt("a")).unwrap();
    log.append(&receipt("b")).unwrap();
    assert_eq!(
        log.append(&receipt("a")),
        Err(ReceiptLogError::DuplicateAttempt("a".into()))
    );

    let reopened = ReceiptLog::open(scratch.log_path()).unwrap();
    let all = reopened.read_all().unwrap();
    assert_eq!(all, vec![receipt("a"), receipt("b")]);
    assert_eq!(
        reopened.append(&receipt("b")),
        Err(ReceiptLogError::DuplicateAttempt("b".into()))
    );
}

#[test]
fn truncated_or_forged_lines_are_reported_not_skipped() {
    let scratch = ScratchDir::new("corrupt");
    let log = ReceiptLog::open(scratch.log_path()).unwrap();
    log.append(&receipt("a")).unwrap();

    let good = std::fs::read(scratch.log_path()).unwrap();
    let mut torn = good.clone();
    torn.extend_from_slice(b"{\"attempt_id\":\"b\",\"kind\":\"model_call\"");
    std::fs::write(scratch.log_path(), &torn).unwrap();
    assert!(matches!(
        log.read_all(),
        Err(ReceiptLogError::Corrupt { line: 2, .. })
    ));
    assert!(matches!(
        log.append(&receipt("c")),
        Err(ReceiptLogError::Corrupt { .. })
    ));
    assert_eq!(
        std::fs::read(scratch.log_path()).unwrap(),
        torn,
        "a corrupt log is never rewritten"
    );

    let mut duplicated = good.clone();
    duplicated.extend_from_slice(&good);
    std::fs::write(scratch.log_path(), &duplicated).unwrap();
    assert!(matches!(
        log.read_all(),
        Err(ReceiptLogError::Corrupt { line: 2, message, .. }) if message.contains("duplicate")
    ));

    let mut unterminated = good.clone();
    unterminated.pop();
    std::fs::write(scratch.log_path(), &unterminated).unwrap();
    assert!(matches!(
        log.read_all(),
        Err(ReceiptLogError::Corrupt { message, .. }) if message.contains("newline")
    ));
}

#[cfg(unix)]
#[test]
fn symlinked_log_is_refused_without_touching_its_target() {
    use std::os::unix::fs::symlink;

    let scratch = ScratchDir::new("symlink");
    let victim = scratch.0.join("victim");
    std::fs::write(&victim, b"do-not-touch").unwrap();
    symlink(&victim, scratch.log_path()).unwrap();
    assert!(matches!(
        ReceiptLog::open(scratch.log_path()),
        Err(ReceiptLogError::NotRegularFile(_))
    ));

    let ok = ScratchDir::new("swapped");
    let log = ReceiptLog::open(ok.log_path()).unwrap();
    std::fs::remove_file(ok.log_path()).unwrap();
    symlink(&victim, ok.log_path()).unwrap();
    assert!(matches!(
        log.append(&receipt("a")),
        Err(ReceiptLogError::NotRegularFile(_))
    ));
    assert_eq!(std::fs::read(&victim).unwrap(), b"do-not-touch");
}

#[test]
fn relative_path_is_bound_to_the_directory_at_open() {
    let scratch = ScratchDir::new("relative");
    let canonical = std::fs::canonicalize(&scratch.0).unwrap();
    let log = ReceiptLog::open(scratch.log_path()).unwrap();
    assert_eq!(log.path(), canonical.join("receipts.jsonl"));
    assert!(log.path().is_absolute());
}
