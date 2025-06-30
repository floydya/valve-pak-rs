use anyhow::Result;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use tempfile::TempDir;
use valve_pak::VPK;

/// Helper function to create a test directory with sample files
fn create_test_directory(base_path: &std::path::Path) -> Result<()> {
    fs::create_dir_all(base_path)?;
    fs::create_dir_all(base_path.join("scripts"))?;
    fs::create_dir_all(base_path.join("textures"))?;
    fs::create_dir_all(base_path.join("sounds"))?;

    // Create various test files
    fs::write(
        base_path.join("readme.txt"),
        "This is a test readme file.\nSecond line.\n",
    )?;
    fs::write(
        base_path.join("config.cfg"),
        "setting1=value1\nsetting2=value2\n",
    )?;
    fs::write(
        base_path.join("scripts/test.lua"),
        "print('Hello from Lua')\n",
    )?;
    fs::write(base_path.join("textures/test.dds"), vec![0u8; 256])?; // Binary file
    fs::write(
        base_path.join("sounds/beep.wav"),
        [0x52, 0x49, 0x46, 0x46, 0x24, 0x00, 0x00, 0x00],
    )?; // WAV header

    Ok(())
}

#[test]
fn test_create_and_read_vpk() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let source_dir = temp_dir.path().join("source");
    let vpk_path = temp_dir.path().join("test.vpk");

    // Create test files
    create_test_directory(&source_dir)?;

    // Create VPK
    let vpk = VPK::from_directory(&source_dir)?;
    assert_eq!(vpk.file_count(), 5);
    vpk.save(&vpk_path)?;

    // Read VPK back
    let vpk = VPK::open(&vpk_path)?;
    assert_eq!(vpk.file_count(), 5);
    assert!(vpk.contains("readme.txt"));
    assert!(vpk.contains("scripts/test.lua"));
    assert!(vpk.contains("textures/test.dds"));

    Ok(())
}

#[test]
fn test_file_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let source_dir = temp_dir.path().join("source");
    let vpk_path = temp_dir.path().join("test.vpk");

    // Create test files
    create_test_directory(&source_dir)?;

    // Create and save VPK
    let vpk = VPK::from_directory(&source_dir)?;
    vpk.save(&vpk_path)?;

    // Open VPK and test file operations
    let vpk = VPK::open(&vpk_path)?;
    let mut file = vpk.get_file("readme.txt")?;

    // Test reading
    let content = file.read_all_string()?;
    assert_eq!(content, "This is a test readme file.\nSecond line.\n");

    // Test seeking
    file.seek(SeekFrom::Start(5))?;
    let mut buffer = [0u8; 2];
    file.read_exact(&mut buffer)?;
    assert_eq!(&buffer, b"is");

    // Test file length
    assert!(file.length() > 0);

    Ok(())
}

#[test]
fn test_file_verification() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let source_dir = temp_dir.path().join("source");
    let vpk_path = temp_dir.path().join("test.vpk");

    // Create test files
    create_test_directory(&source_dir)?;

    // Create and save VPK
    let vpk = VPK::from_directory(&source_dir)?;
    vpk.save(&vpk_path)?;

    // Open VPK and verify files
    let vpk = VPK::open(&vpk_path)?;
    let mut file = vpk.get_file("readme.txt")?;

    // Verify file integrity
    assert!(file.verify()?);

    Ok(())
}

#[test]
fn test_binary_file_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let source_dir = temp_dir.path().join("source");
    let vpk_path = temp_dir.path().join("test.vpk");

    // Create test files
    create_test_directory(&source_dir)?;

    // Create and save VPK
    let vpk = VPK::from_directory(&source_dir)?;
    vpk.save(&vpk_path)?;

    // Open VPK and test binary file
    let vpk = VPK::open(&vpk_path)?;
    let mut file = vpk.get_file("textures/test.dds")?;

    // Read binary data
    let data = file.read_all()?;
    assert_eq!(data.len(), 256);
    assert!(data.iter().all(|&b| b == 0));

    Ok(())
}

