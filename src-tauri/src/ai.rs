use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::AppHandle;

#[derive(Debug, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    pub text: String,
    pub model: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub model: String,
    pub provider: Provider,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Provider {
    Ollama,
    OpenAI,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TagExtractionRequest {
    pub text: String,
    pub vocabulary: Vec<String>,
    pub max_tags: u32,
    pub confidence_threshold: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TagSuggestion {
    pub tag: String,
    pub confidence: f32,
    pub reasoning: String,
    pub text_spans: Vec<String>, // Parts of text that support this tag
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TagExtractionResult {
    pub suggestions: Vec<TagSuggestion>,
    pub processing_time_ms: u64,
    pub model_used: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ControlledVocabulary {
    pub tags: Vec<VocabularyTag>,
    pub aliases: HashMap<String, String>, // alias -> canonical tag
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VocabularyTag {
    pub name: String,
    pub description: String,
    pub aliases: Vec<String>,
    pub category: String,
    pub examples: Vec<String>,
}

// AI-powered tag extraction
pub async fn extract_tags_ai(
    app_handle: &AppHandle,
    request: TagExtractionRequest,
    provider: Provider,
) -> Result<TagExtractionResult> {
    let start_time = std::time::Instant::now();
    
    match provider {
        Provider::Ollama => extract_tags_ollama(app_handle, request).await,
        Provider::OpenAI => extract_tags_openai(app_handle, request).await,
    }
    .map(|mut result| {
        result.processing_time_ms = start_time.elapsed().as_millis() as u64;
        result
    })
}

// Ollama-based tag extraction
async fn extract_tags_ollama(
    _app_handle: &AppHandle,
    request: TagExtractionRequest,
) -> Result<TagExtractionResult> {
    let client = reqwest::Client::new();
    
    let ollama_url = std::env::var("OLLAMA_URL")
        .unwrap_or_else(|_| "http://localhost:11434".to_string());
    
    // Create a detailed prompt for tag extraction
    let prompt = format!(
        "Analyze the following text and suggest relevant tags from the provided vocabulary. \
        Return your response as JSON format with 'tags' array containing objects with 'tag', 'confidence' (0.0-1.0), and 'reasoning' fields.
        
        Vocabulary: {}
        
        Text to analyze:
        {}
        
        Return only the JSON response:",
        request.vocabulary.join(", "),
        request.text
    );
    
    let request_body = serde_json::json!({
        "model": "llama3.1:8b",
        "prompt": prompt,
        "stream": false,
        "format": "json",
        "options": {
            "temperature": 0.2,
            "num_predict": 300
        }
    });
    
    let response = client
        .post(format!("{}/api/generate", ollama_url))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await;
    
    let suggestions = match response {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<serde_json::Value>().await {
                Ok(json) => {
                    let response_text = json["response"].as_str().unwrap_or("{}");
                    parse_tag_extraction_response(response_text, &request.vocabulary, request.confidence_threshold)
                        .unwrap_or_else(|_| generate_mock_tag_suggestions(&request.text, &request.vocabulary))
                },
                Err(_) => generate_mock_tag_suggestions(&request.text, &request.vocabulary),
            }
        },
        _ => generate_mock_tag_suggestions(&request.text, &request.vocabulary),
    };
    
    Ok(TagExtractionResult {
        suggestions: suggestions.into_iter().take(request.max_tags as usize).collect(),
        processing_time_ms: 0, // Will be set by caller
        model_used: "llama3.1:8b".to_string(),
    })
}

// OpenAI-based tag extraction
async fn extract_tags_openai(
    _app_handle: &AppHandle,
    request: TagExtractionRequest,
) -> Result<TagExtractionResult> {
    let client = reqwest::Client::new();
    
    let api_key = std::env::var("OPENAI_API_KEY")
        .unwrap_or_else(|_| "your-openai-api-key".to_string());
    
    if api_key == "your-openai-api-key" {
        let suggestions = generate_mock_tag_suggestions(&request.text, &request.vocabulary);
        return Ok(TagExtractionResult {
            suggestions: suggestions.into_iter().take(request.max_tags as usize).collect(),
            processing_time_ms: 0, // Will be set by caller
            model_used: "gpt-4o-mini (mock)".to_string(),
        });
    }
    
    let system_message = format!(
        "You are a tag extraction assistant. Analyze the provided text and suggest relevant tags from the given vocabulary. \
        Return your response in JSON format with a 'tags' array containing objects with 'tag', 'confidence' (0.0-1.0), and 'reasoning' fields. \
        Only suggest tags that are highly relevant to the content.

        Available vocabulary: {}", 
        request.vocabulary.join(", ")
    );
    
    let user_message = format!("Please analyze this text and suggest relevant tags:\n\n{}", request.text);
    
    let messages = vec![
        serde_json::json!({
            "role": "system",
            "content": system_message
        }),
        serde_json::json!({
            "role": "user", 
            "content": user_message
        })
    ];
    
    let request_body = serde_json::json!({
        "model": "gpt-4o-mini",
        "messages": messages,
        "temperature": 0.2,
        "max_tokens": 500,
        "response_format": { "type": "json_object" }
    });
    
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await;
    
    let suggestions = match response {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<serde_json::Value>().await {
                Ok(json) => {
                    let response_text = json["choices"][0]["message"]["content"]
                        .as_str()
                        .unwrap_or("{}");
                    parse_tag_extraction_response(response_text, &request.vocabulary, request.confidence_threshold)
                        .unwrap_or_else(|_| generate_mock_tag_suggestions(&request.text, &request.vocabulary))
                },
                Err(_) => generate_mock_tag_suggestions(&request.text, &request.vocabulary),
            }
        },
        _ => generate_mock_tag_suggestions(&request.text, &request.vocabulary),
    };
    
    Ok(TagExtractionResult {
        suggestions: suggestions.into_iter().take(request.max_tags as usize).collect(),
        processing_time_ms: 0, // Will be set by caller
        model_used: "gpt-4o-mini".to_string(),
    })
}

// Parse JSON response from AI models for tag extraction
fn parse_tag_extraction_response(
    response_text: &str, 
    vocabulary: &[String], 
    confidence_threshold: f32
) -> Result<Vec<TagSuggestion>> {
    let json: serde_json::Value = serde_json::from_str(response_text)
        .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))?;
    
    let tags_array = json["tags"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No 'tags' array found in response"))?;
    
    let mut suggestions = Vec::new();
    
    for tag_obj in tags_array {
        if let (Some(tag), Some(confidence)) = (
            tag_obj["tag"].as_str(),
            tag_obj["confidence"].as_f64()
        ) {
            let confidence = confidence as f32;
            
            // Only include tags that are in vocabulary and meet confidence threshold
            if vocabulary.iter().any(|v| v.to_lowercase() == tag.to_lowercase()) 
                && confidence >= confidence_threshold {
                
                suggestions.push(TagSuggestion {
                    tag: tag.to_string(),
                    confidence,
                    reasoning: tag_obj["reasoning"]
                        .as_str()
                        .unwrap_or("AI-suggested tag")
                        .to_string(),
                    text_spans: vec![], // Could be enhanced to extract actual spans
                });
            }
        }
    }
    
    Ok(suggestions)
}

// Rule-based tag extraction (fallback)
pub fn extract_tags_rules(text: &str, vocabulary: &[String]) -> Vec<TagSuggestion> {
    let text_lower = text.to_lowercase();
    let mut suggestions = Vec::new();
    
    // Simple keyword-based matching
    for tag in vocabulary {
        let tag_lower = tag.to_lowercase();
        
        // Check for exact matches
        if text_lower.contains(&tag_lower) {
            suggestions.push(TagSuggestion {
                tag: tag.clone(),
                confidence: 0.8,
                reasoning: format!("Found exact match for '{}'", tag),
                text_spans: vec![tag.clone()],
            });
            continue;
        }
        
        // Check for semantic matches based on tag category
        let confidence = calculate_semantic_match(&text_lower, &tag_lower);
        if confidence > 0.5 {
            suggestions.push(TagSuggestion {
                tag: tag.clone(),
                confidence,
                reasoning: format!("Semantic match for '{}'", tag),
                text_spans: vec![],
            });
        }
    }
    
    // Sort by confidence
    suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
    
    suggestions
}

// Generate mock tag suggestions for testing
fn generate_mock_tag_suggestions(text: &str, vocabulary: &[String]) -> Vec<TagSuggestion> {
    let text_lower = text.to_lowercase();
    let mut suggestions = Vec::new();
    
    // Define keyword patterns for different tag categories
    let patterns = vec![
        ("personal", vec!["i feel", "my", "myself", "personal", "private"]),
        ("work", vec!["work", "job", "office", "meeting", "project", "colleague"]),
        ("travel", vec!["trip", "travel", "vacation", "flight", "hotel", "visit"]),
        ("reflection", vec!["think", "reflect", "realize", "understand", "learn"]),
        ("goals", vec!["goal", "plan", "want to", "hope", "dream", "achieve"]),
        ("relationships", vec!["friend", "family", "relationship", "love", "partner"]),
        ("health", vec!["health", "exercise", "doctor", "sick", "wellness", "fitness"]),
        ("creativity", vec!["creative", "art", "write", "music", "design", "inspiration"]),
        ("learning", vec!["learn", "study", "read", "course", "education", "knowledge"]),
        ("emotions", vec!["happy", "sad", "angry", "excited", "worried", "grateful"]),
    ];
    
    for (tag, keywords) in patterns {
        if vocabulary.contains(&tag.to_string()) {
            let mut matches = 0;
            let mut found_keywords = Vec::new();
            
            for keyword in &keywords {
                if text_lower.contains(keyword) {
                    matches += 1;
                    found_keywords.push(keyword.to_string());
                }
            }
            
            if matches > 0 {
                let confidence = (matches as f32 / keywords.len() as f32).min(0.95);
                suggestions.push(TagSuggestion {
                    tag: tag.to_string(),
                    confidence,
                    reasoning: format!("Found {} relevant keywords", matches),
                    text_spans: found_keywords,
                });
            }
        }
    }
    
    // Sort by confidence
    suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
    
    // Limit to top suggestions
    suggestions.truncate(5);
    
    suggestions
}

// Calculate semantic similarity between text and tag
fn calculate_semantic_match(text: &str, tag: &str) -> f32 {
    // Simple word overlap calculation
    let text_words: std::collections::HashSet<&str> = text.split_whitespace().collect();
    let tag_words: std::collections::HashSet<&str> = tag.split_whitespace().collect();
    
    let intersection = text_words.intersection(&tag_words).count();
    let union = text_words.union(&tag_words).count();
    
    if union == 0 {
        0.0
    } else {
        intersection as f32 / union as f32
    }
}

// Default controlled vocabulary
pub fn get_default_vocabulary() -> ControlledVocabulary {
    let tags = vec![
        VocabularyTag {
            name: "personal".to_string(),
            description: "Personal thoughts, feelings, and experiences".to_string(),
            aliases: vec!["private".to_string(), "self".to_string()],
            category: "general".to_string(),
            examples: vec!["personal reflection".to_string(), "my thoughts".to_string()],
        },
        VocabularyTag {
            name: "work".to_string(),
            description: "Work-related entries, career, and professional life".to_string(),
            aliases: vec!["job".to_string(), "career".to_string(), "professional".to_string()],
            category: "general".to_string(),
            examples: vec!["work meeting".to_string(), "project update".to_string()],
        },
        VocabularyTag {
            name: "travel".to_string(),
            description: "Travel experiences, trips, and adventures".to_string(),
            aliases: vec!["trip".to_string(), "vacation".to_string(), "journey".to_string()],
            category: "activities".to_string(),
            examples: vec!["travel diary".to_string(), "vacation memories".to_string()],
        },
        VocabularyTag {
            name: "reflection".to_string(),
            description: "Deep thoughts, introspection, and self-analysis".to_string(),
            aliases: vec!["introspection".to_string(), "contemplation".to_string()],
            category: "mental".to_string(),
            examples: vec!["reflecting on life".to_string(), "deep thoughts".to_string()],
        },
        VocabularyTag {
            name: "goals".to_string(),
            description: "Goals, plans, aspirations, and future objectives".to_string(),
            aliases: vec!["plans".to_string(), "objectives".to_string(), "aspirations".to_string()],
            category: "planning".to_string(),
            examples: vec!["life goals".to_string(), "future plans".to_string()],
        },
        VocabularyTag {
            name: "relationships".to_string(),
            description: "Relationships, family, friends, and social connections".to_string(),
            aliases: vec!["family".to_string(), "friends".to_string(), "social".to_string()],
            category: "social".to_string(),
            examples: vec!["family time".to_string(), "friendship".to_string()],
        },
        VocabularyTag {
            name: "health".to_string(),
            description: "Health, wellness, fitness, and medical topics".to_string(),
            aliases: vec!["wellness".to_string(), "fitness".to_string(), "medical".to_string()],
            category: "lifestyle".to_string(),
            examples: vec!["health journey".to_string(), "fitness goals".to_string()],
        },
        VocabularyTag {
            name: "creativity".to_string(),
            description: "Creative pursuits, art, writing, and inspiration".to_string(),
            aliases: vec!["art".to_string(), "creative".to_string(), "inspiration".to_string()],
            category: "activities".to_string(),
            examples: vec!["creative project".to_string(), "artistic inspiration".to_string()],
        },
        VocabularyTag {
            name: "learning".to_string(),
            description: "Learning, education, skills, and knowledge acquisition".to_string(),
            aliases: vec!["education".to_string(), "study".to_string(), "knowledge".to_string()],
            category: "development".to_string(),
            examples: vec!["learning experience".to_string(), "new skills".to_string()],
        },
        VocabularyTag {
            name: "emotions".to_string(),
            description: "Emotional states, feelings, and mood tracking".to_string(),
            aliases: vec!["feelings".to_string(), "mood".to_string(), "emotional".to_string()],
            category: "mental".to_string(),
            examples: vec!["emotional state".to_string(), "feeling grateful".to_string()],
        },
    ];
    
    // Build aliases map
    let mut aliases = HashMap::new();
    for tag in &tags {
        for alias in &tag.aliases {
            aliases.insert(alias.clone(), tag.name.clone());
        }
    }
    
    ControlledVocabulary { tags, aliases }
}

// Standard embedding generation
pub async fn generate_embedding(request: EmbeddingRequest) -> Result<Vec<f32>> {
    // Default to OpenAI for embeddings unless model suggests Ollama
    if request.model.contains("ollama") || request.model.contains("llama") {
        generate_embedding_ollama(&request.text, &request.model).await
    } else {
        generate_embedding_openai(&request.text, &request.model).await
    }
}

// OpenAI embedding generation
async fn generate_embedding_openai(text: &str, model: &str) -> Result<Vec<f32>> {
    let client = reqwest::Client::new();
    
    // Use text-embedding-3-small as default model
    let model = if model.is_empty() || model == "default" {
        "text-embedding-3-small"
    } else {
        model
    };
    
    let api_key = std::env::var("OPENAI_API_KEY")
        .unwrap_or_else(|_| "your-openai-api-key".to_string());
    
    if api_key == "your-openai-api-key" {
        // Return mock embedding if no API key is set
        return Ok(generate_mock_embedding(text, 1536));
    }
    
    let request_body = serde_json::json!({
        "input": text,
        "model": model
    });
    
    let response = client
        .post("https://api.openai.com/v1/embeddings")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;
    
    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("OpenAI API error: {}", error_text));
    }
    
    let response_json: serde_json::Value = response.json().await?;
    
    let embedding = response_json["data"][0]["embedding"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Invalid OpenAI response format"))?
        .iter()
        .map(|v| v.as_f64().unwrap_or(0.0) as f32)
        .collect();
    
    Ok(embedding)
}

// Ollama embedding generation
async fn generate_embedding_ollama(text: &str, model: &str) -> Result<Vec<f32>> {
    let client = reqwest::Client::new();
    
    // Use nomic-embed-text as default embedding model
    let model = if model.is_empty() || model == "default" {
        "nomic-embed-text"
    } else {
        model
    };
    
    let ollama_url = std::env::var("OLLAMA_URL")
        .unwrap_or_else(|_| "http://localhost:11434".to_string());
    
    let request_body = serde_json::json!({
        "model": model,
        "prompt": text
    });
    
    let response = client
        .post(format!("{}/api/embeddings", ollama_url))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await;
    
    let response = match response {
        Ok(resp) => resp,
        Err(_) => {
            // If Ollama is not available, return mock embedding
            return Ok(generate_mock_embedding(text, 768));
        }
    };
    
    if !response.status().is_success() {
        // Fallback to mock embedding if Ollama request fails
        return Ok(generate_mock_embedding(text, 768));
    }
    
    let response_json: serde_json::Value = response.json().await
        .unwrap_or_else(|_| serde_json::json!({"embedding": []}));
    
    let embedding = response_json["embedding"]
        .as_array()
        .map(|arr| arr.iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect())
        .unwrap_or_else(|| generate_mock_embedding(text, 768));
    
    Ok(embedding)
}

// Generate deterministic mock embedding based on text content
fn generate_mock_embedding(text: &str, dimension: usize) -> Vec<f32> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    let hash = hasher.finish();
    
    let mut embedding = Vec::with_capacity(dimension);
    let mut seed = hash;
    
    for _ in 0..dimension {
        // Simple linear congruential generator for consistent mock embeddings
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        let value = (seed as f32) / (u64::MAX as f32) * 2.0 - 1.0; // Range [-1, 1]
        embedding.push(value * 0.1); // Scale down to reasonable range
    }
    
    // Normalize vector to unit length
    let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if magnitude > 0.0 {
        for value in &mut embedding {
            *value /= magnitude;
        }
    }
    
    embedding
}

