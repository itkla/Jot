use tauri::{Manager, Window, Emitter};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tokio::fs;
use std::path::PathBuf;

fn attach_close_handler(window: &tauri::WebviewWindow) {
    let window_clone = window.clone();
    let window_label = window.label().to_string();
    
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            println!("Close requested for window: {}", window_label);
            api.prevent_close();
            
            match window_clone.emit("quit-requested", ()) {
                Ok(_) => println!("Successfully emitted quit-requested event for window: {}", window_label),
                Err(e) => println!("Failed to emit quit-requested event for window {}: {}", window_label, e),
            }
        }
    });
}

// commands for file operations
#[tauri::command]
async fn new_file(app: tauri::AppHandle) -> Result<(), String> {
    match app.webview_windows().get("main") {
        Some(_main_window) => {
            let new_window = tauri::WebviewWindowBuilder::new(
                &app,
                format!("notepad_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()),
                tauri::WebviewUrl::App("index.html".into())
            )
            .title("Untitled - Jot")
            .inner_size(800.0, 600.0)
            .build()
            .map_err(|e| e.to_string())?;
            
            // Attach close event handler to the new window
            attach_close_handler(&new_window);
            
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
            .title("Unsaved Changes")
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::YesNo)
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
            // Clear recovery file after successful save
            let _ = clear_recovery_file_internal(&window).await;
            
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
                    // Clear recovery file after successful save
                    let _ = clear_recovery_file_internal(&window).await;
                    
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
            .title("Unsaved Changes")
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::YesNo)
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

// Auto-recovery functionality
fn get_recovery_dir() -> Result<PathBuf, String> {
    let app_data_dir = dirs::data_dir()
        .ok_or("Could not find app data directory")?
        .join("jot");
    
    if !app_data_dir.exists() {
        std::fs::create_dir_all(&app_data_dir)
            .map_err(|e| format!("Failed to create recovery directory: {}", e))?;
    }
    
    Ok(app_data_dir)
}

fn get_recovery_file_path(window_label: &str) -> Result<PathBuf, String> {
     let recovery_dir = get_recovery_dir()?;
    let sanitized_label = window_label
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect::<String>();
    let file_name = if sanitized_label == "main" {
         "recovery_main.txt".to_string()
     } else {
        format!("recovery_{}.txt", sanitized_label)
     };
     Ok(recovery_dir.join(file_name))
 }

#[tauri::command]
async fn auto_save_draft(window: Window, content: String) -> Result<(), String> {
    let recovery_path = get_recovery_file_path(window.label())?;
    if !content.trim().is_empty() {
        fs::write(&recovery_path, content).await
            .map_err(|e| format!("Failed to auto-save draft: {}", e))?;
        
        // Debug: Log successful auto-save
        // log::debug!("Auto-saved to: {:?}", recovery_path);
    } else {
        // Remove recovery file if content is empty
        if recovery_path.exists() {
            let _ = fs::remove_file(&recovery_path).await;
            println!("Removed empty recovery file: {:?}", recovery_path);
        }
    }
    
    Ok(())
}

#[tauri::command]
async fn get_recovery_content(window: Window) -> Result<Option<String>, String> {
    let recovery_path = get_recovery_file_path(window.label())?;
    
    // Debug: Log recovery attempt
    println!("Checking for recovery file: {:?}", recovery_path);
    
    if recovery_path.exists() {
        match fs::read_to_string(&recovery_path).await {
            Ok(content) => {
                if !content.trim().is_empty() {
                    println!("Found recovery content: {} chars", content.len());
                    Ok(Some(content))
                } else {
                    println!("Recovery file exists but is empty");
                    Ok(None)
                }
            },
            Err(e) => {
                println!("Failed to read recovery file: {}", e);
                Ok(None)
            }
        }
    } else {
        println!("No recovery file found");
        Ok(None)
    }
}

 async fn clear_recovery_file_internal(window: &Window) -> Result<(), String> {
     let recovery_path = get_recovery_file_path(window.label())?;
     
     if recovery_path.exists() {
         fs::remove_file(&recovery_path).await
             .map_err(|e| format!("Failed to clear recovery file: {}", e))?;
     }
     
     Ok(())
 }

#[tauri::command]
async fn clear_recovery_file(window: Window) -> Result<(), String> {
    clear_recovery_file_internal(&window).await
}

#[tauri::command]
async fn show_recovery_dialog(window: Window) -> Result<bool, String> {
    let confirmed = window.app_handle().dialog()
        .message("We found unsaved changes from your previous session. Would you like to recover them?")
        .title("Recovery")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::YesNo)
        .blocking_show();
    
    Ok(confirmed)
}

#[tauri::command]
async fn handle_quit_request(window: Window, has_unsaved_changes: bool) -> Result<bool, String> {
    if has_unsaved_changes {
        let confirmed = window.app_handle().dialog()
            .message("You have unsaved changes. Are you sure you want to quit without saving?")
            .title("Unsaved Changes")
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::YesNo)
            .blocking_show();
        
        if confirmed {
            let _ = clear_recovery_file_internal(&window).await;
            return Ok(true);
        } else {
            return Ok(false);
        }
    }
    
    let _ = clear_recovery_file_internal(&window).await;
    Ok(true)
}

#[tauri::command]
async fn exit_app(app: tauri::AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}

fn setup_shortcuts(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    for window in app.webview_windows().values() {
        attach_close_handler(window);
    }
    
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
            update_title,
            auto_save_draft,
            get_recovery_content,
            clear_recovery_file,
            show_recovery_dialog,
            handle_quit_request,
            exit_app
        ])
        .setup(|app| {
            setup_shortcuts(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
