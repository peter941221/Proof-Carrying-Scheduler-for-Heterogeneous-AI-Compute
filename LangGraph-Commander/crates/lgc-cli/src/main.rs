use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow, bail};
use clap::{Parser, Subcommand};
use lgc_core::config::{CommanderConfig, read_framework_version};
use lgc_core::runtime::{ActivityEntry, RuntimeLayout, WorkerThreadState};
use lgc_supervisor::{
    SnapshotBundle, SupervisorOneShot, SupervisorSession, command_requires_live_panel,
    read_runtime_snapshot, running_control_state, submit_remote_command,
};

const LIVE_PANEL_REQUIRED_MESSAGE: &str =
    "Please run commander in the project root to open the live commander panel first.";
const STREAM_POLL_INTERVAL: Duration = Duration::from_millis(250);
const STREAM_FLUSH_DELAY: Duration = Duration::from_millis(350);

#[derive(Clone, Debug, Default)]
struct WorkerView {
    name: String,
    status: String,
    current_activity: String,
    launch_blocked: bool,
    pending_action: String,
}

#[derive(Clone, Debug, Default)]
struct StreamWatchOptions {
    command_name: String,
    target_workers: Vec<String>,
    show_patrol: bool,
}

#[derive(Parser)]
#[command(name = "lgc", about = "LangGraph-Commander V1.2.0")]
struct Cli {
    #[arg(long, default_value = "commander.toml")]
    config: PathBuf,
    #[arg(long)]
    require_running: bool,
    #[arg(long)]
    stream: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Tui,
    Open,
    Status,
    Brief,
    Check,
    Intake,
    Approve,
    Review { target: Option<String> },
    Report,
    Refresh,
    Ping { text: Vec<String> },
    Start { target: String },
    Stop { target: String },
    Patrol { mode: Option<String> },
    Phase { name: String },
    Command { text: Vec<String> },
    Version,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config_path = resolve_config_path(&cli.config)?;

    match cli.command.unwrap_or(Commands::Tui) {
        Commands::Tui => lgc_tui::run(&config_path),
        Commands::Open => {
            open_new_console(&config_path)?;
            println!("opened LangGraph-Commander in a new console");
            Ok(())
        }
        Commands::Status => {
            let snapshot = refresh_snapshot(&config_path)?;
            println!("{}", render_status(&snapshot));
            Ok(())
        }
        Commands::Brief => {
            let snapshot = refresh_snapshot(&config_path)?;
            println!(
                "{}",
                snapshot.status.render_brief(snapshot.coordination.as_ref())
            );
            Ok(())
        }
        Commands::Check => {
            println!(
                "{}",
                dispatch_command(&config_path, "check", cli.require_running, cli.stream)?
            );
            Ok(())
        }
        Commands::Intake => {
            println!(
                "{}",
                dispatch_command(&config_path, "intake", cli.require_running, cli.stream)?
            );
            Ok(())
        }
        Commands::Approve => {
            println!(
                "{}",
                dispatch_command(&config_path, "approve", cli.require_running, cli.stream)?
            );
            Ok(())
        }
        Commands::Review { target } => {
            let command = match target {
                Some(value) if !value.trim().is_empty() => format!("review {}", value.trim()),
                _ => "review".to_string(),
            };
            println!(
                "{}",
                dispatch_command(&config_path, &command, cli.require_running, cli.stream)?
            );
            Ok(())
        }
        Commands::Report => {
            println!(
                "{}",
                dispatch_command(&config_path, "report", cli.require_running, cli.stream)?
            );
            Ok(())
        }
        Commands::Refresh => {
            println!(
                "{}",
                dispatch_command(&config_path, "refresh", cli.require_running, cli.stream)?
            );
            Ok(())
        }
        Commands::Ping { text } => {
            let command = if text.is_empty() {
                "ping".to_string()
            } else {
                format!("ping {}", text.join(" "))
            };
            println!(
                "{}",
                dispatch_command(&config_path, &command, cli.require_running, cli.stream)?
            );
            Ok(())
        }
        Commands::Start { target } => {
            println!(
                "{}",
                dispatch_command(
                    &config_path,
                    &format!("start {target}"),
                    cli.require_running,
                    cli.stream,
                )?
            );
            Ok(())
        }
        Commands::Stop { target } => {
            println!(
                "{}",
                dispatch_command(
                    &config_path,
                    &format!("stop {target}"),
                    cli.require_running,
                    cli.stream,
                )?
            );
            Ok(())
        }
        Commands::Patrol { mode } => {
            let mode = mode.unwrap_or_else(|| "once".to_string());
            println!(
                "{}",
                dispatch_command(
                    &config_path,
                    &format!("patrol {mode}"),
                    cli.require_running,
                    cli.stream,
                )?
            );
            Ok(())
        }
        Commands::Phase { name } => {
            println!(
                "{}",
                dispatch_command(
                    &config_path,
                    &format!("phase {name}"),
                    cli.require_running,
                    cli.stream,
                )?
            );
            Ok(())
        }
        Commands::Command { text } => {
            if text.is_empty() {
                bail!("command text is required");
            }
            println!(
                "{}",
                dispatch_command(
                    &config_path,
                    &text.join(" "),
                    cli.require_running,
                    cli.stream
                )?
            );
            Ok(())
        }
        Commands::Version => {
            let workspace = workspace_root();
            let version = read_framework_version(&workspace)?;
            println!("LangGraph-Commander {version}");
            println!("workspace={}", workspace.display());
            Ok(())
        }
    }
}

