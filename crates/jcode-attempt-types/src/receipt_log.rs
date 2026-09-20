//! Append-only durable receipt log beside the ledger.
//!
//! One JSON receipt per line. Each append opens the log fresh, refuses
//! symlinks and nonregular paths, writes a single line with `O_APPEND`, and
//! syncs the file before returning. Reading validates every line and refuses
//! duplicate attempt ids, so a partial or forged line is detected rather than
//! silently skipped. Same private trusted-directory assumption as the ledger.

use crate::Receipt;
use std::collections::BTreeSet;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiptLogError {
    NotRegularFile(PathBuf),
    Io {
        path: PathBuf,
        message: String,
    },
    /// A line failed to parse, or an attempt id appears twice.
    Corrupt {
        path: PathBuf,
        line: usize,
        message: String,
    },
    /// A receipt for this attempt id is already recorded.
    DuplicateAttempt(String),
}

impl std::fmt::Display for ReceiptLogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReceiptLogError::NotRegularFile(p) => {
                write!(f, "receipt log is not a regular file: {}", p.display())
            }
            ReceiptLogError::Io { path, message } => {
                write!(f, "receipt log {}: {message}", path.display())
            }
            ReceiptLogError::Corrupt {
                path,
                line,
                message,
            } => write!(
                f,
                "corrupt receipt log {} at line {line}: {message}",
                path.display()
            ),
            ReceiptLogError::DuplicateAttempt(id) => {
                write!(f, "receipt for attempt `{id}` already recorded")
            }
        }
    }
}

impl std::error::Error for ReceiptLogError {}

/// Handle on a receipt log path. Holds no lock; the caller's ledger lock is
/// the intended serialization point for one operator directory.
#[derive(Debug, Clone)]
pub struct ReceiptLog {
    path: PathBuf,
}

impl ReceiptLog {
    /// Bind to a path in a private directory. The parent is resolved once so
    /// a later cwd change cannot redirect appends. The file is created empty
    /// if absent; an existing symlink or nonregular path is refused.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ReceiptLogError> {
        let path = path.as_ref();
        let file_name = path
            .file_name()
            .ok_or_else(|| ReceiptLogError::NotRegularFile(path.to_path_buf()))?;
        let parent = match path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent,
            _ => Path::new("."),
        };
        let parent = std::fs::canonicalize(parent).map_err(|error| ReceiptLogError::Io {
            path: parent.to_path_buf(),
            message: error.to_string(),
        })?;
        let path = parent.join(file_name);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_file() => {}
            Ok(_) => return Err(ReceiptLogError::NotRegularFile(path)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&path)
                    .map_err(|error| ReceiptLogError::Io {
                        path: path.clone(),
                        message: error.to_string(),
                    })?;
            }
            Err(error) => {
                return Err(ReceiptLogError::Io {
                    path,
                    message: error.to_string(),
                });
            }
        }
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Append one receipt. Refuses a duplicate attempt id by reading the log
    /// first, then writes one line and syncs. A line is either fully present
    /// or detected as corrupt on the next read.
    pub fn append(&self, receipt: &Receipt) -> Result<(), ReceiptLogError> {
        let existing = self.read_all()?;
        if existing
            .iter()
            .any(|recorded| recorded.attempt_id == receipt.attempt_id)
        {
            return Err(ReceiptLogError::DuplicateAttempt(
                receipt.attempt_id.clone(),
            ));
        }
        let mut line = serde_json::to_vec(receipt).map_err(|error| ReceiptLogError::Io {
            path: self.path.clone(),
            message: format!("serialize receipt: {error}"),
        })?;
        line.push(b'\n');
        self.require_regular()?;
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.path)
            .map_err(|error| self.io(error))?;
        file.write_all(&line).map_err(|error| self.io(error))?;
        file.sync_all().map_err(|error| self.io(error))
    }

    /// Every recorded receipt in order. Any unparsable line or duplicate
    /// attempt id is an error, never skipped.
    pub fn read_all(&self) -> Result<Vec<Receipt>, ReceiptLogError> {
        self.require_regular()?;
        let bytes = std::fs::read(&self.path).map_err(|error| self.io(error))?;
        let mut receipts = Vec::new();
        let mut seen = BTreeSet::new();
        for (index, raw) in bytes.split(|byte| *byte == b'\n').enumerate() {
            if raw.is_empty() {
                continue;
            }
            let receipt: Receipt =
                serde_json::from_slice(raw).map_err(|error| ReceiptLogError::Corrupt {
                    path: self.path.clone(),
                    line: index + 1,
                    message: error.to_string(),
                })?;
            if !seen.insert(receipt.attempt_id.clone()) {
                return Err(ReceiptLogError::Corrupt {
                    path: self.path.clone(),
                    line: index + 1,
                    message: format!("duplicate attempt id `{}`", receipt.attempt_id),
                });
            }
            receipts.push(receipt);
        }
        if bytes.last().is_some_and(|byte| *byte != b'\n') {
            return Err(ReceiptLogError::Corrupt {
                path: self.path.clone(),
                line: receipts.len(),
                message: "final line is not newline-terminated".into(),
            });
        }
        Ok(receipts)
    }

    fn require_regular(&self) -> Result<(), ReceiptLogError> {
        let metadata = std::fs::symlink_metadata(&self.path).map_err(|error| self.io(error))?;
        if !metadata.file_type().is_file() {
            return Err(ReceiptLogError::NotRegularFile(self.path.clone()));
        }
        Ok(())
    }

    fn io(&self, error: std::io::Error) -> ReceiptLogError {
        ReceiptLogError::Io {
            path: self.path.clone(),
            message: error.to_string(),
        }
    }
}

#[cfg(test)]
#[path = "receipt_log_tests.rs"]
mod tests;
