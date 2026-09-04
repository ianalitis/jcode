use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize)]
struct TreeUsage {
    apparent_bytes: u64,
    files: u64,
    directories: u64,
    errors: u64,
}

impl TreeUsage {
    fn add(&mut self, other: &Self) {
        self.apparent_bytes = self.apparent_bytes.saturating_add(other.apparent_bytes);
        self.files = self.files.saturating_add(other.files);
        self.directories = self.directories.saturating_add(other.directories);
        self.errors = self.errors.saturating_add(other.errors);
    }
}

#[derive(Debug, Clone, Serialize)]
struct StorageEntry {
    name: String,
    path: String,
    #[serde(flatten)]
    usage: TreeUsage,
}

#[derive(Debug, Clone, Serialize)]
struct BuildVersionUsage {
    version: String,
    path: String,
    #[serde(flatten)]
    usage: TreeUsage,
    active_references: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct DanglingBuildReference {
    version: String,
    active_references: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct BuildInventory {
    path: String,
    reference_scope: &'static str,
    installed_versions: Vec<BuildVersionUsage>,
    dangling_references: Vec<DanglingBuildReference>,
    unreferenced_versions: usize,
    unreferenced_apparent_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
struct StorageReport {
    home: String,
    apparent_bytes: u64,
    entries: Vec<StorageEntry>,
    builds: BuildInventory,
    warnings: Vec<String>,
    measurement_note: &'static str,
}

pub(crate) fn run_storage_status_command(json: bool) -> Result<()> {
    if !json {
        crate::cli::output::stderr_info("Scanning Jcode storage (read-only)...");
    }
    let home = crate::storage::jcode_dir()?;
    let builds = crate::build::builds_dir_path()?;
    let current_exe = std::env::current_exe().ok();
    let report = collect_storage_report(&home, &builds, current_exe.as_deref())?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{}", render_storage_report(&report));
    }
    Ok(())
}

fn collect_storage_report(
    home: &Path,
    builds: &Path,
    current_exe: Option<&Path>,
) -> Result<StorageReport> {
    let mut warnings = Vec::new();
    let (build_inventory, builds_usage) = scan_builds(builds, current_exe, &mut warnings);
    let mut entries = scan_top_level(home, builds, &builds_usage, &mut warnings)?;

    if !builds.starts_with(home) && builds.exists() {
        entries.push(StorageEntry {
            name: "builds".to_string(),
            path: builds.display().to_string(),
            usage: builds_usage,
        });
    }
    entries.sort_by(|left, right| {
        right
            .usage
            .apparent_bytes
            .cmp(&left.usage.apparent_bytes)
            .then_with(|| left.name.cmp(&right.name))
    });

    let apparent_bytes = entries.iter().fold(0_u64, |total, entry| {
        total.saturating_add(entry.usage.apparent_bytes)
    });
    Ok(StorageReport {
        home: home.display().to_string(),
        apparent_bytes,
        entries,
        builds: build_inventory,
        warnings,
        measurement_note: "Apparent file bytes are reported. Hard links can be counted more than once; symbolic links are not followed.",
    })
}

fn scan_top_level(
    home: &Path,
    builds: &Path,
    builds_usage: &TreeUsage,
    warnings: &mut Vec<String>,
) -> Result<Vec<StorageEntry>> {
    let read_dir = match std::fs::read_dir(home) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(error).with_context(|| format!("read Jcode home {}", home.display()));
        }
    };

    let mut entries = Vec::new();
    for entry in read_dir {
        match entry {
            Ok(entry) => {
                let path = entry.path();
                entries.push(StorageEntry {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    path: path.display().to_string(),
                    usage: if path == builds {
                        builds_usage.clone()
                    } else {
                        scan_path(&path)
                    },
                });
            }
            Err(error) => warnings.push(format!(
                "Could not inspect one entry under {}: {error}",
                home.display()
            )),
        }
    }
    Ok(entries)
}

