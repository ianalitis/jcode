//! Route table as data (`~/.jcode/routes.toml`).
//!
//! Admission reads this table; workers never write it. Each entry promotes one
//! task class to a concrete, exact route together with the evidence that
//! justified the promotion, so a promotion is reversible and auditable (R9).
//! The table is validated on load with the same fail-closed rules used by
//! [`crate::AttemptRecord::freeze`]: banned router families, floating aliases
//! and data-class/route-class mismatches are rejected before any route is used.

use crate::{DataClass, Effort, RouteClass, is_banned_router_family, is_floating_alias};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// The whole table. Unknown task classes at the call site are an admission
/// error, never a silent fallback to some default route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteTable {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub routes: BTreeMap<String, RouteEntry>,
}

fn default_version() -> u32 {
    1
}

/// One promoted task class -> route decision, with its evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteEntry {
    pub provider: String,
    /// Exact model or version slug. No `latest` aliases.
    pub model_exact: String,
    pub endpoint: String,
    pub route_class: RouteClass,
    #[serde(default)]
    pub effort: Effort,
    /// The most sensitive data class this route was vetted to carry. A request
    /// for a more sensitive class is refused even when the route class would
    /// otherwise permit it.
    pub admitted_data_class: DataClass,
    pub promoted_at: DateTime<Utc>,
    /// Reference to the measurement (receipt or report) that justified the
    /// promotion. Required: a promotion without evidence is not a promotion.
    pub evidence_path: String,
    pub policy_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "detail")]
pub enum RouteTableError {
    Parse(String),
    Serialize(String),
    Io(String),
    UnsupportedVersion(u32),
    EmptyTable,
    UnknownTaskClass(String),
    /// The requested class is more sensitive than the route was vetted for.
    OutsidePromotionScope {
        task_class: String,
        requested: DataClass,
        admitted: DataClass,
    },
    DataClassNotEligible {
        task_class: String,
        data_class: DataClass,
        route_class: RouteClass,
    },
    EmptyField {
        task_class: String,
        field: &'static str,
    },
    BannedRouterFamily {
        task_class: String,
        provider: String,
        model: String,
    },
    FloatingAlias {
        task_class: String,
        model: String,
    },
}

impl std::fmt::Display for RouteTableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RouteTableError::Parse(e) => write!(f, "route table does not parse: {e}"),
            RouteTableError::Serialize(e) => write!(f, "route table does not serialize: {e}"),
            RouteTableError::Io(e) => write!(f, "route table I/O failed: {e}"),
            RouteTableError::UnsupportedVersion(v) => {
                write!(f, "route table version {v} is not supported")
            }
            RouteTableError::EmptyTable => write!(f, "route table has no routes"),
            RouteTableError::UnknownTaskClass(task_class) => {
                write!(f, "no route promoted for task class `{task_class}`")
            }
            RouteTableError::OutsidePromotionScope {
                task_class,
                requested,
                admitted,
            } => write!(
                f,
                "task class `{task_class}` is only vetted for {admitted:?}, not {requested:?}"
            ),
            RouteTableError::DataClassNotEligible {
                task_class,
                data_class,
                route_class,
            } => write!(
                f,
                "task class `{task_class}`: data class {data_class:?} may not use route {route_class:?}"
            ),
            RouteTableError::EmptyField { task_class, field } => {
                write!(f, "route `{task_class}` has an empty `{field}`")
            }
            RouteTableError::BannedRouterFamily {
                task_class,
                provider,
                model,
            } => write!(
                f,
                "route `{task_class}` reaches a banned family: `{model}` via `{provider}`"
            ),
            RouteTableError::FloatingAlias { task_class, model } => {
                write!(f, "route `{task_class}` uses floating alias `{model}`")
            }
        }
    }
}

impl std::error::Error for RouteTableError {}

impl Default for RouteTable {
    fn default() -> Self {
        Self {
            version: default_version(),
            routes: BTreeMap::new(),
        }
    }
}

