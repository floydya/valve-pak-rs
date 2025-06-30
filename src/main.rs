use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use valve_pak::VPK;

#[derive(Parser)]
#[command(name = "vpk")]
#[command(about = "A CLI tool for working with Valve Pak (VPK) files")]
#[command(version = "1.4.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Pack a directory into a VPK file
    Pack {
        /// Directory to pack
        directory: PathBuf,
        /// Output VPK file path
        output: PathBuf,
        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    /// Unpack a VPK file to a directory
    Unpack {
        /// VPK file to unpack
        input: PathBuf,
        /// Output directory
        output: PathBuf,
        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    /// List files in a VPK
    List {
        /// VPK file to list
        input: PathBuf,
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },
    /// Verify VPK checksums (V2 only)
    Verify {
        /// VPK file to verify
        input: PathBuf,
    },
    /// Extract a single file from VPK
    Extract {
        /// VPK file to extract from
        input: PathBuf,
        /// File path within the VPK
        file_path: String,
        /// Output file path
        output: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Pack {
            directory,
            output,
            verbose,
        } => pack_command(directory, output, verbose),
        Commands::Unpack {
            input,
            output,
            verbose,
        } => unpack_command(input, output, verbose),
        Commands::List { input, detailed } => list_command(input, detailed),
        Commands::Verify { input } => verify_command(input),
        Commands::Extract {
            input,
            file_path,
            output,
        } => extract_command(input, file_path, output),
    }
}

fn pack_command(directory: PathBuf, output: PathBuf, verbose: bool) -> Result<()> {
    if !directory.is_dir() {
        anyhow::bail!("Input path is not a directory: {}", directory.display());
    }

    if verbose {
        println!("Packing directory: {}", directory.display());
    }

    let vpk = VPK::from_directory(&directory).with_context(|| {
        format!(
            "Failed to create VPK from directory: {}",
            directory.display()
        )
    })?;

    if verbose {
        println!("Found {} files", vpk.file_count());
        println!("Writing VPK to: {}", output.display());
    }

    vpk.save(&output)
        .with_context(|| format!("Failed to save VPK to: {}", output.display()))?;

    println!(
        "Successfully packed {} files into {}",
        vpk.file_count(),
        output.display()
    );
    Ok(())
}

fn unpack_command(input: PathBuf, output: PathBuf, verbose: bool) -> Result<()> {
    if !input.is_file() {
        anyhow::bail!("Input path is not a file: {}", input.display());
    }

    if verbose {
        println!("Opening VPK: {}", input.display());
    }

    let vpk =
        VPK::open(&input).with_context(|| format!("Failed to open VPK: {}", input.display()))?;

    if verbose {
        println!("VPK version: {:?}", vpk.version());
        println!("Found {} files", vpk.file_count());
        println!("Extracting to: {}", output.display());
    }

    // Create output directory if it doesn't exist
    fs::create_dir_all(&output)
        .with_context(|| format!("Failed to create output directory: {}", output.display()))?;

    let mut extracted_count = 0;
    for file_path in vpk.file_paths() {
        if verbose {
            println!("Extracting: {file_path}");
        }

        let mut vpk_file = vpk
            .get_file(file_path)
            .with_context(|| format!("Failed to get file: {file_path}"))?;

        let output_file_path = output.join(file_path);

        // Create parent directories if needed
        if let Some(parent) = output_file_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create parent directory: {}", parent.display())
            })?;
        }

        vpk_file
            .save(&output_file_path)
            .with_context(|| format!("Failed to extract file: {file_path}"))?;

        extracted_count += 1;
    }

    println!(
        "Successfully extracted {} files to {}",
        extracted_count,
        output.display()
    );
    Ok(())
}

