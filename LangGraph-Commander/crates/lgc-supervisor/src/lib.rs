use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use lgc_core::config::{CommanderConfig, WorkerConfig, read_framework_version};
use lgc_core::runtime::{
    ActivityEntry, ControlSnapshot, CoordinationSnapshot, FeedDensity, PatrolStatus, RemoteAck,
    RemoteCommand, RuntimeLayout, StatusSnapshot, StreamScope, WorkerSnapshot, WorkerThreadState,
    now_string,
};
use lgc_provider_openai::{ProviderProfile, load_profile};
use reqwest::blocking::Client;
use serde::Deserialize;
use sysinfo::{Pid, ProcessesToUpdate, System};
use uuid::Uuid;
use wait_timeout::ChildExt;

const CONTROL_STALE_SECONDS: f64 = 8.0;
const REMOTE_POLL_MILLIS: u64 = 350;
const WORKER_PROGRESS_PREFIX: &str = "__LGC_PROGRESS__";

#[derive(Debug, Clone, Default)]
pub struct SnapshotBundle {
    pub status: StatusSnapshot,
    pub patrol: PatrolStatus,
    pub control: Option<ControlSnapshot>,
    pub provider: Option<ProviderProfile>,
    pub coordination: Option<CoordinationSnapshot>,
}

pub struct SupervisorSession {
    inner: Arc<SupervisorInner>,
    poll_thread: Option<thread::JoinHandle<()>>,
    remote_thread: Option<thread::JoinHandle<()>>,
}

pub struct SupervisorOneShot {
    inner: Arc<SupervisorInner>,
}

struct SupervisorInner {
    config: CommanderConfig,
    runtime: RuntimeLayout,
    framework_version: String,
    session_id: Option<String>,
    started_at: String,
    provider_profile: Option<ProviderProfile>,
    stop_flag: AtomicBool,
    state: Arc<Mutex<SupervisorState>>,
}

struct SupervisorState {
    current_phase: String,
    last_check_output: String,
    last_check_status: String,
    last_check_warning: String,
    activity: Vec<ActivityEntry>,
    next_activity_seq: u64,
    patrol: PatrolStatus,
    worker_processes: BTreeMap<String, ManagedWorker>,
    status: StatusSnapshot,
    ui: UiState,
}

#[derive(Clone)]
struct UiState {
    selected_worker: String,
    focused_panel: String,
    stream_scope: StreamScope,
    density_mode: FeedDensity,
    follow_tail: bool,
    help_visible: bool,
}

struct ManagedWorker {
    child: Child,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct WorkerProgressPayload {
    source: String,
    level: String,
    channel: String,
    worker_name: String,
    message: String,
    current_activity: String,
    tags: Vec<String>,
}

pub struct CommandOutcome {
    pub message: String,
    pub quit_requested: bool,
}

struct CommandOutput {
    exit_code: i32,
    output: String,
}

impl SupervisorSession {
    pub fn start(config_path: impl AsRef<Path>) -> Result<Self> {
        let inner = SupervisorInner::new(config_path.as_ref(), true)?;
        inner.refresh_and_persist()?;
        inner.log(
            "commander",
            "info",
            "system",
            "",
            "LangGraph-Commander session started",
        );

        let poll_inner = Arc::clone(&inner);
        let poll_thread = thread::spawn(move || {
            while !poll_inner.stop_flag.load(Ordering::Relaxed) {
                let sleep_seconds = poll_inner.config.runtime.poll_interval_seconds.max(1);
                if let Err(error) = poll_inner.poll_once() {
                    poll_inner.log(
                        "commander",
                        "error",
                        "system",
                        "",
                        format!("poll failed: {error}"),
                    );
                }
                for _ in 0..sleep_seconds {
                    if poll_inner.stop_flag.load(Ordering::Relaxed) {
                        return;
                    }
                    thread::sleep(Duration::from_secs(1));
                }
            }
        });

        let remote_inner = Arc::clone(&inner);
        let remote_thread = thread::spawn(move || {
            while !remote_inner.stop_flag.load(Ordering::Relaxed) {
                if let Err(error) = remote_inner.process_remote_queue() {
                    remote_inner.log(
                        "remote",
                        "error",
                        "remote",
                        "",
                        format!("remote queue failed: {error}"),
                    );
                }
                thread::sleep(Duration::from_millis(REMOTE_POLL_MILLIS));
            }
        });

        Ok(Self {
            inner,
            poll_thread: Some(poll_thread),
            remote_thread: Some(remote_thread),
        })
    }

    pub fn snapshot(&self) -> Result<SnapshotBundle> {
        self.inner.snapshot()
    }

    pub fn execute_command(&self, command: &str, source: &str) -> Result<CommandOutcome> {
        self.inner.execute_command(command, source)
    }

    pub fn refresh_now(&self) -> Result<()> {
        self.inner.refresh_and_persist()
    }

    pub fn shutdown(&mut self) -> Result<()> {
        self.inner.shutdown()
    }
}

impl Drop for SupervisorSession {
    fn drop(&mut self) {
        let _ = self.inner.shutdown();
        if let Some(handle) = self.poll_thread.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.remote_thread.take() {
            let _ = handle.join();
        }
    }
}

impl SupervisorOneShot {
    pub fn new(config_path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            inner: SupervisorInner::new(config_path.as_ref(), false)?,
        })
    }

    pub fn execute_command(&self, command: &str, source: &str) -> Result<CommandOutcome> {
        self.inner.execute_command(command, source)
    }

    pub fn snapshot(&self) -> Result<SnapshotBundle> {
        self.inner.snapshot()
    }
}

pub fn read_runtime_snapshot(config_path: impl AsRef<Path>) -> Result<SnapshotBundle> {
    let config = CommanderConfig::load_from(config_path.as_ref())?;
    let runtime = RuntimeLayout::new(config.runtime_dir());
    let status = runtime
        .read_json::<StatusSnapshot>(&runtime.status_file)?
        .unwrap_or_default();
    let patrol = runtime
        .read_json::<PatrolStatus>(&runtime.patrol_file)?
        .unwrap_or_default();
    let coordination =
        runtime.read_json::<CoordinationSnapshot>(&runtime.coordination_state_file)?;
    let control = running_control_state(&runtime)?;
    let provider = load_profile(config.provider_config_path().as_deref())?;
    Ok(SnapshotBundle {
        status,
        patrol,
        control,
        provider,
        coordination,
    })
}

pub fn command_requires_live_panel(config_path: impl AsRef<Path>, command: &str) -> Result<bool> {
    let config = CommanderConfig::load_from(config_path.as_ref())?;
    Ok(command_matches_gate(
        &config.ui.require_live_panel_for,
        command.trim(),
    ))
}

pub fn submit_remote_command(
    config_path: impl AsRef<Path>,
    command: &str,
    source: &str,
    require_running: bool,
) -> Result<RemoteAck> {
    let config = CommanderConfig::load_from(config_path.as_ref())?;
    let runtime = RuntimeLayout::new(config.runtime_dir());
    runtime.ensure_dirs()?;

    let control = running_control_state(&runtime)?;
    if control.is_none() && require_running {
        bail!("no live LangGraph-Commander session is running");
    }
    let state = control.ok_or_else(|| anyhow!("no live LangGraph-Commander session is running"))?;
    if !control_supports_command(&state, command) {
        bail!(
            "live commander panel does not support command `{}`; restart the panel to load the current command surface",
            command.trim()
        );
    }

    let expected_repo_root = canonicalize_path(&config.repo_root());
    let live_repo_root = canonicalize_path(Path::new(&state.repo_root));
    if expected_repo_root != live_repo_root {
        bail!(
            "live commander panel repo mismatch: expected {} but found {}",
            expected_repo_root.display(),
            live_repo_root.display()
        );
    }

    let request_id = format!("{}-{}", now_compact(), Uuid::new_v4().simple());
    let inbox_path = runtime.remote_inbox_dir.join(format!("{request_id}.json"));
    let ack_path = runtime.remote_ack_dir.join(format!("{request_id}.json"));
    let payload = RemoteCommand {
        id: request_id,
        command: command.to_string(),
        source: source.to_string(),
        created_at: now_string(),
        session_id: Some(state.session_id),
    };
    runtime.write_json(&inbox_path, &payload)?;

    let ack_timeout_seconds = config
        .runtime
        .command_timeout_seconds
        .max(30)
        .saturating_add(5);
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(ack_timeout_seconds) {
        if let Some(ack) = runtime.read_json::<RemoteAck>(&ack_path)? {
            let _ = fs::remove_file(&ack_path);
            return Ok(ack);
        }
        thread::sleep(Duration::from_millis(200));
    }

    bail!(
        "timed out waiting for commander ack after {}s: {}",
        ack_timeout_seconds,
        command
    );
}

pub fn running_control_state(runtime: &RuntimeLayout) -> Result<Option<ControlSnapshot>> {
    let Some(control) = runtime.read_json::<ControlSnapshot>(&runtime.remote_control_file)? else {
        return Ok(None);
    };
    if !control.running {
        return Ok(None);
    }
    let age = current_epoch() - control.heartbeat_epoch;
    if age > CONTROL_STALE_SECONDS {
        return Ok(None);
    }
    Ok(Some(control))
}

