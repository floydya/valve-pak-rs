use anyhow::{Context, Result};
use std::io::{BufRead, Read};

/// VPK magic signature
pub const VPK_SIGNATURE: u32 = 0x55aa1234;

/// Maximum archive index for embedded files
pub const EMBEDDED_ARCHIVE_INDEX: u16 = 0x7fff;

/// Suffix value for valid metadata entries
pub const METADATA_SUFFIX: u16 = 0xffff;

/// Reads a null-terminated string from the reader
pub fn read_cstring<R: Read>(reader: &mut R) -> Result<String> {
    let mut buffer = Vec::new();
    let mut byte = [0u8; 1];

    loop {
        reader
            .read_exact(&mut byte)
            .context("Failed to read byte for cstring")?;

        if byte[0] == 0 {
            break;
        }

        buffer.push(byte[0]);
    }

    String::from_utf8(buffer).context("Invalid UTF-8 in cstring")
}

/// Reads a null-terminated string from a buffered reader (more efficient)
pub fn read_cstring_buffered<R: BufRead>(reader: &mut R) -> Result<String> {
    let mut buffer = Vec::new();

    reader
        .read_until(0, &mut buffer)
        .context("Failed to read cstring")?;

    // Remove the null terminator
    if buffer.last() == Some(&0) {
        buffer.pop();
    }

    String::from_utf8(buffer).context("Invalid UTF-8 in cstring")
}

/// Writes a null-terminated string to the writer
pub fn write_cstring<W: std::io::Write>(writer: &mut W, s: &str) -> Result<()> {
    writer
        .write_all(s.as_bytes())
        .context("Failed to write string")?;
    writer
        .write_all(&[0])
        .context("Failed to write null terminator")?;
    Ok(())
}

/// Calculates the length needed to store a null-terminated string
pub fn cstring_length(s: &str) -> usize {
    s.len() + 1
}

/// Normalizes a path for VPK storage (uses forward slashes)
pub fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

/// Splits a filename into name and extension parts
pub fn split_filename(filename: &str) -> Result<(String, String)> {
    if let Some(dot_pos) = filename.rfind('.') {
        let name = filename[..dot_pos].to_string();
        let ext = filename[dot_pos + 1..].to_string();
        Ok((name, ext))
    } else {
        anyhow::bail!("Files without an extension are not supported: {}", filename);
    }
}

/// Joins filename parts back together
pub fn join_filename(name: &str, ext: &str) -> String {
    if ext.is_empty() {
        name.to_string()
    } else {
        format!("{name}.{ext}")
    }
}

/// Reads exactly n bytes from reader into a new Vec
pub fn read_exact_vec<R: Read>(reader: &mut R, count: usize) -> Result<Vec<u8>> {
    let mut buffer = vec![0u8; count];
    reader
        .read_exact(&mut buffer)
        .context("Failed to read exact bytes")?;
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_cstring_operations() -> Result<()> {
        let test_str = "hello world";
        let mut buffer = Vec::new();

        write_cstring(&mut buffer, test_str)?;
        assert_eq!(buffer, b"hello world\0");

        let mut cursor = Cursor::new(buffer);
        let read_str = read_cstring(&mut cursor)?;
        assert_eq!(read_str, test_str);

        Ok(())
    }

    #[test]
    fn test_filename_split() -> Result<()> {
        let (name, ext) = split_filename("test.txt")?;
        assert_eq!(name, "test");
        assert_eq!(ext, "txt");

        let (name, ext) = split_filename("complex.file.name.dat")?;
        assert_eq!(name, "complex.file.name");
        assert_eq!(ext, "dat");

        Ok(())
    }

    #[test]
    fn test_path_normalization() {
        assert_eq!(normalize_path("path\\to\\file"), "path/to/file");
        assert_eq!(normalize_path("path/to/file"), "path/to/file");
    }
}
