pub mod builder;

use std::process::Command;
use std::path::{Path, PathBuf};
use std::io::Cursor;

use clap::{Parser, Subcommand};
use flate2::read::GzDecoder;
use tar::Archive;

use crate::builder::RomBuilder;

// Embed the SDK template tarball at compile time
static SDK_TEMPLATE: &[u8] = include_bytes!("../sdk-template.tar.gz");

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
}

/// Check if we're running inside a container
fn is_in_container() -> bool {
    Path::new("/.dockerenv").exists()
        || Path::new("/run/.containerenv").exists()
        || std::env::var("container").is_ok()
}

/// Find the workspace root (where .git or root Cargo.toml is)
fn find_workspace_root() -> Result<std::path::PathBuf, String> {
    let mut current = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;
    
    loop {
        // Check for .git directory (repo root)
        if current.join(".git").exists() {
            return Ok(current);
        }
        // Check for workspace Cargo.toml with [workspace] section
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                if content.contains("[workspace]") {
                    return Ok(current);
                }
            }
        }
        
        if !current.pop() {
            break;
        }
    }
    
    // Fallback to current dir
    std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))
}

/// Ensure the build container is running
fn ensure_container() -> Result<std::path::PathBuf, String> {
    let workspace_root = find_workspace_root()?;
    
    // Check if container is already running
    let output = Command::new("podman")
        .args(["ps", "--filter", "name=gametank", "--filter", "status=running", "--format", "{{.Names}}"])
        .output()
        .map_err(|e| format!("Failed to check container status: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.contains("gametank") {
        return Ok(workspace_root);
    }

    // Start the container
    println!("Starting build container...");
    let status = Command::new("podman")
        .args([
            "run", "-d",
            "--name", "gametank",
            "-v", &format!("{}:/workspace:z", workspace_root.display()),
            "--replace",
            "rust-mos:gte",
            "sleep", "infinity"
        ])
        .status()
        .map_err(|e| format!("Failed to start container: {}", e))?;

    if status.success() {
        Ok(workspace_root)
    } else {
        Err("Failed to start build container".to_string())
    }
}

/// Execute a command inside the container
fn podman_exec(workdir: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new("podman")
        .args(["exec", "-t", "-w", workdir, "gametank"])
        .args(args)
        .status()
        .map_err(|e| format!("Failed to exec in container: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Command failed: {:?}", args))
    }
}

/// Build assembly files into libasm.a
fn build_asm(workdir: &str) -> Result<(), String> {
    println!("Assembling .asm files...");
    
    let asm_dir = Path::new(workdir).join("src/asm");
    let target_dir = Path::new(workdir).join("target/asm");
    
    std::fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Failed to create target/asm: {}", e))?;

    // Find and assemble all .asm files
    if asm_dir.exists() {
        for entry in std::fs::read_dir(&asm_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "asm") {
                let filename = path.file_stem().unwrap().to_string_lossy();
                println!("  Assembling {}...", filename);
                
                let status = Command::new("llvm-mc")
                    .args([
                        "--filetype=obj",
                        "-triple=mos",
                        "-mcpu=mosw65c02",
                        path.to_str().unwrap(),
                        "-o",
                        &format!("{}/target/asm/{}.o", workdir, filename),
                    ])
                    .status()
                    .map_err(|e| format!("Failed to assemble {}: {}", filename, e))?;

                if !status.success() {
                    return Err(format!("Failed to assemble {}", filename));
                }
            }
        }
    }

    // Archive into libasm.a
    println!("  Creating libasm.a...");
    let o_files: Vec<_> = std::fs::read_dir(&target_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "o"))
        .map(|e| e.path().to_string_lossy().to_string())
        .collect();

    if !o_files.is_empty() {
        let mut args = vec!["rcs".to_string(), format!("{}/target/asm/libasm.a", workdir)];
        args.extend(o_files.clone());
        
        let status = Command::new("llvm-ar")
            .args(&args)
            .status()
            .map_err(|e| format!("Failed to archive: {}", e))?;

        if !status.success() {
            return Err("Failed to create libasm.a".to_string());
        }

        // Clean up .o files
        for o_file in o_files {
            let _ = std::fs::remove_file(o_file);
        }
    }

    Ok(())
}

