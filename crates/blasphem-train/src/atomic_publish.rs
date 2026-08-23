use std::path::Path;

use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum AtomicPublishError {
    #[error("the publication destination exists")]
    DestinationExists,
    #[allow(dead_code)]
    #[error("this target has no supported atomic no-replace rename primitive")]
    Unsupported,
    #[error("the atomic publication rename failed: {0}")]
    Rename(#[source] std::io::Error),
    #[error("the staging directory sync failed: {0}")]
    StagingSync(#[source] std::io::Error),
    #[error("the publication parent directory sync failed: {0}")]
    ParentSync(#[source] std::io::Error),
    #[error("the staging cleanup failed: {0}")]
    Cleanup(#[source] std::io::Error),
}

pub(crate) fn atomic_publish_noreplace(
    staged: &Path,
    output: &Path,
) -> Result<(), AtomicPublishError> {
    let publication = (|| {
        sync_directory(staged).map_err(AtomicPublishError::StagingSync)?;
        rename_noreplace(staged, output)?;
        let parent = output.parent().unwrap_or_else(|| Path::new("."));
        sync_directory(parent).map_err(AtomicPublishError::ParentSync)?;
        Ok(())
    })();
    if publication.is_ok() {
        return Ok(());
    }

    if staged.exists() {
        remove_staged(staged).map_err(AtomicPublishError::Cleanup)?;
    }
    publication
}

#[cfg(any(
    target_vendor = "apple",
    target_os = "linux",
    target_os = "android",
    target_os = "redox"
))]
fn rename_noreplace(staged: &Path, output: &Path) -> Result<(), AtomicPublishError> {
    use rustix::fs::{CWD, RenameFlags, renameat_with};
    use rustix::io::Errno;

    renameat_with(CWD, staged, CWD, output, RenameFlags::NOREPLACE).map_err(|error| {
        if matches!(error, Errno::EXIST | Errno::NOTEMPTY) {
            AtomicPublishError::DestinationExists
        } else {
            AtomicPublishError::Rename(std::io::Error::from_raw_os_error(error.raw_os_error()))
        }
    })
}

#[cfg(not(any(
    target_vendor = "apple",
    target_os = "linux",
    target_os = "android",
    target_os = "redox"
)))]
fn rename_noreplace(_staged: &Path, _output: &Path) -> Result<(), AtomicPublishError> {
    Err(AtomicPublishError::Unsupported)
}

fn remove_staged(staged: &Path) -> std::io::Result<()> {
    if staged.is_dir() {
        std::fs::remove_dir_all(staged)
    } else {
        std::fs::remove_file(staged)
    }
}

fn sync_directory(path: &Path) -> std::io::Result<()> {
    std::fs::File::open(path)?.sync_all()
}

#[cfg(all(
    test,
    any(
        target_vendor = "apple",
        target_os = "linux",
        target_os = "android",
        target_os = "redox"
    )
))]
mod tests {
    use tempfile::tempdir;

    use super::{AtomicPublishError, atomic_publish_noreplace};

    #[test]
    fn preserves_a_concurrent_file_and_removes_the_staged_file() {
        let directory = tempdir().expect("temporary directory");
        let staged = directory.path().join("source.staging");
        let output = directory.path().join("source.tsv");
        std::fs::write(&staged, "staged").expect("write staged file");
        std::fs::write(&output, "concurrent").expect("write concurrent file");

        let error = atomic_publish_noreplace(&staged, &output).expect_err("existing output");

        assert!(matches!(error, AtomicPublishError::DestinationExists));
        assert_eq!(
            std::fs::read_to_string(&output).expect("concurrent output"),
            "concurrent"
        );
        assert!(!staged.exists());
    }

    #[test]
    fn preserves_a_concurrent_directory_and_removes_the_staged_directory() {
        let directory = tempdir().expect("temporary directory");
        let staged = directory.path().join("prepared.staging");
        let output = directory.path().join("prepared");
        std::fs::create_dir(&staged).expect("create staged directory");
        std::fs::write(staged.join("data.tsv"), "staged").expect("write staged data");
        std::fs::create_dir(&output).expect("create concurrent directory");
        std::fs::write(output.join("owner.txt"), "concurrent").expect("write concurrent data");

        let error = atomic_publish_noreplace(&staged, &output).expect_err("existing output");

        assert!(matches!(error, AtomicPublishError::DestinationExists));
        assert_eq!(
            std::fs::read_to_string(output.join("owner.txt")).expect("concurrent output"),
            "concurrent"
        );
        assert!(!staged.exists());
    }
}