impl SupervisorInner {
    fn new(config_path: &Path, acquire_instance: bool) -> Result<Arc<Self>> {
        let config = CommanderConfig::load_from(config_path)?;
        let framework_root = workspace_root();
        let framework_version = read_framework_version(&framework_root)?;
        config.ensure_framework_version(&framework_version)?;
        let runtime = RuntimeLayout::new(config.runtime_dir());
        runtime.ensure_dirs()?;
        let provider_profile = load_profile(config.provider_config_path().as_deref())?;

        let existing_status = runtime.read_json::<StatusSnapshot>(&runtime.status_file)?;
        let existing_patrol = runtime.read_json::<PatrolStatus>(&runtime.patrol_file)?;
        let current_phase = existing_status
            .as_ref()
            .map(|status| status.phase.as_str())
            .filter(|phase| config.phases.contains_key(*phase))
            .unwrap_or(config.active_phase_name())
            .to_string();
        let now = now_string();
        let selected_worker = default_selected_worker(&config, &current_phase);
        let ui = UiState {
            selected_worker: selected_worker.clone(),
            focused_panel: "workers".to_string(),
            stream_scope: parse_stream_scope(&config.ui.default_stream_scope, &selected_worker),
            density_mode: parse_density(&config.ui.default_density),
            follow_tail: true,
            help_visible: false,
        };

        let status = existing_status.unwrap_or_else(|| StatusSnapshot {
            updated_at: now.clone(),
            repo_root: config.repo_root().display().to_string(),
            framework_version: framework_version.clone(),
            project_name: config.project.name.clone(),
            phase: current_phase.clone(),
            activation_required: false,
            activation_reason: "warming up".to_string(),
            agent_room_healthy: false,
            last_handoff_check: String::new(),
            last_check_status: "wait".to_string(),
            last_check_warning: String::new(),
            workers: Vec::new(),
            recent_activity: Vec::new(),
        });
        let patrol = existing_patrol.unwrap_or_else(|| PatrolStatus {
            updated_at: now.clone(),
            phase: current_phase.clone(),
            enabled: true,
            last_run_at: None,
            last_result: "waiting".to_string(),
            last_error: String::new(),
            last_warning: String::new(),
            activation_required: false,
            summary: "Patrol will refresh status and audit phase health.".to_string(),
        });
        let (activity, next_activity_seq) =
            load_activity_buffer(&runtime, config.ui.event_buffer_size.max(100))?;

        let inner = Arc::new(Self {
            config,
            runtime,
            framework_version,
            session_id: if acquire_instance {
                Some(format!("{}-{}", current_epoch() as u64, std::process::id()))
            } else {
                None
            },
            started_at: now,
            provider_profile,
            stop_flag: AtomicBool::new(false),
            state: Arc::new(Mutex::new(SupervisorState {
                current_phase,
                last_check_output: status.last_handoff_check.clone(),
                last_check_status: status.last_check_status.clone(),
                last_check_warning: status.last_check_warning.clone(),
                activity,
                next_activity_seq,
                patrol,
                worker_processes: BTreeMap::new(),
                status,
                ui,
            })),
        });

        if acquire_instance {
            inner.acquire_instance_lock()?;
        }
        Ok(inner)
    }

