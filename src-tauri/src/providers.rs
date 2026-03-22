use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ProviderType {
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "openai")]
    OpenAI,
    #[serde(rename = "ollama")]
    Ollama,
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
        }
    }
}

impl ProviderType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "openai" => Self::OpenAI,
            "ollama" => Self::Ollama,
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
