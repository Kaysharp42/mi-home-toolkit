// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

extern crate anyhow;
extern crate miio;
extern crate serde_json;


use miio::{Device, MiCloudProtocol, Credentials, SecureSession};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fs, path::PathBuf};

use tauri::{Emitter, Manager, AppHandle, WindowEvent, tray::{TrayIconBuilder, MouseButton}, menu::{MenuBuilder, MenuItem}};
use tauri_plugin_log::{Builder, Target, TargetKind};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutWrapper};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tokio::sync::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref MI_CLOUD_PROTOCOL: std::sync::Arc<Mutex<MiCloudProtocol>> = std::sync::Arc::new(Mutex::new(MiCloudProtocol::new()));
}

// Simple struct for backwards compatibility - still used for some operations
#[derive(Serialize, Deserialize, Debug, Clone)]
struct SavedCredentials {
    username: String,
    country: String,
}

// Struct for saved commands
#[derive(Serialize, Deserialize, Debug, Clone)]
struct SavedCommand {
    name: String,
    method: String,
    params: String,
    shortcut: Option<String>,
}

// Struct for the commands JSON file
#[derive(Serialize, Deserialize, Debug, Clone)]
struct SavedCommands {
    commands: Vec<SavedCommand>,
}

// Struct for user settings
#[derive(Serialize, Deserialize, Debug, Clone)]
struct AppSettings {
    close_to_tray: Option<bool>,
    auto_start: Option<bool>,
    auto_hide_to_tray: Option<bool>,
}

// Get the config file path for storing session credentials
fn get_session_path(app_handle: &AppHandle) -> PathBuf {
    let app_dir = app_handle.path().app_data_dir().expect("Failed to get app data dir");
    
    // Create the directory if it doesn't exist
    if !app_dir.exists() {
        fs::create_dir_all(&app_dir).expect("Failed to create config directory");
    }
    
    app_dir.join("session.json")
}

// Get the config file path for storing basic credentials (legacy)
fn get_credentials_path(app_handle: &AppHandle) -> PathBuf {
    let app_dir = app_handle.path().app_data_dir().expect("Failed to get app data dir");
    
    // Create the directory if it doesn't exist
    if !app_dir.exists() {
        fs::create_dir_all(&app_dir).expect("Failed to create config directory");
    }
    
    app_dir.join("credentials.json")
}

// Get the config file path for storing saved commands
fn get_commands_path(app_handle: &AppHandle) -> PathBuf {
    let app_dir = app_handle.path().app_data_dir().expect("Failed to get app data dir");
    
    // Create the directory if it doesn't exist
    if !app_dir.exists() {
        fs::create_dir_all(&app_dir).expect("Failed to create config directory");
    }
    
    app_dir.join("saved_commands.json")
}

// Get the config file path for storing app settings
fn get_settings_path(app_handle: &AppHandle) -> PathBuf {
    let app_dir = app_handle.path().app_data_dir().expect("Failed to get app data dir");
    
    if !app_dir.exists() {
        fs::create_dir_all(&app_dir).expect("Failed to create config directory");
    }
    
    app_dir.join("settings.json")
}

// Save secure session (without password) to a file
fn save_secure_session(app_handle: &AppHandle, session: &SecureSession) -> Result<(), String> {
    let path = get_session_path(app_handle);
    let json = serde_json::to_string(&session).map_err(|e| e.to_string())?;
    fs::write(&path, &json).map_err(|e| e.to_string())?;
    Ok(())
}

// Load secure session from a file
fn load_secure_session(app_handle: &AppHandle) -> Option<SecureSession> {
    let path = get_session_path(app_handle);
    if !path.exists() {
        return None;
    }
    
    match fs::read_to_string(path) {
        Ok(json) => match serde_json::from_str(&json) {
            Ok(session) => Some(session),
            Err(_) => None,
        },
        Err(_) => None,
    }
}

