//! Explicit outbound eligibility of packet bytes.
//!
//! A frozen attempt carries a `data_class`, but nothing so far proves that the
//! bytes actually handed to the caller were derived only from content of that
//! class. [`OutboundPacket::assemble`] builds the text from declared parts:
//! literals with an explicit class and files classified through the operator's
//! [`DataClassPolicy`]. Undeclared paths are Private, symlinks and oversize
//! files are refused, and the assembled text is scanned for secret shapes. The
//! resulting class must be no wider than the frozen attempt's declared class.

use crate::{DataClass, DataClassPolicy, FrozenAttempt, SecretShape, find_secret_shapes};
use std::path::{Path, PathBuf};

/// One declared input to the packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutboundPart {
    /// Text the captain authored. Its class is declared, never inferred.
    Literal { class: DataClass, text: String },
    /// A file whose bytes are read here and classified through the policy.
    File { path: PathBuf },
}

/// Text proven to carry a single data class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboundPacket {
    text: String,
    data_class: DataClass,
    input_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EligibilityError {
    /// A file part is a symlink, directory or otherwise not a regular file.
    NotRegularFile(PathBuf),
    Read {
        path: PathBuf,
        message: String,
    },
    NotUtf8(PathBuf),
    FileTooLarge {
        path: PathBuf,
        bytes: u64,
        max: u64,
    },
    PacketTooLarge {
        bytes: u64,
        max: u64,
    },
    /// Secret-class content never leaves the process, whatever the route.
    SecretPart(PathBuf),
    SecretShapes(Vec<SecretShape>),
    /// The packet is more restrictive than the attempt admits.
    ClassExceedsFrozen {
        packet: DataClass,
        frozen: DataClass,
    },
}

impl std::fmt::Display for EligibilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EligibilityError::NotRegularFile(p) => {
                write!(f, "packet input is not a regular file: {}", p.display())
            }
            EligibilityError::Read { path, message } => {
                write!(f, "read packet input {}: {message}", path.display())
            }
            EligibilityError::NotUtf8(p) => {
                write!(f, "packet input is not UTF-8: {}", p.display())
            }
            EligibilityError::FileTooLarge { path, bytes, max } => write!(
                f,
                "packet input {} is {bytes} bytes, above {max}",
                path.display()
            ),
            EligibilityError::PacketTooLarge { bytes, max } => {
                write!(f, "packet is {bytes} bytes, above {max}")
            }
            EligibilityError::SecretPart(p) => {
                write!(f, "packet input is secret-class: {}", p.display())
            }
            EligibilityError::SecretShapes(found) => {
                write!(f, "packet contains {} secret-shaped value(s)", found.len())
            }
            EligibilityError::ClassExceedsFrozen { packet, frozen } => write!(
                f,
                "packet data class {packet:?} is more restrictive than frozen {frozen:?}"
            ),
        }
    }
}

impl std::error::Error for EligibilityError {}

impl OutboundPacket {
    /// Assemble parts in order, separated by a newline. `max_bytes` bounds
    /// each file and the whole packet. Literal-only packets with no policy
    /// are allowed; file parts without a policy are Private.
    pub fn assemble(
        policy: Option<&DataClassPolicy>,
        parts: &[OutboundPart],
        max_bytes: u64,
    ) -> Result<Self, EligibilityError> {
        let mut text = String::new();
        let mut data_class = DataClass::Public;
        let mut input_paths = Vec::new();
        for part in parts {
            let (class, piece) = match part {
                OutboundPart::Literal { class, text } => (*class, text.clone()),
                OutboundPart::File { path } => {
                    let class = policy
                        .map(|policy| policy.classify(path))
                        .unwrap_or_default();
                    if class == DataClass::Secret {
                        return Err(EligibilityError::SecretPart(path.clone()));
                    }
                    input_paths.push(path.clone());
                    (class, read_regular_utf8(path, max_bytes)?)
                }
            };
            if class == DataClass::Secret {
                return Err(EligibilityError::SecretPart(PathBuf::from("<literal>")));
            }
            data_class = data_class.max(class);
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&piece);
            if text.len() as u64 > max_bytes {
                return Err(EligibilityError::PacketTooLarge {
                    bytes: text.len() as u64,
                    max: max_bytes,
                });
            }
        }
        let found = find_secret_shapes(&serde_json::Value::String(text.clone()));
        if !found.is_empty() {
            return Err(EligibilityError::SecretShapes(found));
        }
        Ok(Self {
            text,
            data_class,
            input_paths,
        })
    }

    /// Refuse unless the frozen attempt declared a class at least as
    /// restrictive as what the bytes actually carry.
    pub fn check_frozen(&self, attempt: &FrozenAttempt) -> Result<(), EligibilityError> {
        let frozen = attempt.record().data_class;
        if self.data_class > frozen {
            return Err(EligibilityError::ClassExceedsFrozen {
                packet: self.data_class,
                frozen,
            });
        }
        Ok(())
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn data_class(&self) -> DataClass {
        self.data_class
    }

    pub fn input_paths(&self) -> &[PathBuf] {
        &self.input_paths
    }
}

fn read_regular_utf8(path: &Path, max_bytes: u64) -> Result<String, EligibilityError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| EligibilityError::Read {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    if !metadata.file_type().is_file() {
        return Err(EligibilityError::NotRegularFile(path.to_path_buf()));
    }
    if metadata.len() > max_bytes {
        return Err(EligibilityError::FileTooLarge {
            path: path.to_path_buf(),
            bytes: metadata.len(),
            max: max_bytes,
        });
    }
    let bytes = std::fs::read(path).map_err(|error| EligibilityError::Read {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    if bytes.len() as u64 > max_bytes {
        return Err(EligibilityError::FileTooLarge {
            path: path.to_path_buf(),
            bytes: bytes.len() as u64,
            max: max_bytes,
        });
    }
    String::from_utf8(bytes).map_err(|_| EligibilityError::NotUtf8(path.to_path_buf()))
}

#[cfg(test)]
#[path = "eligibility_tests.rs"]
mod tests;
