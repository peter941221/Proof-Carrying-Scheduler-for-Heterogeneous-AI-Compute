use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow, bail};
use clap::{Parser, Subcommand};
use lgc_core::config::{CommanderConfig, read_framework_version};
use lgc_supervisor::{
    SnapshotBundle, SupervisorOneShot, command_requires_live_panel, read_runtime_snapshot,
    running_control_state, submit_remote_command,
};

const LIVE_PANEL_REQUIRED_MESSAGE: &str =
    "Please run commander in the project root to open the live commander panel first.";

#[derive(Parser)]
#[command(name = "lgc", about = "LangGraph-Commander V1.2.0")]
struct Cli {
    #[arg(long, default_value = "commander.toml")]
    config: PathBuf,
    #[arg(long)]
    require_running: bool,
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
            println!("{}", snapshot.status.render_brief(snapshot.coordination.as_ref()));
            Ok(())
        }
        Commands::Check => {
            println!(
                "{}",
                dispatch_command(&config_path, "check", cli.require_running)?
            );
            Ok(())
        }
        Commands::Intake => {
            println!(
                "{}",
                dispatch_command(&config_path, "intake", cli.require_running)?
            );
            Ok(())
        }
        Commands::Approve => {
            println!(
                "{}",
                dispatch_command(&config_path, "approve", cli.require_running)?
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
                dispatch_command(&config_path, &command, cli.require_running)?
            );
            Ok(())
        }
        Commands::Report => {
            println!(
                "{}",
                dispatch_command(&config_path, "report", cli.require_running)?
            );
            Ok(())
        }
        Commands::Refresh => {
            println!(
                "{}",
                dispatch_command(&config_path, "refresh", cli.require_running)?
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
                dispatch_command(&config_path, &command, cli.require_running)?
            );
            Ok(())
        }
        Commands::Start { target } => {
            println!(
                "{}",
                dispatch_command(&config_path, &format!("start {target}"), true)?
            );
            Ok(())
        }
        Commands::Stop { target } => {
            println!(
                "{}",
                dispatch_command(&config_path, &format!("stop {target}"), cli.require_running)?
            );
            Ok(())
        }
        Commands::Patrol { mode } => {
            let mode = mode.unwrap_or_else(|| "once".to_string());
            println!(
                "{}",
                dispatch_command(&config_path, &format!("patrol {mode}"), cli.require_running)?
            );
            Ok(())
        }
        Commands::Phase { name } => {
            println!(
                "{}",
                dispatch_command(&config_path, &format!("phase {name}"), cli.require_running)?
            );
            Ok(())
        }
        Commands::Command { text } => {
            if text.is_empty() {
                bail!("command text is required");
            }
            println!(
                "{}",
                dispatch_command(&config_path, &text.join(" "), cli.require_running)?
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

fn dispatch_command(config_path: &Path, command: &str, require_running: bool) -> Result<String> {
    let panel_required = command_requires_live_panel(config_path, command)?;
    match submit_remote_command(config_path, command, "lgc-cli", require_running) {
        Ok(ack) => {
            if ack.ok {
                Ok(format!("sent `{command}` at {}", ack.processed_at))
            } else {
                bail!(ack.error)
            }
        }
        Err(error) => {
            let message = error.to_string();
            if panel_required
                && (message.contains("no live LangGraph-Commander session is running")
                    || message.contains("repo mismatch"))
            {
                bail!(LIVE_PANEL_REQUIRED_MESSAGE);
            }
            if require_running
                || !message.contains("no live LangGraph-Commander session is running")
            {
                return Err(error);
            }
            let oneshot = SupervisorOneShot::new(config_path)?;
            Ok(oneshot.execute_command(command, "lgc-cli")?.message)
        }
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
        lines.push(format!("patrol_last_warning={}", snapshot.patrol.last_warning));
    }
    if let Some(coordination) = &snapshot.coordination {
        let (approved_reviews, rework_reviews, escalated_reviews) =
            coordination.latest_review_counts();
        lines.extend([
            format!("coordination_plan={}", coordination.plan_id),
            format!("coordination_approved={}", coordination.approved),
            format!("coordination_open_escalations={}", coordination.open_escalations().len()),
            format!(
                "coordination_reviews=approve:{} rework:{} escalate:{}",
                approved_reviews, rework_reviews, escalated_reviews
            ),
        ]);
    }
    for worker in &snapshot.status.workers {
        lines.push(format!(
            "{} status={} handoff={} git_clean={} launch_blocked={} scope={} pending_action={}",
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