    fn snapshot(&self) -> Result<SnapshotBundle> {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow!("supervisor state poisoned"))?;
        Ok(SnapshotBundle {
            status: state.status.clone(),
            patrol: state.patrol.clone(),
            control: self.control_snapshot_for(&state),
            provider: self.provider_profile.clone(),
            coordination: self
                .runtime
                .read_json::<CoordinationSnapshot>(&self.runtime.coordination_state_file)?,
        })
    }

    fn poll_once(&self) -> Result<()> {
        let patrol_enabled = {
            let state = self
                .state
                .lock()
                .map_err(|_| anyhow!("supervisor state poisoned"))?;
            state.patrol.enabled
        };
        self.refresh_and_persist()?;
        if patrol_enabled {
            let _ = self.run_audit("patrol");
        }
        self.refresh_and_persist()
    }

    fn refresh_and_persist(&self) -> Result<()> {
        self.reconcile_worker_processes()?;
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| anyhow!("supervisor state poisoned"))?;
            let mut status = self.build_status_snapshot(&state)?;
            status.recent_activity = state.activity.clone();
            state.status = status;
            state.patrol.updated_at = now_string();
            state.patrol.phase = state.current_phase.clone();
            state.patrol.activation_required = state.status.activation_required;
            if state
                .status
                .last_check_status
                .eq_ignore_ascii_case("warning")
                && !state.status.last_check_warning.trim().is_empty()
            {
                state.patrol.summary =
                    format!("handoff audit warning: {}", state.status.last_check_warning);
            } else if !state.status.activation_reason.trim().is_empty() {
                state.patrol.summary = state.status.activation_reason.clone();
            }
        }
        self.persist_runtime_files()
    }

    fn persist_runtime_files(&self) -> Result<()> {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow!("supervisor state poisoned"))?;
        self.runtime
            .write_json(&self.runtime.status_file, &state.status)?;
        self.runtime.write_text(
            &self.runtime.brief_file,
            &state.status.render_brief(
                self.runtime
                    .read_json::<CoordinationSnapshot>(&self.runtime.coordination_state_file)?
                    .as_ref(),
            ),
        )?;
        self.runtime
            .write_json(&self.runtime.patrol_file, &state.patrol)?;
        if let Some(control) = self.control_snapshot_for(&state) {
            self.runtime
                .write_json(&self.runtime.remote_control_file, &control)?;
        } else {
            let _ = fs::remove_file(&self.runtime.remote_control_file);
        }
        Ok(())
    }

    fn execute_command(&self, command: &str, source: &str) -> Result<CommandOutcome> {
        let command = command.trim();
        if command.is_empty() {
            return Ok(CommandOutcome {
                message: "empty command ignored".to_string(),
                quit_requested: false,
            });
        }

        let lowered = command.to_ascii_lowercase();
        let message = if lowered == "help" || lowered == "?" {
            render_help()
        } else if lowered == "help keys" {
            render_key_help()
        } else if lowered == "status" || lowered == "refresh" {
            self.refresh_and_persist()?;
            "status refreshed".to_string()
        } else if lowered == "check" {
            self.run_audit(source)?
        } else if lowered == "intake" {
            self.handle_coordination_command(&["intake"])?
        } else if lowered == "approve" {
            self.handle_coordination_command(&["approve"])?
        } else if lowered == "brief" {
            self.refresh_and_persist()?;
            format!(
                "assistant brief updated at {}",
                self.runtime.brief_file.display()
            )
        } else if lowered == "clear" {
            {
                let mut state = self
                    .state
                    .lock()
                    .map_err(|_| anyhow!("supervisor state poisoned"))?;
                state.activity.clear();
                state.next_activity_seq = 1;
                state.status.recent_activity.clear();
            }
            let _ = fs::write(&self.runtime.event_stream_file, "");
            self.refresh_and_persist()?;
            "activity cleared".to_string()
        } else if lowered == "quit" || lowered == "exit" {
            self.log(source, "warning", "system", "", "shutdown requested");
            self.stop_flag.store(true, Ordering::Relaxed);
            return Ok(CommandOutcome {
                message: "quit requested".to_string(),
                quit_requested: true,
            });
        } else if lowered == "ping" || lowered.starts_with("ping ") {
            let payload = command.get(4..).unwrap_or("").trim();
            if payload.is_empty() {
                "pong".to_string()
            } else {
                format!("PING {payload}")
            }
        } else if lowered.starts_with("start ") {
            self.handle_start(command[6..].trim())?
        } else if lowered.starts_with("stop ") {
            self.handle_stop(command[5..].trim())?
        } else if lowered.starts_with("phase ") {
            self.handle_phase(command[6..].trim())?
        } else if lowered == "review" || lowered.starts_with("review ") {
            self.handle_review(command.get(6..).unwrap_or("").trim())?
        } else if lowered == "report" {
            self.handle_coordination_command(&["report"])?
        } else if lowered == "patrol" || lowered.starts_with("patrol ") {
            self.handle_patrol(command.get(6..).unwrap_or("").trim())?
        } else if lowered.starts_with("density ") {
            self.handle_density(command[8..].trim())?
        } else if lowered.starts_with("follow ") {
            self.handle_follow(command[7..].trim())?
        } else if lowered.starts_with("focus ") {
            self.handle_focus(command[6..].trim())?
        } else if lowered.starts_with("stream ") {
            self.handle_stream(command[7..].trim())?
        } else if lowered.starts_with("select ") {
            self.handle_select(command[7..].trim())?
        } else {
            bail!("unknown command: {command}");
        };

        let level = if message.to_ascii_lowercase().contains("fail") {
            "error"
        } else {
            "info"
        };
        self.log(source, level, "command", "", &message);
        self.refresh_and_persist()?;
        Ok(CommandOutcome {
            message,
            quit_requested: false,
        })
    }

    fn handle_start(&self, target: &str) -> Result<String> {
        let worker_names = if target.eq_ignore_ascii_case("all") {
            self.active_phase()?
                .workers
                .iter()
                .map(|worker| worker.name.clone())
                .collect()
        } else {
            vec![target.to_string()]
        };

        let mut results = Vec::new();
        for worker_name in worker_names {
            let worker = self.find_worker(&worker_name)?;
            let result = self.start_worker(&worker)?;
            results.push(result);
        }
        Ok(results.join(" | "))
    }

    fn handle_stop(&self, target: &str) -> Result<String> {
        let worker_names = if target.eq_ignore_ascii_case("all") {
            self.active_phase()?
                .workers
                .iter()
                .map(|worker| worker.name.clone())
                .collect()
        } else {
            vec![target.to_string()]
        };

        let mut results = Vec::new();
        for worker_name in worker_names {
            let worker = self.find_worker(&worker_name)?;
            let result = self.stop_worker(&worker)?;
            results.push(result);
        }
        Ok(results.join(" | "))
    }

    fn handle_phase(&self, phase_name: &str) -> Result<String> {
        self.config.phase(phase_name)?;
        let selected_worker = default_selected_worker(&self.config, phase_name);
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow!("supervisor state poisoned"))?;
        state.current_phase = phase_name.to_string();
        state.patrol.phase = phase_name.to_string();
        state.ui.selected_worker = selected_worker.clone();
        if matches!(state.ui.stream_scope, StreamScope::Selected) && selected_worker.is_empty() {
            state.ui.stream_scope = StreamScope::All;
        }
        drop(state);
        Ok(format!("phase switched to {phase_name}"))
    }

    fn handle_review(&self, target: &str) -> Result<String> {
        let normalized = if target.trim().is_empty() {
            "all".to_string()
        } else {
            target.trim().to_string()
        };
        self.handle_coordination_command(&["review", normalized.as_str()])
    }

    fn handle_coordination_command(&self, args: &[&str]) -> Result<String> {
        let label = format!("coordination {}", args.join(" "));
        self.log(
            "commander",
            "info",
            "coordination",
            "",
            format!("{label}: started"),
        );
        let mut command = vec![
            "python".to_string(),
            "LangGraph-Commander/scripts/coordination_bridge.py".to_string(),
        ];
        command.extend(args.iter().map(|item| item.to_string()));
        let started = Instant::now();
        let progress_label = args.join(" ");
        let output = run_command_streaming(
            &command,
            &self.config.repo_root(),
            Duration::from_secs(self.config.runtime.command_timeout_seconds.max(60)),
            |line| {
                self.log(
                    "commander",
                    "info",
                    "coordination",
                    "",
                    format!("{progress_label} | {line}"),
                );
            },
            |line| {
                self.log(
                    "commander",
                    "warning",
                    "coordination",
                    "",
                    format!("{progress_label} | {line}"),
                );
            },
        )?;
        if output.exit_code != 0 {
            bail!(
                "coordination command failed (exit {}): {}",
                output.exit_code,
                output.output
            );
        }
        let elapsed = started.elapsed();
        Ok(format_coordination_result(
            &label,
            elapsed,
            output.output.trim(),
        ))
    }

    fn handle_patrol(&self, mode: &str) -> Result<String> {
        let normalized = mode.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "" | "once" => self.run_audit("patrol"),
            "start" | "on" => {
                let mut state = self
                    .state
                    .lock()
                    .map_err(|_| anyhow!("supervisor state poisoned"))?;
                state.patrol.enabled = true;
                state.patrol.last_result = "enabled".to_string();
                Ok("patrol enabled".to_string())
            }
            "stop" | "off" => {
                let mut state = self
                    .state
                    .lock()
                    .map_err(|_| anyhow!("supervisor state poisoned"))?;
                state.patrol.enabled = false;
                state.patrol.last_result = "disabled".to_string();
                Ok("patrol disabled".to_string())
            }
            "status" => {
                let state = self
                    .state
                    .lock()
                    .map_err(|_| anyhow!("supervisor state poisoned"))?;
                Ok(format!(
                    "patrol={} last_result={} activation_required={} warning={}",
                    state.patrol.enabled,
                    state.patrol.last_result,
                    state.patrol.activation_required,
                    if state.patrol.last_warning.trim().is_empty() {
                        "none"
                    } else {
                        state.patrol.last_warning.as_str()
                    }
                ))
            }
            other => bail!("unknown patrol mode `{other}`"),
        }
    }

    fn handle_density(&self, mode: &str) -> Result<String> {
        let density = match mode.trim().to_ascii_lowercase().as_str() {
            "standard" => FeedDensity::Standard,
            "realtime" | "high" | "high-density" => FeedDensity::Realtime,
            other => bail!("unknown density mode `{other}`"),
        };
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow!("supervisor state poisoned"))?;
        state.ui.density_mode = density;
        Ok(format!("density set to {}", density_label(density)))
    }

    fn handle_follow(&self, mode: &str) -> Result<String> {
        let enabled = match mode.trim().to_ascii_lowercase().as_str() {
            "on" | "true" | "tail" => true,
            "off" | "false" => false,
            other => bail!("unknown follow mode `{other}`"),
        };
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow!("supervisor state poisoned"))?;
        state.ui.follow_tail = enabled;
        Ok(format!(
            "follow-tail {}",
            if enabled { "enabled" } else { "disabled" }
        ))
    }

    fn handle_focus(&self, panel: &str) -> Result<String> {
        let normalized = panel.trim().to_ascii_lowercase();
        let allowed = ["workers", "feed", "details", "cmd"];
        if !allowed.contains(&normalized.as_str()) {
            bail!("unknown focus target `{panel}`");
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow!("supervisor state poisoned"))?;
        state.ui.focused_panel = normalized.clone();
        Ok(format!("focus set to {normalized}"))
    }

    fn handle_stream(&self, raw: &str) -> Result<String> {
        let candidate = raw.trim().strip_prefix("scope ").unwrap_or(raw).trim();
        let selected_worker = {
            let state = self
                .state
                .lock()
                .map_err(|_| anyhow!("supervisor state poisoned"))?;
            state.ui.selected_worker.clone()
        };
        let scope = parse_stream_scope(candidate, &selected_worker);
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow!("supervisor state poisoned"))?;
        state.ui.stream_scope = scope.clone();
        Ok(format!(
            "stream scope set to {}",
            stream_scope_label(&scope)
        ))
    }

    fn handle_select(&self, worker_name: &str) -> Result<String> {
        let worker = self.find_worker(worker_name)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow!("supervisor state poisoned"))?;
        state.ui.selected_worker = worker.name.clone();
        if let StreamScope::Worker(name) = &state.ui.stream_scope {
            if name != &worker.name {
                state.ui.stream_scope = StreamScope::Selected;
            }
        }
        Ok(format!("selected worker {}", worker.name))
    }

    fn run_audit(&self, source: &str) -> Result<String> {
        let phase = self.active_phase()?;
        if phase.audit_command.is_empty() {
            let message = "no audit command configured".to_string();
            self.log(source, "warning", "audit", "", &message);
            return Ok(message);
        }
        let output = run_command(
            &phase.audit_command,
            &self.config.repo_root(),
            Duration::from_secs(self.config.runtime.command_timeout_seconds.max(30)),
        )?;

        let (status_label, state_label, log_level, warning_text) = classify_audit_result(&output);

        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| anyhow!("supervisor state poisoned"))?;
            state.last_check_output = output.output.trim().to_string();
            state.last_check_status = state_label.to_string();
            state.last_check_warning = warning_text.clone();
            state.patrol.last_run_at = Some(now_string());
            state.patrol.last_result = status_label.to_string();
            state.patrol.last_error = if status_label == "FAIL" {
                output.output.trim().to_string()
            } else {
                String::new()
            };
            state.patrol.last_warning = if status_label == "WARN" {
                warning_text.clone()
            } else {
                String::new()
            };
        }
        let message = format!("handoff audit {status_label} (exit {})", output.exit_code);
        self.log(source, log_level, "audit", "", &message);
        if !warning_text.is_empty() {
            self.log(
                source,
                "warning",
                "audit",
                "",
                format!("audit warning: {warning_text}"),
            );
        }
        self.persist_runtime_files()?;
        Ok(message)
    }

    fn start_worker(&self, worker: &WorkerConfig) -> Result<String> {
        let worktree = worker.worktree_path(&self.config);
        let phase_name = {
            let state = self
                .state
                .lock()
                .map_err(|_| anyhow!("supervisor state poisoned"))?;
            state.current_phase.clone()
        };
        let thread_state = self.load_thread_state(&worker.name, &phase_name)?;
        if !worktree.exists() {
            let message = format!("{} worktree missing: {}", worker.name, worktree.display());
            self.update_thread_state(worker, |state| {
                state.status = "missing".to_string();
                state.last_error = message.clone();
                state.last_summary = message.clone();
                state.current_activity = "worktree missing".to_string();
            })?;
            self.log("commander", "error", "worker", &worker.name, &message);
            return Ok(message);
        }
        if worker.launch_command.is_empty() {
            let message = format!(
                "{} launch_command not configured in commander.toml",
                worker.name
            );
            self.update_thread_state(worker, |state| {
                state.status = "idle".to_string();
                state.pending_action = "manual-activation".to_string();
                state.launch_blocked = false;
                state.execution_scope = "manual".to_string();
                state.last_summary = message.clone();
                state.last_error.clear();
                state.current_activity = "manual activation required".to_string();
            })?;
            self.log("commander", "warning", "worker", &worker.name, &message);
            return Ok(message);
        }
        if thread_state.launch_blocked || pending_action_blocks_start(&thread_state.pending_action)
        {
            let message = format!(
                "{} start blocked: {}",
                worker.name, thread_state.pending_action
            );
            self.update_thread_state(worker, |state| {
                if !thread_state.pending_action.trim().is_empty() {
                    state.current_activity = thread_state.pending_action.clone();
                }
            })?;
            self.log("commander", "warning", "worker", &worker.name, &message);
            return Ok(message);
        }

        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| anyhow!("supervisor state poisoned"))?;
            if let Some(existing) = state.worker_processes.get_mut(&worker.name) {
                if existing.child.try_wait()?.is_none() {
                    let message = format!("{} already running", worker.name);
                    drop(state);
                    self.log("commander", "info", "worker", &worker.name, &message);
                    return Ok(message);
                }
            }
        }

        let worktree_env = worktree.to_string_lossy().to_string();
        let runtime_env = self.runtime.root.to_string_lossy().to_string();
        let phase_env = {
            let state = self
                .state
                .lock()
                .map_err(|_| anyhow!("supervisor state poisoned"))?;
            state.current_phase.clone()
        };
        let mut child = spawn_worker_process(
            &worker.launch_command,
            &self.config.repo_root(),
            &[
                ("LGC_WORKER_NAME", worker.name.as_str()),
                ("LGC_WORKTREE", worktree_env.as_str()),
                ("LGC_PHASE", phase_env.as_str()),
                ("LGC_RUNTIME_DIR", runtime_env.as_str()),
            ],
        )?;
        let pid = child.id();

        if let Some(stdout) = child.stdout.take() {
            spawn_activity_reader(
                Arc::clone(&self.state),
                self.runtime.clone(),
                self.config.ui.event_buffer_size.max(100),
                worker.name.clone(),
                "stdout".to_string(),
                "info".to_string(),
                stdout,
            );
        }
        if let Some(stderr) = child.stderr.take() {
            spawn_activity_reader(
                Arc::clone(&self.state),
                self.runtime.clone(),
                self.config.ui.event_buffer_size.max(100),
                worker.name.clone(),
                "stderr".to_string(),
                "error".to_string(),
                stderr,
            );
        }

        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| anyhow!("supervisor state poisoned"))?;
            state
                .worker_processes
                .insert(worker.name.clone(), ManagedWorker { child });
            if state.ui.selected_worker.trim().is_empty() {
                state.ui.selected_worker = worker.name.clone();
            }
        }

        self.update_thread_state(worker, |thread_state| {
            thread_state.status = "running".to_string();
            thread_state.pid = Some(pid);
            thread_state.last_started_at = Some(now_string());
            thread_state.last_finished_at = None;
            thread_state.last_exit_code = None;
            thread_state.last_summary = format!("running {}", worker.launch_command.join(" "));
            thread_state.last_error.clear();
            thread_state.current_activity = "starting worker round".to_string();
            thread_state.launch_blocked = false;
        })?;

        let message = format!("started {} (pid {})", worker.name, pid);
        self.log("commander", "info", "worker", &worker.name, &message);
        Ok(message)
    }

    fn stop_worker(&self, worker: &WorkerConfig) -> Result<String> {
        if !worker.stop_command.is_empty() {
            let output = run_command(
                &worker.stop_command,
                &worker.worktree_path(&self.config),
                Duration::from_secs(30),
            )?;
            let message = format!(
                "stop command for {} exited {}",
                worker.name, output.exit_code
            );
            self.update_thread_state(worker, |state| {
                state.status = "stopped".to_string();
                state.last_finished_at = Some(now_string());
                state.last_exit_code = Some(output.exit_code);
                state.last_summary = message.clone();
                state.last_error = if output.exit_code == 0 {
                    String::new()
                } else {
                    output.output.clone()
                };
                state.current_activity = "stopped".to_string();
                state.pid = None;
            })?;
            self.log("commander", "warning", "worker", &worker.name, &message);
            return Ok(message);
        }

        let managed = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| anyhow!("supervisor state poisoned"))?;
            state.worker_processes.remove(&worker.name)
        };

        let Some(mut managed) = managed else {
            let message = format!("{} is not running", worker.name);
            self.log("commander", "info", "worker", &worker.name, &message);
            return Ok(message);
        };
        managed
            .child
            .kill()
            .with_context(|| format!("failed to stop {}", worker.name))?;
        let _ = managed.child.wait();

        let message = format!("stopped {}", worker.name);
        self.update_thread_state(worker, |thread_state| {
            thread_state.status = "stopped".to_string();
            thread_state.pid = None;
            thread_state.last_finished_at = Some(now_string());
            thread_state.last_exit_code = Some(130);
            thread_state.last_summary = message.clone();
            thread_state.last_error.clear();
            thread_state.current_activity = "stopped".to_string();
        })?;
        self.log("commander", "warning", "worker", &worker.name, &message);
        Ok(message)
    }

    fn active_phase(&self) -> Result<lgc_core::config::PhaseConfig> {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow!("supervisor state poisoned"))?;
        Ok(self.config.phase(&state.current_phase)?.clone())
    }

    fn find_worker(&self, worker_name: &str) -> Result<WorkerConfig> {
        let phase = self.active_phase()?;
        phase
            .workers
            .iter()
            .find(|worker| worker.name == worker_name)
            .cloned()
            .ok_or_else(|| anyhow!("unknown worker `{worker_name}` in current phase"))
    }

    fn build_status_snapshot(&self, state: &SupervisorState) -> Result<StatusSnapshot> {
        let phase = self.config.phase(&state.current_phase)?;
        let mut workers = Vec::new();
        for worker in &phase.workers {
            workers.push(self.build_worker_snapshot(worker, &state.current_phase)?);
        }

        let agent_room_healthy = self.check_agent_room_health();
        let coordination = self
            .runtime
            .read_json::<CoordinationSnapshot>(&self.runtime.coordination_state_file)?;
        let (activation_required, activation_reason) =
            determine_activation(&workers, &state.last_check_status, coordination.as_ref());

        Ok(StatusSnapshot {
            updated_at: now_string(),
            repo_root: self.config.repo_root().display().to_string(),
            framework_version: self.framework_version.clone(),
            project_name: self.config.project.name.clone(),
            phase: state.current_phase.clone(),
            activation_required,
            activation_reason,
            agent_room_healthy,
            last_handoff_check: state.last_check_output.clone(),
            last_check_status: state.last_check_status.clone(),
            last_check_warning: state.last_check_warning.clone(),
            workers,
            recent_activity: state.activity.clone(),
        })
    }

    fn build_worker_snapshot(
        &self,
        worker: &WorkerConfig,
        phase_name: &str,
    ) -> Result<WorkerSnapshot> {
        let worktree = worker.worktree_path(&self.config);
        let mut issues = Vec::new();
        let thread_state = self.load_thread_state(&worker.name, phase_name)?;
        let branch = git_output(&worktree, &["branch", "--show-current"]).unwrap_or_default();
        let git_clean = git_output(&worktree, &["status", "--porcelain"])
            .map(|value| value.trim().is_empty())
            .unwrap_or(false);
        let handoff_status = find_handoff_status(worker, &worktree);
        let pending_action = thread_state.pending_action.trim();
        let policy_context = pending_action.starts_with("scoped:")
            || pending_action.starts_with("contract-freeze owner:")
            || pending_action.eq_ignore_ascii_case("manual-activation");

        if !worktree.exists() {
            issues.push("worktree missing".to_string());
        }
        if !branch.is_empty() && branch.trim() != worker.branch {
            issues.push(format!("branch mismatch: {}", branch.trim()));
        }
        if !git_clean {
            issues.push("worktree dirty".to_string());
        }
        if handoff_status == "blocked" {
            issues.push("handoff blocked".to_string());
        }
        if handoff_status == "pending" {
            issues.push("handoff pending".to_string());
        }
        if thread_state.launch_blocked {
            if !policy_context {
                issues.push(format!("launch blocked: {}", thread_state.pending_action));
            }
        } else if !pending_action.is_empty() && !policy_context {
            issues.push(format!("pending action: {}", thread_state.pending_action));
        }

        let status = if !thread_state.status.trim().is_empty() {
            thread_state.status.clone()
        } else if !issues.is_empty() {
            "attention".to_string()
        } else {
            "idle".to_string()
        };

        Ok(WorkerSnapshot {
            name: worker.name.clone(),
            worktree_path: worktree.display().to_string(),
            branch: branch.trim().to_string(),
            expected_branch: worker.branch.clone(),
            model_name: worker.display_model(&self.config),
            reasoning_effort: reasoning_effort_label(&worker.display_model(&self.config)),
            status,
            git_clean,
            handoff_status,
            last_exit_code: thread_state.last_exit_code,
            last_started_at: thread_state.last_started_at,
            last_finished_at: thread_state.last_finished_at,
            last_summary: thread_state.last_summary,
            last_error: thread_state.last_error,
            current_activity: thread_state.current_activity,
            pending_action: thread_state.pending_action,
            launch_blocked: thread_state.launch_blocked,
            execution_scope: thread_state.execution_scope,
            issues,
        })
    }

    fn load_thread_state(&self, worker_name: &str, phase_name: &str) -> Result<WorkerThreadState> {
        let path = self.runtime.worker_thread_state_file(worker_name);
        let state = self
            .runtime
            .read_json::<WorkerThreadState>(&path)?
            .unwrap_or_else(|| WorkerThreadState {
                worker_name: worker_name.to_string(),
                phase: phase_name.to_string(),
                status: String::new(),
                pid: None,
                last_started_at: None,
                last_finished_at: None,
                last_exit_code: None,
                last_summary: "Waiting for first command.".to_string(),
                last_error: String::new(),
                current_activity: String::new(),
                pending_action: String::new(),
                launch_blocked: false,
                execution_scope: String::new(),
            });
        Ok(state)
    }

    fn update_thread_state(
        &self,
        worker: &WorkerConfig,
        mut updater: impl FnMut(&mut WorkerThreadState),
    ) -> Result<()> {
        let phase_name = {
            let state = self
                .state
                .lock()
                .map_err(|_| anyhow!("supervisor state poisoned"))?;
            state.current_phase.clone()
        };
        let mut thread_state = self.load_thread_state(&worker.name, &phase_name)?;
        thread_state.worker_name = worker.name.clone();
        thread_state.phase = phase_name;
        updater(&mut thread_state);
        let path = self.runtime.worker_thread_state_file(&worker.name);
        self.runtime.write_json(&path, &thread_state)
    }

    fn reconcile_worker_processes(&self) -> Result<()> {
        let phase_name = {
            let state = self
                .state
                .lock()
                .map_err(|_| anyhow!("supervisor state poisoned"))?;
            state.current_phase.clone()
        };
        let phase = self.config.phase(&phase_name)?.clone();
        let mut finished = Vec::new();
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| anyhow!("supervisor state poisoned"))?;
            for worker in &phase.workers {
                let Some(managed) = state.worker_processes.get_mut(&worker.name) else {
                    continue;
                };
                if let Some(exit_status) = managed.child.try_wait()? {
                    finished.push((worker.clone(), exit_status.code().unwrap_or(1)));
                }
            }
            for (worker, _) in &finished {
                state.worker_processes.remove(&worker.name);
            }
        }

        for (worker, code) in finished {
            let existing = self.load_thread_state(&worker.name, &phase_name)?;
            let status_label = if code == 0 { "finished" } else { "failed" };
            self.update_thread_state(&worker, |thread_state| {
                thread_state.status = status_label.to_string();
                thread_state.pid = None;
                thread_state.last_exit_code = Some(code);
                thread_state.last_finished_at = Some(now_string());
                thread_state.last_summary = if code == 0 && !existing.last_summary.trim().is_empty()
                {
                    existing.last_summary.clone()
                } else {
                    format!("{status_label} with exit code {code}")
                };
                thread_state.last_error = if code == 0 {
                    String::new()
                } else if !existing.last_error.trim().is_empty() {
                    existing.last_error.clone()
                } else {
                    format!("worker process exited with code {code}")
                };
                thread_state.current_activity = status_label.to_string();
            })?;
            self.log(
                "worker",
                if code == 0 { "info" } else { "error" },
                "worker",
                &worker.name,
                format!("{} {}", worker.name, status_label),
            );
        }

        Ok(())
    }

    fn process_remote_queue(&self) -> Result<()> {
        self.persist_runtime_files()?;
        let mut entries = fs::read_dir(&self.runtime.remote_inbox_dir)
            .with_context(|| format!("failed to read {}", self.runtime.remote_inbox_dir.display()))?
            .filter_map(|entry| entry.ok().map(|value| value.path()))
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
            .collect::<Vec<_>>();
        entries.sort();

        for entry in entries {
            let ack_path = self
                .runtime
                .remote_ack_dir
                .join(entry.file_name().unwrap_or_default());
            let payload = match self.runtime.read_json::<RemoteCommand>(&entry)? {
                Some(value) => value,
                None => {
                    self.write_ack(
                        &ack_path,
                        false,
                        "",
                        "remote",
                        "invalid remote command payload",
                    )?;
                    let _ = fs::remove_file(&entry);
                    continue;
                }
            };

            if let Some(session_id) = &payload.session_id {
                if self.session_id.as_ref() != Some(session_id) {
                    self.write_ack(
                        &ack_path,
                        false,
                        &payload.command,
                        &payload.source,
                        "stale commander session; reopen the live panel and retry",
                    )?;
                    let _ = fs::remove_file(&entry);
                    continue;
                }
            }

            let result = self.execute_command(&payload.command, &payload.source);
            match result {
                Ok(outcome) => {
                    self.write_ack(&ack_path, true, &payload.command, &payload.source, "")?;
                    if outcome.quit_requested {
                        self.stop_flag.store(true, Ordering::Relaxed);
                    }
                }
                Err(error) => {
                    self.write_ack(
                        &ack_path,
                        false,
                        &payload.command,
                        &payload.source,
                        &error.to_string(),
                    )?;
                }
            }
            let _ = fs::remove_file(&entry);
        }

        self.persist_runtime_files()
    }

    fn write_ack(
        &self,
        path: &Path,
        ok: bool,
        command: &str,
        source: &str,
        error: &str,
    ) -> Result<()> {
        let ack = RemoteAck {
            ok,
            command: command.to_string(),
            source: source.to_string(),
            processed_at: now_string(),
            error: error.to_string(),
        };
        self.runtime.write_json(path, &ack)
    }

    fn control_snapshot_for(&self, state: &SupervisorState) -> Option<ControlSnapshot> {
        let Some(session_id) = &self.session_id else {
            return None;
        };
        Some(ControlSnapshot {
            session_id: session_id.clone(),
            pid: std::process::id(),
            started_at: self.started_at.clone(),
            updated_at: now_string(),
            heartbeat_epoch: current_epoch(),
            repo_root: self.config.repo_root().display().to_string(),
            runtime_dir: self.runtime.root.display().to_string(),
            status_file: self.runtime.status_file.display().to_string(),
            brief_file: self.runtime.brief_file.display().to_string(),
            patrol_file: self.runtime.patrol_file.display().to_string(),
            remote_inbox_dir: self.runtime.remote_inbox_dir.display().to_string(),
            remote_ack_dir: self.runtime.remote_ack_dir.display().to_string(),
            running: !self.stop_flag.load(Ordering::Relaxed),
            supported_commands: supported_commands(),
            current_phase: state.current_phase.clone(),
            activation_required: state.status.activation_required,
            selected_worker: state.ui.selected_worker.clone(),
            focused_panel: state.ui.focused_panel.clone(),
            stream_scope: state.ui.stream_scope.clone(),
            density_mode: state.ui.density_mode,
            follow_tail: state.ui.follow_tail,
            help_visible: state.ui.help_visible,
        })
    }

    fn check_agent_room_health(&self) -> bool {
        let Some(service) = &self.config.services.agent_room else {
            return false;
        };
        if !service.enabled || service.url.trim().is_empty() {
            return false;
        }
        let client = match Client::builder().timeout(Duration::from_secs(2)).build() {
            Ok(client) => client,
            Err(_) => return false,
        };
        client
            .get(format!("{}/health", service.url.trim_end_matches('/')))
            .send()
            .map(|response| response.status().is_success())
            .unwrap_or(false)
    }

    fn acquire_instance_lock(&self) -> Result<()> {
        loop {
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&self.runtime.remote_instance_file)
            {
                Ok(file) => {
                    let instance_payload = serde_json::json!({
                        "pid": std::process::id(),
                        "created_at": now_string(),
                        "repo_root": self.config.repo_root().display().to_string(),
                    });
                    serde_json::to_writer_pretty(file, &instance_payload)?;
                    return Ok(());
                }
                Err(_) => {
                    let instance = self
                        .runtime
                        .read_json::<serde_json::Value>(&self.runtime.remote_instance_file)?;
                    let pid = instance
                        .as_ref()
                        .and_then(|payload| payload.get("pid"))
                        .and_then(|value| value.as_u64())
                        .unwrap_or_default() as u32;
                    let running = pid > 0 && process_exists(pid);
                    if running {
                        bail!("LangGraph-Commander already active in pid {pid}");
                    }
                    let _ = fs::remove_file(&self.runtime.remote_instance_file);
                }
            }
        }
    }

    fn shutdown(&self) -> Result<()> {
        self.stop_flag.store(true, Ordering::Relaxed);
        let _ = self.reconcile_worker_processes();
        let phase_name = {
            let state = self
                .state
                .lock()
                .map_err(|_| anyhow!("supervisor state poisoned"))?;
            state.current_phase.clone()
        };
        let workers_to_stop = {
            let state = self
                .state
                .lock()
                .map_err(|_| anyhow!("supervisor state poisoned"))?;
            state.worker_processes.keys().cloned().collect::<Vec<_>>()
        };
        for worker_name in workers_to_stop {
            if let Ok(worker) = self.find_worker(&worker_name) {
                if let Ok(thread_state) = self.load_thread_state(&worker.name, &phase_name) {
                    if worker_status_terminal(&thread_state.status) {
                        continue;
                    }
                }
                let _ = self.stop_worker(&worker);
            }
        }
        if self.session_id.is_some() {
            let _ = self.persist_runtime_files();
            let _ = fs::remove_file(&self.runtime.remote_control_file);
            let _ = fs::remove_file(&self.runtime.remote_instance_file);
        }
        Ok(())
    }

    fn log(
        &self,
        source: &str,
        level: &str,
        channel: &str,
        worker_name: &str,
        message: impl Into<String>,
    ) {
        let tags = if worker_name.trim().is_empty() {
            vec![source.to_string(), channel.to_string()]
        } else {
            vec![
                source.to_string(),
                channel.to_string(),
                "worker".to_string(),
                worker_name.to_string(),
            ]
        };
        let _ = append_activity(
            &self.state,
            &self.runtime,
            self.config.ui.event_buffer_size.max(100),
            source,
            level,
            channel,
            worker_name,
            message.into(),
            tags,
        );
    }
}

