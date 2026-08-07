//! Audio firmware building
//!
//! Handles building audio coprocessor firmware from ASM or Rust sources.

use std::path::Path;
use std::process::Command;

use crate::container::{ensure_container, is_in_container, podman_exec};

/// Get firmware name from directory name
fn get_firmware_name(path: &Path) -> Result<String, String> {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Invalid path".to_string())
}

/// Build audio firmware (ASM project) - runs directly
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

/// Build audio firmware (Rust project) - runs directly
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
pub fn do_audio_build(path_str: &str) -> Result<(), String> {
    let path = Path::new(path_str);
    
    if !path.exists() {
        return Err(format!("Path does not exist: {}", path_str));
    }
    
    let name = get_firmware_name(path)?;
    
    // Output to gametank/audiofw/
    let working_dir = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;
    
    // Find gametank/audiofw/ directory
    let output_dir = if working_dir.join("gametank/audiofw").exists() || working_dir.join("gametank").exists() {
        working_dir.join("gametank/audiofw")
    } else if working_dir.join("audiofw").exists() || working_dir.file_name().map_or(false, |n| n == "gametank") {
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
        let (workspace_root, _runtime) = ensure_container()?;
        
        if path.join("Cargo.toml").exists() {
            // TODO: Rust audio build via container
            Err("Rust audio firmware build from outside container not yet implemented".to_string())
        } else {
            build_audio_asm_in_container(path, &name, &output_dir, &workspace_root)
        }
    }
}
