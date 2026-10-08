//! Unix preparation stays anchored to opened directories below the trusted
//! JCODE_HOME. No browser/profile component or file is opened through a symlink.
//! This protects preparation from path substitution, not Firefox's later
//! pathname lookup at launch or arbitrary interference by the same user.

use anyhow::{Context, Result, bail};
use std::ffi::{CStr, CString};
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::MetadataExt;

fn child_directory(parent: &File, name: &CStr) -> Result<File> {
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    // SAFETY: descriptors and NUL-terminated names remain live through each call.
    let mut fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound {
        if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } < 0
            && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
        {
            return Err(std::io::Error::last_os_error())
                .context("Cannot create agent profile directory");
        }
        fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    }
    if fd < 0 {
        return Err(std::io::Error::last_os_error())
            .context("Cannot open agent profile directory without following links");
    }
    // SAFETY: successful openat returns a newly owned descriptor.
    let directory = unsafe { File::from_raw_fd(fd) };
    let metadata = directory.metadata()?;
    if metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o022 != 0 {
        bail!("Agent profile directories must be owned by this user and not writable by others");
    }
    Ok(directory)
}

fn read_file(parent: &File, name: &CStr) -> Result<Option<Vec<u8>>> {
    // O_NONBLOCK prevents a substituted FIFO from hanging the preparation.
    let flags = libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC;
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        let error = std::io::Error::last_os_error();
        if error.kind() == std::io::ErrorKind::NotFound {
            return Ok(None);
        }
        return Err(error).context("Cannot read agent profile file without following links");
    }
    let mut file = unsafe { File::from_raw_fd(fd) };
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.nlink() != 1 || metadata.uid() != unsafe { libc::geteuid() }
    {
        bail!("Agent profile files must be regular, owned files without hard links");
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(Some(bytes))
}

fn write_file(parent: &File, name: &CStr, bytes: &[u8]) -> Result<()> {
    let temporary = CString::new(format!(".jcode-profile-{}", uuid::Uuid::new_v4()))?;
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            temporary.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error()).context("Cannot stage agent profile file");
    }
    let result = (|| -> Result<()> {
        let mut file = unsafe { File::from_raw_fd(fd) };
        file.write_all(bytes)?;
        file.sync_all()?;
        // Atomic replacement never follows a raced-in symlink or modifies the
        // contents of a hard-linked target. Both names use the pinned directory.
        if unsafe {
            libc::renameat(
                parent.as_raw_fd(),
                temporary.as_ptr(),
                parent.as_raw_fd(),
                name.as_ptr(),
            )
        } < 0
        {
            return Err(std::io::Error::last_os_error())
                .context("Cannot publish agent profile file");
        }
        Ok(())
    })();
    if unsafe { libc::unlinkat(parent.as_raw_fd(), temporary.as_ptr(), 0) } < 0
        && std::io::Error::last_os_error().kind() != std::io::ErrorKind::NotFound
    {
        crate::logging::warn("Could not remove a staged agent profile file");
    }
    result
}

pub(super) fn prepare(home: &std::path::Path) -> Result<()> {
    // The configured state root and its ancestors are operator-controlled.
    // Starting here also permits the OS's /var -> /private/var on macOS.
    std::fs::create_dir_all(home).context("Cannot create Jcode state root")?;
    let home = File::open(home).context("Cannot open Jcode state root")?;
    let browser = child_directory(&home, c"browser")?;
    let profile = child_directory(&browser, c"agent-profile")?;
    let extensions = child_directory(&profile, c"extensions")?;
    let target = CString::new(format!("{}.xpi", super::EXTENSION_ID_LISTED))?;

    // Validate every existing file before modifying either profile payload.
    let prefs = read_file(&profile, c"user.js")?;
    let existing = read_file(&extensions, &target)?;
    let source = read_file(&browser, c"browser-agent-bridge.xpi")?;
    if let Some(source) = source
        && existing.as_deref() != Some(source.as_slice())
    {
        write_file(&extensions, &target, &source)?;
    }
    if prefs.as_deref() != Some(super::AGENT_PROFILE_USER_JS.as_bytes()) {
        write_file(
            &profile,
            c"user.js",
            super::AGENT_PROFILE_USER_JS.as_bytes(),
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn profile_writes_remain_anchored_after_directory_substitution() {
        let temp = tempfile::tempdir().unwrap();
        let root = File::open(temp.path()).unwrap();
        let browser = child_directory(&root, c"browser").unwrap();
        let profile = child_directory(&browser, c"agent-profile").unwrap();
        let personal = temp.path().join("personal");
        std::fs::create_dir_all(personal.join("agent-profile")).unwrap();
        let personal_prefs = personal.join("agent-profile/user.js");
        std::fs::write(&personal_prefs, b"personal prefs").unwrap();
        std::fs::rename(temp.path().join("browser"), temp.path().join("original")).unwrap();
        symlink(&personal, temp.path().join("browser")).unwrap();
        write_file(&profile, c"user.js", b"agent prefs").unwrap();
        assert_eq!(std::fs::read(personal_prefs).unwrap(), b"personal prefs");
        assert_eq!(
            std::fs::read(temp.path().join("original/agent-profile/user.js")).unwrap(),
            b"agent prefs"
        );
        assert!(child_directory(&root, c"browser").is_err());
    }

    #[test]
    fn raced_file_symlink_is_replaced_without_touching_its_target() {
        let temp = tempfile::tempdir().unwrap();
        let directory = File::open(temp.path()).unwrap();
        let personal = temp.path().join("personal-prefs");
        std::fs::write(&personal, b"personal prefs").unwrap();
        symlink(&personal, temp.path().join("user.js")).unwrap();
        assert!(read_file(&directory, c"user.js").is_err());
        write_file(&directory, c"user.js", b"agent prefs").unwrap();
        assert_eq!(std::fs::read(personal).unwrap(), b"personal prefs");
        assert_eq!(
            std::fs::read(temp.path().join("user.js")).unwrap(),
            b"agent prefs"
        );
    }
}