fn reasoning_effort_label(model_name: &str) -> String {
    let lowered = model_name.trim().to_ascii_lowercase();
    if lowered.contains("high") {
        "high".to_string()
    } else if lowered.contains("medium") {
        "medium".to_string()
    } else if lowered.contains("low") {
        "low".to_string()
    } else {
        "standard".to_string()
    }
}

fn worker_status_terminal(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "finished" | "failed" | "stopped" | "missing"
    )
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

fn supported_commands() -> Vec<String> {
    vec![
        "help".to_string(),
        "help keys".to_string(),
        "status".to_string(),
        "refresh".to_string(),
        "check".to_string(),
        "intake".to_string(),
        "approve".to_string(),
        "review [all|<worker>]".to_string(),
        "report".to_string(),
        "brief".to_string(),
        "ping [text]".to_string(),
        "start all|<worker>".to_string(),
        "stop all|<worker>".to_string(),
        "patrol [start|stop|once|status]".to_string(),
        "phase <name>".to_string(),
        "select <worker>".to_string(),
        "density standard|realtime".to_string(),
        "stream scope all|selected|commander|worker:<name>".to_string(),
        "follow on|off".to_string(),
        "focus workers|feed|details|cmd".to_string(),
        "clear".to_string(),
        "quit".to_string(),
    ]
}

