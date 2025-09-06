use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use tauri::AppHandle;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub id: String,
    pub title: Option<String>,
    pub body: String,
    pub entry_date: DateTime<Utc>,
    pub source_path: String,
    pub source_type: String,
    pub tags: Vec<String>,
    pub score: f32,
    pub snippet: String,
    pub rank_source: String, // "fts", "vector", or "hybrid"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchFilters {
    pub date_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub tags: Option<Vec<String>>,
    pub source_types: Option<Vec<String>>,
    pub min_score: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub filters: SearchFilters,
    pub limit: u32,
    pub offset: u32,
    pub search_type: SearchType,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SearchType {
    FullText,
    Semantic,
    Hybrid,
}

#[derive(Debug)]
struct RankedResult {
    result: SearchResult,
    fts_rank: Option<usize>,
    vector_rank: Option<usize>,
    rrf_score: f32,
}

// FTS5 Search Implementation
pub async fn full_text_search(
    app_handle: &AppHandle,
    query: &str,
    filters: &SearchFilters,
    limit: u32,
) -> Result<Vec<SearchResult>> {
    use crate::database::search_entries_fts;
    
    // Get entries from database using FTS search
    let entries = search_entries_fts(app_handle, query, limit * 2).await?;
    
    // Convert database entries to search results
    let mut results = Vec::new();
    for entry in entries {
        let snippet = generate_snippet(&entry.body, query, 200);
        let score = calculate_fts_score(&entry.body, &entry.title, query);
        
        let search_result = SearchResult {
            id: entry.id,
            title: entry.title,
            body: entry.body,
            entry_date: entry.entry_date,
            source_path: entry.source_path,
            source_type: entry.source_type,
            tags: vec![], // TODO: Load tags from database
            score,
            snippet,
            rank_source: "fts".to_string(),
        };
        results.push(search_result);
    }
    
    Ok(apply_filters(results, filters, limit))
}

// Vector Similarity Search Implementation
pub async fn vector_search(
    app_handle: &AppHandle,
    query: &str,
    filters: &SearchFilters,
    limit: u32,
) -> Result<Vec<SearchResult>> {
    use crate::database::list_entries;
    
    // Implement actual vector similarity search using embeddings
    use crate::ai::{generate_embedding, EmbeddingRequest};
    
    // Generate embedding for the query
    let embedding_request = EmbeddingRequest {
        text: query.to_string(),
        model: "default".to_string(),
    };
    
    let query_embedding = match generate_embedding(embedding_request).await {
        Ok(embedding) => embedding,
        Err(_) => {
            // Fallback to semantic keyword matching if embedding fails
            return semantic_keyword_search(app_handle, query, filters, limit).await;
        }
    };
    
    // Get all entries from database
    let entries = list_entries(app_handle, Some(limit * 5), None).await?;
    
    let mut results = Vec::new();
    for entry in entries {
        // Generate embedding for entry content
        let entry_text = format!("{} {}", 
            entry.title.as_ref().unwrap_or(&String::new()), 
            entry.body
        );
        
        let entry_embedding_request = EmbeddingRequest {
            text: entry_text,
            model: "default".to_string(),
        };
        
        let entry_embedding = match generate_embedding(entry_embedding_request).await {
            Ok(embedding) => embedding,
            Err(_) => continue, // Skip entries we can't generate embeddings for
        };
        
        // Calculate cosine similarity
        let similarity = cosine_similarity(&query_embedding, &entry_embedding);
        
        if similarity > 0.1 { // Only include entries with some similarity
            let snippet = generate_snippet(&entry.body, query, 200);
            
            let search_result = SearchResult {
                id: entry.id,
                title: entry.title,
                body: entry.body,
                entry_date: entry.entry_date,
                source_path: entry.source_path,
                source_type: entry.source_type,
                tags: vec![], // TODO: Load tags from database
                score: similarity,
                snippet,
                rank_source: "vector".to_string(),
            };
            results.push(search_result);
        }
    }
    
    // Sort by similarity score (descending)
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(limit as usize);
    
    Ok(apply_filters(results, filters, limit))
}