// Legacy function - kept for backward compatibility during migration
// TODO: Remove after migration period
#[allow(dead_code)]
fn save_session(app_handle: &AppHandle, credentials: &Credentials) -> Result<(), String> {
    let path = get_session_path(app_handle);
    let json = serde_json::to_string(&credentials).map_err(|e| e.to_string())?;
    fs::write(&path, &json).map_err(|e| e.to_string())?;
    Ok(())
}

// Legacy function - kept for backward compatibility but will migrate to secure sessions
// Load full session credentials from a file
fn load_session(app_handle: &AppHandle) -> Option<Credentials> {
    let path = get_session_path(app_handle);
    if !path.exists() {
        return None;
    }
    
    match fs::read_to_string(path) {
        Ok(json) => match serde_json::from_str(&json) {
            Ok(credentials) => Some(credentials),
            Err(_) => None,
        },
        Err(_) => None,
    }
}

// Save credentials to a file (for login form pre-filling only)
#[allow(dead_code)]
fn save_credentials(app_handle: &AppHandle, credentials: &SavedCredentials) -> Result<(), String> {
    let path = get_credentials_path(app_handle);
    let json = serde_json::to_string(&credentials).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

// Load credentials from a file
fn load_credentials(app_handle: &AppHandle) -> Option<SavedCredentials> {
    let path = get_credentials_path(app_handle);
    if !path.exists() {
        return None;
    }
    
    match fs::read_to_string(path) {
        Ok(json) => match serde_json::from_str(&json) {
            Ok(credentials) => Some(credentials),
            Err(_) => None,
        },
        Err(_) => None,
    }
}

// Save command to the commands file
fn save_command_to_file(app_handle: &AppHandle, command: &SavedCommand, update_if_exists: bool) -> Result<(), String> {
    let path = get_commands_path(app_handle);
    
    // Load existing commands or create new list
    let mut saved_commands = load_all_commands(app_handle).unwrap_or(SavedCommands { commands: vec![] });
    
    // Check if command name already exists
    if let Some(existing_index) = saved_commands.commands.iter().position(|c| c.name == command.name) {
        if update_if_exists {
            // Update existing command
            saved_commands.commands[existing_index] = command.clone();
        } else {
            return Err(format!("Command with name '{}' already exists", command.name));
        }
    } else {
        // Add new command
        saved_commands.commands.push(command.clone());
    }
    
    // Save to file
    let json = serde_json::to_string_pretty(&saved_commands).map_err(|e| e.to_string())?;
    fs::write(&path, &json).map_err(|e| e.to_string())?;
    
    Ok(())
}

// Delete command from the commands file
fn delete_command_from_file(app_handle: &AppHandle, command_name: &str) -> Result<(), String> {
    let path = get_commands_path(app_handle);
    
    // Load existing commands
    let mut saved_commands = load_all_commands(app_handle).unwrap_or(SavedCommands { commands: vec![] });
    
    // Find and remove the command
    let original_len = saved_commands.commands.len();
    saved_commands.commands.retain(|c| c.name != command_name);
    
    if saved_commands.commands.len() == original_len {
        return Err(format!("Command with name '{}' not found", command_name));
    }
    
    // Save to file
    let json = serde_json::to_string_pretty(&saved_commands).map_err(|e| e.to_string())?;
    fs::write(&path, &json).map_err(|e| e.to_string())?;
    
    Ok(())
}

// Load all commands from the commands file
fn load_all_commands(app_handle: &AppHandle) -> Option<SavedCommands> {
    let path = get_commands_path(app_handle);
    if !path.exists() {
        return None;
    }
    
    match fs::read_to_string(path) {
        Ok(json) => match serde_json::from_str(&json) {
            Ok(commands) => Some(commands),
            Err(_) => None,
        },
        Err(_) => None,
    }
}

// Load app settings from the settings file
fn load_app_settings(app_handle: &AppHandle) -> AppSettings {
    let path = get_settings_path(app_handle);
    if !path.exists() {
        return AppSettings { 
            close_to_tray: None,
            auto_start: None,
            auto_hide_to_tray: None,
        };
    }
    
    match fs::read_to_string(path) {
        Ok(json) => match serde_json::from_str(&json) {
            Ok(settings) => settings,
            Err(_) => AppSettings { 
                close_to_tray: None,
                auto_start: None,
                auto_hide_to_tray: None,
            },
        },
        Err(_) => AppSettings { 
            close_to_tray: None,
            auto_start: None,
            auto_hide_to_tray: None,
        },
    }
}

// Save app settings to the settings file
fn save_app_settings(app_handle: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = get_settings_path(app_handle);
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(&path, &json).map_err(|e| e.to_string())?;
    Ok(())
}

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
async fn login(
    app_handle: AppHandle, 
    email: String, 
    password: String, 
    country: Option<String>, 
    should_save_credentials: bool
) -> Result<(), String> {    
    // Default country is "cn" if not specified
    let _current_country = country.clone().unwrap_or_else(|| "cn".to_string());
    
    let mut guard = MI_CLOUD_PROTOCOL.lock().await;
    
    // Set country if provided
    if let Some(c) = &country {
        guard.set_country(c);
    }
      // Perform login
    guard
        .login(email.as_str(), password.as_str())
        .await
        .map_err(|err| err.to_string())?;
      // Save secure session (without password) if requested
    if should_save_credentials {
        if let Some(secure_session) = guard.export_secure_session() {
            save_secure_session(&app_handle, &secure_session)?;
        }
    }
    
    Ok(())
}

#[tauri::command]
async fn get_countries() -> Vec<Vec<&'static str>> {
    let guard = MI_CLOUD_PROTOCOL.lock().await;
    guard.get_available_countries()
}

