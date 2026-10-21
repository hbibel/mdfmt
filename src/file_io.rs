#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, io};

    #[test]
    fn writes_changes_but_leaves_identical_files_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.md");
        fs::write(&path, "original\r\n").unwrap();
        let snapshot = read_input(&path).unwrap();
        let before = fs::metadata(&path).unwrap().modified().unwrap();
        assert!(!write_if_changed(&snapshot, "original\r\n").unwrap());
        assert_eq!(before, fs::metadata(&path).unwrap().modified().unwrap());
        assert!(write_if_changed(&snapshot, "changed\r\n").unwrap());
        assert_eq!(fs::read(&path).unwrap(), b"changed\r\n");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn failed_replacement_and_stale_snapshots_preserve_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.md");
        fs::write(&path, "original").unwrap();
        let snapshot = read_input(&path).unwrap();
        assert!(
            replace_with(&snapshot, "changed", |_, _| Err(io::Error::other(
                "injected failure"
            )))
            .is_err()
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
        fs::write(&path, "concurrent user edit").unwrap();
        assert!(write_if_changed(&snapshot, "changed").is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "concurrent user edit");
    }

    #[test]
    fn refuses_invalid_utf8_and_non_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("invalid.md");
        fs::write(&path, [0xff]).unwrap();
        assert!(read_input(&path).is_err());
        assert!(read_input(dir.path()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_link_writes_even_when_unchanged_and_preserves_permissions() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.md");
        let alias = dir.path().join("alias.md");
        fs::write(&path, "original").unwrap();
        symlink(&path, &alias).unwrap();
        let snapshot = read_input(&alias).unwrap();
        assert_eq!(snapshot.identity, read_input(&path).unwrap().identity);
        assert!(write_if_changed(&snapshot, "original").is_err());
        fs::remove_file(&alias).unwrap();
        fs::hard_link(&path, &alias).unwrap();
        let snapshot = read_input(&path).unwrap();
        assert_eq!(snapshot.identity, read_input(&alias).unwrap().identity);
        assert!(write_if_changed(&snapshot, "original").is_err());
        fs::remove_file(&alias).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        let snapshot = read_input(&path).unwrap();
        assert!(write_if_changed(&snapshot, "changed").unwrap());
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }

    #[cfg(unix)]
    #[test]
    fn refuses_fifo_without_opening_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pipe");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&path)
                .status()
                .unwrap()
                .success()
        );
        assert!(read_input(&path).is_err());
    }
}
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct FileIdentity {
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(not(unix))]
    path: PathBuf,
}

pub(crate) struct FileSnapshot {
    pub text: String,
    pub path: PathBuf,
    pub identity: FileIdentity,
    metadata: fs::Metadata,
}

pub(crate) fn read_input(path: &Path) -> io::Result<FileSnapshot> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(io::Error::other("input is not a regular file"));
    }
    let file = fs::File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(io::Error::other("input is not a regular file"));
    }
    #[cfg(unix)]
    let identity = {
        use std::os::unix::fs::MetadataExt;
        FileIdentity {
            device: metadata.dev(),
            inode: metadata.ino(),
        }
    };
    #[cfg(not(unix))]
    let identity = FileIdentity {
        path: fs::canonicalize(path)?,
    };
    let text = io::read_to_string(file)?;
    Ok(FileSnapshot {
        text,
        path: path.into(),
        identity,
        metadata,
    })
}

fn validate_target(snapshot: &FileSnapshot) -> io::Result<()> {
    let metadata = fs::symlink_metadata(&snapshot.path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(io::Error::other(
            "write target must be a regular file, not a symlink",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() != 1 {
            return Err(io::Error::other(
                "refusing to replace a multiply linked file",
            ));
        }
    }
    let current = read_input(&snapshot.path)?;
    if current.identity != snapshot.identity
        || current.text != snapshot.text
        || current.metadata.modified()? != snapshot.metadata.modified()?
        || current.metadata.permissions() != snapshot.metadata.permissions()
    {
        return Err(io::Error::other("write target changed since it was read"));
    }
    Ok(())
}

pub(crate) fn write_if_changed(snapshot: &FileSnapshot, output: &str) -> io::Result<bool> {
    replace_with(snapshot, output, |temporary, path| {
        temporary
            .persist(path)
            .map(|_| ())
            .map_err(|error| error.error)
    })
}

fn replace_with(
    snapshot: &FileSnapshot,
    output: &str,
    replace: impl FnOnce(tempfile::NamedTempFile, &Path) -> io::Result<()>,
) -> io::Result<bool> {
    validate_target(snapshot)?;
    if output == snapshot.text {
        return Ok(false);
    }
    let parent = snapshot
        .path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(output.as_bytes())?;
    temporary.flush()?;
    temporary
        .as_file()
        .set_permissions(snapshot.metadata.permissions())?;
    temporary.as_file().sync_all()?;
    validate_target(snapshot)?;
    replace(temporary, &snapshot.path)?;
    Ok(true)
}
