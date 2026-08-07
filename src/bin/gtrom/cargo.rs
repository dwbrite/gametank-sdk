//! Cargo build orchestration
//!
//! Handles running cargo builds for the ROM, both directly and via container.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::container::podman_exec;

/// Get crate name from Cargo.toml in the given directory
pub fn get_crate_name(dir: &Path) -> Result<String, String> {
    let cargo_toml_path = dir.join("../../../Cargo.toml");
    let cargo_content = std::fs::read_to_string(&cargo_toml_path)
        .map_err(|e| format!("Failed to read Cargo.toml: {}", e))?;
    parse_crate_name(&cargo_content)
}

/// Parse crate name from Cargo.toml content
pub fn parse_crate_name(content: &str) -> Result<String, String> {
    content.lines()
        .find(|l| l.trim().starts_with("name"))
        .and_then(|l| l.split('=').nth(1))
        .map(|s| s.trim().trim_matches('"').to_string())
        .ok_or_else(|| "Could not find crate name in Cargo.toml".to_string())
}

/// Find the ROM directory (either rom/ subdirectory or current dir with Cargo.toml)
/// Walks up the directory tree to find the project root
pub fn find_rom_dir() -> Result<(PathBuf, PathBuf), String> {
    let mut current_dir = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;
    
    // Walk up the directory tree to find a rom/ dir or Cargo.toml
    loop {
        // Check if this directory has a rom/ subdirectory with a GameTank project
        if current_dir.join("rom").exists() {
            let rom_dir = current_dir.join("rom");
            if is_gametank_project(&rom_dir) {
                return Ok((current_dir, rom_dir));
            }
        }
        
        // Check if this directory itself is a GameTank ROM project
        if is_gametank_project(&current_dir) {
            return Ok((current_dir.clone(), current_dir));
        }
        
        // Move up to parent directory
        if let Some(parent) = current_dir.parent() {
            current_dir = parent.to_path_buf();
        } else {
            // Reached filesystem root without finding project
            return Err("Could not find ROM project (no rom/ dir or GameTank project found)".to_string());
        }
    }
}

/// Check if a directory is a GameTank ROM project
/// A GameTank project has Cargo.toml and either:
/// - src/asm/ directory (unique to GameTank projects)
/// - asset-macros/ directory 
/// - sdk/ subdirectory (when sdk is a separate crate)
/// - gametank-asset-macros or sdk dependency in Cargo.toml
fn is_gametank_project(dir: &Path) -> bool {
    if !dir.join("../../../Cargo.toml").exists() {
        return false;
    }
    
    // Check for unique GameTank markers
    if dir.join("src/asm").exists() {
        return true;
    }
    
    if dir.join("asset-macros").exists() {
        return true;
    }
    
    // Check for sdk/ subdirectory (when sdk is a separate crate)
    if dir.join("sdk").exists() && dir.join("sdk/Cargo.toml").exists() {
        return true;
    }
    
    // Check Cargo.toml for gametank dependencies
    if let Ok(cargo_content) = std::fs::read_to_string(dir.join("../../../Cargo.toml")) {
        if cargo_content.contains("gametank-asset-macros") 
            || cargo_content.contains("gametank-sdk")
            || (cargo_content.contains("sdk") && cargo_content.contains("path = \"sdk\"")) {
            return true;
        }
    }
    
    false
}

/// Run cargo build for the ROM (runs directly)
pub fn cargo_build(workdir: &str, release: bool) -> Result<(), String> {
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

/// Run cargo build via container
pub fn cargo_build_in_container(workdir: &Path, working_dir: &Path, release: bool) -> Result<(), String> {
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
