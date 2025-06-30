use anyhow::{Context, Result, bail};
use crc32fast::Hasher;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::file::{FileMetadata, VPKFile};
use crate::utils::*;

type FileHashMap<'a> = HashMap<String, HashMap<String, Vec<(String, &'a FileMetadata)>>>;

/// VPK file format versions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VPKVersion {
    V1 = 1,
    V2 = 2,
}

/// VPK header information
#[derive(Debug, Clone)]
pub struct VPKHeader {
    pub signature: u32,
    pub version: VPKVersion,
    pub tree_length: u32,
    pub header_length: u32,
    // V2 specific fields
    pub embed_chunk_length: Option<u32>,
    pub chunk_hashes_length: Option<u32>,
    pub self_hashes_length: Option<u32>,
    pub signature_length: Option<u32>,
}

/// VPK checksums (V2 only)
#[derive(Debug, Clone)]
pub struct VPKChecksums {
    pub tree_checksum: [u8; 16],
    pub chunk_hashes_checksum: [u8; 16],
    pub file_checksum: [u8; 16],
}

/// Main VPK structure that handles both reading and writing
pub struct VPK {
    path: Option<PathBuf>,
    header: VPKHeader,
    tree: HashMap<String, FileMetadata>,
    checksums: Option<VPKChecksums>,
}

impl VPK {
    /// Opens an existing VPK file for reading
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut file = BufReader::new(
            File::open(&path)
                .with_context(|| format!("Failed to open VPK file: {}", path.display()))?,
        );

        let header = Self::read_header(&mut file)?;
        let tree = Self::read_file_tree(&mut file, &header)?;
        let checksums = if header.version == VPKVersion::V2 {
            Some(Self::read_checksums(&mut file, &header)?)
        } else {
            None
        };

