//! VPK (Valve Pak) file format library
//!
//! This library provides functionality to read, write, and manipulate VPK files
//! used by Valve's Source engine games.

pub mod file;
pub mod utils;
pub mod vpk;

pub use file::VPKFile;
pub use vpk::VPK;

use anyhow::Result;

/// Opens an existing VPK file for reading
pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<VPK> {
    VPK::open(path)
}

/// Creates a new VPK from a directory
pub fn from_directory<P: AsRef<std::path::Path>>(path: P) -> Result<VPK> {
    VPK::from_directory(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_create_and_read_vpk() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let src_dir = temp_dir.path().join("source");
        let vpk_path = temp_dir.path().join("test.vpk");

        // Create test directory structure
        fs::create_dir_all(&src_dir)?;
        fs::write(src_dir.join("test.txt"), b"Hello, World!")?;

        // Create VPK
        let vpk = from_directory(&src_dir)?;
        vpk.save(&vpk_path)?;

        // Read VPK back
        let vpk = open(&vpk_path)?;
        assert!(vpk.contains("test.txt"));

        Ok(())
    }
}
