//! Per-machine migration leases for sessions moved between machines
//! (`jcode cloud move` / `jcode cloud return`).
//!
//! A conversation is a single linear transcript, so exactly one machine may run
//! turns for it at a time. Each machine keeps `~/.jcode/session_leases/<id>.json`
//! recording the latest migration `epoch` it knows about and whether the session
//! currently lives here or on another host. A session copy carries the epoch of
//! the migration that delivered it (`Session::migration_epoch`). A copy may run
//! turns and persist only when the lease says the session lives here and the
//! copy is at least as new as the lease. That blocks two failure modes:
//!
//! - the old machine's in-memory agent continuing after the session moved, and
//! - a stale in-memory agent overwriting a freshly returned transcript.
//!
//! Sessions that never migrated have no lease file and are unaffected.

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::jcode_dir;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionLease {
    pub session_id: String,
    /// Monotonic migration counter shared by both machines.
    pub epoch: u64,
    /// `None` when the session lives on this machine, otherwise the host that
    /// currently owns it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub away_host: Option<String>,
    /// Absolute repository root the session works in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo_root: Option<String>,
    /// Snapshot commit both machines last agreed on. Used as the merge base
    /// when work comes back.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_sync_snapshot: Option<String>,
    /// Branch HEAD both machines last agreed on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_sync_head: Option<String>,
    pub updated_at: String,
}

impl SessionLease {
    pub fn is_here(&self) -> bool {
        self.away_host.is_none()
    }
}

/// Why a session copy may not run a turn or persist on this machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionLeaseBlock {
    MovedAway { host: String, epoch: u64 },
    StaleCopy { copy_epoch: u64, lease_epoch: u64 },
    UnavailableOrInvalid,
}

impl std::fmt::Display for SessionLeaseBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnavailableOrInvalid => write!(
                f,
                "the migration lease could not be verified; inspect its storage and contents before retrying"
            ),
            Self::MovedAway { host, .. } => write!(
                f,
                "this session moved to `{host}`. Attach there with `jcode cloud attach`, or bring it back with `jcode cloud return`"
            ),
            Self::StaleCopy {
                copy_epoch,
                lease_epoch,
            } => write!(
                f,
                "this in-memory copy of the session is stale (migration epoch {copy_epoch} < {lease_epoch}); reopen the session to load the current transcript"
            ),
        }
    }
}

pub fn session_leases_dir() -> Result<PathBuf> {
    Ok(jcode_dir()?.join("session_leases"))
}

fn lease_path(session_id: &str) -> Result<PathBuf> {
    if session_id.is_empty()
        || !session_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b))
        || session_id.starts_with('.')
    {
        bail!("invalid session id for migration lease");
    }
    Ok(session_leases_dir()?.join(format!("{session_id}.json")))
}

pub fn read_session_lease(session_id: &str) -> Result<Option<SessionLease>> {
    let path = lease_path(session_id)?;
    let raw = match std::fs::read(&path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // A dangling link (including a parent link) is not evidence that
            // this session never migrated. Confirm absence without repairing
            // or removing any lease evidence. This is not a filesystem lock.
            match std::fs::symlink_metadata(&path) {
                Ok(_) => return Err(error).context("migration lease target is unavailable"),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error).context("inspect migration lease"),
            }
            for ancestor in path.ancestors().skip(1) {
                match std::fs::symlink_metadata(ancestor) {
                    Ok(_) => {
                        ensure!(
                            std::fs::metadata(ancestor)
                                .context("inspect migration lease parent")?
                                .is_dir(),
                            "migration lease parent is not a directory"
                        );
                        return Ok(None);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error).context("inspect migration lease parent"),
                }
            }
            bail!("could not confirm migration lease absence");
        }
        Err(error) => return Err(error).context("read migration lease"),
    };
    let lease: SessionLease = serde_json::from_slice(&raw).context("decode migration lease")?;
    ensure!(
        lease.session_id == session_id,
        "migration lease session id mismatch"
    );
    Ok(Some(lease))
}

pub fn write_session_lease(lease: &SessionLease) -> anyhow::Result<()> {
    let path = lease_path(&lease.session_id)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(lease)?)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

pub fn remove_session_lease(session_id: &str) -> Result<()> {
    match std::fs::remove_file(lease_path(session_id)?) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).context("remove migration lease"),
    }
}

