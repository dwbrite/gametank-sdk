//! gtrom - GameTank ROM build tool
//!
//! A unified CLI for building, running, and managing GameTank ROM projects.

mod asm;
mod audio;
mod cargo;
mod container;
mod init;
mod rom_builder;

use std::path::PathBuf;
use std::process::Command;

use clap::{Parser, Subcommand};

use crate::asm::{build_asm, build_asm_in_container};
use crate::audio::do_audio_build;
use crate::cargo::{cargo_build, cargo_build_in_container, find_rom_dir, get_crate_name};
use crate::container::{ensure_container, is_in_container};
use crate::init::do_init;
use crate::rom_builder::RomBuilder;

#[derive(Parser)]
#[command(name = "gtrom")]
#[command(version, about = "GameTank ROM build tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build the ROM (handles container orchestration automatically)
    Build {
        /// Build in release mode
        #[arg(short, long, default_value_t = true)]
        release: bool,
    },

    /// Build audio coprocessor firmware
    Audio {
        /// Path to the audio firmware project directory
        path: String,
    },

    /// Convert an ELF binary to a .gtr ROM file
    Convert {
        /// Path to the ELF binary
        elf_path: String,

        /// Output .gtr file path
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Initialize a new GameTank project
    Init {
        /// Project directory (defaults to current directory)
        #[arg(default_value = ".")]
        path: String,

        /// Set the project name (defaults to directory name)
        #[arg(long)]
        name: Option<String>,

        /// Include audio firmware source (for customization)
        #[arg(long)]
        with_audiofw_src: bool,

        /// Audio firmware to use
        #[arg(long, default_value = "wavetable-8v")]
        audio: String,
    },

    /// Build and run in the emulator (gte)
    Run {},

    /// Build and flash to cartridge via gtld
    Flash {
        /// Serial port (auto-detected if not specified)
        #[arg(short, long)]
        port: Option<String>,
    },

    /// Build and open SDK documentation in your browser
    Docs {},
}

/// Convert ELF to GTR
fn convert_elf_to_gtr(elf_path: &str, output: &str) -> Result<(), String> {
    println!("Converting ELF to GTR: {} -> {}", elf_path, output);
    RomBuilder::build(elf_path.to_string(), output.to_string());
    Ok(())
}

/// Build and open SDK documentation
fn do_docs() -> Result<(), String> {
    let (_working_dir, rom_dir) = find_rom_dir()?;
    
    println!("Building documentation...");
    
    let status = Command::new("cargo")
        .args(["doc", "--document-private-items"])
        .current_dir(&rom_dir)
        .status()
        .map_err(|e| format!("Failed to run cargo doc: {}", e))?;
    
    if !status.success() {
        return Err("Failed to build documentation".to_string());
    }
    
    // Open the SDK docs directly
    let doc_path = rom_dir.join("target/doc/gametank/index.html");
    if !doc_path.exists() {
        return Err(format!("Documentation not found at {:?}", doc_path));
    }
    
    println!("Opening documentation...");
    open::that(&doc_path).map_err(|e| format!("Failed to open browser: {}", e))?;
    
    Ok(())
}

/// Full build process
fn do_build(release: bool) -> Result<PathBuf, String> {
    let (working_dir, rom_dir) = find_rom_dir()?;

    if is_in_container() {
        // Direct build inside container
        let rom_dir_str = rom_dir.to_string_lossy().to_string();
        build_asm(&rom_dir_str)?;
        cargo_build(&rom_dir_str, release)?;
    } else {
        // Orchestrate from outside container
        let (workspace_root, _runtime) = ensure_container()?;
        build_asm_in_container(&rom_dir, &workspace_root)?;
        cargo_build_in_container(&rom_dir, &workspace_root, release)?;
    }

    let crate_name = get_crate_name(&rom_dir)?;

    // Convert to GTR (runs on host, doesn't need llvm)
    let profile = if release { "release" } else { "debug" };
    let elf_path = rom_dir.join(format!("target/mos-unknown-none/{}/{}", profile, crate_name));
    let gtr_path = working_dir.join(format!("{}.gtr", crate_name));
    
    convert_elf_to_gtr(
        elf_path.to_str().unwrap(),
        gtr_path.to_str().unwrap(),
    )?;

    println!("Build complete: {}", gtr_path.display());
    Ok(gtr_path)
}

fn main() {
    let cli = Cli::parse();

    let result: Result<(), String> = match cli.command {
        Commands::Build { release } => {
            do_build(release).map(|_| ())
        }
        
        Commands::Audio { path } => {
            do_audio_build(&path)
        }
        
        Commands::Convert { elf_path, output } => {
            let out = output.unwrap_or_else(|| "game.gtr".to_string());
            convert_elf_to_gtr(&elf_path, &out)
        }

        Commands::Init { path, name, with_audiofw_src, audio } => {
            do_init(&path, name.as_deref(), with_audiofw_src, &audio)
        }
        
        Commands::Run {} => {
            do_build(true).and_then(|gtr_path| {
                // Launch emulator
                println!("Launching emulator...");
                let status = Command::new("gte")
                    .arg(&gtr_path)
                    .status()
                    .map_err(|e| format!("Failed to launch gte: {}", e))?;
                
                if status.success() {
                    Ok(())
                } else {
                    Err("Emulator exited with error".to_string())
                }
            })
        }
        
        Commands::Flash { port } => {
            do_build(true).and_then(|gtr_path| {
                // Flash via gtld
                println!("Flashing to cartridge...");
                let gtr_str = gtr_path.to_string_lossy().to_string();
                let mut args = vec!["load".to_string(), gtr_str];
                if let Some(ref p) = port {
                    args.push("--port".to_string());
                    args.push(p.clone());
                }
                
                let status = Command::new("gtld")
                    .args(&args)
                    .status()
                    .map_err(|e| format!("Failed to run gtld: {}", e))?;
                
                if status.success() {
                    Ok(())
                } else {
                    Err("Flash failed".to_string())
                }
            })
        }

        Commands::Docs {} => {
            do_docs()
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