fn scan_path(root: &Path) -> TreeUsage {
    let mut usage = TreeUsage::default();
    let mut pending = vec![root.to_path_buf()];

    while let Some(path) = pending.pop() {
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(_) => {
                usage.errors += 1;
                continue;
            }
        };
        let file_type = metadata.file_type();
        if file_type.is_dir() {
            usage.directories += 1;
            match std::fs::read_dir(&path) {
                Ok(entries) => {
                    for entry in entries {
                        match entry {
                            Ok(entry) => pending.push(entry.path()),
                            Err(_) => usage.errors += 1,
                        }
                    }
                }
                Err(_) => usage.errors += 1,
            }
        } else {
            usage.files += 1;
            usage.apparent_bytes = usage.apparent_bytes.saturating_add(metadata.len());
        }
    }

    usage
}

fn scan_builds(
    builds: &Path,
    current_exe: Option<&Path>,
    warnings: &mut Vec<String>,
) -> (BuildInventory, TreeUsage) {
    let versions_dir = builds.join("versions");
    let mut references = collect_build_references(builds, &versions_dir, current_exe, warnings);
    let mut installed_versions = Vec::new();
    let mut versions_usage = TreeUsage::default();

    match std::fs::symlink_metadata(&versions_dir) {
        Ok(metadata) if metadata.file_type().is_dir() => {
            versions_usage.directories += 1;
            let entries = match std::fs::read_dir(&versions_dir) {
                Ok(entries) => entries,
                Err(error) => {
                    versions_usage.errors += 1;
                    warnings.push(format!(
                        "Could not list installed builds at {}: {error}",
                        versions_dir.display()
                    ));
                    return finish_build_scan(
                        builds,
                        &versions_dir,
                        versions_usage,
                        installed_versions,
                        references,
                    );
                }
            };
            for entry in entries {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(error) => {
                        versions_usage.errors += 1;
                        warnings.push(format!(
                            "Could not inspect one installed build under {}: {error}",
                            versions_dir.display()
                        ));
                        continue;
                    }
                };
                let path = entry.path();
                let usage = scan_path(&path);
                versions_usage.add(&usage);
                let is_dir = match entry.file_type() {
                    Ok(file_type) => file_type.is_dir(),
                    Err(error) => {
                        warnings.push(format!(
                            "Could not inspect installed build {}: {error}",
                            entry.path().display()
                        ));
                        false
                    }
                };
                if !is_dir {
                    continue;
                }

                let version = entry.file_name().to_string_lossy().into_owned();
                installed_versions.push(BuildVersionUsage {
                    active_references: references.remove(&version).unwrap_or_default(),
                    version,
                    path: path.display().to_string(),
                    usage,
                });
            }
        }
        Ok(_) => versions_usage = scan_path(&versions_dir),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            versions_usage.errors += 1;
            warnings.push(format!(
                "Could not inspect installed builds at {}: {error}",
                versions_dir.display()
            ));
        }
    }

    finish_build_scan(
        builds,
        &versions_dir,
        versions_usage,
        installed_versions,
        references,
    )
}

fn finish_build_scan(
    builds: &Path,
    versions_dir: &Path,
    versions_usage: TreeUsage,
    mut installed_versions: Vec<BuildVersionUsage>,
    references: BTreeMap<String, Vec<String>>,
) -> (BuildInventory, TreeUsage) {
    installed_versions.sort_by(|left, right| {
        right
            .usage
            .apparent_bytes
            .cmp(&left.usage.apparent_bytes)
            .then_with(|| left.version.cmp(&right.version))
    });
    let unreferenced_versions = installed_versions
        .iter()
        .filter(|version| version.active_references.is_empty())
        .count();
    let unreferenced_apparent_bytes = installed_versions
        .iter()
        .filter(|version| version.active_references.is_empty())
        .fold(0_u64, |total, version| {
            total.saturating_add(version.usage.apparent_bytes)
        });
    let dangling_references = references
        .into_iter()
        .map(|(version, active_references)| DanglingBuildReference {
            version,
            active_references,
        })
        .collect();

    let builds_usage = scan_builds_root(builds, versions_dir, &versions_usage);
    (
        BuildInventory {
            path: builds.display().to_string(),
            reference_scope: "Build metadata, channel symlinks, and this process's executable. Other running processes are not inspected.",
            installed_versions,
            dangling_references,
            unreferenced_versions,
            unreferenced_apparent_bytes,
        },
        builds_usage,
    )
}

