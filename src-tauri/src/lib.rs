// use tauri::Manager; // not needed currently
use serde::{Deserialize, Serialize};

mod commands;
mod database;
mod import;
// mod search; // removed in simplified build
// mod ai; // removed in simplified build

#[derive(Debug, Serialize, Deserialize)]
pub struct AppError {
    pub message: String,
    pub code: Option<String>,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        Self {
            message: error.to_string(),
            code: None,
        }
    }
}

type Result<T> = std::result::Result<T, AppError>;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_sql::Builder::default().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::init_database,
            commands::get_settings,
            commands::update_setting,
            commands::scan_import_files,
            commands::import_files_with_dates,
            commands::get_available_years,
            commands::get_month_counts_for_year,
            commands::list_entries_for_month,
            commands::get_entry_by_id,
            commands::search_entries_simple,
            commands::get_db_diagnostics,
            commands::test_ai_connection,
            commands::get_google_oauth_status,
            commands::google_oauth_start,
            commands::google_oauth_complete,
            commands::google_import_doc_by_file_id,
            
        ])
        .setup(|app| {
            // Initialize the database on startup
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = database::init_database(&app_handle).await {
                    eprintln!("Failed to initialize database: {}", e);
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
