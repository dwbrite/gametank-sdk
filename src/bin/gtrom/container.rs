//! Container orchestration for builds
//! 
//! Manages the podman/docker container lifecycle for llvm-mos toolchain access.

use std::path::Path;
use std::process::{Command, Stdio};

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

/// Get the mount root for the container.
/// This is the current working directory - the user's project root.
pub fn get_mount_root() -> Result<std::path::PathBuf, String> {
    std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))
}

/// Ensure the build container is running with the correct mount point
pub fn ensure_container() -> Result<(std::path::PathBuf, ContainerRuntime), String> {
    let runtime = ContainerRuntime::detect()
        .ok_or_else(|| "No container runtime found. Please install podman or docker.".to_string())?;
    
    let mount_root = get_mount_root()?;
    let cmd = runtime.as_str();
    
    // Check if container is already running
    let output = Command::new(cmd)
        .args(["ps", "--filter", "name=gametank", "--filter", "status=running", "--format", "{{.Names}}"])
        .output()
        .map_err(|e| format!("Failed to check container status: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.contains("gametank") {
        // Container is running - verify it's mounted to this workspace
        // Write a uniquely-named temp file, check if container can see it, then delete it
        let marker_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let marker_name = format!(".gametank-check-{}", marker_id);
        let marker_path = mount_root.join(&marker_name);
        let container_marker = format!("/workspace/{}", marker_name);
        
        // Write the marker file
        let _ = std::fs::write(&marker_path, "");
        
        // Check if container can see it
        let test_output = Command::new(cmd)
            .args(["exec", "gametank", "test", "-f", &container_marker])
            .status();
        
        // Clean up marker file
        let _ = std::fs::remove_file(&marker_path);
        
        if test_output.map(|s| s.success()).unwrap_or(false) {
            return Ok((mount_root, runtime));
        }
        
        // Container can't see our workspace - recreate
        println!("Workspace changed, recreating container...");
        let _ = Command::new(cmd)
            .args(["rm", "-f", "gametank"])
            .status();
    }

    // Start the container
    println!("Starting build container with {}...", cmd);

    // Docker has no "--replace" equivalent so we need to stop and delete the old container
    // Piping stdout to null here since docker complains if the container doesn't exist
    if runtime == ContainerRuntime::Docker {
        let _ = Command::new(cmd)
            .args(["rm", "-f", "gametank"])
            .stdout(Stdio::null())
            .status();
    }
    
    // Build volume mount arg - podman uses :z for SELinux, docker doesn't need it
    let volume_arg = match runtime {
        ContainerRuntime::Podman => format!("{}:/workspace:z", mount_root.display()),
        ContainerRuntime::Docker => format!("{}:/workspace", mount_root.display()),
    };

    let mut start_args = vec![
        "run", "-d", 
        "--name", "gametank", 
        "-v", &volume_arg
    ];
    
    // Include --replace for the Podman runner
    if runtime == ContainerRuntime::Podman {
        start_args.push("--replace");
    }

    start_args.extend([
        "docker.io/dwbrite/rust-mos:gte", 
        "sleep", "infinity"
    ]);
    
    let status = Command::new(cmd)
        .args(start_args)
        .status()
        .map_err(|e| format!("Failed to start container: {}", e))?;

    if status.success() {
        Ok((mount_root, runtime))
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