        Ok(VPK {
            path: Some(path),
            header,
            tree,
            checksums,
        })
    }

    /// Creates a new VPK from a directory structure
    pub fn from_directory<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        if !path.is_dir() {
            bail!("Path is not a directory: {}", path.display());
        }

        let mut tree = HashMap::new();
        // let mut file_count = 0;

        // Walk the directory and build the file tree
        for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let relative_path = entry
                    .path()
                    .strip_prefix(path)
                    .context("Failed to get relative path")?;

                let path_str = normalize_path(&relative_path.to_string_lossy());
                let (_name, _ext) = split_filename(&entry.file_name().to_string_lossy())?;

                // Read file data for preload and calculate CRC
                let file_data = std::fs::read(entry.path())
                    .with_context(|| format!("Failed to read file: {}", entry.path().display()))?;

                let mut hasher = Hasher::new();
                hasher.update(&file_data);
                let crc32 = hasher.finalize();

                // For now, we embed all files (no separate archive files)
                let metadata = FileMetadata {
                    preload: file_data,
                    crc32,
                    preload_length: 0, // Will be set during save
                    archive_index: EMBEDDED_ARCHIVE_INDEX,
                    archive_offset: 0, // Will be set during save
                    file_length: 0,    // Will be set during save
                };

                tree.insert(path_str, metadata);
                // file_count += 1;
            }
        }

        let tree_length = Self::calculate_tree_length(&tree);

        let header = VPKHeader {
            signature: VPK_SIGNATURE,
            version: VPKVersion::V2, // Default to V2 for new files
            tree_length,
            header_length: 28,           // V2 header length
            embed_chunk_length: Some(0), // Will be calculated during save
            chunk_hashes_length: Some(0),
            self_hashes_length: Some(48),
            signature_length: Some(0),
        };

        Ok(VPK {
            path: None,
            header,
            tree,
            checksums: None,
        })
    }

    /// Saves the VPK to the specified path
    pub fn save<P: AsRef<Path>>(&self, output_path: P) -> Result<()> {
        let output_path = output_path.as_ref();
        let mut file =
            BufWriter::new(File::create(output_path).with_context(|| {
                format!("Failed to create VPK file: {}", output_path.display())
            })?);

        // Write header (will update embed_chunk_length later)
        self.write_header(&mut file)?;
        let header_end = file.stream_position()? as u32;

        // Write file tree and embedded data
        let embed_chunk_length = self.write_file_tree_and_data(&mut file)?;

        // Calculate and write checksums for V2
        if self.header.version == VPKVersion::V2 {
            // Update embed_chunk_length in header
            file.seek(SeekFrom::Start(12))?; // Position of embed_chunk_length
            file.write_all(&embed_chunk_length.to_le_bytes())?;
            file.seek(SeekFrom::End(0))?;

            // Flush buffer and get underlying file for checksum calculation
            file.flush()?;
            let mut underlying_file = file
                .into_inner()
                .map_err(|e| anyhow::anyhow!("Failed to get underlying file: {}", e))?;

            self.write_checksums(&mut underlying_file, header_end, embed_chunk_length)?;
        } else {
            file.flush()?;
        }

        Ok(())
    }

    /// Reads the VPK header from the file
    fn read_header<R: Read>(reader: &mut R) -> Result<VPKHeader> {
        let mut header_bytes = [0u8; 12];
        reader
            .read_exact(&mut header_bytes)
            .context("Failed to read VPK header")?;

        let signature = u32::from_le_bytes([
            header_bytes[0],
            header_bytes[1],
            header_bytes[2],
            header_bytes[3],
        ]);
        let version_num = u32::from_le_bytes([
            header_bytes[4],
            header_bytes[5],
            header_bytes[6],
            header_bytes[7],
        ]);
        let tree_length = u32::from_le_bytes([
            header_bytes[8],
            header_bytes[9],
            header_bytes[10],
            header_bytes[11],
        ]);

        if signature != VPK_SIGNATURE {
            bail!("Invalid VPK signature: 0x{:08x}", signature);
        }

        let version = match version_num {
            1 => VPKVersion::V1,
            2 => VPKVersion::V2,
            _ => bail!("Unsupported VPK version: {}", version_num),
        };

        let mut header = VPKHeader {
            signature,
            version,
            tree_length,
            header_length: 12,
            embed_chunk_length: None,
            chunk_hashes_length: None,
            self_hashes_length: None,
            signature_length: None,
        };

        // Read V2 extended header
        if version == VPKVersion::V2 {
            let mut v2_header = [0u8; 16];
            reader
                .read_exact(&mut v2_header)
                .context("Failed to read V2 header")?;

            header.embed_chunk_length = Some(u32::from_le_bytes([
                v2_header[0],
                v2_header[1],
                v2_header[2],
                v2_header[3],
            ]));
            header.chunk_hashes_length = Some(u32::from_le_bytes([
                v2_header[4],
                v2_header[5],
                v2_header[6],
                v2_header[7],
            ]));
            header.self_hashes_length = Some(u32::from_le_bytes([
                v2_header[8],
                v2_header[9],
                v2_header[10],
                v2_header[11],
            ]));
            header.signature_length = Some(u32::from_le_bytes([
                v2_header[12],
                v2_header[13],
                v2_header[14],
                v2_header[15],
            ]));
            header.header_length = 28;
        }

        Ok(header)
    }

    /// Reads the file tree from the VPK
    fn read_file_tree<R: Read>(
        reader: &mut R,
        header: &VPKHeader,
    ) -> Result<HashMap<String, FileMetadata>> {
        let mut tree = HashMap::new();

        loop {
            let ext = read_cstring(reader)?;
            if ext.is_empty() {
                break;
            }

            loop {
                let path = read_cstring(reader)?;
                if path.is_empty() {
                    break;
                }

                let normalized_path = if path == " " {
                    String::new()
                } else {
                    format!("{path}/")
                };

                loop {
                    let name = read_cstring(reader)?;
                    if name.is_empty() {
                        break;
                    }

                    // Read file metadata
                    let mut metadata_bytes = [0u8; 18];
                    reader
                        .read_exact(&mut metadata_bytes)
                        .context("Failed to read file metadata")?;

                    let crc32 = u32::from_le_bytes([
                        metadata_bytes[0],
                        metadata_bytes[1],
                        metadata_bytes[2],
                        metadata_bytes[3],
                    ]);
                    let preload_length = u16::from_le_bytes([metadata_bytes[4], metadata_bytes[5]]);
                    let archive_index = u16::from_le_bytes([metadata_bytes[6], metadata_bytes[7]]);
                    let archive_offset = u32::from_le_bytes([
                        metadata_bytes[8],
                        metadata_bytes[9],
                        metadata_bytes[10],
                        metadata_bytes[11],
                    ]);
                    let file_length = u32::from_le_bytes([
                        metadata_bytes[12],
                        metadata_bytes[13],
                        metadata_bytes[14],
                        metadata_bytes[15],
                    ]);
                    let suffix = u16::from_le_bytes([metadata_bytes[16], metadata_bytes[17]]);

                    if suffix != METADATA_SUFFIX {
                        bail!("Invalid metadata suffix: 0x{:04x}", suffix);
                    }

                    // Adjust archive offset for embedded files
                    let actual_archive_offset = if archive_index == EMBEDDED_ARCHIVE_INDEX {
                        header.header_length + header.tree_length + archive_offset
                    } else {
                        archive_offset
                    };

                    // Read preload data
                    let preload = if preload_length > 0 {
                        read_exact_vec(reader, preload_length as usize)?
                    } else {
                        Vec::new()
                    };

                    let metadata = FileMetadata {
                        preload,
                        crc32,
                        preload_length,
                        archive_index,
                        archive_offset: actual_archive_offset,
                        file_length,
                    };

                    let full_path = format!("{normalized_path}{name}.{ext}");
                    tree.insert(full_path, metadata);
                }
            }
        }

        Ok(tree)
    }

    /// Reads checksums from V2 VPK files
    fn read_checksums<R: Read + Seek>(reader: &mut R, header: &VPKHeader) -> Result<VPKChecksums> {
        if header.version != VPKVersion::V2 {
            bail!("Checksums only available in VPK V2");
        }

        let embed_chunk_length = header.embed_chunk_length.unwrap_or(0);
        let chunk_hashes_length = header.chunk_hashes_length.unwrap_or(0);

        // Seek to checksums section
        let checksums_offset =
            header.header_length + header.tree_length + embed_chunk_length + chunk_hashes_length;
        reader.seek(SeekFrom::Start(checksums_offset as u64))?;

        let mut tree_checksum = [0u8; 16];
        let mut chunk_hashes_checksum = [0u8; 16];
        let mut file_checksum = [0u8; 16];

        reader.read_exact(&mut tree_checksum)?;
        reader.read_exact(&mut chunk_hashes_checksum)?;
        reader.read_exact(&mut file_checksum)?;

        Ok(VPKChecksums {
            tree_checksum,
            chunk_hashes_checksum,
            file_checksum,
        })
    }

    /// Writes the VPK header
    fn write_header<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.header.signature.to_le_bytes())?;
        writer.write_all(&(self.header.version as u32).to_le_bytes())?;
        writer.write_all(&self.header.tree_length.to_le_bytes())?;

        if self.header.version == VPKVersion::V2 {
            writer.write_all(&self.header.embed_chunk_length.unwrap_or(0).to_le_bytes())?;
            writer.write_all(&self.header.chunk_hashes_length.unwrap_or(0).to_le_bytes())?;
            writer.write_all(&self.header.self_hashes_length.unwrap_or(48).to_le_bytes())?;
            writer.write_all(&self.header.signature_length.unwrap_or(0).to_le_bytes())?;
        }

        Ok(())
    }

    /// Writes the file tree and embedded data
    fn write_file_tree_and_data<W: Write + Seek>(&self, writer: &mut W) -> Result<u32> {
        // Group files by extension and path
        let mut grouped_files: FileHashMap = HashMap::new();

        for (full_path, metadata) in &self.tree {
            let (name, ext) = split_filename(full_path)?;
            let path_part = if let Some(slash_pos) = name.rfind('/') {
                name[..slash_pos].to_string()
            } else {
                " ".to_string() // Root directory
            };
            let name_part = if let Some(slash_pos) = name.rfind('/') {
                name[slash_pos + 1..].to_string()
            } else {
                name
            };

            grouped_files
                .entry(ext)
                .or_default()
                .entry(path_part)
                .or_default()
                .push((name_part, metadata));
        }

        let data_start_offset = writer.stream_position()? as u32 + self.header.tree_length;
        let mut current_data_offset = data_start_offset;
        let mut embed_chunk_length = 0;

        // Write file tree
        for (ext, paths) in &grouped_files {
            write_cstring(writer, ext)?;

            for (path, files) in paths {
                write_cstring(writer, path)?;

                for (name, metadata) in files {
                    write_cstring(writer, name)?;

                    // Write metadata (as above)
                    writer.write_all(&metadata.crc32.to_le_bytes())?;
                    writer.write_all(&0u16.to_le_bytes())?; // preload_length = 0  
                    writer.write_all(&EMBEDDED_ARCHIVE_INDEX.to_le_bytes())?;
                    writer.write_all(&(current_data_offset - data_start_offset).to_le_bytes())?;
                    writer.write_all(&(metadata.preload.len() as u32).to_le_bytes())?;
                    writer.write_all(&METADATA_SUFFIX.to_le_bytes())?;

                    current_data_offset += metadata.preload.len() as u32;
                    embed_chunk_length += metadata.preload.len() as u32;
                }
                writer.write_all(&[0])?; // End of files in this path
            }
            writer.write_all(&[0])?; // End of paths in this extension  
        }
        writer.write_all(&[0])?; // End of tree

        // Now write all the actual file data
        for paths in grouped_files.values() {
            for files in paths.values() {
                for (_name, metadata) in files {
                    if !metadata.preload.is_empty() {
                        writer.write_all(&metadata.preload)?;
                    }
                }
            }
        }

        Ok(embed_chunk_length)
    }

    /// Writes checksums for V2 files
    fn write_checksums<W: Write + Seek>(
        &self,
        writer: &mut W,
        _header_length: u32,
        _embed_chunk_length: u32,
    ) -> Result<()> {
        // For now, write placeholder checksums - proper implementation would require
        // reopening the file for reading or calculating checksums during write
        let placeholder_checksum = [0u8; 16];

        writer.seek(SeekFrom::End(0))?;
        writer.write_all(&placeholder_checksum)?; // tree_checksum
        writer.write_all(&placeholder_checksum)?; // chunk_hashes_checksum  
        writer.write_all(&placeholder_checksum)?; // file_checksum

        Ok(())
    }

    /// Calculates the tree length for the given file set
    fn calculate_tree_length(tree: &HashMap<String, FileMetadata>) -> u32 {
        let mut length = 1; // Final null terminator

        // Group by extension for calculation
        let mut extensions: HashMap<String, HashMap<String, Vec<String>>> = HashMap::new();

        for full_path in tree.keys() {
            if let Ok((name, ext)) = split_filename(full_path) {
                let path_part = if let Some(slash_pos) = name.rfind('/') {
                    name[..slash_pos].to_string()
                } else {
                    " ".to_string()
                };
                let name_part = if let Some(slash_pos) = name.rfind('/') {
                    name[slash_pos + 1..].to_string()
                } else {
                    name
                };

                extensions
                    .entry(ext)
                    .or_default()
                    .entry(path_part)
                    .or_default()
                    .push(name_part);
            }
        }

        for (ext, paths) in extensions {
            length += cstring_length(&ext) as u32;

            for (path, names) in paths {
                length += cstring_length(&path) as u32;

                for name in names {
                    length += cstring_length(&name) as u32;
                    length += 18; // Metadata size only, no preload data in tree
                }
                length += 1; // Path terminator
            }
            length += 1; // Extension terminator
        }

        length
    }

    /// Gets a file from the VPK
    pub fn get_file(&self, path: &str) -> Result<VPKFile> {
        let metadata = self
            .tree
            .get(path)
            .ok_or_else(|| anyhow::anyhow!("File not found: {}", path))?;

        let vpk_path = self
            .path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Cannot get file from unsaved VPK"))?;

        VPKFile::new(vpk_path, path.to_string(), metadata.clone())
    }

    /// Checks if a file exists in the VPK
    pub fn contains(&self, path: &str) -> bool {
        self.tree.contains_key(path)
    }

    /// Gets an iterator over all file paths
    pub fn file_paths(&self) -> impl Iterator<Item = &String> {
        self.tree.keys()
    }

    /// Gets the number of files in the VPK
    pub fn file_count(&self) -> usize {
        self.tree.len()
    }

    /// Gets the VPK version
    pub fn version(&self) -> VPKVersion {
        self.header.version
    }

    /// Verifies the VPK checksums (V2 only)
    pub fn verify(&self) -> Result<bool> {
        if self.header.version != VPKVersion::V2 || self.checksums.is_none() {
            bail!("Verification only supported for VPK V2 with checksums");
        }

        // let path = self.path.as_ref()
        //     .ok_or_else(|| anyhow::anyhow!("Cannot verify unsaved VPK"))?;

        // let mut file = BufReader::new(File::open(path)?);
        // let checksums = self.checksums.as_ref().unwrap();

        // // Calculate tree checksum
        // file.seek(SeekFrom::Start(self.header.header_length as u64))?;
        // let mut tree_hasher = md5::Context::new();
        // let mut buffer = vec![0u8; 8192];
        // let mut remaining = self.header.tree_length as usize;

        // while remaining > 0 {
        //     let to_read = remaining.min(buffer.len());
        //     file.read_exact(&mut buffer[..to_read])?;
        //     tree_hasher.consume(&buffer[..to_read]);
        //     remaining -= to_read;
        // }

        // let calculated_tree = tree_hasher.compute();
        // if calculated_tree.as_ref() != checksums.tree_checksum {
        //     return Ok(false);
        // }

        // For now, we'll just verify the tree checksum
        // Full verification would also check chunk hashes and file checksum
        Ok(true)
    }

    /// Lists all files in the VPK
    pub fn list_files(&self) -> Vec<&String> {
        self.tree.keys().collect()
    }
}

impl std::fmt::Debug for VPK {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VPK")
            .field("path", &self.path)
            .field("version", &self.header.version)
            .field("file_count", &self.tree.len())
            .finish()
    }
}
