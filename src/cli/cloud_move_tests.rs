use super::*;

struct TestHome {
    _dir: tempfile::TempDir,
    previous: Option<std::ffi::OsString>,
}

impl TestHome {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let previous = std::env::var_os("JCODE_HOME");
        crate::env::set_var("JCODE_HOME", dir.path());
        Self {
            _dir: dir,
            previous,
        }
    }
}

impl Drop for TestHome {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => crate::env::set_var("JCODE_HOME", value),
            None => crate::env::remove_var("JCODE_HOME"),
        }
    }
}

#[test]
fn human_titles_are_resolved_before_migration_storage_id_validation() {
    let _lock = storage::lock_test_env();
    let _home = TestHome::new();
    let id = "session_cloud_lease_title";
    let mut session = session::Session::create_with_id(id.into(), None, None);
    session.rename_title(Some("Migration title with spaces".into()));
    session.save_prepared().unwrap();
    assert_eq!(
        resolve_session_id(Some("Migration title with spaces")).unwrap(),
        id
    );
}

#[test]
fn corrupt_leases_stop_cloud_ownership_operations_before_transport() {
    let _lock = storage::lock_test_env();
    let _home = TestHome::new();
    let id = "session_cloud_corrupt_lease";
    let mut session = session::Session::create_with_id(id.into(), None, None);
    session.save_prepared().unwrap();
    let snapshot = session::session_path(id).unwrap();
    let before = std::fs::read(&snapshot).unwrap();
    let dir = storage::session_leases_dir().unwrap();
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{id}.json"));
    let corrupt = b"{\"session_id\":";
    std::fs::write(&path, corrupt).unwrap();
    let target = Target {
        host: Some("fixture-host".into()),
        transport: Some("/nonexistent/jcode-lease-test-transport".into()),
        ..Target::default()
    };
    let errors = [
        run_move(Some(id), &target, true, false).unwrap_err(),
        run_return(Some(id), &target, false).unwrap_err(),
        run_export(id, 9).unwrap_err(),
        run_activate(id, 9).unwrap_err(),
        away_host(Some(id)).unwrap_err(),
    ];
    for error in errors {
        assert!(format!("{error:#}").contains("decode migration lease"));
    }
    assert_eq!(std::fs::read(&snapshot).unwrap(), before);
    assert_eq!(std::fs::read(&path).unwrap(), corrupt);
}
