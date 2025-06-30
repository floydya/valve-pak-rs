# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-06-30

### Added
- Complete Rust implementation of VPK library
- Support for both VPK v1 and v2 formats
- Unified VPK struct for reading and writing operations
- Full CLI tool with pack, unpack, list, verify, and extract commands
- MD5 checksum verification for VPK v2 files
- CRC32 verification for individual files
- File-like access with Read and Seek traits
- Comprehensive error handling with anyhow
- Extensive test suite covering all major functionality
- Performance benchmarks
- Documentation and examples

### Features
- **Library Features:**
  - Read existing VPK files
  - Create new VPK files from directories
  - Extract individual files or entire archives
  - Verify file integrity with checksums
  - Stream large files efficiently
  - Memory-safe operations with proper error handling

- **CLI Features:**
  - `vpk pack <directory> <output.vpk>` - Pack directory into VPK
  - `vpk unpack <input.vpk> <output_dir>` - Extract VPK to directory
  - `vpk list <input.vpk>` - List files in VPK
  - `vpk verify <input.vpk>` - Verify VPK and file checksums
  - `vpk extract <input.vpk> <file> <output>` - Extract single file
  - Verbose output options
  - Detailed file information display

- **Performance Optimizations:**
  - Buffered I/O for efficient file operations
  - Lazy loading of file trees
  - Streaming for large files
  - Minimal memory footprint

### Technical Details
- **Dependencies:**
  - `anyhow` for error handling
  - `clap` for CLI parsing
  - `md5` for VPK v2 checksums
  - `crc32fast` for file verification
  - `walkdir` for directory traversal

- **Supported Platforms:**
  - Linux (x86_64)
  - Windows (x86_64)
  - macOS (x86_64)

### Documentation
- Comprehensive README with usage examples
- API documentation with rustdoc
- Basic usage example
- Build scripts and automation
- CI/CD pipeline with GitHub Actions

### Testing
- Unit tests for all major components
- Integration tests for CLI operations
- Property-based testing for edge cases
- Performance benchmarks
- Cross-platform testing

## [Unreleased]

### Planned
- Recursive VPK unpack
- More efficient checksum calculation during write
- Enhanced verification
- Additional CLI options and features
- Performance improvements for very large VPK files