#[tauri::command]
async fn set_country(app_handle: AppHandle, country: String) -> Result<(), String> {
    let mut guard = MI_CLOUD_PROTOCOL.lock().await;
    guard.set_country(&country);
    
    // Save the updated session with new country
    if let Some(secure_session) = guard.export_secure_session() {
        save_secure_session(&app_handle, &secure_session)?;
    }
    
    Ok(())
}

#[tauri::command]
async fn get_devices() -> Result<Vec<Device>, ()> {
    let guard = MI_CLOUD_PROTOCOL.lock().await;
    guard.get_devices(None, None).await.map_err(|_| ())
}

#[tauri::command]
async fn get_device(did: String) -> Result<Vec<Device>, ()> {
    let guard = MI_CLOUD_PROTOCOL.lock().await;
    guard.get_device(&did, None).await.map_err(|_| ())
}

#[tauri::command]
async fn call_device(did: String, method: String, params: Option<String>) -> Result<Value, String> {
    let guard = MI_CLOUD_PROTOCOL.lock().await;
    let params = params
        .map(|params| serde_json::from_str::<Value>(params.as_str()).map_err(|err| err.to_string()))
        .transpose()?;
    guard
        .call_device(&did, &method, params, None)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn is_logged_in() -> bool {
    let guard = MI_CLOUD_PROTOCOL.lock().await;
    // Check if we have a valid session (doesn't require password)
    guard.is_session_valid()
}

#[tauri::command]
async fn try_auto_login(app_handle: AppHandle) -> Result<bool, String> {
    // First try to load secure session (new format)
    if let Some(secure_session) = load_secure_session(&app_handle) {
        let mut guard = MI_CLOUD_PROTOCOL.lock().await;
        
        // Import the secure session
        guard.import_secure_session(secure_session);
        
        // Verify the session is still valid by checking session validity
        let is_session_valid = guard.is_session_valid();
        
        if is_session_valid {
            // Test the session by trying to get devices to ensure it's actually valid
            match guard.get_devices(None, None).await {
                Ok(_) => {
                    return Ok(true);
                }
                Err(_) => {
                    // Clear the invalid session
                    let session_path = get_session_path(&app_handle);
                    if session_path.exists() {
                        let _ = std::fs::remove_file(session_path);
                    }
                    return Ok(false);
                }
            }
        }
    }
    // Fallback: try to load legacy session format (with password)
    else if let Some(legacy_credentials) = load_session(&app_handle) {
        let mut guard = MI_CLOUD_PROTOCOL.lock().await;
        
        // Import the legacy credentials
        guard.import_credentials(legacy_credentials);
        
        // Test the session validity
        if guard.is_logged_in() {
            match guard.get_devices(None, None).await {
                Ok(_) => {
                    // Migrate to secure session format
                    if let Some(secure_session) = guard.export_secure_session() {
                        save_secure_session(&app_handle, &secure_session)?;
                    }
                    
                    return Ok(true);
                }
                Err(_) => {
                    // Clear the invalid session
                    let session_path = get_session_path(&app_handle);
                    if session_path.exists() {
                        let _ = std::fs::remove_file(session_path);
                    }
                    return Ok(false);
                }
            }
        }
    }
    
    Ok(false)
}

#[tauri::command]
async fn logout(app_handle: AppHandle) -> Result<(), String> {
    // Reset the protocol
    *MI_CLOUD_PROTOCOL.lock().await = MiCloudProtocol::new();
    
    // Remove stored session
    let session_path = get_session_path(&app_handle);
    if session_path.exists() {
        fs::remove_file(session_path).map_err(|e| e.to_string())?;
    }
    
    // Also remove legacy credentials file if it exists
    let credentials_path = get_credentials_path(&app_handle);
    if credentials_path.exists() {
        fs::remove_file(credentials_path).map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

#[tauri::command]
async fn get_saved_credentials(app_handle: AppHandle) -> Option<SavedCredentials> {
    load_credentials(&app_handle)
}

#[tauri::command]
async fn get_current_user() -> Option<SavedCredentials> {
    let guard = MI_CLOUD_PROTOCOL.lock().await;
    // Use secure session export instead of full credentials export
    // This works even when password is not available (secure session)
    if let Some(secure_session) = guard.export_secure_session() {
        Some(SavedCredentials {
            username: secure_session.username,
            country: secure_session.country,
        })
    } else {
        None
    }
}

#[tauri::command]
async fn is_session_restored() -> bool {
    let guard = MI_CLOUD_PROTOCOL.lock().await;
    // Use session validity check instead of full login check
    // This works for secure sessions without password
    guard.is_session_valid()
}

#[tauri::command]
async fn save_command(app_handle: AppHandle, name: String, method: String, params: String, shortcut: Option<String>) -> Result<(), String> {
    let command = SavedCommand { name: name.clone(), method, params, shortcut: shortcut.clone() };
    save_command_to_file(&app_handle, &command, false)?;
    
    // Register global shortcut if provided
    if let Some(sc) = shortcut {
        let sc_clone = sc.clone();
        let cmd_clone = command.clone();
        println!("Attempting to register shortcut: {}", sc);
        
        // Convert the shortcut to a ShortcutWrapper
        match sc.as_str().try_into() {
            Ok(shortcut_str_parsed) => {
                let shortcut: ShortcutWrapper = shortcut_str_parsed;
                
                // Register the shortcut with a handler function
                match app_handle.global_shortcut().on_shortcut(shortcut, move |_app_handle, _shortcut, _event| {
                    let cmd_clone2 = cmd_clone.clone();
                    
                    // Use std::thread::spawn + block_on for proper runtime context
                    std::thread::spawn(move || {
                        tauri::async_runtime::block_on(async move {
                            if let Err(e) = execute_saved_command(&cmd_clone2).await {
                                eprintln!("Error executing command: {}", e);
                            }
                        });
                    });
                }) {
                    Ok(_) => {
                        println!("Successfully registered shortcut: {}", sc);
                    },
                    Err(e) => {
                        return Err(format!("Failed to register shortcut '{}': {}", sc, e));
                    }
                }
            },
            Err(e) => {
                return Err(format!("Invalid shortcut format '{}': {:?}", sc, e));
            }
        }
    }
    
    Ok(())
}

#[tauri::command]
async fn update_command(app_handle: AppHandle, name: String, method: String, params: String, shortcut: Option<String>) -> Result<(), String> {
    // First, get the old command to unregister its shortcut if needed
    let old_commands = load_all_commands(&app_handle).unwrap_or(SavedCommands { commands: vec![] });
    let old_command = old_commands.commands.iter().find(|c| c.name == name);
    
    if let Some(old_cmd) = old_command {
        // Unregister the old shortcut if it exists
        if let Some(old_sc) = &old_cmd.shortcut {
            // Convert the shortcut string to a Shortcut object and unregister it
            if let Ok(shortcut_str_parsed) = old_sc.as_str().try_into() {
                let shortcut: ShortcutWrapper = shortcut_str_parsed;
                let _ = app_handle.global_shortcut().unregister(shortcut);
            }
        }
    }
  
    // Create and save the updated command
    let command = SavedCommand { name: name.clone(), method, params, shortcut: shortcut.clone() };
    save_command_to_file(&app_handle, &command, true)?;    // Register the new shortcut if provided
    if let Some(sc) = shortcut {
        let sc_clone = sc.clone();
        let cmd_clone = command.clone();
        
        println!("Attempting to update and register shortcut: {}", sc);
        
        // Convert the shortcut string to a ShortcutWrapper
        match sc.as_str().try_into() {
            Ok(shortcut_str_parsed) => {
                let shortcut: ShortcutWrapper = shortcut_str_parsed;
                
                // Register the shortcut with a handler function
                match app_handle.global_shortcut().on_shortcut(shortcut, move |_app_handle, _shortcut, _event| {
                    println!("Shortcut triggered: {}", sc_clone);
                    let cmd_clone2 = cmd_clone.clone();
                    
                    // Use std::thread::spawn + block_on for proper runtime context
                    std::thread::spawn(move || {
                        tauri::async_runtime::block_on(async move {
                            if let Err(e) = execute_saved_command(&cmd_clone2).await {
                                eprintln!("Error executing command: {}", e);
                            }
                        });
                    });
                }) {
                    Ok(_) => {
                        println!("Successfully updated and registered shortcut: {}", sc);
                    },
                    Err(e) => {
                        eprintln!("Failed to register shortcut '{}': {}", sc, e);
                        return Err(format!("Failed to register shortcut '{}': {}", sc, e));
                    }
                }
            },
            Err(e) => {
                eprintln!("Invalid shortcut format '{}': {:?}", sc, e);
                return Err(format!("Invalid shortcut format '{}': {:?}", sc, e));
            }
        }
    }
    
    Ok(())
}

#[tauri::command]
async fn delete_command(app_handle: AppHandle, name: String) -> Result<(), String> {
    // First, get the command to unregister its shortcut if needed
    let commands = load_all_commands(&app_handle).unwrap_or(SavedCommands { commands: vec![] });
    let command = commands.commands.iter().find(|c| c.name == name);
    
    if let Some(cmd) = command {
        // Unregister the shortcut if it exists
        if let Some(sc) = &cmd.shortcut {
            // Convert the shortcut string to a Shortcut object and unregister it
            if let Ok(shortcut_str_parsed) = sc.as_str().try_into() {
                let shortcut: ShortcutWrapper = shortcut_str_parsed;
                let _ = app_handle.global_shortcut().unregister(shortcut);
                println!("Unregistered shortcut: {}", sc);
            }
        }
    }
    
    // Delete the command from the file
    delete_command_from_file(&app_handle, &name)
}

#[tauri::command]
async fn get_saved_commands(app_handle: AppHandle) -> Vec<SavedCommand> {
    load_all_commands(&app_handle)
        .map(|commands| commands.commands)
        .unwrap_or_default()
}

#[tauri::command]
async fn validate_shortcut(app_handle: AppHandle, shortcut: String) -> Result<bool, String> {
    // Validate shortcut format and check for conflicts
    if shortcut.trim().is_empty() {
        return Err("Shortcut cannot be empty".to_string());
    }
    
    // Get existing commands to check for conflicts
    let saved_commands = load_all_commands(&app_handle).unwrap_or(SavedCommands { commands: vec![] });
    
    // Check if shortcut is already in use
    let is_duplicate = saved_commands.commands.iter().any(|cmd| {
        cmd.shortcut.as_ref().map_or(false, |s| s == &shortcut)
    });
    
    if is_duplicate {
        return Err("Shortcut is already in use".to_string());
    }
    
    // Additional validation: check if shortcut format is valid by trying to parse it
    match shortcut.as_str().try_into() {
        Ok(_shortcut_wrapper) => {
            let _: ShortcutWrapper = _shortcut_wrapper;
            Ok(true)
        }
        Err(_) => Err("Invalid shortcut format".to_string())
    }
}

#[tauri::command]
async fn get_app_settings(app_handle: AppHandle) -> AppSettings {
    load_app_settings(&app_handle)
}

#[tauri::command]
async fn save_close_to_tray_preference(app_handle: AppHandle, close_to_tray: bool) -> Result<(), String> {
    let mut settings = load_app_settings(&app_handle);
    settings.close_to_tray = Some(close_to_tray);
    save_app_settings(&app_handle, &settings)
}

#[tauri::command]
async fn save_auto_start_preference(app_handle: AppHandle, auto_start: bool) -> Result<(), String> {
    let mut settings = load_app_settings(&app_handle);
    settings.auto_start = Some(auto_start);
      // Enable/disable autostart using the plugin
    if auto_start {
        // Enable autostart
        app_handle.autolaunch().enable().map_err(|e| format!("Failed to enable autostart: {}", e))?;
    } else {
        // Disable autostart
        app_handle.autolaunch().disable().map_err(|e| format!("Failed to disable autostart: {}", e))?;
    }
    
    save_app_settings(&app_handle, &settings)
}

#[tauri::command]
async fn save_auto_hide_preference(app_handle: AppHandle, auto_hide: bool) -> Result<(), String> {
    let mut settings = load_app_settings(&app_handle);
    settings.auto_hide_to_tray = Some(auto_hide);
    save_app_settings(&app_handle, &settings)
}

#[tauri::command]
async fn save_all_settings(app_handle: AppHandle, close_to_tray: Option<bool>, auto_start: Option<bool>, auto_hide_to_tray: Option<bool>) -> Result<(), String> {
    let mut settings = load_app_settings(&app_handle);
    
    if let Some(ctt) = close_to_tray {
        settings.close_to_tray = Some(ctt);
    }
    
    if let Some(ah) = auto_hide_to_tray {
        settings.auto_hide_to_tray = Some(ah);
    }
    
    if let Some(as_val) = auto_start {
        settings.auto_start = Some(as_val);
          // Enable/disable autostart using the plugin
        if as_val {
            app_handle.autolaunch().enable().map_err(|e| format!("Failed to enable autostart: {}", e))?;
        } else {
            app_handle.autolaunch().disable().map_err(|e| format!("Failed to disable autostart: {}", e))?;
        }
    }
    
    save_app_settings(&app_handle, &settings)
}

// Execute a saved command from a shortcut
async fn execute_saved_command(command: &SavedCommand) -> Result<(), String> {
    // Get the current user and check if logged in
    let guard = MI_CLOUD_PROTOCOL.lock().await;
    if !guard.is_session_valid() {
        drop(guard);
        return Err(format!("Cannot execute command '{}': Not logged in", command.name));
    }
    drop(guard);
    
    // Get the first available device for execution
    let devices = match get_devices().await {
        Ok(devices) => devices,
        Err(_) => {
            return Err(format!("Cannot execute command '{}': Failed to get devices", command.name));
        }
    };
    
    if devices.is_empty() {
        return Err(format!("Cannot execute command '{}': No devices available", command.name));
    }
    
    // Use the first device
    let device = &devices[0];
    let did = device.did.to_string();
    
    // Execute the command
    call_device(did, command.method.clone(), Some(command.params.clone())).await?;
    Ok(())
}

// Helper function to register all saved command shortcuts
fn register_saved_shortcuts(app_handle: &AppHandle) {
    if let Some(saved_commands) = load_all_commands(app_handle) {
        for command in saved_commands.commands {
            if let Some(shortcut_str) = &command.shortcut {
                let shortcut_str = shortcut_str.clone();
                match shortcut_str.as_str().try_into() {
                    Ok(shortcut) => {
                        let shortcut: ShortcutWrapper = shortcut;
                        let command_clone = command.clone();
                        let app_handle_clone = app_handle.clone();
                        
                        match app_handle.global_shortcut().on_shortcut(shortcut, move |_app_handle, _shortcut, _event| {
                            let command_clone2 = command_clone.clone();
                            let app_handle_clone2 = app_handle_clone.clone();
                            
                            std::thread::spawn(move || {
                                tauri::async_runtime::block_on(async move {
                                    match execute_saved_command(&command_clone2).await {
                                        Ok(_) => {
                                            // Emit an event to the frontend
                                            let _ = app_handle_clone2.emit("command-executed", &command_clone2);
                                        },
                                        Err(e) => {
                                            eprintln!("Error executing command: {}", e);
                                        }
                                    }
                                });
                            });
                        }) {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to register shortcut '{}': {}", shortcut_str, e)
                        }
                    },
                    Err(e) => eprintln!("Invalid shortcut format '{}': {:?}", shortcut_str, e)
                }
            }
        }
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--minimized"])))
        .plugin(
            Builder::new()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir { file_name: None }),
                    Target::new(TargetKind::Webview),
                ])
                .build(),
        )
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            
            // Check if we should auto-hide to tray on startup
            let settings = load_app_settings(&app_handle);
            if settings.auto_hide_to_tray == Some(true) {
                // Hide the main window on startup
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            
            // Register saved command shortcuts
            register_saved_shortcuts(&app_handle);            
            // Setup system tray
            let app_handle_clone = app_handle.clone();
            
            // Create tray menu
            let open_item = MenuItem::with_id(app, "open", "Open", true, None::<&str>)?;
            let close_item = MenuItem::with_id(app, "close", "Close", true, None::<&str>)?;
            let tray_menu = MenuBuilder::new(app)
                .item(&open_item)
                .item(&close_item)
                .build()?;
            
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Mi Home Toolkit")
                .menu(&tray_menu)
                .on_tray_icon_event(move |tray, event| {
                    match event {
                        tauri::tray::TrayIconEvent::Click { button: MouseButton::Left, .. } => {
                            if let Some(window) = tray.app_handle().get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        _ => {}
                    }
                })
                .on_menu_event(move |app, event| {
                    match event.id().as_ref() {
                        "open" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "close" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let app_handle = window.app_handle();
                let settings = load_app_settings(&app_handle);
                
                match settings.close_to_tray {
                    Some(true) => {
                        // Hide to tray
                        window.hide().unwrap();
                        api.prevent_close();
                    }
                    Some(false) => {
                        // Close normally
                        // Let the default behavior happen
                    }
                    None => {
                        // First time - ask user and prevent close for now
                        window.emit("show-close-preference-dialog", ()).unwrap();
                        api.prevent_close();
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            login,
            get_countries,
            set_country,
            get_device,
            get_devices,
            call_device,
            is_logged_in,
            try_auto_login,
            logout,
            get_saved_credentials,
            get_current_user,
            is_session_restored,
            save_command,
            update_command,
            delete_command,
            get_saved_commands,
            validate_shortcut,
            get_app_settings,
            save_close_to_tray_preference,
            save_auto_start_preference,
            save_auto_hide_preference,
            save_all_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
