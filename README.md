# Valve Pak (VPK) - Valve Pak File Library and CLI Tool

A Rust library and command-line tool for reading, writing, and manipulating Valve Pak (VPK) files used by Valve's Source engine games.

## Features

- **Full VPK Format Support**: Both VPK v1 and v2 formats
- **Unified API**: Single struct handles both reading and writing operations
- **Checksum Verification**: MD5 checksum verification for VPK v2 files
- **High Performance**: Efficient I/O with buffered readers/writers and streaming
- **Memory Safe**: Written in Rust with comprehensive error handling
- **CLI Tool**: Complete command-line interface for common operations
- **File-like Access**: VPKFile implements standard Read/Seek traits

## Installation

### From Source

```bash
git clone <repository-url>
cd valve-pak-rs
cargo build --release
```

The compiled binary will be available at `target/release/valve_pak`.

### As a Library

Add to your `Cargo.toml`:

```bash
cargo add valve_pak
```

## CLI Usage

### Pack a directory into a VPK file

```bash
valve_pak pack <directory> <output.vpk> [--verbose]
```

Example:
```bash
valve_pak pack my_mod/ my_mod.vpk --verbose
```

### Unpack a VPK file to a directory

```bash
valve_pak unpack <input.vpk> <output_directory> [--verbose]
```

Example:
```bash
valve_pak unpack game_assets.vpk extracted/ --verbose
```

### List files in a VPK

```bash
valve_pak list <input.vpk> [--detailed]
```

Examples:
```bash
valve_pak list game_assets.vpk
valve_pak list game_assets.vpk --detailed  # Shows file sizes and CRC32
```

### Verify VPK checksums and file integrity

```bash
valve_pak verify <input.vpk>
```

Example:
```bash
valve_pak verify game_assets.vpk
```

### Extract a single file from VPK

```bash
valve_pak extract <input.vpk> <file_path> <output_file>
```

Example:
```bash
valve_pak extract game_assets.vpk scripts/game.txt extracted_game.txt
```

## Library Usage

### Basic Operations

```rust
use valve_pak::{VPK, Result};

fn main() -> Result<()> {
    // Open an existing VPK file
    let vpk = VPK::open("game_assets.vpk")?;
    
    // List all files
    for file_path in vpk.file_paths() {
        println!("{}", file_path);
    }
    
    // Get a specific file
    let mut file = vpk.get_file("scripts/game.txt")?;
    
    // Read file content
    let content = file.read_all_string()?;
    println!("File content: {}", content);
    
    // Save file to disk
    file.save("extracted_game.txt")?;
    
    Ok(())
}
```

### Creating VPK files

```rust
use valve_pak::{VPK, Result};

fn main() -> Result<()> {
    // Create VPK from directory
    let vpk = VPK::from_directory("my_mod/")?;
    
    // Save to file
    vpk.save("my_mod.vpk")?;
    
    println!("Packed {} files", vpk.file_count());
    Ok(())
}
```

### File Operations

```rust
use valve_pak::{VPK, Result};
use std::io::{Read, Seek, SeekFrom};

fn main() -> Result<()> {
    let vpk = VPK::open("game_assets.vpk")?;
    let mut file = vpk.get_file("textures/logo.png")?;
    
    // VPKFile implements Read and Seek traits
    let mut buffer = [0u8; 1024];
    let bytes_read = file.read(&mut buffer)?;
    
    // Seek to position
    file.seek(SeekFrom::Start(100))?;
    
    // Verify file integrity
    if file.verify()? {
        println!("File checksum is valid");
    }
    
    Ok(())
}
```

### Error Handling

```rust
use valve_pak::{VPK, Result};

fn main() -> Result<()> {
    match VPK::open("nonexistent.vpk") {
        Ok(vpk) => {
            println!("Opened VPK with {} files", vpk.file_count());
        }
        Err(e) => {
            eprintln!("Failed to open VPK: {}", e);
            // Error context is preserved through the chain
            for cause in e.chain() {
                eprintln!("  Caused by: {}", cause);
            }
        }
    }
    Ok(())
}
```

## VPK Format Support

### Version 1 (V1)
- Basic file tree structure
- CRC32 checksums for individual files
- No global checksums

### Version 2 (V2)
- Extended header with metadata
- MD5 checksums for entire archive
- Support for chunk hashes
- Backward compatible with V1

## Performance

The library is optimized for performance:

- **Streaming I/O**: Uses buffered readers/writers for efficient file operations
- **Lazy Loading**: File tree is loaded on demand when needed
- **Memory Efficient**: Large files are streamed rather than loaded entirely into memory
- **Zero-Copy**: Minimal data copying during read operations

## Error Handling

All operations return `Result<T>` types with descriptive error messages using the `anyhow` crate. Errors include full context chains to help with debugging.

## Testing

Run the test suite:

```bash
cargo test
```

Run tests with output:

```bash
cargo test -- --nocapture
```

## Dependencies

- `anyhow` - Error handling with context
- `clap` - Command line argument parsing
- `md5` - MD5 checksum calculation (VPK v2)
- `crc32fast` - Fast CRC32 calculation
- `walkdir` - Recursive directory traversal

## License

MIT License - see LICENSE file for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes with tests
4. Submit a pull request