/// Check whether a session copy at `copy_epoch` may run turns / persist here.
/// Genuine absence permits never-migrated sessions; unreadable evidence blocks.
pub fn session_lease_block(session_id: &str, copy_epoch: u64) -> Option<SessionLeaseBlock> {
    let lease = match read_session_lease(session_id) {
        Ok(lease) => lease?,
        Err(_) => return Some(SessionLeaseBlock::UnavailableOrInvalid),
    };
    if let Some(host) = lease.away_host {
        return Some(SessionLeaseBlock::MovedAway {
            host,
            epoch: lease.epoch,
        });
    }
    (copy_epoch < lease.epoch).then_some(SessionLeaseBlock::StaleCopy {
        copy_epoch,
        lease_epoch: lease.epoch,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_home<T>(f: impl FnOnce() -> T) -> T {
        let _guard = crate::test_jcode_home_lock();
        let temp = tempfile::tempdir().unwrap();
        jcode_core::env::set_var("JCODE_HOME", temp.path());
        let out = f();
        jcode_core::env::remove_var("JCODE_HOME");
        out
    }

    fn lease(epoch: u64, away: Option<&str>) -> SessionLease {
        SessionLease {
            session_id: "session_x".into(),
            epoch,
            away_host: away.map(str::to_string),
            repo_root: None,
            last_sync_snapshot: None,
            last_sync_head: None,
            updated_at: "now".into(),
        }
    }

    #[test]
    fn no_lease_never_blocks() {
        with_home(|| assert_eq!(session_lease_block("session_x", 0), None));
    }

    #[test]
    fn optional_and_unknown_fields_remain_compatible() {
        with_home(|| {
            let path = lease_path("session_x").unwrap();
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(
                path,
                br#"{"session_id":"session_x","epoch":4,"updated_at":"now","future_field":true}"#,
            )
            .unwrap();
            assert_eq!(
                read_session_lease("session_x").unwrap(),
                Some(lease(4, None))
            );
            assert_eq!(session_lease_block("session_x", 4), None);
        });
    }

    #[test]
    fn moved_away_blocks_every_copy() {
        with_home(|| {
            write_session_lease(&lease(3, Some("cloud"))).unwrap();
            assert!(matches!(
                session_lease_block("session_x", 3),
                Some(SessionLeaseBlock::MovedAway { .. })
            ));
        });
    }

    #[test]
    fn returned_session_blocks_only_stale_copies() {
        with_home(|| {
            write_session_lease(&lease(4, None)).unwrap();
            assert_eq!(
                session_lease_block("session_x", 2),
                Some(SessionLeaseBlock::StaleCopy {
                    copy_epoch: 2,
                    lease_epoch: 4
                })
            );
            assert_eq!(session_lease_block("session_x", 4), None);
            assert_eq!(session_lease_block("session_x", 5), None);
        });
    }

    #[test]
    fn rejects_path_like_ids() {
        with_home(|| {
            assert!(read_session_lease("../x").is_err());
            assert!(
                write_session_lease(&SessionLease {
                    session_id: "../x".into(),
                    ..lease(1, None)
                })
                .is_err()
            );
        });
    }

    #[test]
    fn invalid_or_unreadable_leases_block_without_rewriting_evidence() {
        let blocked = with_home(|| {
            let path = lease_path("session_x").unwrap();
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut blocked = Vec::new();
            for bytes in [b"not json".as_slice(), b"{\"session_id\":"] {
                std::fs::write(&path, bytes).unwrap();
                blocked.push(session_lease_block("session_x", 99).is_some());
                assert_eq!(std::fs::read(&path).unwrap(), bytes);
            }
            let mut wrong = lease(1, None);
            wrong.session_id = "session_other".into();
            let bytes = serde_json::to_vec(&wrong).unwrap();
            std::fs::write(&path, &bytes).unwrap();
            blocked.push(session_lease_block("session_x", 99).is_some());
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
            std::fs::remove_file(&path).unwrap();
            std::fs::create_dir(&path).unwrap();
            blocked.push(session_lease_block("session_x", 99).is_some());
            assert!(path.is_dir());
            blocked
        });
        assert_eq!(blocked, vec![true; 4]);
    }

    #[test]
    fn invalid_gate_ids_are_not_treated_as_never_migrated() {
        let blocked = with_home(|| {
            ["", "../x", ".", "..", "a/b", "a\\b", "a b"]
                .map(|id| session_lease_block(id, 0).is_some())
        });
        assert!(blocked.into_iter().all(|blocked| blocked));
    }

    #[cfg(unix)]
    #[test]
    fn dangling_lease_and_parent_symlinks_are_not_confirmed_absence() {
        let blocked = with_home(|| {
            let path = lease_path("session_x").unwrap();
            let parent = path.parent().unwrap();
            std::fs::create_dir_all(parent).unwrap();
            std::os::unix::fs::symlink(parent.join("missing"), &path).unwrap();
            let file_blocked = session_lease_block("session_x", 0).is_some();
            std::fs::remove_file(&path).unwrap();
            std::fs::remove_dir(parent).unwrap();
            std::os::unix::fs::symlink(parent.with_extension("missing"), parent).unwrap();
            [file_blocked, session_lease_block("session_x", 0).is_some()]
        });
        assert_eq!(blocked, [true; 2]);
    }
}
