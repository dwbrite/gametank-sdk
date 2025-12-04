//! Cargo build orchestration
//!
//! Handles running cargo builds for the ROM, both directly and via container.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::container::podman_exec;

/// Get crate name from Cargo.toml in the given directory
pub fn get_crate_name(dir: &Path) -> Result<String, String> {
    let cargo_toml_path = dir.join("Cargo.toml");
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
pub fn find_rom_dir() -> Result<(PathBuf, PathBuf), String> {
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