// Fallback semantic search using keyword matching
async fn semantic_keyword_search(
    app_handle: &AppHandle,
    query: &str,
    filters: &SearchFilters,
    limit: u32,
) -> Result<Vec<SearchResult>> {
    use crate::database::list_entries;
    
    // Get all entries from database
    let entries = list_entries(app_handle, Some(limit * 3), None).await?;
    
    // Calculate semantic similarity scores using keyword matching
    let mut results = Vec::new();
    for entry in entries {
        let semantic_score = calculate_semantic_similarity(&entry.body, &entry.title, query);
        
        if semantic_score > 0.3 { // Only include entries with reasonable similarity
            let snippet = generate_snippet(&entry.body, query, 200);
            
            let search_result = SearchResult {
                id: entry.id,
                title: entry.title,
                body: entry.body,
                entry_date: entry.entry_date,
                source_path: entry.source_path,
                source_type: entry.source_type,
                tags: vec![], // TODO: Load tags from database
                score: semantic_score,
                snippet,
                rank_source: "semantic".to_string(),
            };
            results.push(search_result);
        }
    }
    
    // Sort by similarity score
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(limit as usize);
    
    Ok(apply_filters(results, filters, limit))
}

// Hybrid Search with Reciprocal Rank Fusion (RRF)
pub async fn hybrid_search(
    app_handle: &AppHandle,
    query: &str,
    filters: &SearchFilters,
    limit: u32,
) -> Result<Vec<SearchResult>> {
    // Get results from both search methods
    let fts_results = full_text_search(app_handle, query, filters, limit * 2).await?;
    let vector_results = vector_search(app_handle, query, filters, limit * 2).await?;
    
    // Apply RRF to combine rankings
    let combined_results = reciprocal_rank_fusion(fts_results, vector_results, 60.0)?;
    
    // Apply final filtering and limit
    Ok(apply_filters(combined_results, filters, limit))
}

// Reciprocal Rank Fusion Algorithm
fn reciprocal_rank_fusion(
    fts_results: Vec<SearchResult>,
    vector_results: Vec<SearchResult>,
    k: f32,
) -> Result<Vec<SearchResult>> {
    let mut result_map: HashMap<String, RankedResult> = HashMap::new();
    
    // Process FTS results
    for (rank, result) in fts_results.into_iter().enumerate() {
        let rrf_score = 1.0 / (k + rank as f32 + 1.0);
        
        result_map.insert(result.id.clone(), RankedResult {
            result,
            fts_rank: Some(rank),
            vector_rank: None,
            rrf_score,
        });
    }
    
    // Process vector results and combine scores
    for (rank, result) in vector_results.into_iter().enumerate() {
        let rrf_score = 1.0 / (k + rank as f32 + 1.0);
        
        if let Some(existing) = result_map.get_mut(&result.id) {
            // Combine scores for entries found in both searches
            existing.rrf_score += rrf_score;
            existing.vector_rank = Some(rank);
            existing.result.rank_source = "hybrid".to_string();
        } else {
            // Add new entry from vector search only
            result_map.insert(result.id.clone(), RankedResult {
                result,
                fts_rank: None,
                vector_rank: Some(rank),
                rrf_score,
            });
        }
    }
    
    // Sort by RRF score and convert back to SearchResult
    let mut ranked_results: Vec<RankedResult> = result_map.into_values().collect();
    ranked_results.sort_by(|a, b| b.rrf_score.partial_cmp(&a.rrf_score).unwrap());
    
    let final_results = ranked_results
        .into_iter()
        .map(|mut ranked| {
            ranked.result.score = ranked.rrf_score;
            ranked.result
        })
        .collect();
    
    Ok(final_results)
}

// Apply filters to search results
fn apply_filters(
    mut results: Vec<SearchResult>,
    filters: &SearchFilters,
    limit: u32,
) -> Vec<SearchResult> {
    // Apply date range filter
    if let Some((start, end)) = &filters.date_range {
        results.retain(|r| r.entry_date >= *start && r.entry_date <= *end);
    }
    
    // Apply tag filter
    if let Some(required_tags) = &filters.tags {
        if !required_tags.is_empty() {
            results.retain(|r| {
                required_tags.iter().any(|tag| r.tags.contains(tag))
            });
        }
    }
    
    // Apply source type filter
    if let Some(source_types) = &filters.source_types {
        if !source_types.is_empty() {
            results.retain(|r| source_types.contains(&r.source_type));
        }
    }
    
    // Apply minimum score filter
    if let Some(min_score) = filters.min_score {
        results.retain(|r| r.score >= min_score);
    }
    
    // Apply limit
    results.truncate(limit as usize);
    
    results
}

// Generate snippet from content
pub fn generate_snippet(content: &str, query: &str, max_length: usize) -> String {
    let query_lower = query.to_lowercase();
    let content_lower = content.to_lowercase();
    
    // Find the first occurrence of any query term
    if let Some(pos) = content_lower.find(&query_lower) {
        let start = pos.saturating_sub(50);
        let end = (pos + query.len() + 50).min(content.len());
        
        let mut snippet = content[start..end].to_string();
        
        // Add ellipsis if we're not at the beginning/end
        if start > 0 {
            snippet = format!("...{}", snippet);
        }
        if end < content.len() {
            snippet = format!("{}...", snippet);
        }
        
        // Truncate if still too long
        if snippet.len() > max_length {
            snippet.truncate(max_length - 3);
            snippet.push_str("...");
        }
        
        snippet
    } else {
        // No query match, return beginning of content
        let mut snippet = content.chars().take(max_length - 3).collect::<String>();
        if content.len() > max_length - 3 {
            snippet.push_str("...");
        }
        snippet
    }
}

