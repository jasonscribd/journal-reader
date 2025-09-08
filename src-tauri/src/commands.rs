use crate::Result;
use serde::{Deserialize, Serialize};
use sha2::Digest as _;

#[derive(Debug, Serialize, Deserialize)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportRequest {
    pub paths: Vec<String>,
    pub default_date: Option<String>,
    pub bulk_date_mode: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileImportItem {
    pub path: String,
    pub title: Option<String>,
    pub size_bytes: u64,
    pub file_type: String,
    pub suggested_date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportResult {
    pub imported: u32,
    pub failed: u32,
    pub errors: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileWithDate {
    pub path: String,
    pub entry_date: String,
    pub entry_timezone: String,
}

// Removed search types in simplified app

#[derive(Debug, Serialize, Deserialize)]
pub struct TimelineData {
    pub years: Vec<YearData>,
    pub total_entries: u32,
    pub date_range: Option<(String, String)>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct YearData {
    pub year: i32,
    pub total_count: u32,
    pub months: Vec<MonthData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MonthData {
    pub month: u32,
    pub month_name: String,
    pub count: u32,
    pub entries: Vec<EntryPreview>,
}

// Simplified timeline: provided via year/month counts and list entries per month

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EntryPreview {
    pub id: String,
    pub title: Option<String>,
    pub preview: String,
    pub entry_date: String,
    pub tags: Vec<String>,
}
#[tauri::command]
pub async fn search_entries_simple(app_handle: tauri::AppHandle, query: String, limit: Option<u32>) -> Result<Vec<EntryPreview>> {
    use tokio::time::{timeout, Duration};
    let lim = limit.unwrap_or(50);
    let trimmed = query.trim().to_string();

    println!("[search] start query='{}' limit={}", trimmed, lim);
    let started = std::time::Instant::now();

    let fut = crate::database::search_entries_fts_simple(&app_handle, &trimmed, lim);
    let timed = timeout(Duration::from_secs(10), fut).await;

    let results = match timed {
        Ok(inner) => inner.map_err(|e| crate::AppError { message: format!("Search error: {}", e), code: Some("SEARCH_ERROR".into()) })?,
        Err(_) => {
            println!("[search] timeout query='{}'", trimmed);
            return Err(crate::AppError { message: "Search timed out".into(), code: Some("TIMEOUT".into()) });
        }
    };

    let elapsed = started.elapsed().as_millis();
    println!("[search] done query='{}' ms={} results={}", trimmed, elapsed, results.len());

    Ok(results.into_iter().map(|(e, snip)| EntryPreview {
        id: e.id,
        title: e.title,
        preview: if snip.is_empty() { create_preview(&e.body, 240) } else { snip },
        entry_date: e.entry_date.to_rfc3339(),
        tags: vec![],
    }).collect())
}

// Removed chat request in simplified app

#[tauri::command]
pub async fn greet(name: &str) -> Result<String> {
    Ok(format!("Hello, {}! Welcome to Journal Reader!", name))
}

#[tauri::command]
pub async fn init_database(app_handle: tauri::AppHandle) -> Result<()> {
    crate::database::init_database(&app_handle).await?;
    // Backfill FTS on startup
    if let Err(e) = crate::database::ensure_fts_populated(&app_handle).await {
        eprintln!("[fts] backfill error: {}", e);
    }
    Ok(())
}

#[tauri::command]
pub async fn get_settings(app_handle: tauri::AppHandle) -> Result<Vec<Setting>> {
    let items = crate::database::get_settings(&app_handle).await.map_err(|e| crate::AppError { message: e.to_string(), code: Some("SETTINGS_READ".into()) })?;
    Ok(items.into_iter().map(|(key, value)| Setting { key, value }).collect())
}

#[tauri::command]
pub async fn update_setting(app_handle: tauri::AppHandle, key: String, value: String) -> Result<()> {
    crate::database::update_setting(&app_handle, &key, &value).await.map_err(|e| crate::AppError { message: e.to_string(), code: Some("SETTINGS_WRITE".into()) })?;
    Ok(())
}

#[tauri::command]
pub async fn test_ai_connection(app_handle: tauri::AppHandle) -> Result<bool> {
    use std::time::Duration;
    let settings = crate::database::get_settings(&app_handle).await.map_err(|e| crate::AppError { message: e.to_string(), code: Some("SETTINGS_READ".into()) })?;
    let mut provider = "ollama".to_string();
    let mut ollama_url = "http://localhost:11434".to_string();
    for (k, v) in settings {
        if k == "ai_provider" { provider = v; }
        else if k == "ollama_url" { ollama_url = v; }
    }

    if provider != "ollama" { return Ok(false); }

    let url = format!("{}/api/tags", ollama_url.trim_end_matches('/'));
    let client = reqwest::Client::builder().timeout(Duration::from_secs(3)).build().map_err(|e| crate::AppError { message: e.to_string(), code: Some("HTTP".into()) })?;
    match client.get(url).send().await {
        Ok(resp) => Ok(resp.status().is_success() || resp.status().as_u16() == 404),
        Err(_) => Ok(false),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleOAuthStatus {
    pub connected: bool,
}

#[tauri::command]
pub async fn get_google_oauth_status(app_handle: tauri::AppHandle) -> Result<GoogleOAuthStatus> {
    let settings = crate::database::get_settings(&app_handle).await.map_err(|e| crate::AppError { message: e.to_string(), code: Some("SETTINGS_READ".into()) })?;
    let mut has_token = false;
    for (k, _) in settings {
        if k == "google_access_token" { has_token = true; break; }
    }
    Ok(GoogleOAuthStatus { connected: has_token })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleOAuthInit {
    pub auth_url: String,
    pub state: String,
    pub code_verifier: String,
}

#[tauri::command]
pub async fn google_oauth_start(app_handle: tauri::AppHandle) -> Result<GoogleOAuthInit> {
    use rand::{distributions::Alphanumeric, Rng};
    let settings = crate::database::get_settings(&app_handle).await.map_err(|e| crate::AppError { message: e.to_string(), code: Some("SETTINGS_READ".into()) })?;
    let mut client_id = String::new();
    for (k, v) in settings {
        if k == "google_client_id" { client_id = v; }
    }
    if client_id.is_empty() {
        return Err(crate::AppError { message: "Missing Google Client ID in settings".into(), code: Some("GOOGLE_CLIENT_ID".into()) });
    }

    // PKCE code_verifier and challenge
    let code_verifier: String = rand::thread_rng().sample_iter(&Alphanumeric).take(64).map(char::from).collect();
    let sha = sha2::Sha256::digest(code_verifier.as_bytes());
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(sha);
    let state: String = rand::thread_rng().sample_iter(&Alphanumeric).take(24).map(char::from).collect();

    // Loopback redirect
    let redirect_uri = "http://127.0.0.1:8765/callback";
    let scope = urlencoding::encode("https://www.googleapis.com/auth/drive.readonly");
    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?response_type=code&client_id={}&redirect_uri={}&scope={}&access_type=offline&prompt=consent&code_challenge_method=S256&code_challenge={}&state={}",
        urlencoding::encode(&client_id),
        urlencoding::encode(redirect_uri),
        scope,
        challenge,
        state
    );

    Ok(GoogleOAuthInit { auth_url, state, code_verifier })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleOAuthCompleteRequest {
    pub code: String,
    pub state: String,
    pub code_verifier: String,
}

#[tauri::command]
pub async fn google_oauth_complete(app_handle: tauri::AppHandle, req: GoogleOAuthCompleteRequest) -> Result<bool> {
    // Exchange code for tokens
    let settings = crate::database::get_settings(&app_handle).await.map_err(|e| crate::AppError { message: e.to_string(), code: Some("SETTINGS_READ".into()) })?;
    let mut client_id = String::new();
    for (k, v) in settings.clone() {
        if k == "google_client_id" { client_id = v; }
    }
    if client_id.is_empty() {
        return Err(crate::AppError { message: "Missing Google Client ID in settings".into(), code: Some("GOOGLE_CLIENT_ID".into()) });
    }
    let redirect_uri = "http://127.0.0.1:8765/callback";
    let token_url = "https://oauth2.googleapis.com/token";
    let client = reqwest::Client::new();
    let params = [
        ("grant_type", "authorization_code"),
        ("code", req.code.as_str()),
        ("client_id", client_id.as_str()),
        ("redirect_uri", redirect_uri),
        ("code_verifier", req.code_verifier.as_str()),
    ];
    let resp = client.post(token_url).form(&params).send().await.map_err(|e| crate::AppError { message: e.to_string(), code: Some("HTTP".into()) })?;
    if !resp.status().is_success() {
        return Err(crate::AppError { message: format!("Token exchange failed: {}", resp.status()), code: Some("TOKEN".into()) });
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| crate::AppError { message: e.to_string(), code: Some("JSON".into()) })?;
    let access = json.get("access_token").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let refresh = json.get("refresh_token").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if access.is_empty() {
        return Ok(false);
    }
    // Store tokens
    crate::database::update_setting(&app_handle, "google_access_token", &access).await.map_err(|e| crate::AppError { message: e.to_string(), code: Some("SETTINGS_WRITE".into()) })?;
    if !refresh.is_empty() {
        let _ = crate::database::update_setting(&app_handle, "google_refresh_token", &refresh).await;
    }
    Ok(true)
}

async fn google_get_valid_access_token(app_handle: &tauri::AppHandle) -> std::result::Result<String, anyhow::Error> {
    let settings = crate::database::get_settings(app_handle).await?;
    let mut client_id = String::new();
    let mut access = String::new();
    let mut refresh = String::new();
    for (k, v) in settings {
        if k == "google_client_id" { client_id = v; }
        else if k == "google_access_token" { access = v; }
        else if k == "google_refresh_token" { refresh = v; }
    }
    if access.is_empty() && refresh.is_empty() { return Err(anyhow::anyhow!("No Google tokens")); }
    // Try a lightweight call to validate access token
    if !access.is_empty() {
        let resp = reqwest::Client::new()
            .get("https://www.googleapis.com/drive/v3/about?fields=user")
            .bearer_auth(&access)
            .send().await;
        if let Ok(r) = resp { if r.status().is_success() { return Ok(access); } }
    }
    // Refresh
    if !refresh.is_empty() && !client_id.is_empty() {
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh.as_str()),
            ("client_id", client_id.as_str()),
        ];
        let token_url = "https://oauth2.googleapis.com/token";
        let resp = reqwest::Client::new().post(token_url).form(&params).send().await?;
        if !resp.status().is_success() { return Err(anyhow::anyhow!("Refresh failed: {}", resp.status())); }
        let json: serde_json::Value = resp.json().await?;
        let new_access = json.get("access_token").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if new_access.is_empty() { return Err(anyhow::anyhow!("No access_token in refresh response")); }
        // Persist
        let _ = crate::database::update_setting(app_handle, "google_access_token", &new_access).await;
        return Ok(new_access);
    }
    Err(anyhow::anyhow!("No valid Google token"))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportGDocByIdRequest {
    pub file_id: String,
    pub entry_date: String,       // RFC3339
    pub entry_timezone: String,   // e.g., "UTC"
}

#[tauri::command]
pub async fn google_import_doc_by_file_id(app_handle: tauri::AppHandle, req: ImportGDocByIdRequest) -> Result<String> {
    use chrono::{DateTime, Utc};
    use crate::import::{ParsedFile, FileType, normalize_content};
    use sha2::Sha256;

    let access = google_get_valid_access_token(&app_handle).await
        .map_err(|e| crate::AppError { message: format!("Google token error: {}", e), code: Some("GOOGLE_TOKEN".into()) })?;

    // Try text export first
    let base = format!("https://www.googleapis.com/drive/v3/files/{}", req.file_id);
    let txt_url = format!("{}/export?mimeType=text/plain", base);
    let client = reqwest::Client::new();
    let mut content = String::new();
    let resp = client.get(&txt_url).bearer_auth(&access).send().await
        .map_err(|e| crate::AppError { message: e.to_string(), code: Some("HTTP".into()) })?;
    if resp.status().is_success() {
        content = resp.text().await.unwrap_or_default();
    } else {
        // Fallback to docx export
        let docx_url = format!("{}/export?mimeType=application/vnd.openxmlformats-officedocument.wordprocessingml.document", base);
        let resp2 = client.get(&docx_url).bearer_auth(&access).send().await
            .map_err(|e| crate::AppError { message: e.to_string(), code: Some("HTTP".into()) })?;
        if resp2.status().is_success() {
            let bytes = resp2.bytes().await.unwrap_or_default();
            let tmp = std::env::temp_dir().join(format!("{}.docx", req.file_id));
            let _ = std::fs::write(&tmp, &bytes);
            if let Ok(text) = crate::import::parse_docx_file(tmp.to_string_lossy().as_ref()).await {
                content = text;
            }
            let _ = std::fs::remove_file(&tmp);
        }
    }
    if content.trim().is_empty() {
        return Err(crate::AppError { message: "Failed to export Google Doc content".into(), code: Some("GDRIVE_EXPORT".into()) });
    }

    let content = normalize_content(&content);

    // Optionally fetch file name for title
    let meta_url = format!("{}?fields=name", base);
    let title = client.get(&meta_url).bearer_auth(&access).send().await.ok()
        .and_then(|r| r.json::<serde_json::Value>().ok())
        .and_then(|j| j.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()));

    // Build ParsedFile
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let text_hash = format!("{:x}", hasher.finalize());
    let parsed = ParsedFile {
        path: format!("gdrive:{}", req.file_id),
        content: content.clone(),
        title,
        file_type: FileType::Txt,
        text_hash,
        size_bytes: content.len() as u64,
    };

    // Parse date
    let entry_date = DateTime::parse_from_rfc3339(&req.entry_date)
        .map_err(|e| crate::AppError { message: format!("Invalid date: {}", e), code: Some("DATE".into()) })?
        .with_timezone(&Utc);

    let id = crate::database::save_entry(&app_handle, parsed, entry_date, req.entry_timezone).await
        .map_err(|e| crate::AppError { message: e.to_string(), code: Some("SAVE".into()) })?;
    Ok(id)
}

#[tauri::command]
pub async fn scan_import_files(_app_handle: tauri::AppHandle, paths: Vec<String>) -> Result<Vec<FileImportItem>> {
    use crate::import::{parse_file, FileType};
    use std::path::Path;
    use walkdir::WalkDir;
    
    let mut files = Vec::new();
    
    for path_str in paths {
        let path = Path::new(&path_str);
        
        if path.is_file() {
            // Single file
            if let Ok(parsed) = parse_file(&path_str).await {
                files.push(FileImportItem {
                    path: path_str,
                    title: parsed.title,
                    size_bytes: parsed.size_bytes,
                    file_type: parsed.file_type.as_str().to_string(),
                    suggested_date: None, // We'll let the user specify dates
                });
            }
        } else if path.is_dir() {
            // Directory - walk recursively
            for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                let entry_path = entry.path();
                if entry_path.is_file() {
                    if let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) {
                        if FileType::from_extension(ext).is_some() {
                            let path_str = entry_path.to_string_lossy().to_string();
                            if let Ok(parsed) = parse_file(&path_str).await {
                                files.push(FileImportItem {
                                    path: path_str,
                                    title: parsed.title,
                                    size_bytes: parsed.size_bytes,
                                    file_type: parsed.file_type.as_str().to_string(),
                                    suggested_date: None,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(files)
}

#[tauri::command]
pub async fn import_files_with_dates(
    app_handle: tauri::AppHandle, 
    files: Vec<FileWithDate>
) -> Result<ImportResult> {
    // use chrono::{DateTime, Utc};
    let mut imported = 0u32;
    let mut failed = 0u32;
    let mut errors: Vec<String> = Vec::new();

    for file in files {
        match process_single_file(&app_handle, file).await {
            Ok(_) => imported += 1,
                Err(e) => {
                    failed += 1;
                errors.push(e.message);
            }
        }
    }

    Ok(ImportResult { imported, failed, errors: if errors.is_empty() { None } else { Some(errors) } })
}

async fn process_single_file(
    app_handle: &tauri::AppHandle,
    file_with_date: FileWithDate,
) -> Result<String> {
    use crate::import::{parse_file, normalize_content};
    use crate::database::{save_entry, check_duplicate};
    use chrono::{DateTime, Utc};
    
    // Parse the file
    let mut parsed_file = parse_file(&file_with_date.path).await
        .map_err(|e| crate::AppError { 
            message: format!("Failed to parse file: {}", e), 
            code: Some("PARSE_ERROR".to_string()) 
        })?;
    
    // Normalize content
    parsed_file.content = normalize_content(&parsed_file.content);
    
    // Check for duplicates
    if let Some(existing_id) = check_duplicate(app_handle, &parsed_file.text_hash).await? {
        return Err(crate::AppError {
            message: format!("Duplicate content found (existing entry: {})", existing_id),
            code: Some("DUPLICATE".to_string()),
        });
    }
    
    // Parse the entry date
    let entry_date = DateTime::parse_from_rfc3339(&file_with_date.entry_date)
        .map_err(|e| crate::AppError {
            message: format!("Invalid date format: {}", e),
            code: Some("INVALID_DATE".to_string()),
        })?
        .with_timezone(&Utc);
    
    // Save to database
    let entry_id = save_entry(
        app_handle,
        parsed_file,
        entry_date,
        file_with_date.entry_timezone,
    ).await?;
    
    Ok(entry_id)
}

// Removed: background import job status

// Removed: complex search; may reintroduce later if needed

#[tauri::command]
pub async fn get_available_years(app_handle: tauri::AppHandle) -> Result<Vec<i32>> {
    let years = crate::database::get_available_years(&app_handle).await?;
    Ok(years)
}

#[tauri::command]
pub async fn get_month_counts_for_year(app_handle: tauri::AppHandle, year: i32) -> Result<Vec<crate::database::MonthCount>> {
    let months = crate::database::get_month_counts_for_year(&app_handle, year).await?;
    Ok(months)
}

#[tauri::command]
pub async fn list_entries_for_month(app_handle: tauri::AppHandle, year: i32, month: u32) -> Result<Vec<EntryPreview>> {
    let entries = crate::database::list_entries_by_month(&app_handle, year, month).await?;
    let previews: Vec<EntryPreview> = entries.into_iter().map(|e| EntryPreview {
        id: e.id,
        title: e.title,
        preview: create_preview(&e.body, 200),
        entry_date: e.entry_date.to_rfc3339(),
        tags: vec![],
    }).collect();
    Ok(previews)
}

// Removed calendar heatmap for simplified UI

// Removed day view for simplified UI

fn create_preview(text: &str, max_len: usize) -> String {
    let mut s = text.trim().replace('\n', " ");
    if s.len() > max_len { s.truncate(max_len); s.push_str("..."); }
    s
}

// Helper function to get month name
fn get_month_name(month: u32) -> String {
    match month {
        1 => "January",
        2 => "February", 
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    }.to_string()
}

#[tauri::command]
pub async fn get_entry_by_id(app_handle: tauri::AppHandle, id: String) -> Result<Option<EntryPreview>> {
    if let Some(e) = crate::database::get_entry_by_id(&app_handle, &id).await? {
        Ok(Some(EntryPreview {
            id: e.id,
            title: e.title,
            preview: e.body,
            entry_date: e.entry_date.to_rfc3339(),
            tags: vec![],
        }))
    } else {
    Ok(None)
}
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DbDiagnostics {
    pub db_path: String,
    pub total_entries: u32,
    pub years: Vec<i32>,
}

#[tauri::command]
pub async fn get_db_diagnostics(app_handle: tauri::AppHandle) -> Result<DbDiagnostics> {
    let info = crate::database::get_db_info(&app_handle).await.map_err(|e| crate::AppError { message: format!("DB info error: {}", e), code: Some("DB_INFO".into()) })?;
    println!("[db] path={} total_entries={}", info.db_path, info.total_entries);
    Ok(DbDiagnostics { db_path: info.db_path, total_entries: info.total_entries, years: info.years })
}

// Removed AI/tagging-related commands in simplified app

// --

// --

// --

// --

// --

// --

// --

#[derive(Debug, Serialize, Deserialize)]
pub struct TagStatistic {
    pub tag: String,
    pub count: u32,
    pub percentage: f32,
    pub recent_usage: String,
}

// --

// Removed AI chat in simplified app

// --

// --

// --

// --

// --

// --