fn refresh_snapshot(config_path: &Path) -> Result<SnapshotBundle> {
    let oneshot = SupervisorOneShot::new(config_path)?;
    let _ = oneshot.execute_command("refresh", "lgc-cli")?;
    read_runtime_snapshot(config_path)
}

fn dispatch_command(
    config_path: &Path,
    command: &str,
    require_running: bool,
    stream: bool,
) -> Result<String> {
    if stream {
        return dispatch_command_streaming(config_path, command, require_running);
    }

    let panel_required = command_requires_live_panel(config_path, command)?;
    match submit_remote_command(config_path, command, "lgc-cli", require_running) {
        Ok(ack) => {
            if ack.ok {
                let message = format!("sent `{command}` at {}", ack.processed_at);
                let targets = command_target_workers(config_path, command)?;
                if targets.is_empty() {
                    Ok(message)
                } else {
                    Ok(append_target_summary(&message, config_path, &targets)?)
                }
            } else {
                bail!(ack.error)
            }
        }
        Err(error) => {
            let message = error.to_string();
            if panel_required
                && (is_no_live_session_error(&message) || message.contains("repo mismatch"))
            {
                bail!(LIVE_PANEL_REQUIRED_MESSAGE);
            }
            if require_running || !is_no_live_session_error(&message) {
                return Err(error);
            }

            if command_target_workers(config_path, command)?.is_empty() {
                let oneshot = SupervisorOneShot::new(config_path)?;
                return Ok(oneshot.execute_command(command, "lgc-cli")?.message);
            }

            let session = SupervisorSession::start(config_path)?;
            let targets = command_target_workers(config_path, command)?;
            let message = session.execute_command(command, "lgc-cli")?.message;
            if !targets.is_empty() {
                wait_for_workers_to_settle(config_path, &targets)?;
                Ok(append_target_summary(&message, config_path, &targets)?)
            } else {
                Ok(message)
            }
        }
    }
}

