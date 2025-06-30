use anyhow::{Context, Result};
use crc32fast::Hasher;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// Metadata for a file stored in a VPK archive
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub preload: Vec<u8>,
    pub crc32: u32,
    pub preload_length: u16,
    pub archive_index: u16,
    pub archive_offset: u32,
    pub file_length: u32,
}

impl FileMetadata {
    /// Total length of the file (preload + file data)
    pub fn total_length(&self) -> u32 {
        self.preload_length as u32 + self.file_length
    }
}

/// A file-like object for files inside VPK archives
#[allow(dead_code)]
pub struct VPKFile {
    vpk_path: PathBuf,
    filepath: String,
    metadata: FileMetadata,
    position: u32,
    file_handle: Option<BufReader<File>>,
}

impl VPKFile {
    pub fn new<P: AsRef<Path>>(
        vpk_path: P,
        filepath: String,
        metadata: FileMetadata,
    ) -> Result<Self> {
        let vpk_path = vpk_path.as_ref().to_path_buf();

        let file_handle = if metadata.file_length > 0 {
            let actual_path = Self::resolve_archive_path(&vpk_path, metadata.archive_index)?;
            let file = File::open(&actual_path).with_context(|| {
                format!("Failed to open VPK archive: {}", actual_path.display())
            })?;
            Some(BufReader::new(file))
        } else {
            None
        };

        Ok(VPKFile {
            vpk_path,
            filepath,
            metadata,
            position: 0,
            file_handle,
        })
    }

    /// Resolves the actual archive file path based on the archive index
    fn resolve_archive_path(vpk_path: &Path, archive_index: u16) -> Result<PathBuf> {
        if archive_index == crate::utils::EMBEDDED_ARCHIVE_INDEX {
            Ok(vpk_path.to_path_buf())
        } else {
            // Replace "dir." with the archive number, e.g., "pak01_001.vpk" -> "pak01_002.vpk"
            let path_str = vpk_path.to_string_lossy();
            let new_path = path_str.replace("dir.", &format!("{archive_index:03}."));
            Ok(PathBuf::from(&new_path))
        }
    }

    /// Gets the file path within the VPK
    pub fn filepath(&self) -> &str {
        &self.filepath
    }

    /// Gets the file metadata
    pub fn metadata(&self) -> &FileMetadata {
        &self.metadata
    }

    /// Gets the current position in the file
    pub fn position(&self) -> u32 {
        self.position
    }

    /// Gets the total file length
    pub fn length(&self) -> u32 {
        self.metadata.total_length()
    }

    /// Saves the entire file to the specified path
    pub fn save<P: AsRef<Path>>(&mut self, output_path: P) -> Result<()> {
        let current_pos = self.position;
        self.seek(SeekFrom::Start(0))?;

        let mut output_file = File::create(output_path).context("Failed to create output file")?;

        let mut buffer = vec![0u8; 8192];
        loop {
            let bytes_read = self.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            output_file
                .write_all(&buffer[..bytes_read])
                .context("Failed to write to output file")?;
        }

        // Restore position
        self.position = current_pos;
        Ok(())
    }

    /// Verifies the file contents against the stored CRC32
    pub fn verify(&mut self) -> Result<bool> {
        let current_pos = self.position;
        self.seek(SeekFrom::Start(0))?;

        let mut hasher = Hasher::new();
        let mut buffer = vec![0u8; 8192];

        loop {
            let bytes_read = self.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        // Restore position
        self.position = current_pos;

        Ok(hasher.finalize() == self.metadata.crc32)
    }

    /// Reads all content into a Vec<u8>
    pub fn read_all(&mut self) -> Result<Vec<u8>> {
        let current_pos = self.position;
        self.seek(SeekFrom::Start(0))?;

        let mut buffer = Vec::with_capacity(self.length() as usize);
        let mut temp_buffer = vec![0u8; 8192];

        loop {
            let bytes_read = self.read(&mut temp_buffer)?;
            if bytes_read == 0 {
                break;
            }
            buffer.extend_from_slice(&temp_buffer[..bytes_read]);
        }

        self.position = current_pos;
        Ok(buffer)
    }

    /// Reads the entire file as a UTF-8 string
    pub fn read_all_string(&mut self) -> Result<String> {
        let bytes = self.read_all()?;
        String::from_utf8(bytes).context("File contains invalid UTF-8")
    }
}

impl Read for VPKFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.position >= self.length() || buf.is_empty() {
            return Ok(0);
        }

        let mut bytes_read = 0;
        let remaining_length = (self.length() - self.position) as usize;
        let to_read = buf.len().min(remaining_length);

        // Read from preload data first
        if self.position < self.metadata.preload_length as u32 {
            let preload_start = self.position as usize;
            let preload_end =
                (self.metadata.preload_length as u32).min(self.position + to_read as u32) as usize;
            let preload_bytes = preload_end - preload_start;

            buf[..preload_bytes]
                .copy_from_slice(&self.metadata.preload[preload_start..preload_end]);
            bytes_read += preload_bytes;
            self.position += preload_bytes as u32;
        }

        let length = self.length();

        // Read from archive file if there's still data to read and we have file data
        if bytes_read < to_read && self.metadata.file_length > 0 && self.file_handle.is_some() {
            if let Some(ref mut file_handle) = self.file_handle {
                let archive_position = self.metadata.archive_offset + self.position;

                file_handle
                    .seek(SeekFrom::Start(archive_position as u64))
                    .map_err(std::io::Error::other)?;

                let remaining = (length - self.position) as usize;
                let to_read_from_file = (to_read - bytes_read).min(remaining);

                let file_bytes_read =
                    file_handle.read(&mut buf[bytes_read..bytes_read + to_read_from_file])?;
                bytes_read += file_bytes_read;
                self.position += file_bytes_read as u32;
            }
        }

        Ok(bytes_read)
    }
}

impl Seek for VPKFile {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let new_position = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::End(offset) => self.length() as i64 + offset,
            SeekFrom::Current(offset) => self.position as i64 + offset,
        };

        if new_position < 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Cannot seek to negative position",
            ));
        }

        self.position = (new_position as u32).min(self.length());
        Ok(self.position as u64)
    }
}

impl std::fmt::Debug for VPKFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VPKFile")
            .field("filepath", &self.filepath)
            .field("position", &self.position)
            .field("length", &self.length())
            .field("metadata", &self.metadata)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_metadata() {
        let metadata = FileMetadata {
            preload: vec![1, 2, 3],
            crc32: 0x12345678,
            preload_length: 3,
            archive_index: 0,
            archive_offset: 100,
            file_length: 50,
        };

        assert_eq!(metadata.total_length(), 53);
    }

    #[test]
    fn test_vpkfile_seek() -> Result<()> {
        let metadata = FileMetadata {
            preload: vec![0, 1, 2, 3, 4],
            crc32: 0,
            preload_length: 5,
            archive_index: crate::utils::EMBEDDED_ARCHIVE_INDEX,
            archive_offset: 0,
            file_length: 0,
        };

        let temp_file = tempfile::NamedTempFile::new()?;
        let mut vpk_file = VPKFile::new(temp_file.path(), "test".to_string(), metadata)?;

        // Test seeking
        assert_eq!(vpk_file.seek(SeekFrom::Start(2))?, 2);
        assert_eq!(vpk_file.position(), 2);

        assert_eq!(vpk_file.seek(SeekFrom::End(-1))?, 4);
        assert_eq!(vpk_file.position(), 4);

        Ok(())
    }
}