fn scan_builds_root(builds: &Path, versions_dir: &Path, versions_usage: &TreeUsage) -> TreeUsage {
    let metadata = match std::fs::symlink_metadata(builds) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return TreeUsage::default(),
        Err(_) => {
            return TreeUsage {
                errors: 1,
                ..TreeUsage::default()
            };
        }
    };
    if !metadata.file_type().is_dir() {
        return scan_path(builds);
    }

    let mut usage = TreeUsage {
        directories: 1,
        ..TreeUsage::default()
    };
    let entries = match std::fs::read_dir(builds) {
        Ok(entries) => entries,
        Err(_) => {
            usage.errors += 1;
            return usage;
        }
    };
    for entry in entries {
        match entry {
            Ok(entry) if entry.path() == versions_dir => usage.add(versions_usage),
            Ok(entry) => usage.add(&scan_path(&entry.path())),
            Err(_) => usage.errors += 1,
        }
    }
    usage
}

fn collect_build_references(
    builds: &Path,
    versions_dir: &Path,
    current_exe: Option<&Path>,
    warnings: &mut Vec<String>,
) -> BTreeMap<String, Vec<String>> {
    let mut references = BTreeMap::new();

    for (file, label) in [
        ("stable-version", "stable marker"),
        ("current-version", "current marker"),
        ("shared-server-version", "shared-server marker"),
    ] {
        add_marker_reference(&mut references, &builds.join(file), label, warnings);
    }

    let manifest_path = builds.join("manifest.json");
    if manifest_path.exists() {
        match crate::storage::read_json::<crate::build::BuildManifest>(&manifest_path) {
            Ok(manifest) => {
                add_reference(&mut references, manifest.stable, "manifest stable");
                add_reference(&mut references, manifest.canary, "manifest canary");
                if let Some(pending) = manifest.pending_activation {
                    add_reference(
                        &mut references,
                        Some(pending.new_version),
                        "pending activation",
                    );
                    add_reference(
                        &mut references,
                        pending.previous_current_version,
                        "pending current rollback",
                    );
                    add_reference(
                        &mut references,
                        pending.previous_shared_server_version,
                        "pending shared-server rollback",
                    );
                }
            }
            Err(error) => warnings.push(format!(
                "Could not read build manifest {}: {error}",
                manifest_path.display()
            )),
        }
    }

    for (channel, label) in [
        ("stable", "stable channel"),
        ("current", "current channel"),
        ("canary", "canary channel"),
        ("shared-server", "shared-server channel"),
    ] {
        add_path_reference(
            &mut references,
            &builds.join(channel).join(crate::build::binary_name()),
            versions_dir,
            label,
            warnings,
        );
    }
    if let Some(current_exe) = current_exe {
        add_path_reference(
            &mut references,
            current_exe,
            versions_dir,
            "running executable",
            warnings,
        );
    }

    for labels in references.values_mut() {
        labels.sort();
        labels.dedup();
    }
    references
}

fn add_marker_reference(
    references: &mut BTreeMap<String, Vec<String>>,
    path: &Path,
    label: &str,
    warnings: &mut Vec<String>,
) {
    match std::fs::read_to_string(path) {
        Ok(value) => add_reference(references, Some(value), label),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => warnings.push(format!("Could not read {}: {error}", path.display())),
    }
}