/// Run cargo build for the ROM
fn cargo_build(workdir: &str, release: bool) -> Result<(), String> {
    println!("Building ROM with cargo...");
    
    let mut args = vec![
        "+mos", "build",
        "-Z", "build-std=core",
        "--target", "mos-unknown-none",
    ];
    
    if release {
        args.push("--release");
    }

    let status = Command::new("cargo")
        .current_dir(workdir)
        .args(&args)
        .status()
        .map_err(|e| format!("Failed to run cargo: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err("Cargo build failed".to_string())
    }
}

/// Convert ELF to GTR
fn convert_elf_to_gtr(elf_path: &str, output: &str) -> Result<(), String> {
    println!("Converting ELF to GTR: {} -> {}", elf_path, output);
    RomBuilder::build(elf_path.to_string(), output.to_string());
    Ok(())
}

/// Build assembly files via container
fn build_asm_in_container(workdir: &Path, working_dir: &Path) -> Result<(), String> {
    println!("Assembling .asm files...");
    
    let asm_dir = workdir.join("src/asm");
    let target_dir = workdir.join("target/asm");
    
    std::fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Failed to create target/asm: {}", e))?;

    let rel_workdir = workdir.strip_prefix(working_dir).unwrap_or(workdir);
    let workspace_dir = format!("/workspace/{}", rel_workdir.to_string_lossy());

    // Find and assemble all .asm files
    if asm_dir.exists() {
        for entry in std::fs::read_dir(&asm_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "asm") {
                let filename = path.file_stem().unwrap().to_string_lossy();
                println!("  Assembling {}...", filename);
                
                podman_exec("/workspace", &[
                    "llvm-mc",
                    "--filetype=obj",
                    "-triple=mos",
                    "-mcpu=mosw65c02",
                    &format!("{}/src/asm/{}.asm", workspace_dir, filename),
                    "-o",
                    &format!("{}/target/asm/{}.o", workspace_dir, filename),
                ])?;
            }
        }
    }

    // Archive into libasm.a
    let o_files: Vec<_> = std::fs::read_dir(&target_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "o"))
        .map(|e| format!("{}/target/asm/{}", workspace_dir, e.path().file_name().unwrap().to_string_lossy()))
        .collect();

    if !o_files.is_empty() {
        println!("  Creating libasm.a...");
        let mut args = vec![
            "llvm-ar".to_string(),
            "rcs".to_string(),
            format!("{}/target/asm/libasm.a", workspace_dir),
        ];
        args.extend(o_files);
        
        let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        podman_exec("/workspace", &args_ref)?;

        // Clean up .o files
        for entry in std::fs::read_dir(&target_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.path().extension().map_or(false, |ext| ext == "o") {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }

    Ok(())
}

/// Run cargo build via container
fn cargo_build_in_container(workdir: &Path, working_dir: &Path, release: bool) -> Result<(), String> {
    println!("Building ROM with cargo...");
    
    let rel_workdir = workdir.strip_prefix(working_dir).unwrap_or(workdir);
    let workspace_dir = format!("/workspace/{}", rel_workdir.to_string_lossy());

    let mut args = vec![
        "cargo", "+mos", "build",
        "-Z", "build-std=core",
        "--target", "mos-unknown-none",
    ];
    
    if release {
        args.push("--release");
    }

    podman_exec(&workspace_dir, &args)
}

/// Full build process
/// Get crate name from Cargo.toml in the given directory
fn get_crate_name(dir: &Path) -> Result<String, String> {
    let cargo_toml_path = dir.join("Cargo.toml");
    let cargo_content = std::fs::read_to_string(&cargo_toml_path)
        .map_err(|e| format!("Failed to read Cargo.toml: {}", e))?;
    cargo_content.lines()
        .find(|l| l.trim().starts_with("name"))
        .and_then(|l| l.split('=').nth(1))
        .map(|s| s.trim().trim_matches('"').to_string())
        .ok_or_else(|| "Could not find crate name in Cargo.toml".to_string())
}

/// Find the ROM directory (either rom/ subdirectory or current dir with Cargo.toml)
fn find_rom_dir() -> Result<(PathBuf, PathBuf), String> {
    let working_dir = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;
    
    let rom_dir = if working_dir.join("rom").exists() {
        working_dir.join("rom")
    } else if working_dir.join("Cargo.toml").exists() {
        working_dir.clone()
    } else {
        return Err("Could not find ROM project (no rom/ dir or Cargo.toml)".to_string());
    };
    
    Ok((working_dir, rom_dir))
}

fn do_build(release: bool) -> Result<PathBuf, String> {
    let (working_dir, rom_dir) = find_rom_dir()?;

    if is_in_container() {
        // Direct build inside container
        let rom_dir_str = rom_dir.to_string_lossy().to_string();
        build_asm(&rom_dir_str)?;
        cargo_build(&rom_dir_str, release)?;
    } else {
        // Orchestrate from outside container
        let workspace_root = ensure_container()?;
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

/// Read audio.toml to get firmware name
fn read_audio_toml(path: &Path) -> Result<String, String> {
    let toml_path = path.join("audio.toml");
    let content = std::fs::read_to_string(&toml_path)
        .map_err(|e| format!("Failed to read audio.toml: {}", e))?;
    
    // Simple TOML parsing - just look for name = "..."
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("name") {
            if let Some(value) = line.split('=').nth(1) {
                let name = value.trim().trim_matches('"').trim_matches('\'');
                return Ok(name.to_string());
            }
        }
    }
    
    Err("Could not find 'name' in audio.toml".to_string())
}

/// Build audio firmware (ASM project)
fn build_audio_asm(path: &Path, name: &str, output_dir: &Path) -> Result<(), String> {
    println!("Building ASM audio firmware: {}", name);
    
    let build_dir = path.join("build");
    std::fs::create_dir_all(&build_dir)
        .map_err(|e| format!("Failed to create build dir: {}", e))?;
    
    // Assemble all .asm files
    for entry in std::fs::read_dir(path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_path = entry.path();
        if file_path.extension().map_or(false, |ext| ext == "asm") {
            let filename = file_path.file_stem().unwrap().to_string_lossy();
            println!("  Assembling {}...", filename);
            
            let status = Command::new("llvm-mc")
                .args([
                    "--filetype=obj",
                    "-triple=mos",
                    "-mcpu=mosw65c02",
                    file_path.to_str().unwrap(),
                    "-o",
                    build_dir.join(format!("{}.o", filename)).to_str().unwrap(),
                ])
                .status()
                .map_err(|e| format!("Failed to assemble: {}", e))?;
            
            if !status.success() {
                return Err(format!("Failed to assemble {}", filename));
            }
        }
    }
    
    // Link
    let linker_script = path.join("linker.ld");
    let elf_path = build_dir.join("audio.elf");
    
    let o_files: Vec<_> = std::fs::read_dir(&build_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "o"))
        .map(|e| e.path())
        .collect();
    
    let mut link_args = vec![
        "-T".to_string(),
        linker_script.to_str().unwrap().to_string(),
    ];
    link_args.extend(o_files.iter().map(|p| p.to_str().unwrap().to_string()));
    link_args.push("-o".to_string());
    link_args.push(elf_path.to_str().unwrap().to_string());
    
    let status = Command::new("ld.lld")
        .args(&link_args)
        .status()
        .map_err(|e| format!("Failed to link: {}", e))?;
    
    if !status.success() {
        return Err("Linking failed".to_string());
    }
    
    // Extract binary
    let bin_path = output_dir.join(format!("{}.bin", name));
    let status = Command::new("llvm-objcopy")
        .args(["-O", "binary", elf_path.to_str().unwrap(), bin_path.to_str().unwrap()])
        .status()
        .map_err(|e| format!("Failed to objcopy: {}", e))?;
    
    if !status.success() {
        return Err("objcopy failed".to_string());
    }
    
    println!("Created: {}", bin_path.display());
    Ok(())
}

/// Build audio firmware (Rust project)
fn build_audio_rust(path: &Path, name: &str, output_dir: &Path) -> Result<(), String> {
    println!("Building Rust audio firmware: {}", name);
    
    // Build with cargo
    let status = Command::new("cargo")
        .current_dir(path)
        .args([
            "+mos", "build",
            "-Z", "build-std=core",
            "--target", "mos-unknown-none",
            "--release",
        ])
        .status()
        .map_err(|e| format!("Failed to run cargo: {}", e))?;
    
    if !status.success() {
        return Err("Cargo build failed".to_string());
    }
    
    // Find the ELF - use the crate name from Cargo.toml
    let cargo_toml = std::fs::read_to_string(path.join("Cargo.toml"))
        .map_err(|e| format!("Failed to read Cargo.toml: {}", e))?;
    
    let crate_name = cargo_toml.lines()
        .find(|l| l.trim().starts_with("name"))
        .and_then(|l| l.split('=').nth(1))
        .map(|s| s.trim().trim_matches('"'))
        .ok_or("Could not find crate name in Cargo.toml")?;
    
    let elf_path = path.join(format!("target/mos-unknown-none/release/{}", crate_name));
    
    // Extract binary
    let bin_path = output_dir.join(format!("{}.bin", name));
    let status = Command::new("llvm-objcopy")
        .args(["-O", "binary", elf_path.to_str().unwrap(), bin_path.to_str().unwrap()])
        .status()
        .map_err(|e| format!("Failed to objcopy: {}", e))?;
    
    if !status.success() {
        return Err("objcopy failed".to_string());
    }
    
    println!("Created: {}", bin_path.display());
    Ok(())
}

/// Build audio firmware (ASM project) - runs inside container
fn build_audio_asm_in_container(path: &Path, name: &str, output_dir: &Path, working_dir: &Path) -> Result<(), String> {
    println!("Building ASM audio firmware: {}", name);
    
    let build_dir = path.join("build");
    std::fs::create_dir_all(&build_dir)
        .map_err(|e| format!("Failed to create build dir: {}", e))?;
    
    // Convert paths to container-relative
    let rel_path = path.strip_prefix(working_dir).unwrap_or(path);
    let rel_build = build_dir.strip_prefix(working_dir).unwrap_or(&build_dir);
    let rel_output = output_dir.strip_prefix(working_dir).unwrap_or(output_dir);
    
    let workspace_path = format!("/workspace/{}", rel_path.to_string_lossy());
    let workspace_build = format!("/workspace/{}", rel_build.to_string_lossy());
    let workspace_output = format!("/workspace/{}", rel_output.to_string_lossy());
    
    // Assemble all .asm files
    for entry in std::fs::read_dir(path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_path = entry.path();
        if file_path.extension().map_or(false, |ext| ext == "asm") {
            let filename = file_path.file_stem().unwrap().to_string_lossy();
            println!("  Assembling {}...", filename);
            
            podman_exec("/workspace", &[
                "llvm-mc",
                "--filetype=obj",
                "-triple=mos",
                "-mcpu=mosw65c02",
                &format!("{}/{}.asm", workspace_path, filename),
                "-o",
                &format!("{}/{}.o", workspace_build, filename),
            ])?;
        }
    }
    
    // Link
    let linker_script = format!("{}/linker.ld", workspace_path);
    let elf_path = format!("{}/audio.elf", workspace_build);
    
    let o_files: Vec<_> = std::fs::read_dir(&build_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "o"))
        .map(|e| format!("{}/{}", workspace_build, e.path().file_name().unwrap().to_string_lossy()))
        .collect();
    
    let mut link_args = vec![
        "ld.lld".to_string(),
        "-T".to_string(),
        linker_script,
    ];
    link_args.extend(o_files);
    link_args.push("-o".to_string());
    link_args.push(elf_path.clone());
    
    let link_args_ref: Vec<&str> = link_args.iter().map(|s| s.as_str()).collect();
    podman_exec("/workspace", &link_args_ref)?;
    
    // Extract binary
    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output dir: {}", e))?;
    
    let bin_path = format!("{}/{}.bin", workspace_output, name);
    podman_exec("/workspace", &[
        "llvm-objcopy",
        "-O", "binary",
        &elf_path,
        &bin_path,
    ])?;
    
    println!("Created: {}/{}.bin", output_dir.display(), name);
    Ok(())
}

