use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use chrono::{DateTime, Utc};
use sha2::{Sha256, Digest};

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportJob {
    pub id: String,
    pub root_path: String,
    pub status: ImportStatus,
    pub total_files: u32,
    pub processed: u32,
    pub succeeded: u32,
    pub failed: u32,
    pub error_log: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ImportStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedFile {
    pub path: String,
    pub content: String,
    pub title: Option<String>,
    pub file_type: FileType,
    pub text_hash: String,
    pub size_bytes: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum FileType {
    Txt,
    Docx,
    GDoc,
}

impl FileType {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "txt" => Some(FileType::Txt),
            "doc" | "docx" => Some(FileType::Docx),
            "gdoc" => Some(FileType::GDoc),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            FileType::Txt => "txt",
            FileType::Docx => "docx",
            FileType::GDoc => "gdoc",
        }
    }
}

pub async fn parse_file(file_path: &str) -> Result<ParsedFile> {
    let path = Path::new(file_path);
    
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .context("Failed to get file extension")?;
    
    let file_type = FileType::from_extension(extension)
        .context("Unsupported file type")?;
    
    let metadata = fs::metadata(path)
        .context("Failed to read file metadata")?;
    
    let content = match file_type {
        FileType::Txt => parse_txt_file(file_path).await?,
        FileType::Docx => parse_docx_file(file_path).await?,
        FileType::GDoc => parse_gdoc_file(file_path).await?,
    };
    
    // Generate content hash for deduplication
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let text_hash = format!("{:x}", hasher.finalize());
    
    // Extract title from first line or filename
    let title = extract_title(&content, path);
    
    Ok(ParsedFile {
        path: file_path.to_string(),
        content,
        title,
        file_type,
        text_hash,
        size_bytes: metadata.len(),
    })
}

pub async fn parse_txt_file(path: &str) -> Result<String> {
    let content = fs::read_to_string(path)
        .context("Failed to read TXT file")?;
    
    // Normalize line endings and clean up whitespace
    let normalized = content
        .replace("\r\n", "\n")
        .replace("\r", "\n")
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();
    
    Ok(normalized)
}

pub async fn parse_docx_file(path: &str) -> Result<String> {
    use std::process::Command;
    
    // Try to use pandoc if available to convert DOCX to text
    match Command::new("pandoc")
        .args(["-f", "docx", "-t", "plain", path])
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                let content = String::from_utf8_lossy(&output.stdout).to_string();
                return Ok(normalize_content(&content));
            }
        }
        Err(_) => {
            // Pandoc not available, continue to fallback
        }
    }
    
    // Fallback: Try to extract text using basic ZIP parsing
    // DOCX files are ZIP archives with XML content
    match extract_docx_text_basic(path) {
        Ok(content) => Ok(normalize_content(&content)),
        Err(_) => {
            // If all methods fail, return a helpful error
            Err(anyhow::anyhow!(
                "DOCX parsing failed. Please install pandoc or convert to TXT format. File: {}", 
                path
            ))
        }
    }
}

// Parse Google Docs link files (.gdoc). These are small JSON files pointing to the web URL.
// We import a placeholder entry containing the doc URL so it shows up in the timeline/search.
// For full text, export from Google Docs to .docx or .txt and import that file.
pub async fn parse_gdoc_file(path: &str) -> Result<String> {
    let text = std::fs::read_to_string(path).context("Failed to read GDOC file")?;
    let json: serde_json::Value = serde_json::from_str(&text).context("Failed to parse GDOC JSON")?;
    let url = json.get("url").and_then(|v| v.as_str()).unwrap_or("");
    let name = json.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let placeholder = if !url.is_empty() {
        format!(
            "Google Doc link: {}\n\nTitle: {}\n\nNote: Export the Google Doc as .docx or .txt and re-import to capture full text.",
            url,
            name
        )
    } else {
        "Google Doc placeholder. Note: Export the Google Doc as .docx or .txt and re-import to capture full text.".to_string()
    };
    Ok(placeholder)
}

// Basic DOCX text extraction using ZIP parsing
fn extract_docx_text_basic(path: &str) -> Result<String> {
    use std::fs::File;
    use std::io::Read;
    
    // Read the file as a ZIP archive
    let file = File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    
    // Look for the main document XML file
    let mut xml_content = String::new();
    {
        let mut document_file = archive.by_name("word/document.xml")?;
        document_file.read_to_string(&mut xml_content)?;
    }
    
    // Basic XML text extraction (remove tags, keep text content)
    let text = extract_text_from_xml(&xml_content);
    Ok(text)
}

// Extract text from XML by removing tags
fn extract_text_from_xml(xml: &str) -> String {
    let mut text = String::new();
    let mut inside_tag = false;
    let mut inside_text = false;
    
    for ch in xml.chars() {
        match ch {
            '<' => {
                inside_tag = true;
                if inside_text {
                    text.push(' '); // Add space between text elements
                }
                inside_text = false;
            }
            '>' => {
                inside_tag = false;
            }
            _ if !inside_tag => {
                text.push(ch);
                inside_text = true;
            }
            _ => {}
        }
    }
    
    // Clean up the extracted text
    text.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn extract_title(content: &str, path: &Path) -> Option<String> {
    // Try to extract title from first line if it looks like a title
    let first_line = content.lines().next()?.trim();
    
    // If first line is short and doesn't end with punctuation, use it as title
    if first_line.len() > 0 && first_line.len() < 100 && !first_line.ends_with('.') {
        // Check if it looks like a date or title
        if !first_line.chars().all(|c| c.is_numeric() || c == '-' || c == '/' || c == ' ') {
            return Some(first_line.to_string());
        }
    }
    
    // Fallback to filename without extension
    path.file_stem()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
}

pub fn normalize_content(content: &str) -> String {
    // Normalize whitespace, quotes, and common formatting
    content
        .replace('"', "\"")
        .replace('"', "\"")
        .replace('\'', "'")
        .replace('\'', "'")
        .replace('—', "--")
        .replace('–', "-")
        // Collapse multiple spaces
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

pub fn detect_language(_content: &str) -> String {
    // Simple language detection - for now just return English
    // In a real implementation, we might use a language detection library
    "en".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_file_type_from_extension() {
        assert!(matches!(FileType::from_extension("txt"), Some(FileType::Txt)));
        assert!(matches!(FileType::from_extension("TXT"), Some(FileType::Txt)));
        assert!(matches!(FileType::from_extension("docx"), Some(FileType::Docx)));
        assert!(matches!(FileType::from_extension("doc"), Some(FileType::Docx)));
        assert!(FileType::from_extension("pdf").is_none());
    }
    
    #[test]
    fn test_normalize_content() {
        let input = "Hello   \"world\"  with—dashes";
        let expected = "Hello \"world\" with--dashes";
        assert_eq!(normalize_content(input), expected);
    }
}