fn render_help() -> String {
    supported_commands().join("\n")
}

fn render_key_help() -> String {
    [
        "GRID: arrows/WASD move panes like a game menu",
        "PANEL: arrows/WASD keep working inside the current pane",
        "Enter/Space/E: dive in, confirm, or commit the hovered worker",
        "Tab/Shift-Tab or 1/2/3/4: jump workers/feed/details/command",
        "F/Z/V: follow, density, stream scope",
        "R/C/P: refresh, check, patrol once",
        "PageUp/PageDown and Home/End: fast move while engaged",
        "t/T or [/] : cycle detail tabs",
        "/: filter workers",
        ": command mode",
        "?: help overlay",
        "x: stop selected worker",
        "X: stop all workers",
        "Esc twice: quit from grid mode",
        "q: quit panel immediately",
    ]
    .join("\n")
}

fn run_command(command: &[String], cwd: &Path, timeout: Duration) -> Result<CommandOutput> {
    if command.is_empty() {
        bail!("cannot run empty command");
    }
    let mut child = Command::new(&command[0])
        .args(&command[1..])
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to start {}", command.join(" ")))?;

    let status = match child.wait_timeout(timeout)? {
        Some(status) => status,
        None => {
            child.kill()?;
            let _ = child.wait();
            bail!(
                "command timed out after {}s: {}",
                timeout.as_secs(),
                command.join(" ")
            );
        }
    };
    let output = child.wait_with_output()?;
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    if !output.stderr.is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    Ok(CommandOutput {
        exit_code: status.code().unwrap_or(1),
        output: text.trim().to_string(),
    })
}

