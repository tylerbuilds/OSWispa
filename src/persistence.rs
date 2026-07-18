//! Durable, private persistence helpers for user configuration and transcripts.

use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::io::Write;
use std::path::Path;

/// Create an application-owned directory and keep it private on Unix.
pub fn ensure_private_dir(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path)
        .with_context(|| format!("Failed to create private directory {:?}", path))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
            .with_context(|| format!("Failed to secure private directory {:?}", path))?;
    }

    Ok(())
}

/// Atomically replace a private file using an owner-only temporary file.
pub fn write_private(path: &Path, contents: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Private file path has no parent: {:?}", path))?;
    ensure_private_dir(parent)?;

    let mut temp = tempfile::Builder::new()
        .prefix(".oswispa-")
        .tempfile_in(parent)
        .with_context(|| format!("Failed to create temporary file in {:?}", parent))?;
    temp.write_all(contents)
        .with_context(|| format!("Failed to write temporary file for {:?}", path))?;
    temp.as_file_mut()
        .sync_all()
        .with_context(|| format!("Failed to sync temporary file for {:?}", path))?;
    temp.persist(path)
        .map_err(|err| err.error)
        .with_context(|| format!("Failed to atomically replace {:?}", path))?;

    #[cfg(unix)]
    std::fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .with_context(|| format!("Failed to sync private directory {:?}", parent))?;

    Ok(())
}

pub fn write_json_private<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<()> {
    let mut json = serde_json::to_vec_pretty(value)?;
    json.push(b'\n');
    write_private(path, &json)
}

pub fn read_json_private<T: DeserializeOwned>(path: &Path) -> Result<T> {
    reject_symlink(path)?;
    harden_private_file(path)?;
    let bytes = std::fs::read(path).with_context(|| format!("Failed to read {:?}", path))?;
    serde_json::from_slice(&bytes).with_context(|| format!("Failed to parse {:?}", path))
}

pub fn read_private_string(path: &Path) -> Result<String> {
    reject_symlink(path)?;
    harden_private_file(path)?;
    std::fs::read_to_string(path).with_context(|| format!("Failed to read {:?}", path))
}

fn reject_symlink(path: &Path) -> Result<()> {
    if path.exists() && std::fs::symlink_metadata(path)?.file_type().is_symlink() {
        anyhow::bail!(
            "Refusing to read private data through a symbolic link: {:?}",
            path
        );
    }
    Ok(())
}

fn harden_private_file(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .with_context(|| format!("Failed to secure private file {:?}", path))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct TestValue {
        message: String,
    }

    #[test]
    fn json_round_trip_replaces_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("private").join("value.json");

        write_json_private(
            &path,
            &TestValue {
                message: "first".to_string(),
            },
        )
        .unwrap();
        write_json_private(
            &path,
            &TestValue {
                message: "second".to_string(),
            },
        )
        .unwrap();

        let loaded: TestValue = read_json_private(&path).unwrap();
        assert_eq!(loaded.message, "second");
    }

    #[cfg(unix)]
    #[test]
    fn persisted_files_and_directories_are_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let private_dir = dir.path().join("private");
        let path = private_dir.join("value");
        write_private(&path, b"secret").unwrap();

        assert_eq!(
            std::fs::metadata(&private_dir)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[cfg(unix)]
    #[test]
    fn private_reads_reject_symbolic_links() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target");
        let link = dir.path().join("link");
        std::fs::write(&target, "secret").unwrap();
        symlink(&target, &link).unwrap();

        assert!(read_private_string(&link).is_err());
    }
}
