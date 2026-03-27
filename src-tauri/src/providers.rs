use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ProviderType {
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "openai")]
    OpenAI,
    #[serde(rename = "ollama")]
    Ollama,
    #[serde(rename = "custom")]
    Custom,
}

impl Default for ProviderType {
    fn default() -> Self {
        Self::Anthropic
    }
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Anthropic => write!(f, "anthropic"),
            Self::OpenAI => write!(f, "openai"),
            Self::Ollama => write!(f, "ollama"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

impl ProviderType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "openai" => Self::OpenAI,
            "ollama" => Self::Ollama,
            "custom" => Self::Custom,
            _ => Self::Anthropic,
        }
    }
}

/// Resolved provider config for sending a message
pub struct ResolvedProvider {
    pub provider_type: ProviderType,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub api_format: String, // "anthropic" or "openai" — determines which streaming function to use
    pub endpoint_id: Option<String>, // for custom endpoints
}

/// Fetch available models from an Ollama instance
#[tauri::command]
pub async fn fetch_ollama_models(base_url: String) -> Result<Vec<String>, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/tags", base_url.trim_end_matches('/'));
    let resp = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("Cannot connect to Ollama at {}: {}", base_url, e))?;

    if !resp.status().is_success() {
        return Err(format!("Ollama returned status {}", resp.status()));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let models = json
        .get("models")
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    Ok(models)
}

/// Test connectivity to a custom endpoint
#[tauri::command]
pub async fn test_custom_endpoint(base_url: String, api_key: String, api_format: String) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    if api_format == "anthropic" {
        let resp = client
            .post(format!("{}/v1/messages", base_url.trim_end_matches('/')))
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .body(serde_json::json!({
                "model": "claude-haiku-4-5-20251001",
                "max_tokens": 1,
                "messages": [{"role": "user", "content": "hi"}]
            }).to_string())
            .send()
            .await
            .map_err(|e| format!("Connection failed: {}", e))?;
        if resp.status().is_success() || resp.status().as_u16() == 400 {
            Ok("Connected successfully".to_string())
        } else {
            Err(format!("Server returned {}", resp.status()))
        }
    } else {
        let url = format!("{}/v1/models", base_url.trim_end_matches('/'));
        let mut req = client.get(&url);
        if !api_key.is_empty() {
            req = req.header("authorization", format!("Bearer {}", api_key));
        }
        let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
        if resp.status().is_success() {
            Ok("Connected successfully".to_string())
        } else {
            Err(format!("Server returned {}", resp.status()))
        }
    }
}