fn dispatch_command_streaming(
    config_path: &Path,
    command: &str,
    require_running: bool,
) -> Result<String> {
    let config = CommanderConfig::load_from(config_path)?;
    let runtime = RuntimeLayout::new(config.runtime_dir());
    let panel_required = command_requires_live_panel(config_path, command)?;
    let targets = command_target_workers(config_path, command)?;
    let watch_options = build_stream_watch_options(command, &targets);
    let start_offset = current_event_stream_len(&runtime)?;
    let stop_flag = Arc::new(AtomicBool::new(false));
    let watcher = spawn_stream_watcher(
        config_path.to_path_buf(),
        start_offset,
        Arc::clone(&stop_flag),
        watch_options,
    );
    let running_control = running_control_state(&runtime)?;
    let mut local_session: Option<SupervisorSession> = None;
    if running_control.is_none() && !panel_required && !require_running {
        local_session = Some(SupervisorSession::start(config_path)?);
    }

    let result = match running_control {
        Some(_) => match submit_remote_command(config_path, command, "lgc-cli", require_running) {
            Ok(ack) => {
                if ack.ok {
                    Ok(format!("sent `{command}` at {}", ack.processed_at))
                } else {
                    Err(anyhow!(ack.error))
                }
            }
            Err(error) => {
                let message = error.to_string();
                if panel_required
                    && (is_no_live_session_error(&message) || message.contains("repo mismatch"))
                {
                    Err(anyhow!(LIVE_PANEL_REQUIRED_MESSAGE))
                } else {
                    Err(error)
                }
            }
        },
        None => {
            if panel_required {
                Err(anyhow!(LIVE_PANEL_REQUIRED_MESSAGE))
            } else if require_running {
                Err(anyhow!("no live LangGraph-Commander session is running"))
            } else {
                let session = local_session
                    .as_ref()
                    .ok_or_else(|| anyhow!("failed to keep temporary supervisor session alive"))?;
                Ok(session.execute_command(command, "lgc-cli")?.message)
            }
        }
    };

    let final_result = match result {
        Ok(message) => {
            if !targets.is_empty() {
                wait_for_workers_to_settle(config_path, &targets)?;
                Ok(append_target_summary(&message, config_path, &targets)?)
            } else {
                Ok(message)
            }
        }
        Err(error) => Err(error),
    };

    thread::sleep(STREAM_FLUSH_DELAY);
    stop_flag.store(true, Ordering::Relaxed);
    let _ = watcher.join();

    final_result
}

fn spawn_stream_watcher(
    config_path: PathBuf,
    initial_offset: u64,
    stop_flag: Arc<AtomicBool>,
    options: StreamWatchOptions,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let config = match CommanderConfig::load_from(&config_path) {
            Ok(config) => config,
            Err(_) => return,
        };
        let runtime = RuntimeLayout::new(config.runtime_dir());
        let mut offset = initial_offset;
        let mut last_fleet = render_fleet_line(&config_path, &options).unwrap_or_default();

        loop {
            if let Ok((entries, next_offset)) = read_new_activity_entries(&runtime, offset) {
                offset = next_offset;
                for entry in entries {
                    if should_render_activity(&entry, &options) {
                        println!("{}", format_stream_activity(&entry));
                    }
                }
            }

            if let Ok(fleet_line) = render_fleet_line(&config_path, &options) {
                if !fleet_line.is_empty() && fleet_line != last_fleet {
                    println!("{fleet_line}");
                    last_fleet = fleet_line;
                }
            }

            if stop_flag.load(Ordering::Relaxed) {
                break;
            }
            thread::sleep(STREAM_POLL_INTERVAL);
        }

        if let Ok((entries, _)) = read_new_activity_entries(&runtime, offset) {
            for entry in entries {
                if should_render_activity(&entry, &options) {
                    println!("{}", format_stream_activity(&entry));
                }
            }
        }
    })
}

fn current_event_stream_len(runtime: &RuntimeLayout) -> Result<u64> {
    Ok(fs_metadata_len(&runtime.event_stream_file).unwrap_or(0))
}

fn read_new_activity_entries(
    runtime: &RuntimeLayout,
    mut offset: u64,
) -> Result<(Vec<ActivityEntry>, u64)> {
    if !runtime.event_stream_file.exists() {
        return Ok((Vec::new(), 0));
    }

    let file_len = fs_metadata_len(&runtime.event_stream_file).unwrap_or(0);
    if file_len < offset {
        offset = 0;
    }

    let mut file = File::open(&runtime.event_stream_file)?;
    file.seek(SeekFrom::Start(offset))?;
    let mut reader = BufReader::new(file);
    let mut entries = Vec::new();
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(entry) = serde_json::from_str::<ActivityEntry>(trimmed) {
            entries.push(entry);
        }
    }

    let next_offset = reader.stream_position()?;
    Ok((entries, next_offset))
}