// RAG-specific structures
#[derive(Debug, Serialize, Deserialize)]
pub struct RagRequest {
    pub question: String,
    pub conversation_id: Option<String>,
    pub max_context_entries: u32,
    pub context_date_range: Option<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>,
    pub context_tags: Option<Vec<String>>,
    pub provider: Provider,
    pub model: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RagResponse {
    pub answer: String,
    pub citations: Vec<Citation>,
    pub context_used: Vec<ContextEntry>,
    pub confidence: f32,
    pub processing_time_ms: u64,
    pub model_used: String,
    pub conversation_id: String,
    pub message_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Citation {
    pub entry_id: String,
    pub entry_title: Option<String>,
    pub entry_date: chrono::DateTime<chrono::Utc>,
    pub snippet: String,
    pub relevance_score: f32,
    pub citation_number: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContextEntry {
    pub entry_id: String,
    pub title: Option<String>,
    pub body: String,
    pub entry_date: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
    pub relevance_score: f32,
    pub snippet: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConversationContext {
    pub conversation_id: String,
    pub messages: Vec<ConversationMessage>,
    pub context_entries: Vec<ContextEntry>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub message_id: String,
    pub role: String, // "user" or "assistant"
    pub content: String,
    pub citations: Option<Vec<Citation>>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

// RAG pipeline implementation
pub async fn process_rag_query(
    app_handle: &tauri::AppHandle,
    request: RagRequest,
) -> Result<RagResponse> {
    let start_time = std::time::Instant::now();
    
    // Step 1: Retrieve relevant context from journal entries
    let context_entries = retrieve_relevant_context(
        app_handle,
        &request.question,
        request.max_context_entries,
        request.context_date_range,
        request.context_tags.as_ref(),
    ).await?;
    
    // Step 2: Generate answer using RAG
    let (answer, citations, confidence) = match request.provider {
        Provider::Ollama => generate_rag_answer_ollama(
            app_handle,
            &request.question,
            &context_entries,
            &request.model,
        ).await?,
        Provider::OpenAI => generate_rag_answer_openai(
            app_handle,
            &request.question,
            &context_entries,
            &request.model,
        ).await?,
    };
    
    // Step 3: Create or update conversation
    let conversation_id = request.conversation_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let message_id = uuid::Uuid::new_v4().to_string();
    
    let processing_time = start_time.elapsed().as_millis() as u64;
    
    Ok(RagResponse {
        answer,
        citations,
        context_used: context_entries,
        confidence,
        processing_time_ms: processing_time,
        model_used: request.model,
        conversation_id,
        message_id,
    })
}

// Retrieve relevant context entries using hybrid search
async fn retrieve_relevant_context(
    app_handle: &tauri::AppHandle,
    question: &str,
    max_entries: u32,
    date_range: Option<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>,
    tags: Option<&Vec<String>>,
) -> Result<Vec<ContextEntry>> {
    use crate::search::{SearchFilters, hybrid_search};
    
    // Create search filters
    let filters = SearchFilters {
        date_range,
        tags: tags.cloned(),
        source_types: None, // Include all source types
        min_score: Some(0.3), // Minimum relevance threshold
    };
    
    // Use hybrid search to find relevant entries
    let search_results = hybrid_search(app_handle, question, &filters, max_entries).await?;
    
    // Convert search results to context entries
    let context_entries: Vec<ContextEntry> = search_results
        .into_iter()
        .map(|result| {
            let snippet = if result.snippet.is_empty() {
                // Generate snippet if not provided
                let words: Vec<&str> = result.body.split_whitespace().collect();
                words.into_iter().take(50).collect::<Vec<_>>().join(" ")
            } else {
                result.snippet.clone()
            };
            
            ContextEntry {
                entry_id: result.id,
                title: result.title,
                body: result.body,
                entry_date: result.entry_date,
                tags: result.tags,
                relevance_score: result.score,
                snippet,
            }
        })
        .collect();
    
    Ok(context_entries)
}

// Generate RAG answer using Ollama
async fn generate_rag_answer_ollama(
    _app_handle: &tauri::AppHandle,
    question: &str,
    context_entries: &[ContextEntry],
    model: &str,
) -> Result<(String, Vec<Citation>, f32)> {
    // Build context string from entries
    let context = build_context_string(context_entries);
    
    // Create RAG prompt
    let prompt = create_rag_prompt(question, &context);
    
    // Make actual Ollama API call
    let client = reqwest::Client::new();
    
    let model = if model.is_empty() || model == "default" {
        "llama3.1:8b"
    } else {
        model
    };
    
    let ollama_url = std::env::var("OLLAMA_URL")
        .unwrap_or_else(|_| "http://localhost:11434".to_string());
    
    let request_body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": false,
        "options": {
            "temperature": 0.3, // Lower temperature for more focused answers
            "num_predict": 1000
        }
    });
    
    let response = client
        .post(format!("{}/api/generate", ollama_url))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await;
    
    let answer = match response {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<serde_json::Value>().await {
                Ok(json) => json["response"].as_str().unwrap_or("").to_string(),
                Err(_) => return Ok(generate_fallback_rag_response(question, context_entries)),
            }
        },
        _ => return Ok(generate_fallback_rag_response(question, context_entries)),
    };
    
    // Extract citations from context entries that were used
    let citations = extract_citations_from_answer(&answer, context_entries);
    let confidence = calculate_answer_confidence(&answer, context_entries);
    
    Ok((answer, citations, confidence))
}

// Generate RAG answer using OpenAI
async fn generate_rag_answer_openai(
    _app_handle: &tauri::AppHandle,
    question: &str,
    context_entries: &[ContextEntry],
    model: &str,
) -> Result<(String, Vec<Citation>, f32)> {
    // Build context string from entries
    let context = build_context_string(context_entries);
    
    // Create RAG prompt
    let prompt = create_rag_prompt(question, &context);
    
    // Make actual OpenAI API call
    let client = reqwest::Client::new();
    
    let model = if model.is_empty() || model == "default" {
        "gpt-4o-mini"
    } else {
        model
    };
    
    let api_key = std::env::var("OPENAI_API_KEY")
        .unwrap_or_else(|_| "your-openai-api-key".to_string());
    
    if api_key == "your-openai-api-key" {
        return Ok(generate_fallback_rag_response(question, context_entries));
    }
    
    let messages = vec![
        serde_json::json!({
            "role": "system",
            "content": "You are a helpful assistant that answers questions based on journal entries. Always cite specific entries when making claims, using the format [Entry N]. Be accurate and only make claims supported by the provided context."
        }),
        serde_json::json!({
            "role": "user", 
            "content": prompt
        })
    ];
    
    let request_body = serde_json::json!({
        "model": model,
        "messages": messages,
        "temperature": 0.3,
        "max_tokens": 1500
    });
    
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await;
    
    let answer = match response {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<serde_json::Value>().await {
                Ok(json) => json["choices"][0]["message"]["content"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                Err(_) => return Ok(generate_fallback_rag_response(question, context_entries)),
            }
        },
        _ => return Ok(generate_fallback_rag_response(question, context_entries)),
    };
    
    // Extract citations from context entries that were used
    let citations = extract_citations_from_answer(&answer, context_entries);
    let confidence = calculate_answer_confidence(&answer, context_entries);
    
    Ok((answer, citations, confidence))
}

// Build context string from entries
fn build_context_string(context_entries: &[ContextEntry]) -> String {
    let mut context = String::new();
    
    for (i, entry) in context_entries.iter().enumerate() {
        context.push_str(&format!(
            "[Entry {}] Date: {} | Tags: {} | Content: {}\n\n",
            i + 1,
            entry.entry_date.format("%Y-%m-%d"),
            entry.tags.join(", "),
            entry.snippet
        ));
    }
    
    context
}

// Create RAG prompt with context
fn create_rag_prompt(question: &str, context: &str) -> String {
    format!(
        r#"You are a helpful assistant that answers questions about personal journal entries. 
Use only the provided context to answer the question. If the context doesn't contain enough information to answer the question, say so clearly.

When referencing information from the context, include citation numbers in square brackets like [1], [2], etc.

Context:
{}

Question: {}

Answer:"#,
        context, question
    )
}

// Generate mock RAG response for testing
// Generate fallback response when AI services are unavailable
fn generate_fallback_rag_response(question: &str, context_entries: &[ContextEntry]) -> (String, Vec<Citation>, f32) {
    let (answer, citations) = generate_mock_rag_response(question, context_entries);
    let confidence = calculate_answer_confidence(&answer, context_entries);
    (answer, citations, confidence)
}

// Extract citations from AI answer by looking for [Entry N] patterns
fn extract_citations_from_answer(answer: &str, context_entries: &[ContextEntry]) -> Vec<Citation> {
    let mut citations = Vec::new();
    
    // Look for [Entry N] patterns in the answer
    let re = match regex::Regex::new(r"\[Entry (\d+)\]") {
        Ok(regex) => regex,
        Err(_) => {
            // If regex fails, fallback to simple citation extraction
            return extract_simple_citations(context_entries);
        }
    };
    
    for caps in re.captures_iter(answer) {
        if let Some(num_str) = caps.get(1) {
            if let Ok(entry_num) = num_str.as_str().parse::<usize>() {
                if entry_num > 0 && entry_num <= context_entries.len() {
                    let entry = &context_entries[entry_num - 1];
                    citations.push(Citation {
                        entry_id: entry.entry_id.clone(),
                        entry_title: entry.title.clone(),
                        entry_date: entry.entry_date,
                        snippet: if entry.snippet.len() > 200 {
                            format!("{}...", &entry.snippet[..200])
                        } else {
                            entry.snippet.clone()
                        },
                        relevance_score: entry.relevance_score,
                        citation_number: entry_num as u32,
                    });
                }
            }
        }
    }
    
    // If no explicit citations found, include top relevant entries
    if citations.is_empty() {
        citations = extract_simple_citations(context_entries);
    }
    
    citations
}

// Simple citation extraction when regex is not available
fn extract_simple_citations(context_entries: &[ContextEntry]) -> Vec<Citation> {
    context_entries.iter()
        .enumerate()
        .take(3) // Include top 3 entries as citations
        .filter(|(_, entry)| entry.relevance_score > 0.3)
        .map(|(i, entry)| Citation {
            entry_id: entry.entry_id.clone(),
            entry_title: entry.title.clone(),
            entry_date: entry.entry_date,
            snippet: if entry.snippet.len() > 200 {
                format!("{}...", &entry.snippet[..200])
            } else {
                entry.snippet.clone()
            },
            relevance_score: entry.relevance_score,
            citation_number: (i + 1) as u32,
        })
        .collect()
}

fn generate_mock_rag_response(question: &str, context_entries: &[ContextEntry]) -> (String, Vec<Citation>) {
    let question_lower = question.to_lowercase();
    let mut answer = String::new();
    let mut citations = Vec::new();
    
    // Analyze question type and generate appropriate response
    if question_lower.contains("feel") || question_lower.contains("emotion") {
        answer = "Based on your journal entries, you've experienced a range of emotions. ".to_string();
        
        // Find entries with emotional content
        for (i, entry) in context_entries.iter().enumerate().take(3) {
            if entry.body.to_lowercase().contains("feel") || 
               entry.body.to_lowercase().contains("happy") ||
               entry.body.to_lowercase().contains("sad") ||
               entry.body.to_lowercase().contains("excited") {
                
                answer.push_str(&format!("On {}, you mentioned feeling certain emotions [{}]. ", 
                    entry.entry_date.format("%B %d"), i + 1));
                
                citations.push(Citation {
                    entry_id: entry.entry_id.clone(),
                    entry_title: entry.title.clone(),
                    entry_date: entry.entry_date,
                    snippet: entry.snippet.clone(),
                    relevance_score: entry.relevance_score,
                    citation_number: (i + 1) as u32,
                });
            }
        }
    } else if question_lower.contains("work") || question_lower.contains("job") {
        answer = "Looking at your work-related journal entries, I can see several patterns. ".to_string();
        
        for (i, entry) in context_entries.iter().enumerate().take(3) {
            if entry.tags.contains(&"work".to_string()) || 
               entry.body.to_lowercase().contains("work") ||
               entry.body.to_lowercase().contains("meeting") {
                
                answer.push_str(&format!("You wrote about work experiences on {} [{}]. ", 
                    entry.entry_date.format("%B %d"), i + 1));
                
                citations.push(Citation {
                    entry_id: entry.entry_id.clone(),
                    entry_title: entry.title.clone(),
                    entry_date: entry.entry_date,
                    snippet: entry.snippet.clone(),
                    relevance_score: entry.relevance_score,
                    citation_number: (i + 1) as u32,
                });
            }
        }
    } else if question_lower.contains("goal") || question_lower.contains("plan") {
        answer = "Your journal entries reveal several goals and plans you've set. ".to_string();
        
        for (i, entry) in context_entries.iter().enumerate().take(3) {
            if entry.tags.contains(&"goals".to_string()) || 
               entry.body.to_lowercase().contains("goal") ||
               entry.body.to_lowercase().contains("plan") {
                
                answer.push_str(&format!("On {}, you outlined some objectives [{}]. ", 
                    entry.entry_date.format("%B %d"), i + 1));
                
                citations.push(Citation {
                    entry_id: entry.entry_id.clone(),
                    entry_title: entry.title.clone(),
                    entry_date: entry.entry_date,
                    snippet: entry.snippet.clone(),
                    relevance_score: entry.relevance_score,
                    citation_number: (i + 1) as u32,
                });
            }
        }
    } else {
        // General response
        answer = format!("Based on your journal entries, I found {} relevant entries that relate to your question. ", context_entries.len());
        
        for (i, entry) in context_entries.iter().enumerate().take(3) {
            answer.push_str(&format!("One entry from {} discusses related topics [{}]. ", 
                entry.entry_date.format("%B %d"), i + 1));
            
            citations.push(Citation {
                entry_id: entry.entry_id.clone(),
                entry_title: entry.title.clone(),
                entry_date: entry.entry_date,
                snippet: entry.snippet.clone(),
                relevance_score: entry.relevance_score,
                citation_number: (i + 1) as u32,
            });
        }
    }
    
    if citations.is_empty() {
        answer = "I don't have enough information in your journal entries to answer this question confidently. You might want to add more entries on this topic or try rephrasing your question.".to_string();
    }
    
    (answer, citations)
}

// Calculate confidence based on context relevance
fn calculate_answer_confidence(answer: &str, context_entries: &[ContextEntry]) -> f32 {
    if context_entries.is_empty() {
        return 0.0;
    }
    
    // Base confidence on number and quality of context entries
    let avg_relevance: f32 = context_entries.iter()
        .map(|entry| entry.relevance_score)
        .sum::<f32>() / context_entries.len() as f32;
    
    let context_factor = (context_entries.len() as f32 / 5.0).min(1.0); // More context = higher confidence
    let relevance_factor = avg_relevance;
    let length_factor = (answer.len() as f32 / 200.0).min(1.0); // Longer answers = higher confidence
    
    ((context_factor + relevance_factor + length_factor) / 3.0).min(0.95)
}

// Standard chat completion
pub async fn chat_completion(request: ChatRequest) -> Result<String> {
    match request.provider {
        Provider::OpenAI => chat_completion_openai(request.messages, &request.model).await,
        Provider::Ollama => chat_completion_ollama(request.messages, &request.model).await,
    }
}

// OpenAI chat completion
async fn chat_completion_openai(messages: Vec<ChatMessage>, model: &str) -> Result<String> {
    let client = reqwest::Client::new();
    
    // Use gpt-4o-mini as default model
    let model = if model.is_empty() || model == "default" {
        "gpt-4o-mini"
    } else {
        model
    };
    
    let api_key = std::env::var("OPENAI_API_KEY")
        .unwrap_or_else(|_| "your-openai-api-key".to_string());
    
    if api_key == "your-openai-api-key" {
        return Ok("Please set your OPENAI_API_KEY environment variable to use OpenAI chat completion.".to_string());
    }
    
    let request_body = serde_json::json!({
        "model": model,
        "messages": messages.iter().map(|msg| serde_json::json!({
            "role": msg.role,
            "content": msg.content
        })).collect::<Vec<_>>(),
        "temperature": 0.7,
        "max_tokens": 2000
    });
    
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;
    
    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("OpenAI API error: {}", error_text));
    }
    