fn run_command_streaming(
    command: &[String],
    cwd: &Path,
    timeout: Duration,
    mut on_stdout: impl FnMut(&str),
    mut on_stderr: impl FnMut(&str),
) -> Result<CommandOutput> {
    if command.is_empty() {
        bail!("cannot run empty command");
    }
    let mut child = Command::new(&command[0])
        .args(&command[1..])
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to start {}", command.join(" ")))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("failed to capture stdout for {}", command.join(" ")))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow!("failed to capture stderr for {}", command.join(" ")))?;

    #[derive(Debug)]
    enum StreamPacket {
        Stdout(String),
        Stderr(String),
        Done { stderr: bool },
    }

    fn spawn_stream_reader<R>(reader: R, sender: mpsc::Sender<StreamPacket>, stderr: bool)
    where
        R: Read + Send + 'static,
    {
        thread::spawn(move || {
            let buffered = BufReader::new(reader);
            for line in buffered.lines() {
                match line {
                    Ok(line) => {
                        let message = if stderr {
                            StreamPacket::Stderr(line)
                        } else {
                            StreamPacket::Stdout(line)
                        };
                        let _ = sender.send(message);
                    }
                    Err(error) => {
                        let _ = sender.send(StreamPacket::Stderr(format!(
                            "{} stream read failed: {error}",
                            if stderr { "stderr" } else { "stdout" }
                        )));
                        break;
                    }
                }
            }
            let _ = sender.send(StreamPacket::Done { stderr });
        });
    }

    let (sender, receiver) = mpsc::channel();
    spawn_stream_reader(stdout, sender.clone(), false);
    spawn_stream_reader(stderr, sender.clone(), true);
    drop(sender);

    let started = Instant::now();
    let mut stdout_done = false;
    let mut stderr_done = false;
    let mut status = None;
    let mut stdout_lines = Vec::new();
    let mut stderr_lines = Vec::new();

    while !(stdout_done && stderr_done && status.is_some()) {
        if status.is_none() && started.elapsed() > timeout {
            child.kill()?;
            let _ = child.wait();
            bail!(
                "command timed out after {}s: {}",
                timeout.as_secs(),
                command.join(" ")
            );
        }

        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(StreamPacket::Stdout(line)) => {
                let line = line.trim_end().to_string();
                if !line.is_empty() {
                    on_stdout(&line);
                    stdout_lines.push(line);
                }
            }
            Ok(StreamPacket::Stderr(line)) => {
                let line = line.trim_end().to_string();
                if !line.is_empty() {
                    on_stderr(&line);
                    stderr_lines.push(line);
                }
            }
            Ok(StreamPacket::Done { stderr }) => {
                if stderr {
                    stderr_done = true;
                } else {
                    stdout_done = true;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                stdout_done = true;
                stderr_done = true;
            }
        }

        if status.is_none() {
            status = child.try_wait()?;
        }
    }

    let status = match status {
        Some(status) => status,
        None => child.wait()?,
    };
    let mut text = stdout_lines.join("\n");
    if !stderr_lines.is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&stderr_lines.join("\n"));
    }
    Ok(CommandOutput {
        exit_code: status.code().unwrap_or(1),
        output: text.trim().to_string(),
    })
}

fn spawn_worker_process(command: &[String], cwd: &Path, envs: &[(&str, &str)]) -> Result<Child> {
    if command.is_empty() {
        bail!("cannot spawn empty worker command");
    }
    let mut process = Command::new(&command[0]);
    process
        .args(&command[1..])
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in envs {
        process.env(key, value);
    }
    process
        .spawn()
        .with_context(|| format!("failed to start {}", command.join(" ")))
}

fn git_output(repo: &Path, args: &[&str]) -> Option<String> {
    if !repo.exists() {
        return None;
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn find_handoff_status(worker: &WorkerConfig, worktree: &Path) -> String {
    for relative in &worker.handoff_files {
        let path = worktree.join(relative);
        let Ok(content) = fs::read_to_string(path) else {
            continue;
        };
        let mut inside_status = false;
        for line in content.lines() {
            let stripped = line.trim().to_ascii_lowercase();
            if stripped.starts_with("## status") {
                inside_status = true;
                continue;
            }
            if inside_status && stripped.starts_with('#') {
                break;
            }
            if inside_status {
                if stripped.contains("ready") {
                    return "ready".to_string();
                }
                if stripped.contains("blocked") {
                    return "blocked".to_string();
                }
                if stripped.contains("pending") {
                    return "pending".to_string();
                }
            }
        }
    }
    "unknown".to_string()
}

fn classify_audit_result(
    output: &CommandOutput,
) -> (&'static str, &'static str, &'static str, String) {
    if output.exit_code != 0 {
        return ("FAIL", "blocked", "error", String::new());
    }
    let warning_text = extract_audit_warning(&output.output);
    if warning_text.is_empty() {
        ("PASS", "ready", "info", String::new())
    } else {
        ("WARN", "warning", "warning", warning_text)
    }
}

fn extract_audit_warning(output: &str) -> String {
    let warnings = output
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("WARN:"))
        .map(|line| line.trim_start_matches("WARN:").trim().to_string())
        .collect::<Vec<_>>();
    if !warnings.is_empty() {
        warnings.join(" | ")
    } else if output
        .lines()
        .map(str::trim)
        .any(|line| line.eq_ignore_ascii_case("Overall: WARN"))
    {
        "audit completed with warnings".to_string()
    } else {
        String::new()
    }
}

fn determine_activation(
    workers: &[WorkerSnapshot],
    last_check_status: &str,
    coordination: Option<&CoordinationSnapshot>,
) -> (bool, String) {
    if let Some(coordination) = coordination {
        if !coordination.approved {
            return (
                true,
                "coordination DAG approval is still pending".to_string(),
            );
        }
        let open_escalations = coordination.open_escalations();
        if !open_escalations.is_empty() {
            let titles = open_escalations
                .iter()
                .take(2)
                .map(|item| item.title.as_str())
                .collect::<Vec<_>>();
            let suffix = if open_escalations.len() > 2 {
                " ..."
            } else {
                ""
            };
            return (
                true,
                format!(
                    "open coordination escalations: {}{}",
                    titles.join(" | "),
                    suffix
                ),
            );
        }
        let (_, rework_reviews, escalated_reviews) = coordination.latest_review_counts();
        if rework_reviews > 0 || escalated_reviews > 0 {
            return (true, "coordination review follow-up required".to_string());
        }
    }
    if last_check_status.eq_ignore_ascii_case("blocked") {
        return (true, "handoff audit failed".to_string());
    }
    let blocked_workers = workers
        .iter()
        .filter(|worker| worker.launch_blocked)
        .count();
    if blocked_workers > 0 && blocked_workers == workers.len() {
        return (
            true,
            "all workers are blocked by coordination policy".to_string(),
        );
    }
    if workers
        .iter()
        .any(|worker| pending_action_needs_attention(worker))
    {
        return (true, "coordination follow-up required".to_string());
    }
    if workers.iter().any(|worker| !worker.git_clean) {
        return (true, "dirty worktree detected".to_string());
    }
    if workers
        .iter()
        .any(|worker| worker.handoff_status == "blocked" || worker.handoff_status == "pending")
    {
        return (true, "worker handoff still needs review".to_string());
    }
    if workers.iter().any(|worker| !worker.issues.is_empty()) {
        return (true, "worker attention required".to_string());
    }
    if last_check_status.eq_ignore_ascii_case("warning") {
        return (false, "handoff audit warning only".to_string());
    }
    (
        false,
        "all monitored workers are clean and ready".to_string(),
    )
}

fn pending_action_needs_attention(worker: &WorkerSnapshot) -> bool {
    let trimmed = worker.pending_action.trim();
    !trimmed.is_empty()
        && !trimmed.eq_ignore_ascii_case("manual-activation")
        && !trimmed.starts_with("scoped:")
        && !trimmed.starts_with("contract-freeze owner:")
        && !worker.launch_blocked
}

fn pending_action_blocks_start(pending_action: &str) -> bool {
    let lowered = pending_action.trim().to_ascii_lowercase();
    lowered.contains("awaiting dag approval")
        || lowered.contains("awaiting plan approval")
        || lowered.contains("waiting on peter")
        || lowered.starts_with("frozen:")
}

fn control_supports_command(control: &ControlSnapshot, command: &str) -> bool {
    let normalized = command.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return true;
    }
    let requested = normalized
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim();
    control.supported_commands.iter().any(|entry| {
        entry
            .trim()
            .to_ascii_lowercase()
            .split_whitespace()
            .next()
            .unwrap_or_default()
            == requested
    })
}

