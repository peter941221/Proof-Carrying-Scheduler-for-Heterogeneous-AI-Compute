use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Default)]
pub struct ProviderProfile {
    pub provider_name: String,
    pub model: String,
    pub base_url: String,
    pub api_key_present: bool,
    pub wire_api: String,
    pub source_path: PathBuf,
}

#[derive(Debug, Deserialize, Default)]
struct CodexConfig {
    #[serde(default)]
    model_provider: String,
    #[serde(default)]
    model: String,
    #[serde(default)]
    model_providers: BTreeMap<String, CodexProvider>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct CodexProvider {
    #[serde(default)]
    base_url: String,
    #[serde(default)]
    wire_api: String,
    #[serde(default)]
    api_key: String,
    #[serde(default)]
    experimental_bearer_token: String,
}

pub fn load_profile(explicit_path: Option<&Path>) -> Result<Option<ProviderProfile>> {
    for candidate in candidate_paths(explicit_path) {
        if !candidate.exists() {
            continue;
        }
        let content = fs::read_to_string(&candidate)
            .with_context(|| format!("failed to read {}", candidate.display()))?;
        let parsed: CodexConfig = toml::from_str(&content)
            .with_context(|| format!("failed to parse {}", candidate.display()))?;
        let provider_name = parsed.model_provider.trim().to_string();
        let provider = parsed
            .model_providers
            .get(&provider_name)
            .cloned()
            .unwrap_or_default();
        return Ok(Some(ProviderProfile {
            provider_name,
            model: parsed.model,
            base_url: provider.base_url,
            api_key_present: !provider.api_key.trim().is_empty()
                || !provider.experimental_bearer_token.trim().is_empty(),
            wire_api: provider.wire_api,
            source_path: candidate,
        }));
    }
    Ok(None)
}

fn candidate_paths(explicit_path: Option<&Path>) -> Vec<PathBuf> {
    let mut values = Vec::new();
    if let Some(path) = explicit_path {
        values.push(path.to_path_buf());
    }
    if let Ok(config_home) = std::env::var("CODEX_CONFIG") {
        values.push(PathBuf::from(config_home));
    }
    if let Ok(codex_home) = std::env::var("CODEX_HOME") {
        values.push(PathBuf::from(codex_home).join("config.toml"));
    }
    if let Some(home) = dirs::home_dir() {
        values.push(home.join(".codex").join("config.toml"));
    }

    let mut unique = Vec::new();
    for value in values {
        if !unique.contains(&value) {
            unique.push(value);
        }
    }
    unique
}
