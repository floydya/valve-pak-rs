use anyhow::Result;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use tempfile::TempDir;
use valve_pak::VPK;

fn main() -> Result<()> {
    println!("VPK Library Example");
    println!("===================");

    // Create a temporary directory for our example
    let temp_dir = TempDir::new()?;
    let source_dir = temp_dir.path().join("source");
    let vpk_path = temp_dir.path().join("example.vpk");
    let extract_dir = temp_dir.path().join("extracted");

    // Step 1: Create some example files
    println!("\n1. Creating example files...");
    create_example_files(&source_dir)?;

    // Step 2: Pack directory into VPK
    println!("\n2. Packing directory into VPK...");
    let vpk = VPK::from_directory(&source_dir)?;
    println!("   Found {} files to pack", vpk.file_count());
    vpk.save(&vpk_path)?;
    println!("   Saved VPK to: {}", vpk_path.display());

    // Step 3: Open and inspect the VPK
    println!("\n3. Opening and inspecting VPK...");
    let vpk = VPK::open(&vpk_path)?;
    println!("   VPK Version: {:?}", vpk.version());
    println!("   Total files: {}", vpk.file_count());

    println!("   File listing:");
    for file_path in vpk.file_paths() {
        println!("     - {file_path}");
    }

    // Step 4: Extract and examine individual files
    println!("\n4. Extracting individual files...");

    // Extract a text file and read its content
    if vpk.contains("readme.txt") {
        let mut file = vpk.get_file("readme.txt")?;
        println!("   File: readme.txt");
        println!("     Size: {} bytes", file.length());
        println!("     CRC32: 0x{:08x}", file.metadata().crc32);

        // Read content as string
        let content = file.read_all_string()?;
        println!("     Content: {}", content.trim());

        // Verify file integrity
        if file.verify()? {
            println!("     ✓ File integrity verified");
        } else {
            println!("     ✗ File integrity check failed");
        }
    }

    // Step 5: Extract binary file with custom reading
    if vpk.contains("data/binary.dat") {
        let mut file = vpk.get_file("data/binary.dat")?;
        println!("\n   File: data/binary.dat");
        println!("     Size: {} bytes", file.length());

        // Read first 4 bytes as a u32
        let mut buffer = [0u8; 4];
        file.read_exact(&mut buffer)?;
        let magic = u32::from_le_bytes(buffer);
        println!("     Magic number: 0x{magic:08x}");

        // Seek to middle and read some data
        file.seek(SeekFrom::Start(10))?;
        let mut middle_buffer = [0u8; 5];
        file.read_exact(&mut middle_buffer)?;
        println!("     Bytes 10-14: {middle_buffer:?}");
    }

    // Step 6: Extract all files
    println!("\n5. Extracting all files...");
    fs::create_dir_all(&extract_dir)?;

    for file_path in vpk.file_paths() {
        let mut vpk_file = vpk.get_file(file_path)?;
        let output_path = extract_dir.join(file_path);

        // Create parent directories
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        vpk_file.save(&output_path)?;
        println!("   Extracted: {file_path}");
    }

    // Step 7: Verify the VPK
    println!("\n6. Verifying VPK integrity...");
    match vpk.verify() {
        Ok(true) => println!("   ✓ VPK verification passed"),
        Ok(false) => println!("   ✗ VPK verification failed"),
        Err(e) => println!("   ⚠ VPK verification not supported: {e}"),
    }

    println!("\n✓ Example completed successfully!");
    println!("Temporary files created in: {}", temp_dir.path().display());

    Ok(())
}

fn create_example_files(source_dir: &std::path::Path) -> Result<()> {
    fs::create_dir_all(source_dir)?;
    fs::create_dir_all(source_dir.join("data"))?;
    fs::create_dir_all(source_dir.join("scripts"))?;
    fs::create_dir_all(source_dir.join("textures"))?;

    // Create a text file
    fs::write(
        source_dir.join("readme.txt"),
        "This is a sample text file for VPK testing.\nIt contains multiple lines.\n",
    )?;

    // Create a config file
    fs::write(
        source_dir.join("config.cfg"),
        "# Configuration file\nwidth=1920\nheight=1080\nfullscreen=true\n",
    )?;

    // Create some data files
    fs::write(
        source_dir.join("data/binary.dat"),
        [
            0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x48, 0x65, 0x6C, 0x6C,
            0x6F,
        ], // "Hello" at the end
    )?;

    // Create a script file
    fs::write(
        source_dir.join("scripts/startup.lua"),
        r#"-- Startup script
print("Game starting...")
local config = loadConfig("config.cfg")
initializeGame(config)
"#,
    )?;

    // Create a fake texture file
    fs::write(
        source_dir.join("textures/player.dds"),
        vec![0x44, 0x44, 0x53, 0x20] // DDS header magic
            .into_iter()
            .chain(vec![0; 100]) // Fake texture data
            .collect::<Vec<u8>>(),
    )?;

    println!("   Created example files in: {}", source_dir.display());
    Ok(())
}
