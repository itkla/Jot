use tauri::{Manager, Window};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tokio::fs;
use std::path::PathBuf;

// commands for file operations
#[tauri::command]
async fn new_file(app: tauri::AppHandle) -> Result<(), String> {
    match app.webview_windows().get("main") {
        Some(_main_window) => {
            let _new_window = tauri::WebviewWindowBuilder::new(
                &app,
                format!("notepad_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()),
                tauri::WebviewUrl::App("index.html".into())
            )
            .title("Untitled - Jot")
            .inner_size(800.0, 600.0)
            .build()
            .map_err(|e| e.to_string())?;
            
            Ok(())
        },
        None => Err("Main window not found".to_string())
    }
}

#[tauri::command]
async fn open_file_with_confirmation(window: Window, has_unsaved_changes: bool) -> Result<Option<(String, String)>, String> {
    // confirmation dialog using native Tauri
    if has_unsaved_changes {
        let confirmed = window.app_handle().dialog()
            .message("You have unsaved changes. Do you want to continue without saving?")
            .kind(MessageDialogKind::Warning)
            .blocking_show();
        
        if !confirmed {
            return Ok(None);
        }
    }
    
    // file picker
    match window.app_handle().dialog().file().blocking_pick_file() {
        Some(file_path) => {
            let path_buf = PathBuf::from(file_path.to_string());
            let path_str = path_buf.to_string_lossy().to_string();
            match fs::read_to_string(&path_buf).await {
                Ok(content) => {
                    // Update title immediately
                    let filename = path_buf.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Untitled");
                    let _ = window.set_title(&format!("{} - Jot", filename));
                    
                    Ok(Some((content, path_str)))
                },
                Err(e) => Err(format!("Failed to read file: {}", e))
            }
        },
        None => Ok(None) // User cancelled
    }
}

#[tauri::command]
async fn save_file(window: Window, file_path: Option<String>, content: String) -> Result<Option<String>, String> {
    let path = match file_path {
        Some(p) => p,
        None => {
            // save-as dialog
            match window.app_handle().dialog().file().blocking_save_file() {
                Some(file_path) => file_path.to_string(),
                None => return Ok(None) // User cancelled
            }
        }
    };
    
    match fs::write(&path, content).await {
        Ok(_) => {
            // Update title immediately
            let filename = std::path::Path::new(&path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Untitled");
            let _ = window.set_title(&format!("{} - Jot", filename));
            
            Ok(Some(path))
        },
        Err(e) => Err(format!("Failed to save file: {}", e))
    }
}

#[tauri::command]
async fn save_as_file(window: Window, content: String) -> Result<Option<String>, String> {
    // save-as dialog
    match window.app_handle().dialog().file().blocking_save_file() {
        Some(file_path) => {
            let path_buf = PathBuf::from(file_path.to_string());
            let path_str = path_buf.to_string_lossy().to_string();
            match fs::write(&path_buf, content).await {
                Ok(_) => {
                    // Update title immediately
                    let filename = path_buf.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Untitled");
                    let _ = window.set_title(&format!("{} - Jot", filename));
                    
                    Ok(Some(path_str))
                },
                Err(e) => Err(format!("Failed to save file: {}", e))
            }
        },
        None => Ok(None) // User cancelled
    }
}

#[tauri::command]
async fn clear_document_with_confirmation(window: Window, has_unsaved_changes: bool) -> Result<bool, String> {
    if has_unsaved_changes {
        let confirmed = window.app_handle().dialog()
            .message("You have unsaved changes. Do you want to continue without saving?")
            .kind(MessageDialogKind::Warning)
            .blocking_show();
        
        if !confirmed {
            return Ok(false);
        }
    }
    
    // Update title immediately
    let _ = window.set_title("Untitled - Jot");
    Ok(true)
}

#[tauri::command]
async fn update_title(window: Window, filename: Option<String>, is_modified: bool) -> Result<(), String> {
    let title = match filename {
        Some(name) => {
            if is_modified {
                format!("*{} - Jot", name)
            } else {
                format!("{} - Jot", name)
            }
        },
        None => {
            if is_modified {
                "*Untitled - Jot".to_string()
            } else {
                "Untitled - Jot".to_string()
            }
        }
    };
    
    window.set_title(&title).map_err(|e| e.to_string())
}

// Setup for future native shortcuts if needed
fn setup_shortcuts(_app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            new_file,
            open_file_with_confirmation,
            save_file,
            save_as_file,
            clear_document_with_confirmation,
            update_title
        ])
        .setup(|app| {
            setup_shortcuts(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