#[test]
fn test_extract_all_files() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let source_dir = temp_dir.path().join("source");
    let vpk_path = temp_dir.path().join("test.vpk");
    let extract_dir = temp_dir.path().join("extracted");

    // Create test files
    create_test_directory(&source_dir)?;

    // Create and save VPK
    let vpk = VPK::from_directory(&source_dir)?;
    vpk.save(&vpk_path)?;

    // Open VPK and extract all files
    let vpk = VPK::open(&vpk_path)?;
    fs::create_dir_all(&extract_dir)?;

    for file_path in vpk.file_paths() {
        let mut vpk_file = vpk.get_file(file_path)?;
        let output_path = extract_dir.join(file_path);

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        vpk_file.save(&output_path)?;
    }

    // Verify extracted files
    assert!(extract_dir.join("readme.txt").exists());
    assert!(extract_dir.join("scripts/test.lua").exists());

    let extracted_content = fs::read_to_string(extract_dir.join("readme.txt"))?;
    assert_eq!(
        extracted_content,
        "This is a test readme file.\nSecond line.\n"
    );

    Ok(())
}

#[test]
fn test_vpk_version_info() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let source_dir = temp_dir.path().join("source");
    let vpk_path = temp_dir.path().join("test.vpk");

    // Create test files
    create_test_directory(&source_dir)?;

    // Create and save VPK
    let vpk = VPK::from_directory(&source_dir)?;
    assert_eq!(vpk.version(), valve_pak::vpk::VPKVersion::V2);
    vpk.save(&vpk_path)?;

    // Open VPK and check version
    let vpk = VPK::open(&vpk_path)?;
    assert_eq!(vpk.version(), valve_pak::vpk::VPKVersion::V2);

    Ok(())
}

#[test]
fn test_empty_directory() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let empty_dir = temp_dir.path().join("empty");
    let vpk_path = temp_dir.path().join("empty.vpk");

    fs::create_dir_all(&empty_dir)?;

    // Create VPK from empty directory
    let vpk = VPK::from_directory(&empty_dir)?;
    assert_eq!(vpk.file_count(), 0);
    vpk.save(&vpk_path)?;

    // Open empty VPK
    let vpk = VPK::open(&vpk_path)?;
    assert_eq!(vpk.file_count(), 0);
    assert!(vpk.list_files().is_empty());

    Ok(())
}

#[test]
fn test_large_file() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let source_dir = temp_dir.path().join("source");
    let vpk_path = temp_dir.path().join("large.vpk");

    fs::create_dir_all(&source_dir)?;

    // Create a large file (10KB)
    let large_data: Vec<u8> = (0..10240).map(|i| (i % 256) as u8).collect();
    fs::write(source_dir.join("large.bin"), &large_data)?;

    // Create and save VPK
    let vpk = VPK::from_directory(&source_dir)?;
    vpk.save(&vpk_path)?;

    // Open VPK and verify large file
    let vpk = VPK::open(&vpk_path)?;
    let mut file = vpk.get_file("large.bin")?;

    assert_eq!(file.length(), 10240);

    let read_data = file.read_all()?;
    assert_eq!(read_data.len(), large_data.len());
    assert_eq!(read_data, large_data);

    Ok(())
}

#[test]
fn test_file_with_no_extension_fails() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let source_dir = temp_dir.path().join("source");

    fs::create_dir_all(&source_dir)?;
    fs::write(
        source_dir.join("no_extension"),
        "This file has no extension",
    )?;

    // Should fail to create VPK due to file without extension
    let result = VPK::from_directory(&source_dir);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_special_characters_in_filenames() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let source_dir = temp_dir.path().join("source");
    let vpk_path = temp_dir.path().join("special.vpk");

    fs::create_dir_all(&source_dir)?;
    fs::create_dir_all(source_dir.join("test dir"))?;

    // Create files with various characters
    fs::write(source_dir.join("test-file.txt"), "test content")?;
    fs::write(source_dir.join("test_file.txt"), "test content")?;
    fs::write(source_dir.join("test dir/nested.txt"), "nested content")?;

    // Create and save VPK
    let vpk = VPK::from_directory(&source_dir)?;
    vpk.save(&vpk_path)?;

    // Open VPK and verify files
    let vpk = VPK::open(&vpk_path)?;
    assert!(vpk.contains("test-file.txt"));
    assert!(vpk.contains("test_file.txt"));
    assert!(vpk.contains("test dir/nested.txt"));

    Ok(())
}