impl RouteTable {
    /// Parse and validate a table. Parsing alone never admits a route.
    pub fn from_toml_str(src: &str) -> Result<Self, RouteTableError> {
        let table: RouteTable =
            toml::from_str(src).map_err(|e| RouteTableError::Parse(e.to_string()))?;
        table.validate()?;
        Ok(table)
    }

    /// Serialize for operator tooling. The table is data an operator edits; it
    /// is never written by a worker.
    pub fn to_toml_string(&self) -> Result<String, RouteTableError> {
        toml::to_string_pretty(self).map_err(|e| RouteTableError::Serialize(e.to_string()))
    }

    pub fn load_from_path(path: &Path) -> Result<Self, RouteTableError> {
        let src = std::fs::read_to_string(path).map_err(|e| RouteTableError::Io(e.to_string()))?;
        Self::from_toml_str(&src)
    }

    pub fn save_to_path(&self, path: &Path) -> Result<(), RouteTableError> {
        let src = self.to_toml_string()?;
        std::fs::write(path, src).map_err(|e| RouteTableError::Io(e.to_string()))
    }

    /// Reject any entry that could not have been frozen as an attempt.
    pub fn validate(&self) -> Result<(), RouteTableError> {
        if self.version < 1 {
            return Err(RouteTableError::UnsupportedVersion(self.version));
        }
        if self.routes.is_empty() {
            return Err(RouteTableError::EmptyTable);
        }
        for (task_class, entry) in &self.routes {
            for (field, value) in [
                ("provider", &entry.provider),
                ("model_exact", &entry.model_exact),
                ("endpoint", &entry.endpoint),
                ("evidence_path", &entry.evidence_path),
                ("policy_version", &entry.policy_version),
            ] {
                if value.trim().is_empty() {
                    return Err(RouteTableError::EmptyField {
                        task_class: task_class.clone(),
                        field,
                    });
                }
            }
            if is_banned_router_family(&entry.provider, &entry.model_exact) {
                return Err(RouteTableError::BannedRouterFamily {
                    task_class: task_class.clone(),
                    provider: entry.provider.clone(),
                    model: entry.model_exact.clone(),
                });
            }
            if is_floating_alias(&entry.model_exact) {
                return Err(RouteTableError::FloatingAlias {
                    task_class: task_class.clone(),
                    model: entry.model_exact.clone(),
                });
            }
            if !entry
                .admitted_data_class
                .is_remote_eligible(entry.route_class)
            {
                return Err(RouteTableError::DataClassNotEligible {
                    task_class: task_class.clone(),
                    data_class: entry.admitted_data_class,
                    route_class: entry.route_class,
                });
            }
        }
        Ok(())
    }

    /// Pick the admitted route for a task class and requested data class.
    /// Unknown task class, out-of-scope data class, and route/data mismatch all
    /// refuse rather than falling back.
    pub fn resolve(
        &self,
        task_class: &str,
        data_class: DataClass,
    ) -> Result<&RouteEntry, RouteTableError> {
        let entry = self
            .routes
            .get(task_class)
            .ok_or_else(|| RouteTableError::UnknownTaskClass(task_class.to_string()))?;
        if !data_class.is_remote_eligible(entry.route_class) {
            return Err(RouteTableError::DataClassNotEligible {
                task_class: task_class.to_string(),
                data_class,
                route_class: entry.route_class,
            });
        }
        if data_class > entry.admitted_data_class {
            return Err(RouteTableError::OutsidePromotionScope {
                task_class: task_class.to_string(),
                requested: data_class,
                admitted: entry.admitted_data_class,
            });
        }
        Ok(entry)
    }

    /// Promote (or replace) a task class. Reversible via [`Self::demote`].
    pub fn promote(&mut self, task_class: impl Into<String>, entry: RouteEntry) {
        self.routes.insert(task_class.into(), entry);
    }

    /// Remove a promotion, returning the entry that was dropped.
    pub fn demote(&mut self, task_class: &str) -> Option<RouteEntry> {
        self.routes.remove(task_class)
    }
}
