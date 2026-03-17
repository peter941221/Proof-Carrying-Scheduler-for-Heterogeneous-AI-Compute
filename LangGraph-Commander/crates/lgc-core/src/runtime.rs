use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Local;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ActivityEntry {
    pub seq: u64,
    pub timestamp: String,
    pub source: String,
    pub level: String,
    pub channel: String,
    pub worker_name: String,
    pub message: String,
    pub dense_message: String,
    pub full_message: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum FeedDensity {
    #[default]
    Standard,
    Realtime,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum StreamScope {
    #[default]
    All,
    Selected,
    Commander,
    Worker(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkerSnapshot {
    pub name: String,
    pub worktree_path: String,
    pub branch: String,
    pub expected_branch: String,
    #[serde(default)]
    pub model_name: String,
    #[serde(default)]
    pub reasoning_effort: String,
    pub status: String,
    pub git_clean: bool,
    pub handoff_status: String,
    pub last_exit_code: Option<i32>,
    pub last_started_at: Option<String>,
    pub last_finished_at: Option<String>,
    pub last_summary: String,
    pub last_error: String,
    #[serde(default)]
    pub pending_action: String,
    #[serde(default)]
    pub launch_blocked: bool,
    #[serde(default)]
    pub execution_scope: String,
    #[serde(default)]
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StatusSnapshot {
    pub updated_at: String,
    pub repo_root: String,
    pub framework_version: String,
    pub project_name: String,
    pub phase: String,
    pub activation_required: bool,
    pub activation_reason: String,
    pub agent_room_healthy: bool,
    pub last_handoff_check: String,
    pub last_check_status: String,
    #[serde(default)]
    pub last_check_warning: String,
    #[serde(default)]
    pub workers: Vec<WorkerSnapshot>,
    #[serde(default)]
    pub recent_activity: Vec<ActivityEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ControlSnapshot {
    pub session_id: String,
    pub pid: u32,
    pub started_at: String,
    pub updated_at: String,
    pub heartbeat_epoch: f64,
    pub repo_root: String,
    pub runtime_dir: String,
    pub status_file: String,
    pub brief_file: String,
    pub patrol_file: String,
    pub remote_inbox_dir: String,
    pub remote_ack_dir: String,
    pub running: bool,
    #[serde(default)]
    pub supported_commands: Vec<String>,
    pub current_phase: String,
    pub activation_required: bool,
    pub selected_worker: String,
    pub focused_panel: String,
    pub stream_scope: StreamScope,
    pub density_mode: FeedDensity,
    pub follow_tail: bool,
    pub help_visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PatrolStatus {
    pub updated_at: String,
    pub phase: String,
    pub enabled: bool,
    pub last_run_at: Option<String>,
    pub last_result: String,
    pub last_error: String,
    #[serde(default)]
    pub last_warning: String,
    pub activation_required: bool,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemoteCommand {
    pub id: String,
    pub command: String,
    pub source: String,
    pub created_at: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemoteAck {
    pub ok: bool,
    pub command: String,
    pub source: String,
    pub processed_at: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct WorkerThreadState {
    pub worker_name: String,
    pub phase: String,
    pub status: String,
    pub pid: Option<u32>,
    pub last_started_at: Option<String>,
    pub last_finished_at: Option<String>,
    pub last_exit_code: Option<i32>,
    pub last_summary: String,
    pub last_error: String,
    pub pending_action: String,
    #[serde(default)]
    pub launch_blocked: bool,
    #[serde(default)]
    pub execution_scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CoordinationWorkerSnapshot {
    pub name: String,
    pub branch: String,
    pub worktree_path: String,
    pub goal: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub deliverables: Vec<String>,
    #[serde(default)]
    pub validation: Vec<String>,
    #[serde(default)]
    pub review_focus: Vec<String>,
    #[serde(default)]
    pub open_questions: Vec<String>,
    #[serde(default)]
    pub blocked_topics: Vec<String>,
    pub status: String,
    #[serde(default)]
    pub pending_action: String,
    #[serde(default)]
    pub frozen: bool,
    #[serde(default)]
    pub launch_blocked: bool,
    #[serde(default)]
    pub execution_scope: String,
    #[serde(default)]
    pub parallel_stage: usize,
    #[serde(default)]
    pub packet_path: String,
    #[serde(default)]
    pub worktree_packet_path: String,
    #[serde(default)]
    pub last_disposition: String,
    pub last_review_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CoordinationEscalation {
    pub id: String,
    pub title: String,
    pub scope: String,
    #[serde(default)]
    pub workers: Vec<String>,
    #[serde(default)]
    pub freeze_workers: Vec<String>,
    pub reason: String,
    pub question: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CoordinationReviewSnapshot {
    pub worker_name: String,
    pub reviewed_at: String,
    pub reviewer_mode: String,
    pub disposition: String,
    pub summary: String,
    #[serde(default)]
    pub rationale: Vec<String>,
    #[serde(default)]
    pub required_actions: Vec<String>,
    #[serde(default)]
    pub validation_gaps: Vec<String>,
    pub confidence: String,
    #[serde(default)]
    pub pending_action: String,
    #[serde(default)]
    pub freeze_workers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CoordinationMetrics {
    pub workers_total: usize,
    pub reviewed_total: usize,
    pub approved_total: usize,
    pub rework_total: usize,
    pub escalated_total: usize,
    pub open_escalations: usize,
    pub average_cycle_minutes: String,
    #[serde(default)]
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CoordinationSnapshot {
    pub project_name: String,
    pub phase: String,
    pub generated_at: String,
    pub updated_at: String,
    pub plan_id: String,
    pub approved: bool,
    pub approved_at: Option<String>,
    pub planner_mode: String,
    pub project_summary: String,
    #[serde(default)]
    pub approvals_needed: Vec<String>,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub parallel_sets: Vec<Vec<String>>,
    #[serde(default)]
    pub recommended_start: Vec<String>,
    #[serde(default)]
    pub workers: BTreeMap<String, CoordinationWorkerSnapshot>,
    #[serde(default)]
    pub escalations: Vec<CoordinationEscalation>,
    #[serde(default)]
    pub reviews: BTreeMap<String, Vec<CoordinationReviewSnapshot>>,
    pub metrics: Option<CoordinationMetrics>,
}

impl CoordinationSnapshot {
    pub fn open_escalations(&self) -> Vec<&CoordinationEscalation> {
        self.escalations
            .iter()
            .filter(|item| item.status.eq_ignore_ascii_case("open"))
            .collect()
    }

    pub fn latest_review(&self, worker_name: &str) -> Option<&CoordinationReviewSnapshot> {
        self.reviews.get(worker_name).and_then(|entries| entries.last())
    }

    pub fn latest_review_counts(&self) -> (usize, usize, usize) {
        let mut approved = 0;
        let mut rework = 0;
        let mut escalated = 0;
        for entries in self.reviews.values() {
            let Some(review) = entries.last() else {
                continue;
            };
            if review.disposition.eq_ignore_ascii_case("approve") {
                approved += 1;
            } else if review.disposition.eq_ignore_ascii_case("rework") {
                rework += 1;
            } else if review.disposition.eq_ignore_ascii_case("escalate") {
                escalated += 1;
            }
        }
        (approved, rework, escalated)
    }

    pub fn blocked_worker_count(&self) -> usize {
        self.workers
            .values()
            .filter(|worker| worker.launch_blocked)
            .count()
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeLayout {
    pub root: PathBuf,
    pub status_file: PathBuf,
    pub brief_file: PathBuf,
    pub patrol_file: PathBuf,
    pub event_stream_file: PathBuf,
    pub remote_dir: PathBuf,
    pub remote_inbox_dir: PathBuf,
    pub remote_ack_dir: PathBuf,
    pub remote_control_file: PathBuf,
    pub remote_instance_file: PathBuf,
    pub threads_dir: PathBuf,
    pub coordination_dir: PathBuf,
    pub coordination_state_file: PathBuf,
    pub coordination_events_file: PathBuf,
}

impl RuntimeLayout {
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        let remote_dir = root.join("remote");
        let threads_dir = root.join("threads");
        let coordination_dir = root.join("coordination");
        Self {
            status_file: root.join("status.json"),
            brief_file: root.join("assistant-brief.md"),
            patrol_file: root.join("patrol-status.json"),
            event_stream_file: root.join("event-stream.jsonl"),
            remote_inbox_dir: remote_dir.join("inbox"),
            remote_ack_dir: remote_dir.join("acks"),
            remote_control_file: remote_dir.join("control.json"),
            remote_instance_file: remote_dir.join("instance.json"),
            remote_dir,
            threads_dir,
            coordination_state_file: coordination_dir.join("state.json"),
            coordination_events_file: coordination_dir.join("events.jsonl"),
            coordination_dir,
            root,
        }
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        fs::create_dir_all(&self.root)
            .with_context(|| format!("failed to create {}", self.root.display()))?;
        fs::create_dir_all(&self.remote_inbox_dir)
            .with_context(|| format!("failed to create {}", self.remote_inbox_dir.display()))?;
        fs::create_dir_all(&self.remote_ack_dir)
            .with_context(|| format!("failed to create {}", self.remote_ack_dir.display()))?;
        fs::create_dir_all(&self.threads_dir)
            .with_context(|| format!("failed to create {}", self.threads_dir.display()))?;
        fs::create_dir_all(&self.coordination_dir)
            .with_context(|| format!("failed to create {}", self.coordination_dir.display()))?;
        Ok(())
    }

    pub fn worker_thread_dir(&self, worker_name: &str) -> PathBuf {
        self.threads_dir.join(worker_name)
    }

    pub fn worker_thread_state_file(&self, worker_name: &str) -> PathBuf {
        self.worker_thread_dir(worker_name).join("state.json")
    }

    pub fn read_json<T>(&self, path: &Path) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let value = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(Some(value))
    }

    pub fn write_json<T>(&self, path: &Path, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let content = serde_json::to_string_pretty(value)?;
        fs::write(path, content).with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }

    pub fn write_text(&self, path: &Path, content: &str) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(path, content).with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }

    pub fn append_json_line<T>(&self, path: &Path, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        use std::io::Write;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .with_context(|| format!("failed to open {}", path.display()))?;
        let payload = serde_json::to_string(value)?;
        writeln!(file, "{payload}")
            .with_context(|| format!("failed to append {}", path.display()))?;
        Ok(())
    }

    pub fn read_json_lines<T>(&self, path: &Path) -> Result<Vec<T>>
    where
        T: DeserializeOwned,
    {
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let mut values = Vec::new();
        for (index, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let value = serde_json::from_str(trimmed).with_context(|| {
                format!(
                    "failed to parse jsonl line {} in {}",
                    index + 1,
                    path.display()
                )
            })?;
            values.push(value);
        }
        Ok(values)
    }
}

impl StatusSnapshot {
    pub fn render_brief(&self, coordination: Option<&CoordinationSnapshot>) -> String {
        let mut lines = vec![
            "# Assistant Brief".to_string(),
            String::new(),
            format!("- Updated: {}", self.updated_at),
            format!("- Project: {}", self.project_name),
            format!("- Repo: {}", self.repo_root),
            format!("- Framework: {}", self.framework_version),
            format!("- Phase: {}", self.phase),
            format!(
                "- Activation: {}",
                if self.activation_required {
                    format!("required ({})", self.activation_reason)
                } else {
                    "not needed".to_string()
                }
            ),
            format!("- Handoff audit: {}", fallback_text(&self.last_check_status)),
            if self.last_check_warning.trim().is_empty() {
                "- Audit warning: none".to_string()
            } else {
                format!(
                    "- Audit warning: {}",
                    fallback_text(&self.last_check_warning)
                )
            },
            String::new(),
        ];

        if let Some(coordination) = coordination {
            let (approved_reviews, rework_reviews, escalated_reviews) =
                coordination.latest_review_counts();
            let open_escalations = coordination.open_escalations();
            lines.extend([
                "## Coordination".to_string(),
                String::new(),
                format!("- Plan ID: {}", fallback_text(&coordination.plan_id)),
                format!("- Approved: {}", coordination.approved),
                format!("- Planner mode: {}", fallback_text(&coordination.planner_mode)),
                format!("- Open escalations: {}", open_escalations.len()),
                format!("- Launch blocked workers: {}", coordination.blocked_worker_count()),
                format!(
                    "- Latest reviews: approve {} | rework {} | escalate {}",
                    approved_reviews, rework_reviews, escalated_reviews
                ),
                String::new(),
            ]);
        }

        lines.extend([
            "## Workers".to_string(),
            String::new(),
        ]);

        for worker in &self.workers {
            lines.extend([
                format!("### {}", worker.name),
                format!("- worktree: {}", worker.worktree_path),
                format!(
                    "- branch: {} (expected {})",
                    worker.branch, worker.expected_branch
                ),
                format!("- status: {}", worker.status),
                format!("- git clean: {}", worker.git_clean),
                format!("- handoff: {}", worker.handoff_status),
                format!(
                    "- pending action: {}",
                    fallback_text(&worker.pending_action)
                ),
                format!("- launch blocked: {}", worker.launch_blocked),
                format!(
                    "- execution scope: {}",
                    fallback_text(&worker.execution_scope)
                ),
                format!("- last exit: {}", optional_i32(worker.last_exit_code)),
                format!("- summary: {}", fallback_text(&worker.last_summary)),
                format!("- error: {}", fallback_text(&worker.last_error)),
                if worker.issues.is_empty() {
                    "- issues: none".to_string()
                } else {
                    format!("- issues: {}", worker.issues.join("; "))
                },
                String::new(),
            ]);
        }

        if !self.last_handoff_check.trim().is_empty() {
            lines.extend([
                "## Last Handoff Check".to_string(),
                String::new(),
                "```text".to_string(),
                self.last_handoff_check.clone(),
                "```".to_string(),
                String::new(),
            ]);
        }

        lines.join("\n")
    }
}

fn optional_i32(value: Option<i32>) -> String {
    value
        .map(|item| item.to_string())
        .unwrap_or_else(|| "none".to_string())
}

fn fallback_text(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "none".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn now_string() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}