/// Build audio firmware
fn do_audio_build(path_str: &str) -> Result<(), String> {
    let path = Path::new(path_str);
    
    if !path.exists() {
        return Err(format!("Path does not exist: {}", path_str));
    }
    
    let name = read_audio_toml(path)?;
    
    // Output to sdk/audiofw/
    let working_dir = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;
    
    // Find sdk/audiofw/ directory
    let output_dir = if working_dir.join("sdk/audiofw").exists() || working_dir.join("sdk").exists() {
        working_dir.join("sdk/audiofw")
    } else if working_dir.join("audiofw").exists() || working_dir.file_name().map_or(false, |n| n == "sdk") {
        working_dir.join("audiofw")
    } else {
        // Fallback - just put it next to the source
        path.join("bin")
    };
    
    if is_in_container() {
        // Direct build inside container
        if path.join("Cargo.toml").exists() {
            build_audio_rust(path, &name, &output_dir)
        } else {
            build_audio_asm(path, &name, &output_dir)
        }
    } else {
        // Orchestrate from outside container - run llvm commands via podman exec
        let workspace_root = ensure_container()?;
        
        if path.join("Cargo.toml").exists() {
            // TODO: Rust audio build via container
            Err("Rust audio firmware build from outside container not yet implemented".to_string())
        } else {
            build_audio_asm_in_container(path, &name, &output_dir, &workspace_root)
        }
    }
}