fn list_command(input: PathBuf, detailed: bool) -> Result<()> {
    if !input.is_file() {
        anyhow::bail!("Input path is not a file: {}", input.display());
    }

    let vpk =
        VPK::open(&input).with_context(|| format!("Failed to open VPK: {}", input.display()))?;

    println!("VPK: {}", input.display());
    println!("Version: {:?}", vpk.version());
    println!("Files: {}", vpk.file_count());
    println!();

    if detailed {
        println!("{:<50} {:>10} {:>10}", "Path", "Size", "CRC32");
        println!("{}", "-".repeat(75));
    }

    let mut files: Vec<_> = vpk.file_paths().collect();
    files.sort();

    for file_path in files {
        if detailed {
            if let Ok(vpk_file) = vpk.get_file(file_path) {
                println!(
                    "{:<50} {:>10} {:>10x}",
                    file_path,
                    vpk_file.length(),
                    vpk_file.metadata().crc32
                );
            }
        } else {
            println!("{file_path}");
        }
    }

    Ok(())
}

fn verify_command(input: PathBuf) -> Result<()> {
    if !input.is_file() {
        anyhow::bail!("Input path is not a file: {}", input.display());
    }

    let vpk =
        VPK::open(&input).with_context(|| format!("Failed to open VPK: {}", input.display()))?;

    match vpk.version() {
        valve_pak::vpk::VPKVersion::V1 => {
            println!("VPK V1 files do not support checksum verification");
            return Ok(());
        }
        valve_pak::vpk::VPKVersion::V2 => {
            print!("Verifying VPK checksums... ");
            io::stdout().flush()?;

            match vpk.verify() {
                Ok(true) => println!("✓ VPK checksums are valid"),
                Ok(false) => {
                    println!("✗ VPK checksums are invalid");
                    std::process::exit(1);
                }
                Err(e) => {
                    println!("✗ Failed to verify: {e}");
                    std::process::exit(1);
                }
            }
        }
    }

    // Also verify individual files
    println!("Verifying individual files...");
    let mut verified = 0;
    let mut failed = 0;

    for file_path in vpk.file_paths() {
        if let Ok(mut vpk_file) = vpk.get_file(file_path) {
            match vpk_file.verify() {
                Ok(true) => {
                    verified += 1;
                    print!(".");
                }
                Ok(false) => {
                    failed += 1;
                    println!("\n✗ CRC mismatch: {file_path}");
                }
                Err(_) => {
                    failed += 1;
                    println!("\n✗ Failed to verify: {file_path}");
                }
            }
        }

        if (verified + failed) % 50 == 0 {
            println!();
        }
    }

    println!();
    println!("Verification complete: {verified} verified, {failed} failed");

    if failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

fn extract_command(input: PathBuf, file_path: String, output: PathBuf) -> Result<()> {
    if !input.is_file() {
        anyhow::bail!("Input path is not a file: {}", input.display());
    }

    let vpk =
        VPK::open(&input).with_context(|| format!("Failed to open VPK: {}", input.display()))?;

    if !vpk.contains(&file_path) {
        anyhow::bail!("File not found in VPK: {}", file_path);
    }

    let mut vpk_file = vpk
        .get_file(&file_path)
        .with_context(|| format!("Failed to get file: {file_path}"))?;

    // Create parent directories if needed
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent directory: {}", parent.display()))?;
    }

    vpk_file
        .save(&output)
        .with_context(|| format!("Failed to extract file to: {}", output.display()))?;

    println!(
        "Successfully extracted '{}' to {}",
        file_path,
        output.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_pack_and_unpack() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let src_dir = temp_dir.path().join("source");
        let vpk_path = temp_dir.path().join("test.vpk");
        let extract_dir = temp_dir.path().join("extracted");

        // Create test files
        fs::create_dir_all(&src_dir)?;
        fs::write(src_dir.join("test.txt"), b"Hello, World!")?;
        fs::create_dir_all(src_dir.join("subdir"))?;
        fs::write(
            src_dir.join("subdir").join("nested.dat"),
            b"Nested file content",
        )?;

        // Pack
        pack_command(src_dir.clone(), vpk_path.clone(), false)?;
        assert!(vpk_path.exists());

        // Unpack
        unpack_command(vpk_path, extract_dir.clone(), false)?;

        // Verify extracted files
        assert_eq!(
            fs::read_to_string(extract_dir.join("test.txt"))?,
            "Hello, World!"
        );
        assert_eq!(
            fs::read_to_string(extract_dir.join("subdir").join("nested.dat"))?,
            "Nested file content"
        );

        Ok(())
    }
}
