// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

extern crate anyhow;
extern crate miio;
extern crate serde_json;

use lazy_static::lazy_static;
use miio::{Device, MiCloudProtocol, Credentials, SecureSession};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fs, path::PathBuf, str::FromStr, sync::Arc};
use tauri::{Manager, AppHandle};
use tauri_plugin_log::{Builder, Target, TargetKind};
use tokio::sync::Mutex;

// Simple struct for backwards compatibility - still used for some operations
#[derive(Serialize, Deserialize, Debug, Clone)]
struct SavedCredentials {
    username: String,
    country: String,
}

lazy_static! {
    static ref MI_CLOUD_PROTOCOL: Arc<Mutex<MiCloudProtocol>> =
        Arc::new(Mutex::new(MiCloudProtocol::new()));
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

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
async fn login(
    app_handle: AppHandle, 
    email: String, 
    password: String, 
    country: Option<String>, 
    should_save_credentials: bool
) -> Result<(), String> {    // Default country is "cn" if not specified
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
async fn set_country(country: String) {
    let mut guard = MI_CLOUD_PROTOCOL.lock().await;
    guard.set_country(&country)
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
        .map(|params| Value::from_str(params.as_str()).map_err(|err| err.to_string()))
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

fn main() {
    tauri::Builder::default()
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
        .plugin(tauri_plugin_fs::init())        .invoke_handler(tauri::generate_handler![
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
            is_session_restored
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