/// Initialize a new GameTank project
fn do_init(path: &str, name: Option<&str>, with_audiofw_src: bool, audio: &str) -> Result<(), String> {
    let target_dir = Path::new(path);
    
    // Derive project name from path if not specified
    let project_name = name.map(|s| s.to_string()).unwrap_or_else(|| {
        // For "." or relative paths, canonicalize to get the actual directory name
        let resolved = if path == "." {
            std::env::current_dir().ok()
        } else {
            target_dir.canonicalize().ok().or_else(|| Some(target_dir.to_path_buf()))
        };
        
        resolved
            .and_then(|p| p.file_name().map(|s| s.to_os_string()))
            .and_then(|s| s.into_string().ok())
            .unwrap_or_else(|| "game".to_string())
    });
    
    // Check if directory exists and is not empty (unless it's ".")
    if target_dir.exists() && path != "." {
        return Err(format!("Directory '{}' already exists", path));
    }
    
    if path == "." {
        // Check if current dir already has SDK files
        if target_dir.join("rom").exists() {
            return Err("Current directory already contains a GameTank project".to_string());
        }
    }
    
    println!("Creating new GameTank project: {}", project_name);
    println!("  Audio firmware: {}", audio);
    if with_audiofw_src {
        println!("  Including audio firmware source");
    }
    
    // Create target directory
    std::fs::create_dir_all(target_dir)
        .map_err(|e| format!("Failed to create directory: {}", e))?;
    
    // Extract SDK template
    extract_sdk(target_dir, with_audiofw_src)?;
    
    // Update project name in Cargo.toml
    let cargo_toml_path = target_dir.join("rom/Cargo.toml");
    if cargo_toml_path.exists() {
        let content = std::fs::read_to_string(&cargo_toml_path)
            .map_err(|e| format!("Failed to read Cargo.toml: {}", e))?;
        let updated = content
            .replace("name = \"rom\" # rename me!", &format!("name = \"{}\"", project_name))
            .replace("name = \"rom\"", &format!("name = \"{}\"", project_name));
        std::fs::write(&cargo_toml_path, updated)
            .map_err(|e| format!("Failed to write Cargo.toml: {}", e))?;
    }
    
    // Update audio feature if not default
    if audio != "wavetable-8v" {
        let cargo_toml_path = target_dir.join("rom/Cargo.toml");
        if cargo_toml_path.exists() {
            let content = std::fs::read_to_string(&cargo_toml_path)
                .map_err(|e| format!("Failed to read Cargo.toml: {}", e))?;
            let updated = content.replace(
                "audio = [\"audio-wavetable-8v\"]",
                &format!("audio = [\"audio-{}\"]", audio)
            );
            std::fs::write(&cargo_toml_path, updated)
                .map_err(|e| format!("Failed to write Cargo.toml: {}", e))?;
        }
    }
    
    println!("\nProject created successfully!");
    println!("\nNext steps:");
    if path != "." {
        println!("  cd {}", path);
    }
    println!("  gtrom build");
    
    Ok(())
}

