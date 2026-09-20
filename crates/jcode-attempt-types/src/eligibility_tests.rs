use super::*;
use crate::{AttemptRecord, Effort, LocalBudget, RouteClass};
use chrono::Utc;
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
            "jcode-attempt-eligibility-{label}-{}-{nonce}-{id}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("create scratch directory");
        Self(std::fs::canonicalize(&path).expect("canonical scratch"))
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn policy_for(scratch: &ScratchDir) -> DataClassPolicy {
    DataClassPolicy {
        version: "test".into(),
        roots: vec![
            crate::DataClassRoot {
                path: scratch.0.join("public"),
                class: DataClass::Public,
                reason: "fixture".into(),
            },
            crate::DataClassRoot {
                path: scratch.0.join("public").join("secret.env"),
                class: DataClass::Secret,
                reason: "fixture".into(),
            },
        ],
    }
}

fn frozen_with(class: DataClass) -> FrozenAttempt {
    AttemptRecord {
        task_id: "t".into(),
        attempt_id: "t/n/a".into(),
        node_id: "n".into(),
        provider: "openrouter".into(),
        model_exact: "approved/model".into(),
        endpoint: "loopback".into(),
        route_class: RouteClass::IncludedSubscription,
        effort: Effort::Low,
        tool_allowlist: vec![],
        data_class: class,
        router: None,
        deadline_secs: 1,
        budget: LocalBudget::default(),
        prompt_hash: "p".repeat(64),
        policy_version: "test".into(),
    }
    .freeze(Utc::now())
    .unwrap()
}

fn file(path: &Path) -> OutboundPart {
    OutboundPart::File {
        path: path.to_path_buf(),
    }
}

#[test]
fn literal_only_packet_carries_its_declared_class() {
    let parts = [OutboundPart::Literal {
        class: DataClass::Synthetic,
        text: "fixture prompt".into(),
    }];
    let packet = OutboundPacket::assemble(None, &parts, 1024).unwrap();
    assert_eq!(packet.text(), "fixture prompt");
    assert_eq!(packet.data_class(), DataClass::Synthetic);
    assert!(packet.input_paths().is_empty());
    assert!(
        packet
            .check_frozen(&frozen_with(DataClass::Synthetic))
            .is_ok()
    );
    assert!(
        packet
            .check_frozen(&frozen_with(DataClass::Private))
            .is_ok()
    );
    assert!(matches!(
        packet.check_frozen(&frozen_with(DataClass::Public)),
        Err(EligibilityError::ClassExceedsFrozen {
            packet: DataClass::Synthetic,
            frozen: DataClass::Public
        })
    ));
}

#[test]
fn undeclared_file_makes_the_packet_private() {
    let scratch = ScratchDir::new("undeclared");
    let policy = policy_for(&scratch);
    let public = scratch.0.join("public");
    std::fs::create_dir_all(&public).unwrap();
    std::fs::write(public.join("a.rs"), "pub fn a() {}").unwrap();
    let elsewhere = scratch.0.join("elsewhere.md");
    std::fs::write(&elsewhere, "client notes").unwrap();

    let only_public = [file(&public.join("a.rs"))];
    let packet = OutboundPacket::assemble(Some(&policy), &only_public, 1024).unwrap();
    assert_eq!(packet.data_class(), DataClass::Public);
    assert_eq!(packet.text(), "pub fn a() {}");

    let mixed = [file(&public.join("a.rs")), file(&elsewhere)];
    let packet = OutboundPacket::assemble(Some(&policy), &mixed, 1024).unwrap();
    assert_eq!(packet.data_class(), DataClass::Private);
    assert_eq!(packet.text(), "pub fn a() {}\nclient notes");
    assert!(matches!(
        packet.check_frozen(&frozen_with(DataClass::Public)),
        Err(EligibilityError::ClassExceedsFrozen { .. })
    ));

    let no_policy = OutboundPacket::assemble(None, &only_public, 1024).unwrap();
    assert_eq!(no_policy.data_class(), DataClass::Private);
}

#[cfg(unix)]
#[test]
fn symlink_inside_a_public_root_is_refused_without_reading_its_target() {
    use std::os::unix::fs::symlink;

    let scratch = ScratchDir::new("symlink");
    let policy = policy_for(&scratch);
    let public = scratch.0.join("public");
    std::fs::create_dir_all(&public).unwrap();
    let private = scratch.0.join("private.txt");
    std::fs::write(&private, "do not export").unwrap();
    let link = public.join("looks-public.txt");
    symlink(&private, &link).unwrap();

    assert!(matches!(
        OutboundPacket::assemble(Some(&policy), &[file(&link)], 1024),
        Err(EligibilityError::NotRegularFile(p)) if p == link
    ));
    assert!(matches!(
        OutboundPacket::assemble(Some(&policy), &[file(&public)], 1024),
        Err(EligibilityError::NotRegularFile(_))
    ));
}

#[test]
fn secret_root_and_secret_shapes_are_refused() {
    let scratch = ScratchDir::new("secret");
    let policy = policy_for(&scratch);
    let public = scratch.0.join("public");
    std::fs::create_dir_all(&public).unwrap();
    let env = public.join("secret.env");
    std::fs::write(&env, "PLAIN=1").unwrap();
    assert!(matches!(
        OutboundPacket::assemble(Some(&policy), &[file(&env)], 1024),
        Err(EligibilityError::SecretPart(p)) if p == env
    ));

    let leaky = public.join("leaky.rs");
    std::fs::write(
        &leaky,
        "const K: &str = \"sk-or-v1-abcdefghijklmnopqrstuvwxyz0123456789\";",
    )
    .unwrap();
    assert!(matches!(
        OutboundPacket::assemble(Some(&policy), &[file(&leaky)], 1024),
        Err(EligibilityError::SecretShapes(found)) if !found.is_empty()
    ));

    let literal = [OutboundPart::Literal {
        class: DataClass::Secret,
        text: "x".into(),
    }];
    assert!(matches!(
        OutboundPacket::assemble(None, &literal, 1024),
        Err(EligibilityError::SecretPart(_))
    ));
}

#[test]
fn size_and_encoding_bounds_are_enforced() {
    let scratch = ScratchDir::new("bounds");
    let policy = policy_for(&scratch);
    let public = scratch.0.join("public");
    std::fs::create_dir_all(&public).unwrap();
    let big = public.join("big.txt");
    std::fs::write(&big, "x".repeat(64)).unwrap();
    assert!(matches!(
        OutboundPacket::assemble(Some(&policy), &[file(&big)], 32),
        Err(EligibilityError::FileTooLarge {
            bytes: 64,
            max: 32,
            ..
        })
    ));
    let small = public.join("small.txt");
    std::fs::write(&small, "x".repeat(20)).unwrap();
    assert!(matches!(
        OutboundPacket::assemble(Some(&policy), &[file(&small), file(&small)], 32),
        Err(EligibilityError::PacketTooLarge { bytes: 41, max: 32 })
    ));
    let binary = public.join("bin");
    std::fs::write(&binary, [0xff, 0xfe, 0x00]).unwrap();
    assert!(matches!(
        OutboundPacket::assemble(Some(&policy), &[file(&binary)], 32),
        Err(EligibilityError::NotUtf8(_))
    ));
    let missing = public.join("missing");
    assert!(matches!(
        OutboundPacket::assemble(Some(&policy), &[file(&missing)], 32),
        Err(EligibilityError::Read { .. })
    ));
}
