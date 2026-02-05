//! Path management utilities for CrewAI storage and configuration.
//!
//! Port of crewai/utilities/paths.py

use std::env;
use std::path::PathBuf;

/// Returns the path for SQLite database storage.
///
/// Uses the platform-specific data directory for the current project,
/// creating it if necessary.
///
/// # Returns
/// String path to the data directory.
pub fn db_storage_path() -> String {
    let app_name = get_project_directory_name();
    let app_author = "CrewAI";

    // Use a platform-appropriate data directory.
    // On Linux: ~/.local/share/<app_author>/<app_name>
    // On macOS: ~/Library/Application Support/<app_author>/<app_name>
    // On Windows: C:\Users\<user>\AppData\Local\<app_author>\<app_name>
    let data_dir = if cfg!(target_os = "linux") {
        let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join(app_author)
            .join(&app_name)
    } else if cfg!(target_os = "macos") {
        let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join(app_author)
            .join(&app_name)
    } else if cfg!(target_os = "windows") {
        let local_app_data = env::var("LOCALAPPDATA")
            .unwrap_or_else(|_| env::var("APPDATA").unwrap_or_else(|_| "C:\\tmp".to_string()));
        PathBuf::from(local_app_data)
            .join(app_author)
            .join(&app_name)
    } else {
        PathBuf::from("/tmp")
            .join(app_author)
            .join(&app_name)
    };

    // Create the directory if it doesn't exist
    let _ = std::fs::create_dir_all(&data_dir);

    data_dir.to_string_lossy().to_string()
}

/// Returns the current project directory name.
///
/// Uses the `CREWAI_STORAGE_DIR` environment variable if set,
/// otherwise defaults to the current working directory name.
pub fn get_project_directory_name() -> String {
    env::var("CREWAI_STORAGE_DIR").unwrap_or_else(|_| {
        env::current_dir()
            .ok()
            .and_then(|p| {
                p.file_name()
                    .map(|n| n.to_string_lossy().to_string())
            })
            .unwrap_or_else(|| "crewai_default".to_string())
    })
}
