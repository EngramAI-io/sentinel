use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

#[cfg(target_os = "windows")]
fn get_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|mut p| {
        p.push("Claude");
        p.push("claude_desktop_config.json");
        p
    })
}

#[cfg(not(target_os = "windows"))]
fn get_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|mut p| {
        p.push(".config");
        p.push("claude-desktop");
        p.push("claude_desktop_config.json");
        p
    })
}

pub fn install(server_name: String) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = get_config_path()
        .ok_or("Could not determine config directory")?;

    if !config_path.exists() {
        return Err(format!("Config file not found: {:?}", config_path).into());
    }

    // Read and parse config
    let config_content = fs::read_to_string(&config_path)?;
    let mut config: Value = serde_json::from_str(&config_content)?;

    // Create backup
    let backup_path = config_path.with_extension("json.backup");
    fs::copy(&config_path, &backup_path)?;
    eprintln!("Backup created: {:?}", backup_path);

    // Get sentinel binary path
    let sentinel_path = std::env::current_exe()?
        .to_string_lossy()
        .to_string();

    // Find and modify server config
    let mcp_servers = config
        .get_mut("mcpServers")
        .ok_or("mcpServers key not found in config")?
        .as_object_mut()
        .ok_or("mcpServers is not an object")?;

    let server_config = mcp_servers
        .get_mut(&server_name)
        .ok_or_else(|| format!("Server '{}' not found in config", server_name))?;

    let server_obj = server_config
        .as_object_mut()
        .ok_or("Server config is not an object")?;

    // Save original command and args
    let original_command = server_obj
        .get("command")
        .and_then(|v| v.as_str())
        .ok_or("Server command not found")?
        .to_string();
    
    let original_args = server_obj
        .get("args")
        .and_then(|v| v.as_array())
        .ok_or("Server args not found")?;

    // Build new args: ["run", "--", original_command, ...original_args]
    let mut new_args = vec![json!("run"), json!("--"), json!(original_command)];
    for arg in original_args {
        new_args.push(arg.clone());
    }

    // Update config
    server_obj.insert("command".to_string(), json!(sentinel_path));
    server_obj.insert("args".to_string(), json!(new_args));

    // Write updated config
    let updated_content = serde_json::to_string_pretty(&config)?;
    fs::write(&config_path, updated_content)?;

    eprintln!("Successfully installed sentinel for server '{}'", server_name);
    eprintln!("Original command: {}", original_command);
    eprintln!("Config updated: {:?}", config_path);

    Ok(())
}

pub fn restore_backup() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = get_config_path()
        .ok_or("Could not determine config directory")?;
    
    let backup_path = config_path.with_extension("json.backup");
    
    if !backup_path.exists() {
        return Err("Backup file not found".into());
    }

    fs::copy(&backup_path, &config_path)?;
    eprintln!("Config restored from backup: {:?}", backup_path);
    
    Ok(())
}