// Compute cosine similarity between two vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}

// Calculate FTS score based on query matches
fn calculate_fts_score(body: &str, title: &Option<String>, query: &str) -> f32 {
    let query_lower = query.to_lowercase();
    let body_lower = body.to_lowercase();
    let title_lower = title.as_ref().map(|t| t.to_lowercase()).unwrap_or_default();
    
    let mut score = 0.0;
    
    // Count matches in body (weight: 1.0)
    let body_matches = body_lower.matches(&query_lower).count() as f32;
    score += body_matches * 1.0;
    
    // Count matches in title (weight: 2.0 - titles are more important)
    let title_matches = title_lower.matches(&query_lower).count() as f32;
    score += title_matches * 2.0;
    
    // Normalize by content length
    let content_length = body.len() + title_lower.len();
    if content_length > 0 {
        score = score / (content_length as f32 / 100.0).max(1.0);
    }
    
    // Cap score at 1.0
    score.min(1.0)
}

// Calculate semantic similarity using keyword matching and context
fn calculate_semantic_similarity(body: &str, title: &Option<String>, query: &str) -> f32 {
    let query_lower = query.to_lowercase();
    let query_words: Vec<&str> = query_lower.split_whitespace().collect();
    if query_words.is_empty() {
        return 0.0;
    }
    
    let body_lower = body.to_lowercase();
    let title_lower = title.as_ref().map(|t| t.to_lowercase()).unwrap_or_default();
    let full_content = format!("{} {}", title_lower, body_lower);
    
    let mut total_score = 0.0;
    let mut matched_words = 0;
    
    for query_word in &query_words {
        if query_word.len() < 3 {
            continue; // Skip very short words
        }
        
        let mut word_score = 0.0;
        
        // Exact word match
        if full_content.contains(query_word) {
            word_score += 1.0;
        }
        
        // Partial word match (for stemming-like behavior)
        let partial_matches = full_content.matches(&query_word[..query_word.len().min(4)]).count();
        if partial_matches > 0 {
            word_score += 0.5 * (partial_matches as f32).min(3.0);
        }
        
        // Context-based scoring (words appearing near each other)
        for other_word in &query_words {
            if other_word != query_word {
                let pattern = format!("{} {}", query_word, other_word);
                let reverse_pattern = format!("{} {}", other_word, query_word);
                
                if full_content.contains(&pattern) || full_content.contains(&reverse_pattern) {
                    word_score += 0.3;
                }
            }
        }
        
        if word_score > 0.0 {
            matched_words += 1;
            total_score += word_score;
        }
    }
    
    if matched_words == 0 {
        return 0.0;
    }
    
    // Calculate final score
    let coverage = matched_words as f32 / query_words.len() as f32;
    let avg_score = total_score / matched_words as f32;
    
    (coverage * avg_score).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);
        
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 0.0).abs() < 1e-6);
    }
    
    #[test]
    fn test_generate_snippet() {
        let content = "This is a long piece of content that contains the word example somewhere in the middle of the text.";
        let snippet = generate_snippet(content, "example", 50);
        assert!(snippet.contains("example"));
        assert!(snippet.len() <= 50);
    }
    
    #[test]
    fn test_rrf_scoring() {
        let fts_results = vec![
            SearchResult {
                id: "1".to_string(),
                title: None,
                body: "test".to_string(),
                entry_date: Utc::now(),
                source_path: "test".to_string(),
                source_type: "txt".to_string(),
                tags: vec![],
                score: 0.9,
                snippet: "test".to_string(),
                rank_source: "fts".to_string(),
            }
        ];
        
        let vector_results = vec![
            SearchResult {
                id: "1".to_string(),
                title: None,
                body: "test".to_string(),
                entry_date: Utc::now(),
                source_path: "test".to_string(),
                source_type: "txt".to_string(),
                tags: vec![],
                score: 0.8,
                snippet: "test".to_string(),
                rank_source: "vector".to_string(),
            }
        ];
        
        let combined = reciprocal_rank_fusion(fts_results, vector_results, 60.0).unwrap();
        assert_eq!(combined.len(), 1);
        assert_eq!(combined[0].rank_source, "hybrid");
    }
}
