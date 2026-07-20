//! Project initialization
//!
//! Handles creating new GameTank projects from the embedded SDK template.

use std::io::Cursor;
use std::path::Path;

use flate2::read::GzDecoder;
use tar::Archive;

// Embed the SDK template tarball at compile time
static SDK_TEMPLATE: &[u8] = include_bytes!("../sdk-template.tar.gz");

/// Extract embedded SDK tarball to filesystem
pub fn extract_sdk(base_target: &Path, include_audiofw_src: bool) -> Result<(), String> {
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

/// Sanitize a string to be a valid Cargo crate name
/// - lowercase
/// - replace underscores and spaces with hyphens
/// - remove invalid characters
/// - ensure it starts with a letter
fn sanitize_crate_name(name: &str) -> String {
    let mut result: String = name
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'a'..='z' | '0'..='9' | '-' => c,
            '_' | ' ' => '-',
            _ => '-',
        })
        .collect();
    
    // Remove leading/trailing hyphens and collapse multiple hyphens
    while result.contains("--") {
        result = result.replace("--", "-");
    }
    result = result.trim_matches('-').to_string();
    
    // Ensure it starts with a letter
    if result.chars().next().map(|c| !c.is_ascii_alphabetic()).unwrap_or(true) {
        result = format!("game-{}", result);
    }
    
    if result.is_empty() {
        "game".to_string()
    } else {
        result
    }
}

/// Initialize a new GameTank project
pub fn do_init(path: &str, name: Option<&str>, with_audiofw_src: bool, audio: &str) -> Result<(), String> {
    let target_dir = Path::new(path);
    
    // Derive project name from path if not specified, then sanitize
    let raw_name = name.map(|s| s.to_string()).unwrap_or_else(|| {
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
    
    let project_name = sanitize_crate_name(&raw_name);
    
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