/// Extract embedded SDK tarball to filesystem
fn extract_sdk(base_target: &Path, include_audiofw_src: bool) -> Result<(), String> {
    let cursor = Cursor::new(SDK_TEMPLATE);
    let decoder = GzDecoder::new(cursor);
    let mut archive = Archive::new(decoder);
    
    for entry in archive.entries().map_err(|e| format!("Failed to read tarball: {}", e))? {
        let mut entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let entry_path = entry.path().map_err(|e| format!("Invalid path: {}", e))?;
        
        // Strip the leading "sdk/" from the path
        let relative_path = entry_path.strip_prefix("sdk").unwrap_or(&entry_path);
        
        // Skip audiofw-src if not requested
        if !include_audiofw_src && relative_path.starts_with("audiofw-src") {
            continue;
        }
        
        // Skip Cargo.lock and justfile
        if let Some(filename) = relative_path.file_name() {
            if filename == "Cargo.lock" || filename == "justfile" {
                continue;
            }
        }
        
        let target_path = base_target.join(relative_path);
        
        // Create parent directories
        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create dir {:?}: {}", parent, e))?;
        }
        
        // Extract the entry
        entry.unpack(&target_path)
            .map_err(|e| format!("Failed to extract {:?}: {}", target_path, e))?;
    }
    
    Ok(())
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
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