fn render_fleet_line(config_path: &Path, options: &StreamWatchOptions) -> Result<String> {
    let workers = load_worker_views(config_path)?
        .into_iter()
        .filter(|worker| should_render_worker_in_fleet(worker.name.as_str(), options))
        .collect::<Vec<_>>();
    if workers.is_empty() {
        return Ok(String::new());
    }

    let mut parts = Vec::new();
    for worker in workers {
        parts.push(format!("{}={}", worker.name, render_worker_state(&worker)));
    }
    Ok(format!("[fleet] {}", parts.join(" | ")))
}

fn load_worker_views(config_path: &Path) -> Result<Vec<WorkerView>> {
    let config = CommanderConfig::load_from(config_path)?;
    let runtime = RuntimeLayout::new(config.runtime_dir());
    let snapshot = read_runtime_snapshot(config_path).unwrap_or_default();
    let phase_name = if !snapshot.status.phase.trim().is_empty()
        && config.phase(snapshot.status.phase.as_str()).is_ok()
    {
        snapshot.status.phase.clone()
    } else {
        config.active_phase_name().to_string()
    };
    let phase = config.phase(&phase_name)?;
    let mut views = Vec::new();

    for worker in &phase.workers {
        let base = snapshot
            .status
            .workers
            .iter()
            .find(|entry| entry.name == worker.name);
        let thread_state = runtime
            .read_json::<WorkerThreadState>(&runtime.worker_thread_state_file(&worker.name))?
            .unwrap_or_default();

        let status = if !thread_state.status.trim().is_empty() {
            thread_state.status.clone()
        } else if let Some(base) = base {
            base.status.clone()
        } else {
            "idle".to_string()
        };
        let pending_action = if !thread_state.pending_action.trim().is_empty() {
            thread_state.pending_action.clone()
        } else {
            base.map(|entry| entry.pending_action.clone())
                .unwrap_or_default()
        };
        let current_activity = if !thread_state.current_activity.trim().is_empty() {
            thread_state.current_activity.clone()
        } else if let Some(base) = base {
            if !base.current_activity.trim().is_empty() {
                base.current_activity.clone()
            } else if !base.pending_action.trim().is_empty() {
                base.pending_action.clone()
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        let launch_blocked = if thread_state.launch_blocked {
            true
        } else {
            base.map(|entry| entry.launch_blocked).unwrap_or(false)
        };

        views.push(WorkerView {
            name: worker.name.clone(),
            status,
            current_activity,
            launch_blocked,
            pending_action,
        });
    }

    Ok(views)
}

fn render_worker_state(worker: &WorkerView) -> String {
    if worker.launch_blocked {
        let reason = first_nonempty([
            worker.pending_action.as_str(),
            worker.current_activity.as_str(),
        ]);
        return match reason {
            Some(reason) => format!("blocked({})", truncate_inline(reason, 40)),
            None => "blocked".to_string(),
        };
    }

    if worker.status.eq_ignore_ascii_case("running") {
        return match first_nonempty([
            worker.current_activity.as_str(),
            worker.pending_action.as_str(),
        ]) {
            Some(reason) => format!("running({})", truncate_inline(reason, 40)),
            None => "running".to_string(),
        };
    }

    let status = worker.status.trim();
    if status.is_empty() {
        return "idle".to_string();
    }

    match first_nonempty([worker.current_activity.as_str()]) {
        Some(reason)
            if status.eq_ignore_ascii_case("failed")
                || status.eq_ignore_ascii_case("stopped")
                || status.eq_ignore_ascii_case("missing") =>
        {
            format!(
                "{}({})",
                status.to_ascii_lowercase(),
                truncate_inline(reason, 40)
            )
        }
        _ => status.to_ascii_lowercase(),
    }
}

fn wait_for_workers_to_settle(config_path: &Path, targets: &[String]) -> Result<()> {
    if targets.is_empty() {
        return Ok(());
    }

    let config = CommanderConfig::load_from(config_path)?;
    let (_, phase) = config.active_phase()?;
    let timeout_seconds = phase
        .workers
        .iter()
        .filter(|worker| targets.iter().any(|target| target == &worker.name))
        .map(|worker| worker.timeout_seconds)
        .max()
        .unwrap_or(config.runtime.command_timeout_seconds.max(30))
        .saturating_add(60);

    let started = Instant::now();
    loop {
        let views = load_worker_views(config_path)?;
        if targets
            .iter()
            .all(|target| worker_has_settled(target, &views))
        {
            return Ok(());
        }
        if started.elapsed() > Duration::from_secs(timeout_seconds) {
            bail!(
                "timed out waiting for worker completion after {}s: {}",
                timeout_seconds,
                render_target_summary(config_path, targets)?
            );
        }
        thread::sleep(STREAM_POLL_INTERVAL);
    }
}

fn worker_has_settled(target: &str, workers: &[WorkerView]) -> bool {
    let Some(worker) = workers.iter().find(|worker| worker.name == target) else {
        return false;
    };
    if worker.launch_blocked {
        return true;
    }
    let status = worker.status.trim();
    !status.is_empty() && !status.eq_ignore_ascii_case("running")
}

fn append_target_summary(base: &str, config_path: &Path, targets: &[String]) -> Result<String> {
    if targets.is_empty() {
        return Ok(base.to_string());
    }
    Ok(format!(
        "{base}\n{}",
        render_target_summary(config_path, targets)?
    ))
}

fn render_target_summary(config_path: &Path, targets: &[String]) -> Result<String> {
    let views = load_worker_views(config_path)?;
    let mut parts = Vec::new();
    for target in targets {
        if let Some(view) = views.iter().find(|worker| &worker.name == target) {
            parts.push(format!("{}={}", target, render_worker_state(view)));
        }
    }
    if parts.is_empty() {
        Ok("final: no tracked worker state found".to_string())
    } else {
        Ok(format!("final: {}", parts.join(" | ")))
    }
}

fn command_target_workers(config_path: &Path, command: &str) -> Result<Vec<String>> {
    let command = command.trim();
    let Some(target) = command.strip_prefix("start ") else {
        return Ok(Vec::new());
    };
    let config = CommanderConfig::load_from(config_path)?;
    if target.trim().eq_ignore_ascii_case("all") {
        let (_, phase) = config.active_phase()?;
        Ok(phase
            .workers
            .iter()
            .map(|worker| worker.name.clone())
            .collect())
    } else {
        Ok(vec![target.trim().to_string()])
    }
}

fn build_stream_watch_options(command: &str, targets: &[String]) -> StreamWatchOptions {
    let command_name = command
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let show_patrol = matches!(command_name.as_str(), "check" | "patrol");
    StreamWatchOptions {
        command_name,
        target_workers: targets.to_vec(),
        show_patrol,
    }
}

fn should_render_activity(entry: &ActivityEntry, options: &StreamWatchOptions) -> bool {
    if entry.channel.eq_ignore_ascii_case("command") && entry.source.eq_ignore_ascii_case("lgc-cli")
    {
        return false;
    }
    if is_bridge_telemetry_noise(entry) {
        return false;
    }
    if entry.source.eq_ignore_ascii_case("patrol") {
        return options.show_patrol;
    }
    if options.command_name == "start"
        && !options.target_workers.is_empty()
        && !activity_relates_to_targets(entry, &options.target_workers)
    {
        return entry.level.eq_ignore_ascii_case("error");
    }
    if entry.level.eq_ignore_ascii_case("error") || entry.level.eq_ignore_ascii_case("warning") {
        return true;
    }
    if entry.channel.eq_ignore_ascii_case("progress")
        || entry.channel.eq_ignore_ascii_case("coordination")
        || entry.channel.eq_ignore_ascii_case("worker")
        || entry.channel.eq_ignore_ascii_case("stdout")
        || entry.channel.eq_ignore_ascii_case("stderr")
    {
        return true;
    }
    if entry.channel.eq_ignore_ascii_case("command") {
        let lowered = entry.message.to_ascii_lowercase();
        return lowered.contains("started")
            || lowered.contains("finished")
            || lowered.contains("failed")
            || lowered.contains("blocked")
            || lowered.contains("stopped");
    }
    false
}

fn is_bridge_telemetry_noise(entry: &ActivityEntry) -> bool {
    if !entry.channel.eq_ignore_ascii_case("stdout") {
        return false;
    }
    let worker_name = entry.worker_name.trim();
    if worker_name.is_empty() {
        return false;
    }
    let message = entry.message.trim();
    if !message.starts_with("20") {
        return false;
    }
    message.contains(&format!("[{worker_name}]"))
}

fn activity_relates_to_targets(entry: &ActivityEntry, targets: &[String]) -> bool {
    if targets.is_empty() {
        return true;
    }
    if targets.iter().any(|target| {
        matches_worker(entry.worker_name.as_str(), target)
            || matches_worker(entry.source.as_str(), target)
    }) {
        return true;
    }
    [
        entry.message.as_str(),
        entry.dense_message.as_str(),
        entry.full_message.as_str(),
    ]
    .into_iter()
    .any(|message| message_mentions_targets(message, targets))
}

fn should_render_worker_in_fleet(worker_name: &str, options: &StreamWatchOptions) -> bool {
    if options.command_name != "start" || options.target_workers.is_empty() {
        return true;
    }
    options
        .target_workers
        .iter()
        .any(|target| matches_worker(worker_name, target))
}

fn matches_worker(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

fn message_mentions_targets(message: &str, targets: &[String]) -> bool {
    let lowered = message.to_ascii_lowercase();
    targets
        .iter()
        .any(|target| lowered.contains(&target.to_ascii_lowercase()))
}

fn format_stream_activity(entry: &ActivityEntry) -> String {
    let actor = if !entry.worker_name.trim().is_empty() {
        entry.worker_name.as_str()
    } else {
        entry.source.as_str()
    };
    let message = if !entry.dense_message.trim().is_empty() {
        entry.dense_message.as_str()
    } else {
        entry.message.as_str()
    };
    let repeat_suffix = if entry.repeat_count > 1 {
        format!(" (x{})", entry.repeat_count)
    } else {
        String::new()
    };
    format!(
        "[{}] {} | {}{}",
        timestamp_suffix(&entry.timestamp),
        actor,
        truncate_inline(message, 140),
        repeat_suffix
    )
}

fn timestamp_suffix(raw: &str) -> String {
    raw.split_whitespace().last().unwrap_or(raw).to_string()
}

fn truncate_inline(text: &str, width: usize) -> String {
    let chars = text.chars().collect::<Vec<_>>();
    if chars.len() <= width {
        return text.to_string();
    }
    let keep = width.saturating_sub(1);
    chars[..keep].iter().collect::<String>() + "…"
}

fn first_nonempty<'a>(values: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    values
        .into_iter()
        .map(str::trim)
        .find(|value| !value.is_empty())
}

fn is_no_live_session_error(message: &str) -> bool {
    message.contains("no live LangGraph-Commander session is running")
}

fn fs_metadata_len(path: &Path) -> Option<u64> {
    path.metadata().ok().map(|metadata| metadata.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn sample_entry(
        source: &str,
        level: &str,
        channel: &str,
        worker_name: &str,
        message: &str,
    ) -> ActivityEntry {
        ActivityEntry {
            seq: 1,
            repeat_count: 1,
            timestamp: "2026-03-17 13:00:00".to_string(),
            source: source.to_string(),
            level: level.to_string(),
            channel: channel.to_string(),
            worker_name: worker_name.to_string(),
            message: message.to_string(),
            dense_message: message.to_string(),
            full_message: message.to_string(),
            tags: Vec::new(),
        }
    }

    fn temp_test_root(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("lgc-cli-{label}-{}-{suffix}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn write_test_config(root: &Path) -> PathBuf {
        let config_path = root.join("commander.test.toml");
        let content = r#"
framework_version = "1.2.0"

[project]
name = "cli-stream-test"
repo_root = "."
worktree_root = "."
default_phase = "bootstrap"

[runtime]
dir = "runtime"
poll_interval_seconds = 1
command_timeout_seconds = 30

[ui]
style = "hardcore-ascii"
require_live_panel_for = []
default_stream_scope = "all"
default_density = "standard"
density_persistence = "session"
event_buffer_size = 200
worker_list_page_size = 20

[provider]
kind = "openai-compatible"
model_provider = ""
config_path = ""
default_model = ""
api_mode = "responses"

[phases.bootstrap]
description = "cli stream test"
default_start_set = ["state", "api-spec"]

[[phases.bootstrap.workers]]
name = "state"
branch = "main"
worktree = "."
auto_push = false
max_attempts = 1
timeout_seconds = 30

[[phases.bootstrap.workers]]
name = "api-spec"
branch = "main"
worktree = "."
auto_push = false
max_attempts = 1
timeout_seconds = 30
"#;
        fs::write(&config_path, content.trim_start()).unwrap();
        config_path
    }

    fn write_worker_state(
        runtime: &RuntimeLayout,
        worker_name: &str,
        status: &str,
        activity: &str,
    ) {
        runtime
            .write_json(
                &runtime.worker_thread_state_file(worker_name),
                &WorkerThreadState {
                    worker_name: worker_name.to_string(),
                    phase: "bootstrap".to_string(),
                    status: status.to_string(),
                    pid: Some(1000),
                    last_started_at: None,
                    last_finished_at: None,
                    last_exit_code: None,
                    last_summary: String::new(),
                    last_error: String::new(),
                    current_activity: activity.to_string(),
                    pending_action: String::new(),
                    launch_blocked: false,
                    execution_scope: String::new(),
                },
            )
            .unwrap();
    }

    #[test]
    fn targeted_start_hides_patrol_noise() {
        let options = build_stream_watch_options("start state", &["state".to_string()]);
        let entry = sample_entry(
            "patrol",
            "error",
            "audit",
            "",
            "handoff audit FAIL (exit 2)",
        );
        assert!(!should_render_activity(&entry, &options));
    }

    #[test]
    fn targeted_start_shows_target_progress() {
        let options = build_stream_watch_options("start state", &["state".to_string()]);
        let entry = sample_entry("state", "info", "progress", "state", "reading TASK.md");
        assert!(should_render_activity(&entry, &options));
    }

    #[test]
    fn targeted_start_shows_target_stdout() {
        let options = build_stream_watch_options("start state", &["state".to_string()]);
        let entry = sample_entry("state", "info", "stdout", "state", "cargo test");
        assert!(should_render_activity(&entry, &options));
    }

    #[test]
    fn targeted_start_hides_bridge_telemetry_noise() {
        let options = build_stream_watch_options("start state", &["state".to_string()]);
        let entry = sample_entry(
            "state",
            "info",
            "stdout",
            "state",
            "2026-03-17 14:15:01 [state] tool read_file requested_path='TASK.md'",
        );
        assert!(!should_render_activity(&entry, &options));
    }

    #[test]
    fn targeted_start_hides_other_worker_progress() {
        let options = build_stream_watch_options("start state", &["state".to_string()]);
        let entry = sample_entry(
            "api-spec",
            "info",
            "progress",
            "api-spec",
            "reading TASK.md",
        );
        assert!(!should_render_activity(&entry, &options));
    }

    #[test]
    fn patrol_command_keeps_patrol_output() {
        let options = build_stream_watch_options("patrol once", &[]);
        let entry = sample_entry("patrol", "info", "audit", "", "handoff audit PASS (exit 0)");
        assert!(should_render_activity(&entry, &options));
    }

    #[test]
    fn targeted_start_fleet_only_shows_target_worker() {
        let options = build_stream_watch_options("start state", &["state".to_string()]);
        assert!(should_render_worker_in_fleet("state", &options));
        assert!(!should_render_worker_in_fleet("api-spec", &options));
    }

    #[test]
    fn targeted_start_fleet_filter_reads_only_target_worker_from_runtime() {
        let root = temp_test_root("fleet-filter");
        let config_path = write_test_config(&root);
        let runtime = RuntimeLayout::new(root.join("runtime"));
        runtime.ensure_dirs().unwrap();
        write_worker_state(&runtime, "state", "running", "streaming state stdout");
        write_worker_state(&runtime, "api-spec", "running", "other worker noise");

        let options = build_stream_watch_options("start state", &["state".to_string()]);
        let fleet = render_fleet_line(&config_path, &options).unwrap();

        assert!(fleet.contains("state=running(streaming state stdout)"));
        assert!(!fleet.contains("api-spec"));

        let _ = fs::remove_dir_all(&root);
    }
}

fn render_status(snapshot: &SnapshotBundle) -> String {
    let mut lines = vec![
        format!("project={}", snapshot.status.project_name),
        format!("phase={}", snapshot.status.phase),
        format!("framework={}", snapshot.status.framework_version),
        format!(
            "activation_required={}",
            snapshot.status.activation_required
        ),
        format!("activation_reason={}", snapshot.status.activation_reason),
        format!("last_check_status={}", snapshot.status.last_check_status),
        format!("patrol_last_result={}", snapshot.patrol.last_result),
        format!("patrol_enabled={}", snapshot.patrol.enabled),
    ];
    if !snapshot.status.last_check_warning.trim().is_empty() {
        lines.push(format!(
            "last_check_warning={}",
            snapshot.status.last_check_warning
        ));
    }
    if !snapshot.patrol.last_warning.trim().is_empty() {
        lines.push(format!(
            "patrol_last_warning={}",
            snapshot.patrol.last_warning
        ));
    }
    if let Some(coordination) = &snapshot.coordination {
        let (approved_reviews, rework_reviews, escalated_reviews) =
            coordination.latest_review_counts();
        lines.extend([
            format!("coordination_plan={}", coordination.plan_id),
            format!("coordination_approved={}", coordination.approved),
            format!(
                "coordination_open_escalations={}",
                coordination.open_escalations().len()
            ),
            format!(
                "coordination_reviews=approve:{} rework:{} escalate:{}",
                approved_reviews, rework_reviews, escalated_reviews
            ),
        ]);
    }
    for worker in &snapshot.status.workers {
        lines.push(format!(
            "{} status={} handoff={} git_clean={} launch_blocked={} scope={} pending_action={} activity={}",
            worker.name,
            worker.status,
            worker.handoff_status,
            worker.git_clean,
            worker.launch_blocked,
            if worker.execution_scope.is_empty() {
                "full"
            } else {
                worker.execution_scope.as_str()
            },
            if worker.pending_action.is_empty() {
                "none"
            } else {
                worker.pending_action.as_str()
            },
            if worker.current_activity.is_empty() {
                "none"
            } else {
                worker.current_activity.as_str()
            }
        ));
    }
    lines.join("\n")
}

fn open_new_console(config_path: &Path) -> Result<()> {
    if !cfg!(windows) {
        bail!("`lgc open` is only implemented on Windows; use `lgc tui` here");
    }
    let config = CommanderConfig::load_from(config_path)?;
    let runtime = lgc_core::runtime::RuntimeLayout::new(config.runtime_dir());
    let exe = std::env::current_exe()?;
    let working_dir = config_path
        .parent()
        .ok_or_else(|| anyhow!("config path has no parent"))?;
    let script = format!(
        "Start-Process -FilePath 'powershell' -ArgumentList @('-NoExit','-Command',\"& {} --config {} tui\") -WorkingDirectory {}",
        ps_quote(exe.as_path()),
        ps_quote(config_path),
        ps_quote(working_dir)
    );
    let status = Command::new("powershell")
        .args(["-NoLogo", "-NoProfile", "-Command", &script])
        .status()?;
    if !status.success() {
        bail!("failed to launch a new LangGraph-Commander console");
    }
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(20) {
        if running_control_state(&runtime)?.is_some() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(250));
    }
    bail!("failed to detect a live commander heartbeat after opening the console")
}

fn ps_quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "''"))
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

fn resolve_config_path(candidate: &Path) -> Result<PathBuf> {
    if candidate.exists() {
        return Ok(candidate
            .canonicalize()
            .unwrap_or_else(|_| candidate.to_path_buf()));
    }

    let file_name = candidate
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("commander.toml");
    let mut search_roots = Vec::new();
    search_roots.push(std::env::current_dir()?);
    search_roots.push(workspace_root());

    for root in search_roots {
        for ancestor in root.ancestors() {
            let probe = ancestor.join(file_name);
            if probe.exists() {
                return Ok(probe.canonicalize().unwrap_or(probe));
            }
        }
    }

    bail!("failed to locate {}", candidate.display())
}
