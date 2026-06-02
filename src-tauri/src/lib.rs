pub mod commands;
pub mod data;
pub mod error;
pub mod models;
pub mod services;

use tauri::Manager;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! AnyflipDL is running.", name)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Create a shared HTTP client with 30s timeout and browser User-Agent.
            // Anyflip's CloudFront CDN blocks requests without a browser User-Agent.
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .user_agent("Mozilla/5.0")
                .build()
                .expect("Failed to create HTTP client");

            // Register HTTP client as managed state for command injection
            // (used by update_checker and potentially other services).
            app.manage(client.clone());

            // Start the download queue manager.
            let (queue_handle, _task) =
                services::queue_manager::start(client, app.handle().clone());

            // Register QueueHandle as managed state for command injection.
            app.manage(queue_handle);

            tracing::info!("Queue manager started and registered as managed state");

            // Run stale temp directory cleanup in background (non-blocking).
            std::thread::spawn(|| {
                let temp = data::temp::TempStorage::default();
                match temp.cleanup_stale() {
                    Ok(0) => {
                        tracing::info!("No stale temp directories found");
                    }
                    Ok(count) => {
                        tracing::info!("Cleaned up {} stale temp directories on startup", count);
                    }
                    Err(e) => {
                        tracing::warn!("Temp cleanup failed on startup: {}", e);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::validate_url,
            commands::fetch_document_metadata,
            commands::download_document,
            commands::cancel_download,
            commands::pause_download,
            commands::resume_download,
            commands::submit_password,
            commands::get_settings,
            commands::save_settings,
            commands::pick_directory,
            commands::open_file,
            commands::rename_file,
            commands::get_file_size,
            commands::check_for_updates,
        ])
        .run(tauri::generate_context!())
        .expect("error while running AnyflipDL");
}
