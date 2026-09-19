//! Per-path data-class admission (J1 of `docs/plans/TOKEN_ECONOMY_PLAN.md`).
//!
//! Working-tree content defaults to [`DataClass::Private`], which keeps it off
//! metered remote routes. The operator may declare roots whose content is
//! already public (an open-source checkout) or synthetic (fixtures) so that
//! packets derived from them become eligible for the cheap lane. Nothing here
//! widens eligibility by itself: admission still runs
//! [`DataClass::is_remote_eligible`] and the route table's promotion scope.

use crate::DataClass;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

/// One declared root and the class its content carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataClassRoot {
    /// Absolute path. Relative paths and `..` segments are rejected at load.
    pub path: PathBuf,
    pub class: DataClass,
    /// Why this root carries this class (review note, issue, or receipt).
    #[serde(default)]
    pub reason: String,
}

/// The operator's declared roots. Undeclared paths are Private.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DataClassPolicy {
    #[serde(default)]
    pub version: String,
    #[serde(default, rename = "data_class")]
    pub roots: Vec<DataClassRoot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataClassPolicyError {
    Parse(String),
    Io(String),
    RelativeRoot(PathBuf),
    UnnormalizedRoot(PathBuf),
    DuplicateRoot(PathBuf),
}

impl std::fmt::Display for DataClassPolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataClassPolicyError::Parse(e) => write!(f, "data class policy parse error: {e}"),
            DataClassPolicyError::Io(e) => write!(f, "data class policy io error: {e}"),
            DataClassPolicyError::RelativeRoot(p) => {
                write!(f, "data class root must be absolute: {}", p.display())
            }
            DataClassPolicyError::UnnormalizedRoot(p) => {
                write!(
                    f,
                    "data class root must not contain `.` or `..`: {}",
                    p.display()
                )
            }
            DataClassPolicyError::DuplicateRoot(p) => {
                write!(f, "data class root declared twice: {}", p.display())
            }
        }
    }
}

impl std::error::Error for DataClassPolicyError {}

fn is_normalized_absolute(path: &Path) -> Result<(), DataClassPolicyError> {
    if !path.is_absolute() {
        return Err(DataClassPolicyError::RelativeRoot(path.to_path_buf()));
    }
    if path
        .components()
        .any(|c| matches!(c, Component::CurDir | Component::ParentDir))
    {
        return Err(DataClassPolicyError::UnnormalizedRoot(path.to_path_buf()));
    }
    Ok(())
}

impl DataClassPolicy {
    pub fn from_toml_str(s: &str) -> Result<Self, DataClassPolicyError> {
        let policy: Self =
            toml::from_str(s).map_err(|e| DataClassPolicyError::Parse(e.to_string()))?;
        policy.validate()?;
        Ok(policy)
    }

    pub fn load_from_path(path: &Path) -> Result<Self, DataClassPolicyError> {
        let text =
            std::fs::read_to_string(path).map_err(|e| DataClassPolicyError::Io(e.to_string()))?;
        Self::from_toml_str(&text)
    }

    pub fn validate(&self) -> Result<(), DataClassPolicyError> {
        let mut seen = std::collections::BTreeSet::new();
        for root in &self.roots {
            is_normalized_absolute(&root.path)?;
            if !seen.insert(root.path.clone()) {
                return Err(DataClassPolicyError::DuplicateRoot(root.path.clone()));
            }
        }
        Ok(())
    }

    /// Class of the content at `path`. The longest declared root that
    /// contains `path` wins, so a `Secret` or `Private` subtree inside a
    /// `Public` checkout stays protected. Undeclared, relative or
    /// unnormalized paths are Private.
    pub fn classify(&self, path: &Path) -> DataClass {
        if is_normalized_absolute(path).is_err() {
            return DataClass::Private;
        }
        self.roots
            .iter()
            .filter(|root| path.starts_with(&root.path))
            .max_by_key(|root| root.path.components().count())
            .map(|root| root.class)
            .unwrap_or_default()
    }

