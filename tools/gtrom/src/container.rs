//! Container orchestration for builds
//! 
//! Manages the podman/docker container lifecycle for llvm-mos toolchain access.

use std::path::Path;
use std::process::Command;

/// Container runtime to use
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContainerRuntime {
    Podman,
    Docker,
}

impl ContainerRuntime {
    /// Detect which container runtime is available
    pub fn detect() -> Option<Self> {
        // Prefer podman over docker
        if Command::new("podman").arg("--version").output().is_ok() {
            return Some(Self::Podman);
        }
        if Command::new("docker").arg("--version").output().is_ok() {
            return Some(Self::Docker);
        }
        None
    }
    
    fn as_str(&self) -> &'static str {
        match self {
            Self::Podman => "podman",
            Self::Docker => "docker",
        }
    }
}

/// Check if we're running inside a container
pub fn is_in_container() -> bool {
    Path::new("/.dockerenv").exists()
        || Path::new("/run/.containerenv").exists()
        || std::env::var("container").is_ok()
}

/// Find the workspace root (where .git or root Cargo.toml is)
pub fn find_workspace_root() -> Result<std::path::PathBuf, String> {
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
pub fn ensure_container() -> Result<(std::path::PathBuf, ContainerRuntime), String> {
    let runtime = ContainerRuntime::detect()
        .ok_or_else(|| "No container runtime found. Please install podman or docker.".to_string())?;
    
    let workspace_root = find_workspace_root()?;
    let cmd = runtime.as_str();
    
    // Check if container is already running
    let output = Command::new(cmd)
        .args(["ps", "--filter", "name=gametank", "--filter", "status=running", "--format", "{{.Names}}"])
        .output()
        .map_err(|e| format!("Failed to check container status: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.contains("gametank") {
        return Ok((workspace_root, runtime));
    }

    // Start the container
    println!("Starting build container with {}...", cmd);
    
    // Build volume mount arg - podman uses :z for SELinux, docker doesn't need it
    let volume_arg = match runtime {
        ContainerRuntime::Podman => format!("{}:/workspace:z", workspace_root.display()),
        ContainerRuntime::Docker => format!("{}:/workspace", workspace_root.display()),
    };
    
    let status = Command::new(cmd)
        .args([
            "run", "-d",
            "--name", "gametank",
            "-v", &volume_arg,
            "--replace",
            "rust-mos:gte",
            "sleep", "infinity"
        ])
        .status()
        .map_err(|e| format!("Failed to start container: {}", e))?;

    if status.success() {
        Ok((workspace_root, runtime))
    } else {
        Err("Failed to start build container".to_string())
    }
}

/// Execute a command inside the container
pub fn container_exec(runtime: ContainerRuntime, workdir: &str, args: &[&str]) -> Result<(), String> {
    let cmd = runtime.as_str();
    let status = Command::new(cmd)
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

/// Execute a command inside the container (convenience wrapper that detects runtime)
pub fn podman_exec(workdir: &str, args: &[&str]) -> Result<(), String> {
    let runtime = ContainerRuntime::detect()
        .ok_or_else(|| "No container runtime found".to_string())?;
    container_exec(runtime, workdir, args)
}