fn add_path_reference(
    references: &mut BTreeMap<String, Vec<String>>,
    path: &Path,
    versions_dir: &Path,
    label: &str,
    warnings: &mut Vec<String>,
) {
    let resolved = match path.canonicalize() {
        Ok(path) => path,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => {
            warnings.push(format!("Could not resolve {}: {error}", path.display()));
            return;
        }
    };
    let versions_dir = versions_dir
        .canonicalize()
        .unwrap_or_else(|_| versions_dir.to_path_buf());
    let Ok(relative) = resolved.strip_prefix(&versions_dir) else {
        return;
    };
    let Some(version) = relative.components().next() else {
        return;
    };
    add_reference(
        references,
        Some(version.as_os_str().to_string_lossy().into_owned()),
        label,
    );
}

fn add_reference(
    references: &mut BTreeMap<String, Vec<String>>,
    version: Option<String>,
    label: &str,
) {
    let Some(version) = version.map(|value| value.trim().to_string()) else {
        return;
    };
    if version.is_empty() {
        return;
    }
    references
        .entry(version)
        .or_default()
        .push(label.to_string());
}

fn render_storage_report(report: &StorageReport) -> String {
    let mut output = String::new();
    output.push_str("Jcode storage status (read-only)\n");
    output.push_str(&format!("Home: {}\n", report.home));
    output.push_str(&format!(
        "Apparent size: {} across {} top-level entries\n\n",
        format_bytes(report.apparent_bytes),
        report.entries.len()
    ));

    output.push_str("Top-level usage:\n");
    if report.entries.is_empty() {
        output.push_str("  No Jcode storage found.\n");
    } else {
        for entry in report.entries.iter().take(10) {
            output.push_str(&format!(
                "  {:>9}  {:<20}  {} files, {} dirs",
                format_bytes(entry.usage.apparent_bytes),
                entry.name,
                entry.usage.files,
                entry.usage.directories
            ));
            if entry.usage.errors > 0 {
                output.push_str(&format!(", {} scan errors", entry.usage.errors));
            }
            output.push('\n');
        }
        if report.entries.len() > 10 {
            output.push_str(&format!(
                "  ... {} smaller entries omitted; use --json for all entries\n",
                report.entries.len() - 10
            ));
        }
    }

    let installed = &report.builds.installed_versions;
    let referenced = installed
        .iter()
        .filter(|version| !version.active_references.is_empty())
        .count();
    output.push_str("\nInstalled build versions:\n");
    output.push_str(&format!("  Path: {}\n", report.builds.path));
    output.push_str(&format!("  Installed: {}\n", installed.len()));
    output.push_str(&format!("  Known references: {}\n", referenced));
    output.push_str(&format!(
        "  Without known references: {} ({})\n",
        report.builds.unreferenced_versions,
        format_bytes(report.builds.unreferenced_apparent_bytes)
    ));

    for version in installed
        .iter()
        .filter(|version| !version.active_references.is_empty())
    {
        output.push_str(&format!(
            "  referenced {}: {}\n",
            version.version,
            version.active_references.join(", ")
        ));
    }

    let unreferenced: Vec<_> = installed
        .iter()
        .filter(|version| version.active_references.is_empty())
        .take(5)
        .collect();
    if !unreferenced.is_empty() {
        output.push_str("  Largest versions without known references:\n");
        for version in unreferenced {
            output.push_str(&format!(
                "    {:>9}  {}\n",
                format_bytes(version.usage.apparent_bytes),
                version.version
            ));
        }
    }

    if !report.builds.dangling_references.is_empty() {
        output.push_str("  Dangling references:\n");
        for reference in &report.builds.dangling_references {
            output.push_str(&format!(
                "    {}: {}\n",
                reference.version,
                reference.active_references.join(", ")
            ));
        }
    }

    if !report.warnings.is_empty() {
        output.push_str("\nWarnings:\n");
        for warning in report.warnings.iter().take(5) {
            output.push_str(&format!("  - {warning}\n"));
        }
    }

    output.push_str("\nNo files were deleted or created.\n");
    output.push_str(report.measurement_note);
    output.push('\n');
    output.push_str(report.builds.reference_scope);
    output.push('\n');
    output.push_str(
        "Without known references means no recorded build metadata, channel symlink, pending activation, or this process points at that version; it is not an automatic deletion decision.\n",
    );
    output
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_is_read_only_and_reference_aware() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("jcode-home");
        let builds = home.join("builds");
        let versions = builds.join("versions");
        std::fs::create_dir_all(home.join("source")).unwrap();
        std::fs::write(home.join("source/repo.bin"), vec![0_u8; 3]).unwrap();
        std::fs::create_dir_all(versions.join("active")).unwrap();
        std::fs::write(versions.join("active/jcode"), vec![0_u8; 7]).unwrap();
        std::fs::create_dir_all(versions.join("stale")).unwrap();
        std::fs::write(versions.join("stale/jcode"), vec![0_u8; 11]).unwrap();
        std::fs::write(builds.join("current-version"), "active\n").unwrap();

        let mut manifest = crate::build::BuildManifest::default();
        manifest.canary = Some("missing".to_string());
        std::fs::write(
            builds.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();

        let report = collect_storage_report(&home, &builds, None).unwrap();
        let active = report
            .builds
            .installed_versions
            .iter()
            .find(|version| version.version == "active")
            .unwrap();
        let stale = report
            .builds
            .installed_versions
            .iter()
            .find(|version| version.version == "stale")
            .unwrap();

        assert_eq!(active.active_references, vec!["current marker"]);
        assert!(stale.active_references.is_empty());
        assert_eq!(report.builds.unreferenced_versions, 1);
        assert_eq!(report.builds.unreferenced_apparent_bytes, 11);
        assert_eq!(report.builds.dangling_references.len(), 1);
        assert_eq!(report.builds.dangling_references[0].version, "missing");
        assert!(render_storage_report(&report).contains("No files were deleted or created."));
    }

    #[cfg(unix)]
    #[test]
    fn canary_channel_reference_protects_an_installed_version() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("jcode-home");
        let builds = home.join("builds");
        let version_binary = builds.join("versions/canary-only/jcode");
        std::fs::create_dir_all(version_binary.parent().unwrap()).unwrap();
        std::fs::write(&version_binary, b"binary").unwrap();
        std::fs::create_dir_all(builds.join("canary")).unwrap();
        std::os::unix::fs::symlink(&version_binary, builds.join("canary/jcode")).unwrap();

        let report = collect_storage_report(&home, &builds, None).unwrap();

        assert_eq!(
            report.builds.installed_versions[0].active_references,
            vec!["canary channel"]
        );
        assert_eq!(report.builds.unreferenced_versions, 0);
    }

    #[test]
    fn missing_home_is_not_created() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("missing-home");
        let builds = home.join("builds");

        let report = collect_storage_report(&home, &builds, None).unwrap();

        assert!(report.entries.is_empty());
        assert!(!home.exists());
    }

    #[cfg(unix)]
    #[test]
    fn scanner_does_not_follow_symbolic_links() {
        let temp = tempfile::tempdir().unwrap();
        let outside = temp.path().join("outside.bin");
        std::fs::write(&outside, vec![0_u8; 1024 * 1024]).unwrap();
        let root = temp.path().join("root");
        std::fs::create_dir(&root).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("link")).unwrap();

        let usage = scan_path(&root);

        assert_eq!(usage.files, 1);
        assert!(usage.apparent_bytes < 1024);
    }

    #[test]
    fn byte_formatting_is_compact_and_binary() {
        assert_eq!(format_bytes(999), "999 B");
        assert_eq!(format_bytes(1024), "1.0 KiB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MiB");
    }
}
