use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommanderConfig {
    pub framework_version: String,
    pub project: ProjectConfig,
    pub runtime: RuntimeConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub provider: ProviderConfig,
    #[serde(default)]
    pub services: ServiceCatalog,
    #[serde(default)]
    pub phases: BTreeMap<String, PhaseConfig>,
    #[serde(skip, default)]
    source_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub repo_root: String,
    pub worktree_root: String,
    pub default_phase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub dir: String,
    #[serde(default = "default_poll_interval_seconds")]
    pub poll_interval_seconds: u64,
    #[serde(default = "default_command_timeout_seconds")]
    pub command_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_ui_style")]
    pub style: String,
    #[serde(default = "default_start_gate_commands")]
    pub require_live_panel_for: Vec<String>,
    #[serde(default = "default_stream_scope")]
    pub default_stream_scope: String,
    #[serde(default = "default_density")]
    pub default_density: String,
    #[serde(default = "default_density_persistence")]
    pub density_persistence: String,
    #[serde(default = "default_event_buffer_size")]
    pub event_buffer_size: usize,
    #[serde(default = "default_worker_list_page_size")]
    pub worker_list_page_size: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderConfig {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub model_provider: String,
    #[serde(default)]
    pub config_path: String,
    #[serde(default)]
    pub default_model: String,
    #[serde(default)]
    pub api_mode: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServiceCatalog {
    pub agent_room: Option<ServiceConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServiceConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default)]
    pub command: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhaseConfig {
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub audit_command: Vec<String>,
    #[serde(default)]
    pub default_start_set: Vec<String>,
    #[serde(default)]
    pub workers: Vec<WorkerConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkerConfig {
    pub name: String,
    pub branch: String,
    pub worktree: String,
    #[serde(default)]
    pub model: String,
    #[serde(default = "default_true")]
    pub auto_push: bool,
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    #[serde(default = "default_worker_timeout_seconds")]
    pub timeout_seconds: u64,
    #[serde(default)]
    pub task_files: Vec<String>,
    #[serde(default)]
    pub handoff_files: Vec<String>,
    #[serde(default)]
    pub owned_paths: Vec<String>,
    #[serde(default)]
    pub validation_commands: Vec<String>,
    #[serde(default)]
    pub launch_command: Vec<String>,
    #[serde(default)]
    pub stop_command: Vec<String>,
}

fn default_poll_interval_seconds() -> u64 {
    5
}

fn default_command_timeout_seconds() -> u64 {
    600
}

fn default_worker_timeout_seconds() -> u64 {
    1800
}

fn default_max_attempts() -> u32 {
    10
}

fn default_true() -> bool {
    true
}

fn default_ui_style() -> String {
    "hardcore-ascii".to_string()
}

fn default_start_gate_commands() -> Vec<String> {
    Vec::new()
}

fn default_stream_scope() -> String {
    "all".to_string()
}

fn default_density() -> String {
    "standard".to_string()
}

fn default_density_persistence() -> String {
    "session".to_string()
}

fn default_event_buffer_size() -> usize {
    5000
}

fn default_worker_list_page_size() -> usize {
    100
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            style: default_ui_style(),
            require_live_panel_for: default_start_gate_commands(),
            default_stream_scope: default_stream_scope(),
            default_density: default_density(),
            density_persistence: default_density_persistence(),
            event_buffer_size: default_event_buffer_size(),
            worker_list_page_size: default_worker_list_page_size(),
        }
    }
}

impl CommanderConfig {
    pub fn load_from(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read commander config {}", path.display()))?;
        let mut config: Self = toml::from_str(&content)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        config.source_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        Ok(config)
    }

    pub fn source_path(&self) -> &Path {
        &self.source_path
    }

    pub fn config_dir(&self) -> PathBuf {
        self.source_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    }

    pub fn repo_root(&self) -> PathBuf {
        resolve_path(self.config_dir(), &self.project.repo_root)
    }

    pub fn worktree_root(&self) -> PathBuf {
        resolve_path(self.repo_root(), &self.project.worktree_root)
    }

    pub fn runtime_dir(&self) -> PathBuf {
        resolve_path(self.repo_root(), &self.runtime.dir)
    }

    pub fn provider_config_path(&self) -> Option<PathBuf> {
        let raw = self.provider.config_path.trim();
        if raw.is_empty() {
            return None;
        }
        Some(expand_home(raw))
    }

    pub fn active_phase_name(&self) -> &str {
        self.project.default_phase.as_str()
    }

    pub fn active_phase(&self) -> Result<(&str, &PhaseConfig)> {
        let phase_name = self.active_phase_name();
        let phase = self
            .phases
            .get(phase_name)
            .ok_or_else(|| anyhow!("missing default phase `{phase_name}` in commander.toml"))?;
        Ok((phase_name, phase))
    }

    pub fn phase(&self, phase_name: &str) -> Result<&PhaseConfig> {
        self.phases
            .get(phase_name)
            .ok_or_else(|| anyhow!("unknown phase `{phase_name}`"))
    }

    pub fn ensure_framework_version(&self, expected: &str) -> Result<()> {
        if self.framework_version.trim() != expected.trim() {
            bail!(
                "framework version mismatch: config={}, workspace={}",
                self.framework_version.trim(),
                expected.trim()
            );
        }
        Ok(())
    }
}

impl WorkerConfig {
    pub fn worktree_path(&self, config: &CommanderConfig) -> PathBuf {
        resolve_path(config.worktree_root(), &self.worktree)
    }

    pub fn display_model(&self, config: &CommanderConfig) -> String {
        if self.model.trim().is_empty() {
            return config.provider.default_model.clone();
        }
        self.model.clone()
    }
}

pub fn read_framework_version(workspace_root: impl AsRef<Path>) -> Result<String> {
    let version_path = workspace_root.as_ref().join("VERSION");
    let version = fs::read_to_string(&version_path)
        .with_context(|| format!("failed to read {}", version_path.display()))?;
    Ok(version.trim().to_string())
}

pub fn resolve_path(base: impl AsRef<Path>, raw: &str) -> PathBuf {
    let expanded = expand_home(raw);
    if expanded.is_absolute() {
        return expanded;
    }
    base.as_ref().join(expanded)
}

fn expand_home(raw: &str) -> PathBuf {
    if let Some(rest) = raw.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    if raw == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }
    PathBuf::from(raw)
}