    /// Most restrictive class across several paths: one private file makes
    /// the whole packet private.
    pub fn classify_all<'a>(&self, paths: impl IntoIterator<Item = &'a Path>) -> DataClass {
        paths
            .into_iter()
            .map(|p| self.classify(p))
            .max()
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> DataClassPolicy {
        DataClassPolicy::from_toml_str(
            r#"
version = "2026-09-19"

[[data_class]]
path = "/home/op/src/jcode"
class = "public"
reason = "open-source fork"

[[data_class]]
path = "/home/op/src/jcode/.env"
class = "secret"

[[data_class]]
path = "/home/op/src/jcode/tests/fixtures"
class = "synthetic"
"#,
        )
        .unwrap()
    }

    #[test]
    fn undeclared_paths_are_private() {
        let p = policy();
        assert_eq!(
            p.classify(Path::new("/home/op/clients/acme/src")),
            DataClass::Private
        );
        assert_eq!(
            DataClassPolicy::default().classify(Path::new("/anything")),
            DataClass::Private
        );
    }

    #[test]
    fn declared_root_classifies_its_subtree_and_longest_root_wins() {
        let p = policy();
        assert_eq!(
            p.classify(Path::new("/home/op/src/jcode/src/lib.rs")),
            DataClass::Public
        );
        assert_eq!(
            p.classify(Path::new("/home/op/src/jcode")),
            DataClass::Public
        );
        assert_eq!(
            p.classify(Path::new("/home/op/src/jcode/tests/fixtures/a.json")),
            DataClass::Synthetic
        );
        assert_eq!(
            p.classify(Path::new("/home/op/src/jcode/.env")),
            DataClass::Secret
        );
        // Sibling with a shared prefix string is not inside the root.
        assert_eq!(
            p.classify(Path::new("/home/op/src/jcode-private/x")),
            DataClass::Private
        );
    }

    #[test]
    fn relative_and_unnormalized_paths_never_gain_a_public_class() {
        let p = policy();
        assert_eq!(p.classify(Path::new("src/lib.rs")), DataClass::Private);
        assert_eq!(
            p.classify(Path::new("/home/op/src/jcode/../clients/x")),
            DataClass::Private
        );
    }

    #[test]
    fn a_packet_takes_its_most_restrictive_member() {
        let p = policy();
        let paths = [
            Path::new("/home/op/src/jcode/src/lib.rs"),
            Path::new("/home/op/clients/acme/notes.md"),
        ];
        assert_eq!(p.classify_all(paths), DataClass::Private);
        let paths = [
            Path::new("/home/op/src/jcode/src/lib.rs"),
            Path::new("/home/op/src/jcode/tests/fixtures/a.json"),
        ];
        assert_eq!(p.classify_all(paths), DataClass::Synthetic);
        assert_eq!(p.classify_all(std::iter::empty()), DataClass::Private);
    }

    #[test]
    fn load_rejects_relative_unnormalized_and_duplicate_roots() {
        let rel = "[[data_class]]\npath = \"src\"\nclass = \"public\"\n";
        assert!(matches!(
            DataClassPolicy::from_toml_str(rel),
            Err(DataClassPolicyError::RelativeRoot(_))
        ));
        let dots = "[[data_class]]\npath = \"/a/../b\"\nclass = \"public\"\n";
        assert!(matches!(
            DataClassPolicy::from_toml_str(dots),
            Err(DataClassPolicyError::UnnormalizedRoot(_))
        ));
        let dup = "[[data_class]]\npath = \"/a\"\nclass = \"public\"\n[[data_class]]\npath = \"/a\"\nclass = \"private\"\n";
        assert!(matches!(
            DataClassPolicy::from_toml_str(dup),
            Err(DataClassPolicyError::DuplicateRoot(_))
        ));
    }

    #[test]
    fn classification_composes_with_route_eligibility() {
        let p = policy();
        let public = p.classify(Path::new("/home/op/src/jcode/src/lib.rs"));
        assert!(public.is_remote_eligible(crate::RouteClass::MeteredRemote));
        let private = p.classify(Path::new("/home/op/clients/acme/src/lib.rs"));
        assert!(!private.is_remote_eligible(crate::RouteClass::MeteredRemote));
        let secret = p.classify(Path::new("/home/op/src/jcode/.env"));
        assert!(!secret.is_remote_eligible(crate::RouteClass::Local));
    }
}
