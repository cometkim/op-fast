use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;

use anyhow::{Context, Result};

/// Write secret material to a file such that it is never observable with
/// permissions broader than `mode`. The file is created with `mode` applied
/// at open time (not chmod'd after the contents are written), and pre-existing
/// files are re-permissioned after truncation, before the contents land.
pub fn write_secret_file(path: &Path, contents: &[u8], mode: u32) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(mode)
        .open(path)
        .with_context(|| format!("Failed to create file: {:?}", path))?;

    // The open-time mode only applies to newly created files and is subject to
    // umask; enforce it exactly (covering pre-existing files) while the file
    // is still empty.
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .with_context(|| format!("Failed to set file mode: {:o}", mode))?;

    file.write_all(contents)
        .with_context(|| format!("Failed to write file: {:?}", path))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_path() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!(
            "op-fast-output-test-{}-{}",
            std::process::id(),
            n
        ))
    }

    fn mode_of(path: &Path) -> u32 {
        fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    #[test]
    fn creates_file_with_exact_mode_and_contents() {
        let path = temp_path();
        write_secret_file(&path, b"hunter2", 0o600).unwrap();
        assert_eq!(mode_of(&path), 0o600);
        assert_eq!(fs::read(&path).unwrap(), b"hunter2");
        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn honors_custom_mode() {
        let path = temp_path();
        write_secret_file(&path, b"public", 0o644).unwrap();
        assert_eq!(mode_of(&path), 0o644);
        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn tightens_permissions_of_pre_existing_file_before_rewriting() {
        let path = temp_path();
        fs::write(&path, b"old world-readable contents").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();

        write_secret_file(&path, b"new secret", 0o600).unwrap();
        assert_eq!(mode_of(&path), 0o600);
        assert_eq!(fs::read(&path).unwrap(), b"new secret");
        fs::remove_file(&path).unwrap();
    }
}