fn current_epoch() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs_f64())
        .unwrap_or_default()
}

fn now_compact() -> String {
    chrono::Local::now().format("%Y%m%d-%H%M%S").to_string()
}

fn process_exists(pid: u32) -> bool {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    system.process(Pid::from_u32(pid)).is_some()
}

fn parse_density(raw: &str) -> FeedDensity {
    match raw.trim().to_ascii_lowercase().as_str() {
        "realtime" | "high" | "high-density" => FeedDensity::Realtime,
        _ => FeedDensity::Standard,
    }
}

fn density_label(density: FeedDensity) -> &'static str {
    match density {
        FeedDensity::Standard => "standard",
        FeedDensity::Realtime => "realtime",
    }
}

fn parse_stream_scope(raw: &str, selected_worker: &str) -> StreamScope {
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() || normalized == "all" {
        StreamScope::All
    } else if normalized == "selected" {
        StreamScope::Selected
    } else if normalized == "commander" {
        StreamScope::Commander
    } else if let Some(worker) = normalized.strip_prefix("worker:") {
        StreamScope::Worker(worker.trim().to_string())
    } else if let Some(worker) = normalized.strip_prefix("worker ") {
        StreamScope::Worker(worker.trim().to_string())
    } else if !selected_worker.trim().is_empty() && normalized == "worker" {
        StreamScope::Worker(selected_worker.to_string())
    } else {
        StreamScope::All
    }
}

fn stream_scope_label(scope: &StreamScope) -> String {
    match scope {
        StreamScope::All => "all".to_string(),
        StreamScope::Selected => "selected".to_string(),
        StreamScope::Commander => "commander".to_string(),
        StreamScope::Worker(name) => format!("worker:{name}"),
    }
}

fn default_selected_worker(config: &CommanderConfig, phase_name: &str) -> String {
    config
        .phase(phase_name)
        .ok()
        .and_then(|phase| phase.workers.first().map(|worker| worker.name.clone()))
        .unwrap_or_default()
}

fn load_activity_buffer(
    runtime: &RuntimeLayout,
    limit: usize,
) -> Result<(Vec<ActivityEntry>, u64)> {
    let entries = runtime.read_json_lines::<ActivityEntry>(&runtime.event_stream_file)?;
    let mut compacted = Vec::new();
    let mut next_seq = 1;
    for mut entry in entries {
        normalize_activity_entry(&mut entry, &mut next_seq);
        push_activity_entry(&mut compacted, entry, limit);
    }

    Ok((compacted, next_seq))
}

fn append_activity(
    state: &Arc<Mutex<SupervisorState>>,
    runtime: &RuntimeLayout,
    buffer_limit: usize,
    source: &str,
    level: &str,
    channel: &str,
    worker_name: &str,
    message: String,
    tags: Vec<String>,
) -> Result<()> {
    let dense_message = densify_message(&message);
    let entry = {
        let mut guard = state
            .lock()
            .map_err(|_| anyhow!("supervisor state poisoned"))?;
        let mut entry = ActivityEntry {
            seq: guard.next_activity_seq,
            repeat_count: 1,
            timestamp: now_string(),
            source: source.to_string(),
            level: level.to_string(),
            channel: channel.to_string(),
            worker_name: worker_name.to_string(),
            message: message.clone(),
            dense_message,
            full_message: message,
            tags,
        };
        normalize_activity_entry(&mut entry, &mut guard.next_activity_seq);
        push_activity_entry(&mut guard.activity, entry.clone(), buffer_limit);
        guard.status.recent_activity = guard.activity.clone();
        entry
    };

    runtime.append_json_line(&runtime.event_stream_file, &entry)?;
    Ok(())
}

fn normalize_activity_entry(entry: &mut ActivityEntry, next_seq: &mut u64) {
    if entry.seq == 0 {
        entry.seq = *next_seq;
    }
    *next_seq = (*next_seq).max(entry.seq.saturating_add(1));
    if entry.repeat_count == 0 {
        entry.repeat_count = 1;
    }
    if entry.timestamp.trim().is_empty() {
        entry.timestamp = now_string();
    }
    if entry.source.trim().is_empty() {
        entry.source = "commander".to_string();
    }
    if entry.level.trim().is_empty() {
        entry.level = "info".to_string();
    }
    if entry.channel.trim().is_empty() {
        entry.channel = "status".to_string();
    }
    if entry.message.trim().is_empty() && !entry.full_message.trim().is_empty() {
        entry.message = entry.full_message.clone();
    }
    if entry.message.trim().is_empty() {
        entry.message = "no message".to_string();
    }
    if entry.dense_message.trim().is_empty() {
        entry.dense_message = densify_message(&entry.message);
    }
    if entry.full_message.trim().is_empty() {
        entry.full_message = entry.message.clone();
    }
}

fn push_activity_entry(buffer: &mut Vec<ActivityEntry>, entry: ActivityEntry, limit: usize) {
    if let Some(last) = buffer.last_mut() {
        if activity_entries_mergeable(last, &entry) {
            merge_activity_entry(last, &entry);
        } else {
            buffer.push(entry);
        }
    } else {
        buffer.push(entry);
    }

    if buffer.len() > limit {
        let drain = buffer.len() - limit;
        buffer.drain(0..drain);
    }
}

fn activity_entries_mergeable(left: &ActivityEntry, right: &ActivityEntry) -> bool {
    left.source == right.source
        && left.level == right.level
        && left.channel == right.channel
        && left.worker_name == right.worker_name
        && left.message == right.message
        && left.dense_message == right.dense_message
        && left.full_message == right.full_message
        && left.tags == right.tags
}

fn merge_activity_entry(target: &mut ActivityEntry, incoming: &ActivityEntry) {
    target.seq = incoming.seq;
    target.timestamp = incoming.timestamp.clone();
    target.repeat_count = target
        .repeat_count
        .saturating_add(incoming.repeat_count.max(1));
}

fn format_coordination_result(label: &str, elapsed: Duration, output: &str) -> String {
    let elapsed_seconds = elapsed.as_secs_f32();
    if output.trim().is_empty() {
        format!("{label} completed in {elapsed_seconds:.1}s")
    } else {
        format!("{label} completed in {elapsed_seconds:.1}s\n{output}")
    }
}

fn densify_message(message: &str) -> String {
    let compact = message.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.len() > 180 {
        format!("{}...", &compact[..177])
    } else {
        compact
    }
}