    let response_json: serde_json::Value = response.json().await?;
    
    let content = response_json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("Sorry, I couldn't generate a response.")
        .to_string();
    
    Ok(content)
}

// Ollama chat completion
async fn chat_completion_ollama(messages: Vec<ChatMessage>, model: &str) -> Result<String> {
    let client = reqwest::Client::new();
    
    // Use llama3.1:8b as default model
    let model = if model.is_empty() || model == "default" {
        "llama3.1:8b"
    } else {
        model
    };
    
    let ollama_url = std::env::var("OLLAMA_URL")
        .unwrap_or_else(|_| "http://localhost:11434".to_string());
    
    // Convert messages to a single prompt for Ollama
    let mut prompt = String::new();
    for message in &messages {
        match message.role.as_str() {
            "system" => prompt.push_str(&format!("System: {}\n", message.content)),
            "user" => prompt.push_str(&format!("User: {}\n", message.content)),
            "assistant" => prompt.push_str(&format!("Assistant: {}\n", message.content)),
            _ => prompt.push_str(&format!("{}: {}\n", message.role, message.content)),
        }
    }
    prompt.push_str("Assistant: ");
    
    let request_body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": false,
        "options": {
            "temperature": 0.7,
            "num_predict": 500
        }
    });
    
    let response = client
        .post(format!("{}/api/generate", ollama_url))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await;
    
    let response = match response {
        Ok(resp) => resp,
        Err(e) => {
            return Ok(format!("Ollama is not available ({}). Please make sure Ollama is running at {} or set OLLAMA_URL environment variable.", e, ollama_url));
        }
    };
    
    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Ok(format!("Ollama error: {}. Please check that the model '{}' is available.", error_text, model));
    }
    
    let response_json: serde_json::Value = response.json().await
        .map_err(|e| anyhow::anyhow!("Failed to parse Ollama response: {}", e))?;
    
    let content = response_json["response"]
        .as_str()
        .unwrap_or("Sorry, I couldn't generate a response from Ollama.")
        .to_string();
    
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rule_based_extraction() {
        let text = "Today I went to work and had a great meeting with my colleagues. I'm feeling really happy about the project progress.";
        let vocabulary = vec!["work".to_string(), "emotions".to_string(), "personal".to_string()];
        
        let suggestions = extract_tags_rules(text, &vocabulary);
        
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.tag == "work"));
        assert!(suggestions.iter().any(|s| s.tag == "emotions"));
    }
    
    #[test]
    fn test_semantic_match() {
        let text = "work meeting project";
        let tag = "work";
        
        let score = calculate_semantic_match(text, tag);
        assert!(score > 0.0);
    }
    
    #[test]
    fn test_default_vocabulary() {
        let vocab = get_default_vocabulary();
        
        assert!(!vocab.tags.is_empty());
        assert!(!vocab.aliases.is_empty());
        assert!(vocab.aliases.contains_key("job"));
        assert_eq!(vocab.aliases.get("job"), Some(&"work".to_string()));
    }
}