fn spawn_activity_reader<R>(
    state: Arc<Mutex<SupervisorState>>,
    runtime: RuntimeLayout,
    buffer_limit: usize,
    worker_name: String,
    channel: String,
    level: String,
    reader: R,
) where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let buffered = BufReader::new(reader);
        for line in buffered.lines() {
            match line {
                Ok(line) => {
                    let line = line.trim_end().to_string();
                    if line.is_empty() {
                        continue;
                    }
                    if let Some(progress) = parse_worker_progress(&line, &worker_name) {
                        let mut tags = progress.tags;
                        if !tags.iter().any(|tag| tag.eq_ignore_ascii_case("worker")) {
                            tags.push("worker".to_string());
                        }
                        if !tags
                            .iter()
                            .any(|tag| tag.eq_ignore_ascii_case(&worker_name))
                        {
                            tags.push(worker_name.clone());
                        }
                        if !tags
                            .iter()
                            .any(|tag| tag.eq_ignore_ascii_case(&progress.channel))
                        {
                            tags.push(progress.channel.clone());
                        }
                        let _ = append_activity(
                            &state,
                            &runtime,
                            buffer_limit,
                            &progress.source,
                            &progress.level,
                            &progress.channel,
                            &progress.worker_name,
                            progress.message,
                            tags,
                        );
                        continue;
                    }
                    let tags = vec!["worker".to_string(), worker_name.clone(), channel.clone()];
                    if channel.eq_ignore_ascii_case("stdout")
                        && !is_worker_bridge_stdout(&line, &worker_name)
                    {
                        let _ = update_worker_activity_from_stream(&runtime, &worker_name, &line);
                    }
                    let _ = append_activity(
                        &state,
                        &runtime,
                        buffer_limit,
                        &worker_name,
                        &level,
                        &channel,
                        &worker_name,
                        line,
                        tags,
                    );
                }
                Err(error) => {
                    let _ = append_activity(
                        &state,
                        &runtime,
                        buffer_limit,
                        "commander",
                        "error",
                        "stderr",
                        &worker_name,
                        format!("failed to read {channel} for {worker_name}: {error}"),
                        vec![
                            "worker".to_string(),
                            worker_name.clone(),
                            "stderr".to_string(),
                        ],
                    );
                    break;
                }
            }
        }
    });
}

fn update_worker_activity_from_stream(
    runtime: &RuntimeLayout,
    worker_name: &str,
    activity: &str,
) -> Result<()> {
    let activity = densify_message(activity.trim());
    if activity.is_empty() {
        return Ok(());
    }

    let path = runtime.worker_thread_state_file(worker_name);
    let mut thread_state = runtime
        .read_json::<WorkerThreadState>(&path)?
        .unwrap_or_else(|| WorkerThreadState {
            worker_name: worker_name.to_string(),
            phase: String::new(),
            status: "running".to_string(),
            pid: None,
            last_started_at: None,
            last_finished_at: None,
            last_exit_code: None,
            last_summary: String::new(),
            last_error: String::new(),
            current_activity: String::new(),
            pending_action: String::new(),
            launch_blocked: false,
            execution_scope: String::new(),
        });

    if thread_state.worker_name.trim().is_empty() {
        thread_state.worker_name = worker_name.to_string();
    }

    let status = thread_state.status.trim();
    if !status.is_empty() && !status.eq_ignore_ascii_case("running") {
        return Ok(());
    }
    if status.is_empty() {
        thread_state.status = "running".to_string();
    }
    if thread_state.current_activity == activity {
        return Ok(());
    }

    thread_state.current_activity = activity;
    runtime.write_json(&path, &thread_state)
}

fn is_worker_bridge_stdout(line: &str, worker_name: &str) -> bool {
    let line = line.trim();
    !worker_name.trim().is_empty()
        && line.starts_with("20")
        && line.contains(&format!("[{}]", worker_name.trim()))
}

fn parse_worker_progress(line: &str, default_worker_name: &str) -> Option<WorkerProgressPayload> {
    let payload = line.strip_prefix(WORKER_PROGRESS_PREFIX)?.trim();
    let mut parsed = serde_json::from_str::<WorkerProgressPayload>(payload).ok()?;
    if parsed.worker_name.trim().is_empty() {
        parsed.worker_name = default_worker_name.to_string();
    }
    if parsed.source.trim().is_empty() {
        parsed.source = parsed.worker_name.clone();
    }
    if parsed.level.trim().is_empty() {
        parsed.level = "info".to_string();
    }
    if parsed.channel.trim().is_empty() {
        parsed.channel = "progress".to_string();
    }
    if parsed.message.trim().is_empty() {
        parsed.message = parsed.current_activity.clone();
    }
    if parsed.message.trim().is_empty() {
        parsed.message = "progress update".to_string();
    }
    Some(parsed)
}

fn command_matches_gate(gates: &[String], command: &str) -> bool {
    let Some(head) = command.split_whitespace().next() else {
        return false;
    };
    gates
        .iter()
        .any(|gate| gate.trim().eq_ignore_ascii_case(head))
}

fn canonicalize_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_runtime_root(label: &str) -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "lgc-supervisor-{label}-{}-{suffix}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn push_activity_entry_merges_consecutive_duplicates() {
        let mut buffer = Vec::new();
        let first = ActivityEntry {
            seq: 1,
            repeat_count: 1,
            timestamp: "2026-03-17 10:00:00".to_string(),
            source: "patrol".to_string(),
            level: "info".to_string(),
            channel: "audit".to_string(),
            worker_name: String::new(),
            message: "handoff audit PASS (exit 0)".to_string(),
            dense_message: "handoff audit PASS (exit 0)".to_string(),
            full_message: "handoff audit PASS (exit 0)".to_string(),
            tags: vec!["patrol".to_string(), "audit".to_string()],
        };
        let second = ActivityEntry {
            seq: 2,
            repeat_count: 1,
            timestamp: "2026-03-17 10:00:15".to_string(),
            ..first.clone()
        };

        push_activity_entry(&mut buffer, first, 100);
        push_activity_entry(&mut buffer, second, 100);

        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer[0].repeat_count, 2);
        assert_eq!(buffer[0].seq, 2);
        assert_eq!(buffer[0].timestamp, "2026-03-17 10:00:15");
    }

    #[test]
    fn push_activity_entry_keeps_distinct_messages_separate() {
        let mut buffer = Vec::new();
        push_activity_entry(
            &mut buffer,
            ActivityEntry {
                seq: 1,
                repeat_count: 1,
                timestamp: "2026-03-17 10:00:00".to_string(),
                source: "patrol".to_string(),
                level: "info".to_string(),
                channel: "audit".to_string(),
                worker_name: String::new(),
                message: "handoff audit PASS (exit 0)".to_string(),
                dense_message: "handoff audit PASS (exit 0)".to_string(),
                full_message: "handoff audit PASS (exit 0)".to_string(),
                tags: vec!["patrol".to_string(), "audit".to_string()],
            },
            100,
        );
        push_activity_entry(
            &mut buffer,
            ActivityEntry {
                seq: 2,
                repeat_count: 1,
                timestamp: "2026-03-17 10:00:15".to_string(),
                source: "patrol".to_string(),
                level: "warning".to_string(),
                channel: "audit".to_string(),
                worker_name: String::new(),
                message: "audit warning: cached origin ref used".to_string(),
                dense_message: "audit warning: cached origin ref used".to_string(),
                full_message: "audit warning: cached origin ref used".to_string(),
                tags: vec!["patrol".to_string(), "audit".to_string()],
            },
            100,
        );

        assert_eq!(buffer.len(), 2);
        assert_eq!(buffer[0].repeat_count, 1);
        assert_eq!(buffer[1].repeat_count, 1);
    }

    #[test]
    fn format_coordination_result_includes_elapsed_time() {
        let rendered = format_coordination_result(
            "coordination report",
            Duration::from_millis(1750),
            "report updated",
        );
        assert!(rendered.contains("coordination report completed in 1.8s"));
        assert!(rendered.contains("report updated"));
    }

    #[test]
    fn stdout_updates_running_worker_current_activity() {
        let root = temp_runtime_root("stdout-activity");
        let runtime = RuntimeLayout::new(&root);
        runtime.ensure_dirs().unwrap();
        let path = runtime.worker_thread_state_file("dummy");
        runtime
            .write_json(
                &path,
                &WorkerThreadState {
                    worker_name: "dummy".to_string(),
                    phase: "bootstrap".to_string(),
                    status: "running".to_string(),
                    pid: Some(1234),
                    last_started_at: None,
                    last_finished_at: None,
                    last_exit_code: None,
                    last_summary: String::new(),
                    last_error: String::new(),
                    current_activity: "starting worker round".to_string(),
                    pending_action: String::new(),
                    launch_blocked: false,
                    execution_scope: String::new(),
                },
            )
            .unwrap();

        update_worker_activity_from_stream(&runtime, "dummy", "cargo test --quiet").unwrap();

        let updated = runtime
            .read_json::<WorkerThreadState>(&path)
            .unwrap()
            .unwrap();
        assert_eq!(updated.status, "running");
        assert_eq!(updated.current_activity, "cargo test --quiet");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn stdout_does_not_override_terminal_worker_state() {
        let root = temp_runtime_root("stdout-terminal-guard");
        let runtime = RuntimeLayout::new(&root);
        runtime.ensure_dirs().unwrap();
        let path = runtime.worker_thread_state_file("dummy");
        runtime
            .write_json(
                &path,
                &WorkerThreadState {
                    worker_name: "dummy".to_string(),
                    phase: "bootstrap".to_string(),
                    status: "finished".to_string(),
                    pid: None,
                    last_started_at: None,
                    last_finished_at: Some("2026-03-17 14:10:00".to_string()),
                    last_exit_code: Some(0),
                    last_summary: "finished".to_string(),
                    last_error: String::new(),
                    current_activity: "finished".to_string(),
                    pending_action: String::new(),
                    launch_blocked: false,
                    execution_scope: String::new(),
                },
            )
            .unwrap();

        update_worker_activity_from_stream(&runtime, "dummy", "post-exit trailing stdout").unwrap();

        let updated = runtime
            .read_json::<WorkerThreadState>(&path)
            .unwrap()
            .unwrap();
        assert_eq!(updated.status, "finished");
        assert_eq!(updated.current_activity, "finished");

        let _ = fs::remove_dir_all(&root);
    }
}
