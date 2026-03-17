use std::io::{self, Stdout};
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use lgc_core::runtime::{
    ActivityEntry, FeedDensity, StreamScope, WorkerSnapshot,
};
use lgc_supervisor::{SnapshotBundle, SupervisorSession};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, BorderType, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
};
use ratatui::{Frame, Terminal};

const FRAMEWORK_VERSION: &str = "1.2.0";
const INPUT_POLL_MILLIS: u64 = 16;
const SNAPSHOT_REFRESH_MILLIS: u64 = 650;
const ESC_QUIT_WINDOW: Duration = Duration::from_secs(2);
const COMMAND_LOG_LIMIT: usize = 180;
const HISTORY_LIMIT: usize = 64;
const FEED_SCROLL_STEP: usize = 6;
const WORKER_SCROLL_STEP: usize = 4;
const CONSOLE_SCROLL_STEP: usize = 4;

pub fn run(config_path: &Path) -> Result<()> {
    let mut session = SupervisorSession::start(config_path)?;
    let mut terminal = init_terminal()?;
    let mut app = AppState::default();

    let loop_result = run_loop(&mut terminal, &session, &mut app);
    let restore_result = restore_terminal(&mut terminal);
    let shutdown_result = session.shutdown();

    restore_result?;
    shutdown_result?;
    loop_result
}

struct AppState {
    command_input: String,
    input_cursor: usize,
    command_history: Vec<String>,
    history_cursor: Option<usize>,
    history_draft: Option<String>,
    console_lines: Vec<String>,
    console_scroll: usize,
    worker_filter: String,
    local_view: StreamScope,
    active_worker: Option<String>,
    workers_scroll: usize,
    feed_scroll: usize,
    feed_follow_override: Option<bool>,
    last_feedback: String,
    last_command: String,
    should_quit: bool,
    force_snapshot_refresh: bool,
    esc_quit_armed_at: Option<Instant>,
}

impl Default for AppState {
    fn default() -> Self {
        let mut state = Self {
            command_input: String::new(),
            input_cursor: 0,
            command_history: Vec::new(),
            history_cursor: None,
            history_draft: None,
            console_lines: Vec::new(),
            console_scroll: 0,
            worker_filter: String::new(),
            local_view: StreamScope::All,
            active_worker: None,
            workers_scroll: 0,
            feed_scroll: 0,
            feed_follow_override: Some(true),
            last_feedback: "Panel live. Right-bottom command line is the default control surface."
                .to_string(),
            last_command: "help local".to_string(),
            should_quit: false,
            force_snapshot_refresh: true,
            esc_quit_armed_at: None,
        };
        state.push_notice("Four-quadrant command deck online.");
        state.push_notice("Type `help local` for local panel commands.");
        state.push_notice(
            "Type supervisor commands directly: `start state`, `stop all`, `patrol once`.",
        );
        state.push_notice(
            "Type `show coordination`, `show escalations`, or `show reviews` for local orchestration summaries.",
        );
        state
    }
}

impl AppState {
    fn sync_from_snapshot(&mut self, snapshot: &SnapshotBundle) {
        if self
            .active_worker
            .as_ref()
            .is_none_or(|name| find_worker_name(snapshot, name).is_none())
        {
            self.active_worker = shared_selected_worker(snapshot).or_else(|| {
                snapshot
                    .status
                    .workers
                    .first()
                    .map(|worker| worker.name.clone())
            });
        }

        self.ensure_active_worker_visible(snapshot);
    }

    fn input_len(&self) -> usize {
        self.command_input.chars().count()
    }

    fn set_input(&mut self, value: String) {
        self.command_input = value;
        self.input_cursor = self.input_len();
    }

    fn clear_input(&mut self) {
        self.command_input.clear();
        self.input_cursor = 0;
        self.history_cursor = None;
        self.history_draft = None;
    }

    fn insert_char(&mut self, ch: char) {
        let mut chars = self.command_input.chars().collect::<Vec<_>>();
        chars.insert(self.input_cursor.min(chars.len()), ch);
        self.command_input = chars.into_iter().collect();
        self.input_cursor = self.input_cursor.saturating_add(1);
    }

    fn backspace(&mut self) {
        if self.input_cursor == 0 {
            return;
        }
        let mut chars = self.command_input.chars().collect::<Vec<_>>();
        let position = self.input_cursor.saturating_sub(1).min(chars.len());
        if position < chars.len() {
            chars.remove(position);
            self.command_input = chars.into_iter().collect();
            self.input_cursor = position;
        }
    }

    fn delete(&mut self) {
        let mut chars = self.command_input.chars().collect::<Vec<_>>();
        if self.input_cursor >= chars.len() {
            return;
        }
        chars.remove(self.input_cursor);
        self.command_input = chars.into_iter().collect();
    }

    fn move_cursor_left(&mut self) {
        self.input_cursor = self.input_cursor.saturating_sub(1);
    }

    fn move_cursor_right(&mut self) {
        self.input_cursor = (self.input_cursor + 1).min(self.input_len());
    }

    fn move_cursor_home(&mut self) {
        self.input_cursor = 0;
    }

    fn move_cursor_end(&mut self) {
        self.input_cursor = self.input_len();
    }

    fn push_history(&mut self, command: &str) {
        if self
            .command_history
            .last()
            .is_some_and(|last| last == command)
        {
            self.history_cursor = None;
            self.history_draft = None;
            return;
        }

        self.command_history.push(command.to_string());
        if self.command_history.len() > HISTORY_LIMIT {
            let overflow = self.command_history.len() - HISTORY_LIMIT;
            self.command_history.drain(0..overflow);
        }
        self.history_cursor = None;
        self.history_draft = None;
    }

    fn history_prev(&mut self) {
        if self.command_history.is_empty() {
            return;
        }

        if self.history_cursor.is_none() {
            self.history_draft = Some(self.command_input.clone());
        }

        let next = match self.history_cursor {
            Some(0) => 0,
            Some(index) => index.saturating_sub(1),
            None => self.command_history.len().saturating_sub(1),
        };
        self.history_cursor = Some(next);
        self.set_input(self.command_history[next].clone());
    }

    fn history_next(&mut self) {
        let Some(index) = self.history_cursor else {
            return;
        };

        if index + 1 >= self.command_history.len() {
            self.history_cursor = None;
            let draft = self.history_draft.take().unwrap_or_default();
            self.set_input(draft);
            return;
        }

        let next = index + 1;
        self.history_cursor = Some(next);
        self.set_input(self.command_history[next].clone());
    }

    fn push_console_lines(&mut self, prefix: &str, message: &str) {
        let normalized = if message.trim().is_empty() {
            vec![String::new()]
        } else {
            message
                .lines()
                .map(|line| format!("{prefix}{line}"))
                .collect::<Vec<_>>()
        };
        self.console_lines.extend(normalized);
        if self.console_lines.len() > COMMAND_LOG_LIMIT {
            let overflow = self.console_lines.len() - COMMAND_LOG_LIMIT;
            self.console_lines.drain(0..overflow);
        }
        self.console_scroll = self.console_lines.len().saturating_sub(1);
        if let Some(last_line) = message.lines().last() {
            self.last_feedback = last_line.to_string();
        }
    }

    fn push_notice(&mut self, message: &str) {
        self.push_console_lines("! ", message);
    }

    fn push_command_echo(&mut self, command: &str) {
        self.last_command = command.to_string();
        self.push_console_lines("> ", command);
    }

    fn push_result(&mut self, message: &str) {
        self.push_console_lines("< ", message);
    }

    fn worker_filter_label(&self) -> &str {
        if self.worker_filter.trim().is_empty() {
            "none"
        } else {
            self.worker_filter.as_str()
        }
    }

    fn local_view_label(&self) -> String {
        match &self.local_view {
            StreamScope::All => "all".to_string(),
            StreamScope::Selected => "selected".to_string(),
            StreamScope::Commander => "commander".to_string(),
            StreamScope::Worker(name) => format!("worker:{name}"),
        }
    }

    fn resolved_selected_worker<'a>(&'a self, snapshot: &'a SnapshotBundle) -> Option<&'a str> {
        snapshot
            .control
            .as_ref()
            .and_then(|control| {
                let selected = control.selected_worker.trim();
                (!selected.is_empty()).then_some(selected)
            })
            .or(self.active_worker.as_deref())
    }

    fn feed_follows_tail(&self, snapshot: &SnapshotBundle) -> bool {
        self.feed_follow_override.unwrap_or(
            snapshot
                .control
                .as_ref()
                .map(|control| control.follow_tail)
                .unwrap_or(true),
        )
    }

    fn visible_workers<'a>(&self, snapshot: &'a SnapshotBundle) -> Vec<&'a WorkerSnapshot> {
        snapshot
            .status
            .workers
            .iter()
            .filter(|worker| worker_matches_filter(worker, &self.worker_filter))
            .collect()
    }

    fn worker_names(&self, snapshot: &SnapshotBundle) -> Vec<String> {
        self.visible_workers(snapshot)
            .into_iter()
            .map(|worker| worker.name.clone())
            .collect()
    }

    fn ensure_active_worker_visible(&mut self, snapshot: &SnapshotBundle) {
        let visible = self.worker_names(snapshot);
        if visible.is_empty() {
            return;
        }

        if self
            .active_worker
            .as_ref()
            .is_none_or(|active| !visible.iter().any(|name| name.eq_ignore_ascii_case(active)))
        {
            self.active_worker = Some(visible[0].clone());
            if matches!(self.local_view, StreamScope::Worker(_)) {
                self.local_view = StreamScope::Worker(visible[0].clone());
            }
        }
    }

    fn set_active_worker(&mut self, snapshot: &SnapshotBundle, requested: &str) -> Option<String> {
        let matched = find_worker_name(snapshot, requested)?;
        self.active_worker = Some(matched.clone());
        if matches!(self.local_view, StreamScope::Worker(_)) {
            self.local_view = StreamScope::Worker(matched.clone());
        }
        Some(matched)
    }

    fn cycle_active_worker(&mut self, snapshot: &SnapshotBundle, step: isize) -> Option<String> {
        let visible = self.worker_names(snapshot);
        if visible.is_empty() {
            return None;
        }

        let current = self
            .active_worker
            .as_ref()
            .and_then(|active| {
                visible
                    .iter()
                    .position(|name| name.eq_ignore_ascii_case(active))
            })
            .unwrap_or(0);
        let next = wrapped_index(current, visible.len(), step);
        let selected = visible[next].clone();
        self.active_worker = Some(selected.clone());
        if matches!(self.local_view, StreamScope::Worker(_)) {
            self.local_view = StreamScope::Worker(selected.clone());
        }
        Some(selected)
    }

    fn arm_or_quit(&mut self, now: Instant) -> bool {
        let should_quit = self
            .esc_quit_armed_at
            .is_some_and(|armed_at| now.saturating_duration_since(armed_at) <= ESC_QUIT_WINDOW);
        if should_quit {
            self.esc_quit_armed_at = None;
            true
        } else {
            self.esc_quit_armed_at = Some(now);
            false
        }
    }
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    session: &SupervisorSession,
    app: &mut AppState,
) -> Result<()> {
    let mut snapshot = session.snapshot()?;
    app.sync_from_snapshot(&snapshot);
    let mut last_snapshot_refresh = Instant::now();

    while !app.should_quit {
        terminal.draw(|frame| draw_ui(frame, app, &snapshot))?;

        if app.force_snapshot_refresh
            || last_snapshot_refresh.elapsed() >= Duration::from_millis(SNAPSHOT_REFRESH_MILLIS)
        {
            snapshot = session.snapshot()?;
            app.sync_from_snapshot(&snapshot);
            app.force_snapshot_refresh = false;
            last_snapshot_refresh = Instant::now();
        }

        if !event::poll(Duration::from_millis(INPUT_POLL_MILLIS))? {
            continue;
        }

        loop {
            let Event::Key(key) = event::read()? else {
                if !event::poll(Duration::from_millis(0))? {
                    break;
                }
                continue;
            };

            if key.kind == KeyEventKind::Press {
                process_key_event(session, app, &snapshot, key)?;
            }

            if !event::poll(Duration::from_millis(0))? {
                break;
            }
        }
    }

    Ok(())
}

fn process_key_event(
    session: &SupervisorSession,
    app: &mut AppState,
    snapshot: &SnapshotBundle,
    key: KeyEvent,
) -> Result<()> {
    if key.modifiers.contains(KeyModifiers::CONTROL)
        && handle_control_shortcut(session, app, snapshot, &key)?
    {
        return Ok(());
    }

    if key.modifiers.contains(KeyModifiers::ALT) && handle_alt_shortcut(app, &key.code) {
        return Ok(());
    }

    match key.code {
        KeyCode::Enter => submit_command(session, app, snapshot)?,
        KeyCode::Backspace => app.backspace(),
        KeyCode::Delete => app.delete(),
        KeyCode::Left => app.move_cursor_left(),
        KeyCode::Right => app.move_cursor_right(),
        KeyCode::Home => app.move_cursor_home(),
        KeyCode::End => app.move_cursor_end(),
        KeyCode::Up => app.history_prev(),
        KeyCode::Down => app.history_next(),
        KeyCode::PageUp => {
            app.feed_follow_override = Some(false);
            app.feed_scroll = app.feed_scroll.saturating_sub(FEED_SCROLL_STEP);
            app.last_feedback =
                "Live feed scrolled upward. Use `follow on` or Ctrl+F to tail.".to_string();
        }
        KeyCode::PageDown => {
            app.feed_follow_override = Some(false);
            app.feed_scroll = app.feed_scroll.saturating_add(FEED_SCROLL_STEP);
        }
        KeyCode::Esc => handle_escape(app),
        KeyCode::F(1) => {
            app.push_notice(&render_local_help());
        }
        KeyCode::F(2) => {
            app.local_view = StreamScope::All;
            app.feed_follow_override = Some(true);
            app.push_notice("Local live view set to all streams.");
        }
        KeyCode::F(3) => {
            app.local_view = StreamScope::Selected;
            app.feed_follow_override = Some(true);
            app.push_notice("Local live view set to the shared selected worker.");
        }
        KeyCode::F(4) => {
            execute_supervisor_command(session, app, snapshot, "stop all")?;
        }
        KeyCode::Char(ch) => {
            app.insert_char(ch);
            app.esc_quit_armed_at = None;
        }
        _ => {}
    }

    Ok(())
}

fn handle_control_shortcut(
    session: &SupervisorSession,
    app: &mut AppState,
    snapshot: &SnapshotBundle,
    key: &KeyEvent,
) -> Result<bool> {
    let handled = match key.code {
        KeyCode::Char('c') | KeyCode::Char('C') => {
            app.should_quit = true;
            app.push_notice(
                "Ctrl+C detected. Closing panel; supervisor shutdown will stop tracked workers.",
            );
            true
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            execute_supervisor_command(session, app, snapshot, "refresh")?;
            true
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            execute_supervisor_command(session, app, snapshot, "stop all")?;
            true
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            execute_supervisor_command(session, app, snapshot, toggle_density_command(snapshot))?;
            true
        }
        KeyCode::Char('f') | KeyCode::Char('F') => {
            let shared_follow = snapshot
                .control
                .as_ref()
                .map(|control| control.follow_tail)
                .unwrap_or(true);
            app.feed_follow_override = Some(!shared_follow);
            execute_supervisor_command(session, app, snapshot, toggle_follow_command(snapshot))?;
            true
        }
        KeyCode::Char('n') | KeyCode::Char('N') => {
            if let Some(worker) = app.cycle_active_worker(snapshot, 1) {
                app.push_notice(&format!("Local active worker -> {worker}"));
            } else {
                app.push_notice("No visible worker to advance to.");
            }
            true
        }
        KeyCode::Char('p') | KeyCode::Char('P') => {
            if let Some(worker) = app.cycle_active_worker(snapshot, -1) {
                app.push_notice(&format!("Local active worker -> {worker}"));
            } else {
                app.push_notice("No visible worker to rewind to.");
            }
            true
        }
        KeyCode::Char('l') | KeyCode::Char('L') => {
            app.console_lines.clear();
            app.console_scroll = 0;
            app.push_notice("Local command log cleared.");
            true
        }
        _ => false,
    };

    Ok(handled)
}

fn handle_alt_shortcut(app: &mut AppState, code: &KeyCode) -> bool {
    match code {
        KeyCode::PageUp => {
            app.workers_scroll = app.workers_scroll.saturating_sub(WORKER_SCROLL_STEP);
            true
        }
        KeyCode::PageDown => {
            app.workers_scroll = app.workers_scroll.saturating_add(WORKER_SCROLL_STEP);
            true
        }
        KeyCode::Up => {
            app.console_scroll = app.console_scroll.saturating_sub(CONSOLE_SCROLL_STEP);
            true
        }
        KeyCode::Down => {
            app.console_scroll = app.console_scroll.saturating_add(CONSOLE_SCROLL_STEP);
            true
        }
        _ => false,
    }
}

fn handle_escape(app: &mut AppState) {
    if !app.command_input.is_empty() {
        app.clear_input();
        app.last_feedback = "Input cleared. Press Esc again within 2 seconds to quit.".to_string();
        app.esc_quit_armed_at = None;
        return;
    }

    let now = Instant::now();
    if app.arm_or_quit(now) {
        app.should_quit = true;
        app.push_notice(
            "Esc Esc detected. Closing panel; supervisor shutdown will stop tracked workers.",
        );
    } else {
        app.push_notice("Esc armed. Press Esc again within 2 seconds to quit.");
    }
}

fn submit_command(
    session: &SupervisorSession,
    app: &mut AppState,
    snapshot: &SnapshotBundle,
) -> Result<()> {
    let raw = app.command_input.trim().to_string();
    if raw.is_empty() {
        return Ok(());
    }

    let command = raw.trim_start_matches(':').trim().to_string();
    if command.is_empty() {
        app.clear_input();
        return Ok(());
    }

    app.push_history(&command);
    app.push_command_echo(&command);
    app.clear_input();
    app.esc_quit_armed_at = None;

    if let Some(message) = execute_local_command(app, snapshot, &command) {
        app.push_result(&message);
        return Ok(());
    }

    execute_supervisor_command(session, app, snapshot, &command)
}

fn execute_local_command(
    app: &mut AppState,
    snapshot: &SnapshotBundle,
    command: &str,
) -> Option<String> {
    let lowered = command.to_ascii_lowercase();

    if matches!(lowered.as_str(), "help local" | "help tui" | "local help") {
        return Some(render_local_help());
    }

    if lowered == "view all" {
        app.local_view = StreamScope::All;
        app.feed_follow_override = Some(true);
        return Some("Local live view set to all streams.".to_string());
    }

    if lowered == "view selected" {
        app.local_view = StreamScope::Selected;
        app.feed_follow_override = Some(true);
        return Some("Local live view set to the shared selected worker.".to_string());
    }

    if lowered == "view commander" {
        app.local_view = StreamScope::Commander;
        app.feed_follow_override = Some(true);
        return Some("Local live view set to commander-only events.".to_string());
    }

    if lowered == "show next" {
        return Some(match app.cycle_active_worker(snapshot, 1) {
            Some(worker) => format!("Local active worker -> {worker}"),
            None => "No visible worker available.".to_string(),
        });
    }

    if lowered == "show prev" {
        return Some(match app.cycle_active_worker(snapshot, -1) {
            Some(worker) => format!("Local active worker -> {worker}"),
            None => "No visible worker available.".to_string(),
        });
    }

    if lowered == "filter clear" {
        app.worker_filter.clear();
        app.ensure_active_worker_visible(snapshot);
        return Some("Worker filter cleared.".to_string());
    }

    if let Some(target) = command.strip_prefix("filter ") {
        app.worker_filter = target.trim().to_string();
        app.ensure_active_worker_visible(snapshot);
        return Some(if app.worker_filter.trim().is_empty() {
            "Worker filter cleared.".to_string()
        } else {
            format!("Worker filter set to `{}`.", app.worker_filter)
        });
    }

    if let Some(target) = command.strip_prefix("show worker ") {
        return Some(match app.set_active_worker(snapshot, target.trim()) {
            Some(worker) => format!("Local active worker -> {worker}"),
            None => format!("Unknown worker `{}`.", target.trim()),
        });
    }

    if let Some(target) = command.strip_prefix("view worker ") {
        return Some(match app.set_active_worker(snapshot, target.trim()) {
            Some(worker) => {
                app.local_view = StreamScope::Worker(worker.clone());
                app.feed_follow_override = Some(true);
                format!("Local live view pinned to worker `{worker}`.")
            }
            None => format!("Unknown worker `{}`.", target.trim()),
        });
    }

    if lowered == "show coordination" || lowered == "show graph" {
        return Some(render_coordination_summary(snapshot));
    }

    if lowered == "show escalations" {
        return Some(render_escalation_summary(snapshot));
    }

    if lowered == "show reviews" {
        return Some(render_review_summary(snapshot));
    }

    if lowered == "show retro" {
        return Some(render_retro_summary(snapshot));
    }

    None
}

fn execute_supervisor_command(
    session: &SupervisorSession,
    app: &mut AppState,
    snapshot: &SnapshotBundle,
    command: &str,
) -> Result<()> {
    let outcome = session.execute_command(command, "tui")?;
    if !command.trim().to_ascii_lowercase().starts_with("follow ") {
        app.feed_follow_override = Some(true);
    }
    app.push_result(&outcome.message);
    app.should_quit = outcome.quit_requested;
    app.force_snapshot_refresh = true;
    sync_local_state_after_supervisor_command(app, snapshot, command);
    Ok(())
}

fn sync_local_state_after_supervisor_command(
    app: &mut AppState,
    snapshot: &SnapshotBundle,
    command: &str,
) {
    let lowered = command.to_ascii_lowercase();

    if let Some(target) = command.strip_prefix("select ") {
        if let Some(worker) = find_worker_name(snapshot, target.trim()) {
            app.active_worker = Some(worker);
        }
    }

    if let Some(scope) = lowered.strip_prefix("stream scope ") {
        if let Some(local_view) = parse_stream_scope(scope.trim(), snapshot) {
            app.local_view = local_view;
            app.feed_follow_override = Some(true);
        }
    }

    if lowered.starts_with("follow ") {
        app.feed_follow_override = if lowered.ends_with(" off") {
            Some(false)
        } else {
            Some(true)
        };
    }
}

fn draw_ui(frame: &mut Frame<'_>, app: &mut AppState, snapshot: &SnapshotBundle) {
    let area = frame.area();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(area);
    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(54), Constraint::Percentage(46)])
        .split(rows[0]);
    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(54), Constraint::Percentage(46)])
        .split(rows[1]);

    render_commander_tower(frame, top[0], app, snapshot);
    render_worker_map(frame, top[1], app, snapshot);
    render_live_feed(frame, bottom[0], app, snapshot);
    let cursor = render_console(frame, bottom[1], app, snapshot);
    frame.set_cursor_position(cursor);
}

fn render_commander_tower(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &AppState,
    snapshot: &SnapshotBundle,
) {
    let provider = snapshot
        .provider
        .as_ref()
        .map(|profile| format!("{} @ {}", profile.provider_name, profile.base_url))
        .unwrap_or_else(|| "provider unresolved".to_string());
    let session_id = snapshot
        .control
        .as_ref()
        .map(|control| control.session_id.as_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("offline");
    let shared_scope = snapshot
        .control
        .as_ref()
        .map(|control| stream_scope_label(&control.stream_scope))
        .unwrap_or_else(|| "all".to_string());
    let density = snapshot
        .control
        .as_ref()
        .map(|control| density_label(control.density_mode))
        .unwrap_or("standard");
    let activation_style = if snapshot.status.activation_required {
        alert_style()
    } else {
        ok_style()
    };

    let lines = vec![
        Line::from(vec![Span::styled(
            " .---------------- COMMAND TOWER ----------------. ",
            title_style(),
        )]),
        Line::from(vec![
            Span::raw(" | LangGraph-Commander "),
            Span::styled(FRAMEWORK_VERSION, accent_style()),
            Span::raw(" :: command-first control plane |"),
        ]),
        Line::from(vec![Span::raw(
            " '------------------------------------------------' ",
        )]),
        Line::from(vec![
            Span::styled("phase", label_style()),
            Span::raw(format!(" : {}", snapshot.status.phase)),
            Span::raw("    "),
            Span::styled("session", label_style()),
            Span::raw(format!(" : {session_id}")),
        ]),
        Line::from(vec![
            Span::styled("activation", label_style()),
            Span::raw(" : "),
            Span::styled(
                if snapshot.status.activation_required {
                    "REQUIRED"
                } else {
                    "CLEAR"
                },
                activation_style,
            ),
            Span::raw(format!(" :: {}", snapshot.status.activation_reason)),
        ]),
        Line::from(vec![
            Span::styled("patrol", label_style()),
            Span::raw(format!(
                " : {} / {}",
                if snapshot.patrol.enabled { "on" } else { "off" },
                if snapshot.patrol.last_result.trim().is_empty() {
                    "idle"
                } else {
                    snapshot.patrol.last_result.as_str()
                }
            )),
        ]),
        Line::from(vec![
            Span::styled("audit", label_style()),
            Span::raw(format!(" : {}", snapshot.status.last_check_status)),
        ]),
        Line::from(vec![
            Span::styled("audit warn", label_style()),
            Span::raw(format!(
                " : {}",
                if snapshot.status.last_check_warning.trim().is_empty() {
                    "none".to_string()
                } else {
                    truncate(&snapshot.status.last_check_warning, 84)
                }
            )),
        ]),
        coordination_tower_line(snapshot),
        coordination_reviews_line(snapshot),
        coordination_metrics_line(snapshot),
        Line::from(vec![
            Span::styled("local view", label_style()),
            Span::raw(format!(" : {}", app.local_view_label())),
            Span::raw("    "),
            Span::styled("shared stream", label_style()),
            Span::raw(format!(" : {shared_scope}")),
        ]),
        Line::from(vec![
            Span::styled("active worker", label_style()),
            Span::raw(format!(
                " : {}",
                app.active_worker.as_deref().unwrap_or("none")
            )),
            Span::raw("    "),
            Span::styled("shared selected", label_style()),
            Span::raw(format!(
                " : {}",
                app.resolved_selected_worker(snapshot).unwrap_or("none")
            )),
        ]),
        Line::from(vec![
            Span::styled("provider", label_style()),
            Span::raw(format!(" : {}", truncate(&provider, 64))),
        ]),
        Line::from(vec![
            Span::styled("density", label_style()),
            Span::raw(format!(" : {density}")),
            Span::raw("    "),
            Span::styled("follow", label_style()),
            Span::raw(format!(
                " : {}",
                if app.feed_follows_tail(snapshot) {
                    "on"
                } else {
                    "off"
                }
            )),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Quick deck", title_style()),
            Span::raw(" :: use raw supervisor commands or local view commands"),
        ]),
        Line::from(" start all      stop all       patrol once      refresh"),
        Line::from(" intake         approve        review all       report"),
        Line::from(" phase <name>   select <name>  check            help"),
        Line::from(" view all       view selected  view worker <w>  show worker <w>"),
        Line::from(" show coordination  show escalations  show reviews  show retro"),
        Line::from(" filter <text>  filter clear   help local       Esc Esc quit"),
        Line::from(" Ctrl+R refresh | Ctrl+S stop-all | Ctrl+D density | Ctrl+F follow"),
        Line::from(vec![
            Span::styled("last command", label_style()),
            Span::raw(format!(" : {}", truncate(&app.last_command, 72))),
        ]),
        Line::from(vec![
            Span::styled("last result", label_style()),
            Span::raw(format!(" : {}", truncate(&app.last_feedback, 72))),
        ]),
    ];

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(panel_block("COMMAND TOWER", PanelTone::Tower))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_worker_map(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &mut AppState,
    snapshot: &SnapshotBundle,
) {
    let block = panel_block("WORKER ORCHESTRATION", PanelTone::Workers);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(4)])
        .split(inner);

    let workers = &snapshot.status.workers;
    let visible = app.visible_workers(snapshot);
    let running = workers
        .iter()
        .filter(|worker| worker.status.eq_ignore_ascii_case("running"))
        .count();
    let ready = workers
        .iter()
        .filter(|worker| worker.handoff_status.eq_ignore_ascii_case("ready"))
        .count();
    let dirty = workers.iter().filter(|worker| !worker.git_clean).count();
    let selected_worker = app.resolved_selected_worker(snapshot).unwrap_or("none");
    let active_worker = active_worker_snapshot(snapshot, app);

    let summary = vec![
        Line::from(vec![
            Span::styled("fleet", label_style()),
            Span::raw(format!(
                " : total {} | running {} | ready {} | dirty {}",
                workers.len(),
                running,
                ready,
                dirty
            )),
        ]),
        Line::from(vec![
            Span::styled("filter", label_style()),
            Span::raw(format!(
                " : {} | visible {}",
                app.worker_filter_label(),
                visible.len()
            )),
        ]),
        Line::from(vec![
            Span::styled("shared selected", label_style()),
            Span::raw(format!(" : {selected_worker}")),
        ]),
        Line::from(vec![
            Span::styled("local active", label_style()),
            Span::raw(format!(
                " : {}",
                app.active_worker.as_deref().unwrap_or("none")
            )),
        ]),
        Line::from(vec![
            Span::styled("active lane", label_style()),
            Span::raw(format!(
                " : branch {} | scope {} | blocked {}",
                active_worker
                    .map(|worker| truncate(&worker.branch, 18))
                    .unwrap_or_else(|| "n/a".to_string()),
                active_worker
                    .map(|worker| {
                        truncate(
                            if worker.execution_scope.trim().is_empty() {
                                "full"
                            } else {
                                worker.execution_scope.as_str()
                            },
                            22,
                        )
                    })
                    .unwrap_or_else(|| "n/a".to_string()),
                active_worker
                    .map(|worker| if worker.launch_blocked { "yes" } else { "no" })
                    .unwrap_or("n/a")
            )),
        ]),
        Line::from(vec![
            Span::styled("active gate", label_style()),
            Span::raw(format!(
                " : {}",
                active_worker
                    .and_then(|worker| {
                        let pending = worker.pending_action.trim();
                        (!pending.is_empty()).then_some(truncate(pending, 52))
                    })
                    .unwrap_or_else(|| "none".to_string())
            )),
        ]),
        Line::from(vec![
            Span::styled("active model", label_style()),
            Span::raw(format!(
                " : {} | think {}",
                active_worker
                    .map(worker_model_badge)
                    .unwrap_or_else(|| "n/a".to_string()),
                active_worker
                    .map(worker_reasoning_badge)
                    .unwrap_or_else(|| "n/a".to_string())
            )),
        ]),
        Line::from(" Alt+PgUp/PgDn scroll roster | Ctrl+N/Ctrl+P cycle local active worker"),
    ];
    frame.render_widget(
        Paragraph::new(Text::from(summary)).wrap(Wrap { trim: false }),
        layout[0],
    );

    let roster = if visible.is_empty() {
        vec![
            Line::from("no workers match the current filter"),
            Line::from("try `filter clear` or a broader filter term"),
        ]
    } else {
        visible
            .into_iter()
            .map(|worker| worker_roster_line(worker, app, selected_worker))
            .collect::<Vec<_>>()
    };

    let roster_len = roster.len();
    let max_scroll = max_scroll(roster_len, layout[1].height as usize);
    app.workers_scroll = app.workers_scroll.min(max_scroll);

    frame.render_widget(
        Paragraph::new(Text::from(roster))
            .scroll((app.workers_scroll as u16, 0))
            .wrap(Wrap { trim: false }),
        layout[1],
    );
    render_scrollbar(frame, layout[1], roster_len, app.workers_scroll);
}

fn render_live_feed(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &mut AppState,
    snapshot: &SnapshotBundle,
) {
    let title = format!(
        "LIVE OUTPUT :: {} :: {} :: tail {}",
        app.local_view_label(),
        density_label(
            snapshot
                .control
                .as_ref()
                .map(|control| control.density_mode)
                .unwrap_or(FeedDensity::Standard),
        ),
        if app.feed_follows_tail(snapshot) {
            "on"
        } else {
            "off"
        }
    );

    let entries = feed_lines(snapshot, app);
    let inner_height = area.height.saturating_sub(2) as usize;
    let max_scroll = max_scroll(entries.len(), inner_height);
    let scroll = if app.feed_follows_tail(snapshot) {
        max_scroll
    } else {
        app.feed_scroll.min(max_scroll)
    };
    app.feed_scroll = scroll;

    frame.render_widget(
        Paragraph::new(Text::from(entries))
            .block(panel_block(&title, PanelTone::Feed))
            .scroll((scroll as u16, 0))
            .wrap(Wrap { trim: false }),
        area,
    );
    render_scrollbar(frame, area, scrollable_len_hint(snapshot, app), scroll);
}

fn render_console(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &mut AppState,
    snapshot: &SnapshotBundle,
) -> (u16, u16) {
    let block = panel_block("COMMAND INPUT", PanelTone::Console);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(4),
            Constraint::Length(2),
        ])
        .split(inner);

    let meta = vec![
        Line::from(vec![
            Span::styled("keyboard", label_style()),
            Span::raw(" : Enter send | Up/Down history | Left/Right edit | Esc Esc quit"),
        ]),
        Line::from(vec![
            Span::styled("hotkeys", label_style()),
            Span::raw(" : F1 local-help | F2 all | F3 selected | F4 stop-all"),
        ]),
        Line::from(vec![
            Span::styled("ops", label_style()),
            Span::raw(" : Ctrl+R refresh | Ctrl+D density | Ctrl+F follow | Ctrl+L clear-log"),
        ]),
        Line::from(vec![
            Span::styled("scroll", label_style()),
            Span::raw(" : PgUp/PgDn live feed | Alt+PgUp/PgDn workers | Alt+Up/Down command log"),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(Text::from(meta)).wrap(Wrap { trim: true }),
        layout[0],
    );

    let log_lines = if app.console_lines.is_empty() {
        vec![Line::from("no command output yet")]
    } else {
        app.console_lines
            .iter()
            .cloned()
            .map(Line::from)
            .collect::<Vec<_>>()
    };
    let max_scroll = max_scroll(log_lines.len(), layout[1].height as usize);
    app.console_scroll = app.console_scroll.min(max_scroll);

    frame.render_widget(
        Paragraph::new(Text::from(log_lines))
            .scroll((app.console_scroll as u16, 0))
            .wrap(Wrap { trim: false }),
        layout[1],
    );
    render_scrollbar(
        frame,
        layout[1],
        app.console_lines.len().max(1),
        app.console_scroll,
    );

    let footer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(layout[2]);

    let status = Line::from(vec![
        Span::styled("view", label_style()),
        Span::raw(format!("={}  ", app.local_view_label())),
        Span::styled("filter", label_style()),
        Span::raw(format!("={}  ", app.worker_filter_label())),
        Span::styled("selected", label_style()),
        Span::raw(format!(
            "={}  ",
            app.resolved_selected_worker(snapshot).unwrap_or("none")
        )),
        Span::styled("last", label_style()),
        Span::raw(format!("={}", truncate(&app.last_feedback, 40))),
    ]);
    frame.render_widget(Paragraph::new(status).wrap(Wrap { trim: true }), footer[0]);

    let prompt_width = footer[1].width.saturating_sub(5) as usize;
    let (visible_input, cursor_col) =
        visible_input_window(&app.command_input, app.input_cursor, prompt_width);
    let prompt = Line::from(vec![
        Span::styled("cmd> ", accent_style()),
        Span::raw(visible_input),
    ]);
    frame.render_widget(Paragraph::new(prompt), footer[1]);

    (
        footer[1].x.saturating_add(5 + cursor_col as u16),
        footer[1].y,
    )
}

fn coordination_tower_line(snapshot: &SnapshotBundle) -> Line<'static> {
    let Some(coordination) = snapshot.coordination.as_ref() else {
        return Line::from(vec![
            Span::styled("coordination", label_style()),
            Span::raw(" : no coordination plan loaded"),
        ]);
    };
    Line::from(vec![
        Span::styled("coordination", label_style()),
        Span::raw(format!(
            " : plan {} | approved {} | open esc {} | blocked {}",
            truncate(&coordination.plan_id, 16),
            if coordination.approved { "yes" } else { "no" },
            coordination.open_escalations().len(),
            coordination.blocked_worker_count(),
        )),
    ])
}

fn coordination_reviews_line(snapshot: &SnapshotBundle) -> Line<'static> {
    let Some(coordination) = snapshot.coordination.as_ref() else {
        return Line::from(vec![
            Span::styled("reviews", label_style()),
            Span::raw(" : no review summary available"),
        ]);
    };
    let (approved, rework, escalated) = coordination.latest_review_counts();
    Line::from(vec![
        Span::styled("reviews", label_style()),
        Span::raw(format!(
            " : approve {} | rework {} | escalate {}",
            approved, rework, escalated
        )),
    ])
}

fn coordination_metrics_line(snapshot: &SnapshotBundle) -> Line<'static> {
    let Some(coordination) = snapshot.coordination.as_ref() else {
        return Line::from(vec![
            Span::styled("retro", label_style()),
            Span::raw(" : no metrics yet"),
        ]);
    };
    let average = coordination
        .metrics
        .as_ref()
        .map(|metrics| metrics.average_cycle_minutes.as_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("n/a");
    let recommendation = coordination
        .metrics
        .as_ref()
        .and_then(|metrics| metrics.recommendations.first())
        .map(|value| truncate(value, 38))
        .unwrap_or_else(|| "no retro note".to_string());
    Line::from(vec![
        Span::styled("retro", label_style()),
        Span::raw(format!(" : avg cycle {}m | {}", average, recommendation)),
    ])
}

fn render_coordination_summary(snapshot: &SnapshotBundle) -> String {
    let Some(coordination) = snapshot.coordination.as_ref() else {
        return "No coordination plan loaded.".to_string();
    };
    let mut lines = vec![
        format!("plan={} approved={}", coordination.plan_id, coordination.approved),
        format!(
            "stages={} open_escalations={} blocked_workers={}",
            coordination.parallel_sets.len(),
            coordination.open_escalations().len(),
            coordination.blocked_worker_count()
        ),
    ];
    for (index, stage) in coordination.parallel_sets.iter().enumerate() {
        lines.push(format!("stage {} => {}", index + 1, stage.join(", ")));
    }
    lines.join("\n")
}

fn render_escalation_summary(snapshot: &SnapshotBundle) -> String {
    let Some(coordination) = snapshot.coordination.as_ref() else {
        return "No coordination plan loaded.".to_string();
    };
    let mut lines = Vec::new();
    let open = coordination.open_escalations();
    if open.is_empty() {
        lines.push("open escalations: none".to_string());
    } else {
        for escalation in open {
            lines.push(format!(
                "{} [{}] blocked={} :: {}",
                escalation.id,
                escalation.scope,
                if escalation.freeze_workers.is_empty() {
                    "none".to_string()
                } else {
                    escalation.freeze_workers.join(",")
                },
                truncate(&escalation.question, 96)
            ));
        }
    }
    let tracked = coordination
        .escalations
        .iter()
        .filter(|item| item.status.eq_ignore_ascii_case("tracked"))
        .collect::<Vec<_>>();
    if !tracked.is_empty() {
        lines.push("tracked alignment items:".to_string());
        for escalation in tracked {
            lines.push(format!(
                "- {} :: {}",
                escalation.id,
                truncate(&escalation.title, 88)
            ));
        }
    }
    lines.join("\n")
}

fn render_review_summary(snapshot: &SnapshotBundle) -> String {
    let Some(coordination) = snapshot.coordination.as_ref() else {
        return "No coordination plan loaded.".to_string();
    };
    let mut lines = Vec::new();
    for worker in coordination.workers.values() {
        let review = coordination.latest_review(&worker.name);
        match review {
            Some(review) => lines.push(format!(
                "{} => {} :: {}",
                worker.name,
                review.disposition,
                truncate(&review.summary, 80)
            )),
            None => lines.push(format!("{} => no independent review yet", worker.name)),
        }
    }
    lines.join("\n")
}

fn render_retro_summary(snapshot: &SnapshotBundle) -> String {
    let Some(coordination) = snapshot.coordination.as_ref() else {
        return "No coordination plan loaded.".to_string();
    };
    let Some(metrics) = coordination.metrics.as_ref() else {
        return "No retro metrics available yet.".to_string();
    };
    let mut lines = vec![
        format!(
            "workers={} reviewed={} approved={} rework={} escalated={} open_escalations={}",
            metrics.workers_total,
            metrics.reviewed_total,
            metrics.approved_total,
            metrics.rework_total,
            metrics.escalated_total,
            metrics.open_escalations
        ),
        format!("average_cycle_minutes={}", metrics.average_cycle_minutes),
    ];
    if metrics.recommendations.is_empty() {
        lines.push("recommendations: none".to_string());
    } else {
        lines.push(format!(
            "recommendations: {}",
            metrics.recommendations.join(" | ")
        ));
    }
    lines.join("\n")
}

fn worker_roster_line(
    worker: &WorkerSnapshot,
    app: &AppState,
    selected_worker: &str,
) -> Line<'static> {
    let active = app
        .active_worker
        .as_deref()
        .is_some_and(|name| name.eq_ignore_ascii_case(&worker.name));
    let selected =
        !selected_worker.is_empty() && selected_worker.eq_ignore_ascii_case(&worker.name);
    let marker = match (active, selected) {
        (true, true) => ">*",
        (true, false) => "> ",
        (false, true) => "* ",
        (false, false) => "  ",
    };

    let name_style = if active {
        selected_style()
    } else {
        Style::default().fg(Color::Rgb(214, 222, 235))
    };
    let dirty_style = if worker.git_clean {
        muted_style()
    } else {
        alert_style()
    };

    Line::from(vec![
        Span::styled(marker, accent_style()),
        Span::styled(format!("{:<20}", truncate(&worker.name, 20)), name_style),
        Span::raw(" "),
        Span::styled(
            format!("{:<10}", truncate(&worker.status, 10)),
            worker_status_style(&worker.status),
        ),
        Span::raw(" "),
        Span::styled(
            format!("{:<5}", if worker.git_clean { "clean" } else { "dirty" }),
            dirty_style,
        ),
        Span::raw(" "),
        Span::styled(
            truncate(
                if worker.handoff_status.trim().is_empty() {
                    "none"
                } else {
                    worker.handoff_status.as_str()
                },
                10,
            ),
            handoff_style(&worker.handoff_status),
        ),
        Span::raw(" "),
        Span::styled(
            format!("{:<5}", if worker.launch_blocked { "hold" } else { "go" }),
            if worker.launch_blocked {
                alert_style()
            } else {
                ok_style()
            },
        ),
        Span::raw(" "),
        Span::styled(
            format!("{:<10}", truncate(&worker_model_badge(worker), 10)),
            accent_style(),
        ),
    ])
}

fn feed_lines(snapshot: &SnapshotBundle, app: &AppState) -> Vec<Line<'static>> {
    let density = snapshot
        .control
        .as_ref()
        .map(|control| control.density_mode)
        .unwrap_or(FeedDensity::Standard);
    let selected_worker = app.resolved_selected_worker(snapshot).unwrap_or_default();
    let lines = snapshot
        .status
        .recent_activity
        .iter()
        .filter(|entry| activity_matches_local_view(entry, &app.local_view, selected_worker))
        .map(|entry| Line::from(format_activity(entry, density)))
        .collect::<Vec<_>>();

    if lines.is_empty() {
        return vec![
            Line::from("no activity matched the current local view"),
            Line::from("try `view all`, `view selected`, or `view worker <name>`"),
        ];
    }

    lines
}

fn activity_matches_local_view(
    entry: &ActivityEntry,
    local_view: &StreamScope,
    selected_worker: &str,
) -> bool {
    match local_view {
        StreamScope::All => true,
        StreamScope::Commander => entry.worker_name.trim().is_empty(),
        StreamScope::Selected => {
            !selected_worker.trim().is_empty()
                && (matches_worker(entry.worker_name.as_str(), selected_worker)
                    || matches_worker(entry.source.as_str(), selected_worker))
        }
        StreamScope::Worker(name) => {
            matches_worker(entry.worker_name.as_str(), name)
                || matches_worker(entry.source.as_str(), name)
        }
    }
}

fn matches_worker(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

fn format_activity(entry: &ActivityEntry, density: FeedDensity) -> String {
    match density {
        FeedDensity::Realtime => format!(
            "#{:05} {} [{}:{}] {}",
            entry.seq,
            entry.timestamp,
            entry.source,
            entry.channel,
            if entry.full_message.trim().is_empty() {
                entry.message.as_str()
            } else {
                entry.full_message.as_str()
            }
        ),
        FeedDensity::Standard => format!(
            "{} [{}] {}",
            entry.timestamp,
            if entry.worker_name.trim().is_empty() {
                entry.source.as_str()
            } else {
                entry.worker_name.as_str()
            },
            if entry.dense_message.trim().is_empty() {
                entry.message.as_str()
            } else {
                entry.dense_message.as_str()
            }
        ),
    }
}

fn parse_stream_scope(scope: &str, snapshot: &SnapshotBundle) -> Option<StreamScope> {
    match scope.trim() {
        "all" => Some(StreamScope::All),
        "selected" => Some(StreamScope::Selected),
        "commander" => Some(StreamScope::Commander),
        value => value
            .strip_prefix("worker:")
            .and_then(|name| find_worker_name(snapshot, name.trim()))
            .map(StreamScope::Worker),
    }
}

fn shared_selected_worker(snapshot: &SnapshotBundle) -> Option<String> {
    snapshot.control.as_ref().and_then(|control| {
        let selected = control.selected_worker.trim();
        (!selected.is_empty()).then(|| selected.to_string())
    })
}

fn active_worker_snapshot<'a>(
    snapshot: &'a SnapshotBundle,
    app: &'a AppState,
) -> Option<&'a WorkerSnapshot> {
    let active = app.active_worker.as_deref()?;
    snapshot
        .status
        .workers
        .iter()
        .find(|worker| worker.name.eq_ignore_ascii_case(active))
}

fn find_worker_name(snapshot: &SnapshotBundle, requested: &str) -> Option<String> {
    let requested = requested.trim();
    snapshot
        .status
        .workers
        .iter()
        .find(|worker| worker.name.eq_ignore_ascii_case(requested))
        .map(|worker| worker.name.clone())
}

fn worker_matches_filter(worker: &WorkerSnapshot, filter: &str) -> bool {
    let filter = filter.trim();
    if filter.is_empty() {
        return true;
    }

    let filter = filter.to_ascii_lowercase();
    [
        worker.name.as_str(),
        worker.status.as_str(),
        worker.handoff_status.as_str(),
        worker.branch.as_str(),
        worker.pending_action.as_str(),
        worker.execution_scope.as_str(),
        worker.model_name.as_str(),
        worker.reasoning_effort.as_str(),
    ]
    .into_iter()
    .any(|field| field.to_ascii_lowercase().contains(&filter))
}

fn scrollable_len_hint(snapshot: &SnapshotBundle, app: &AppState) -> usize {
    feed_lines(snapshot, app).len()
}

fn toggle_follow_command(snapshot: &SnapshotBundle) -> &'static str {
    if snapshot
        .control
        .as_ref()
        .map(|control| control.follow_tail)
        .unwrap_or(true)
    {
        "follow off"
    } else {
        "follow on"
    }
}

fn toggle_density_command(snapshot: &SnapshotBundle) -> &'static str {
    match snapshot
        .control
        .as_ref()
        .map(|control| control.density_mode)
        .unwrap_or(FeedDensity::Standard)
    {
        FeedDensity::Standard => "density realtime",
        FeedDensity::Realtime => "density standard",
    }
}

fn render_scrollbar(frame: &mut Frame<'_>, area: Rect, len: usize, position: usize) {
    let mut state = ScrollbarState::new(len.max(1)).position(position);
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(Some("|"))
            .thumb_symbol("#"),
        area.inner(Margin {
            vertical: 0,
            horizontal: 0,
        }),
        &mut state,
    );
}

#[derive(Clone, Copy)]
enum PanelTone {
    Tower,
    Feed,
    Workers,
    Console,
}

fn panel_block(title: &str, tone: PanelTone) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(panel_border_color(tone)))
        .title(Line::from(vec![
            Span::styled(" ", muted_style()),
            Span::styled(title.to_string(), title_style()),
            Span::styled(" ", muted_style()),
        ]))
        .style(
            Style::default()
                .bg(Color::Rgb(17, 24, 28))
                .fg(Color::Rgb(214, 222, 235)),
        )
}

fn panel_border_color(tone: PanelTone) -> Color {
    match tone {
        PanelTone::Tower => Color::Rgb(92, 160, 97),
        PanelTone::Feed => Color::Rgb(96, 156, 193),
        PanelTone::Workers => Color::Rgb(120, 129, 250),
        PanelTone::Console => Color::Rgb(224, 181, 90),
    }
}

fn title_style() -> Style {
    Style::default()
        .fg(Color::Rgb(141, 211, 110))
        .add_modifier(Modifier::BOLD)
}

fn accent_style() -> Style {
    Style::default()
        .fg(Color::Rgb(128, 203, 247))
        .add_modifier(Modifier::BOLD)
}

fn label_style() -> Style {
    Style::default()
        .fg(Color::Rgb(128, 159, 163))
        .add_modifier(Modifier::BOLD)
}

fn muted_style() -> Style {
    Style::default().fg(Color::Rgb(98, 118, 124))
}

fn ok_style() -> Style {
    Style::default()
        .fg(Color::Rgb(141, 211, 110))
        .add_modifier(Modifier::BOLD)
}

fn alert_style() -> Style {
    Style::default()
        .fg(Color::Rgb(246, 113, 113))
        .add_modifier(Modifier::BOLD)
}

fn selected_style() -> Style {
    Style::default()
        .fg(Color::Rgb(244, 255, 201))
        .bg(Color::Rgb(41, 69, 54))
        .add_modifier(Modifier::BOLD)
}

fn worker_status_style(status: &str) -> Style {
    let lowered = status.to_ascii_lowercase();
    if lowered.contains("running") {
        ok_style()
    } else if lowered.contains("fail") || lowered.contains("error") {
        alert_style()
    } else if lowered.contains("finish") || lowered.contains("ready") {
        accent_style()
    } else {
        muted_style()
    }
}

fn handoff_style(status: &str) -> Style {
    let lowered = status.to_ascii_lowercase();
    if lowered.contains("ready") {
        ok_style()
    } else if lowered.contains("block") || lowered.contains("wait") || lowered.contains("pending") {
        Style::default()
            .fg(Color::Rgb(224, 181, 90))
            .add_modifier(Modifier::BOLD)
    } else {
        muted_style()
    }
}

fn density_label(density: FeedDensity) -> &'static str {
    match density {
        FeedDensity::Standard => "standard",
        FeedDensity::Realtime => "realtime",
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

fn worker_model_badge(worker: &WorkerSnapshot) -> String {
    let raw = worker.model_name.trim();
    if raw.is_empty() {
        return "n/a".to_string();
    }

    let lowered = raw
        .to_ascii_lowercase()
        .replace('_', "-")
        .trim_start_matches("gpt-")
        .to_string();
    lowered
}

fn worker_reasoning_badge(worker: &WorkerSnapshot) -> String {
    let raw = worker.reasoning_effort.trim();
    if raw.is_empty() {
        "standard".to_string()
    } else {
        raw.to_ascii_lowercase()
    }
}

fn truncate(text: &str, width: usize) -> String {
    let chars = text.chars().collect::<Vec<_>>();
    if chars.len() <= width {
        return text.to_string();
    }
    let keep = width.saturating_sub(1);
    chars[..keep].iter().collect::<String>() + "~"
}

fn visible_input_window(input: &str, cursor: usize, width: usize) -> (String, usize) {
    if width == 0 {
        return (String::new(), 0);
    }

    let chars = input.chars().collect::<Vec<_>>();
    let len = chars.len();
    let cursor = cursor.min(len);
    let mut start = cursor.saturating_sub(width.saturating_sub(1));
    if len.saturating_sub(start) < width {
        start = len.saturating_sub(width);
    }
    let end = (start + width).min(len);
    let visible = chars[start..end].iter().collect::<String>();
    (visible, cursor.saturating_sub(start))
}

fn max_scroll(lines: usize, height: usize) -> usize {
    lines.saturating_sub(height)
}

fn wrapped_index(current: usize, len: usize, step: isize) -> usize {
    if len == 0 {
        return 0;
    }

    let len = len as isize;
    let current = current as isize;
    ((current + step).rem_euclid(len)) as usize
}

fn render_local_help() -> String {
    [
        "LOCAL PANEL COMMANDS",
        "--------------------",
        "help local",
        "view all",
        "view selected",
        "view commander",
        "view worker <name>",
        "show worker <name>",
        "show next",
        "show prev",
        "show coordination",
        "show escalations",
        "show reviews",
        "show retro",
        "filter <text>",
        "filter clear",
        "",
        "SUPERVISOR COMMANDS",
        "-------------------",
        "help",
        "help keys",
        "refresh",
        "check",
        "start all|<worker>",
        "stop all|<worker>",
        "patrol once|start|stop|status",
        "phase <name>",
        "select <worker>",
        "intake",
        "approve",
        "review all|<worker>",
        "report",
        "density standard|realtime",
        "follow on|off",
        "stream scope all|selected|commander|worker:<name>",
        "clear",
        "quit",
        "",
        "KEYBOARD",
        "--------",
        "Enter submit | Up/Down history | Left/Right edit",
        "PgUp/PgDn live feed scroll",
        "Alt+PgUp/PgDn worker roster scroll",
        "Alt+Up/Down command log scroll",
        "Ctrl+R refresh | Ctrl+S stop-all | Ctrl+D density | Ctrl+F follow",
        "Ctrl+N / Ctrl+P cycle local active worker",
        "Esc Esc quits and stops tracked workers during session shutdown",
    ]
    .join("\n")
}

fn init_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_snapshot() -> SnapshotBundle {
        let mut snapshot = SnapshotBundle::default();
        snapshot.status.phase = "bootstrap".to_string();
        snapshot.status.workers = vec![
            WorkerSnapshot {
                name: "api-spec".to_string(),
                model_name: "GPT-5.4-HIGH".to_string(),
                reasoning_effort: "high".to_string(),
                status: "finished".to_string(),
                handoff_status: "ready".to_string(),
                branch: "module/api-spec".to_string(),
                ..Default::default()
            },
            WorkerSnapshot {
                name: "state".to_string(),
                model_name: "GPT-5.4-HIGH".to_string(),
                reasoning_effort: "high".to_string(),
                status: "running".to_string(),
                handoff_status: "pending".to_string(),
                branch: "module/state".to_string(),
                pending_action: "validate fixtures".to_string(),
                git_clean: false,
                ..Default::default()
            },
            WorkerSnapshot {
                name: "verifier-proofs".to_string(),
                model_name: "GPT-5.4-HIGH".to_string(),
                reasoning_effort: "high".to_string(),
                status: "finished".to_string(),
                handoff_status: "ready".to_string(),
                branch: "module/verifier-proofs".to_string(),
                ..Default::default()
            },
        ];
        snapshot.status.recent_activity = vec![
            ActivityEntry {
                worker_name: "state".to_string(),
                source: "state".to_string(),
                timestamp: "2026-03-16 13:06:00".to_string(),
                dense_message: "validation pass".to_string(),
                ..Default::default()
            },
            ActivityEntry {
                source: "commander".to_string(),
                timestamp: "2026-03-16 13:07:00".to_string(),
                dense_message: "activation clear".to_string(),
                ..Default::default()
            },
        ];
        snapshot.control = Some(lgc_core::runtime::ControlSnapshot {
            selected_worker: "state".to_string(),
            stream_scope: StreamScope::All,
            density_mode: FeedDensity::Standard,
            follow_tail: true,
            ..Default::default()
        });
        snapshot
    }

    #[test]
    fn escape_requires_two_presses() {
        let now = Instant::now();
        let mut app = AppState::default();
        assert!(!app.arm_or_quit(now));
        assert!(app.arm_or_quit(now + Duration::from_secs(1)));
        assert!(!app.arm_or_quit(now + Duration::from_secs(3)));
    }

    #[test]
    fn local_view_worker_command_updates_scope_and_active_worker() {
        let snapshot = sample_snapshot();
        let mut app = AppState::default();
        app.sync_from_snapshot(&snapshot);

        let message = execute_local_command(&mut app, &snapshot, "view worker state")
            .expect("local command should be handled");

        assert_eq!(message, "Local live view pinned to worker `state`.");
        assert_eq!(app.local_view, StreamScope::Worker("state".to_string()));
        assert_eq!(app.active_worker.as_deref(), Some("state"));
    }

    #[test]
    fn filter_command_rehomes_active_worker_to_visible_match() {
        let snapshot = sample_snapshot();
        let mut app = AppState::default();
        app.active_worker = Some("api-spec".to_string());

        let message = execute_local_command(&mut app, &snapshot, "filter state")
            .expect("filter command should be handled");

        assert_eq!(message, "Worker filter set to `state`.");
        assert_eq!(app.active_worker.as_deref(), Some("state"));
    }

    #[test]
    fn selected_view_filters_activity() {
        let snapshot = sample_snapshot();
        let mut app = AppState::default();
        app.local_view = StreamScope::Selected;

        let lines = feed_lines(&snapshot, &app);
        let rendered = lines
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("validation pass"));
        assert!(!rendered.contains("activation clear"));
    }

    #[test]
    fn command_history_restores_draft_after_navigation() {
        let mut app = AppState::default();
        app.push_history("start state");
        app.push_history("stop all");
        app.set_input("draft".to_string());

        app.history_prev();
        assert_eq!(app.command_input, "stop all");

        app.history_next();
        assert_eq!(app.command_input, "draft");
    }

    #[test]
    fn visible_input_window_tracks_cursor_near_tail() {
        let input = "view worker verifier-proofs";
        let (visible, cursor) = visible_input_window(input, input.chars().count(), 10);
        assert_eq!(visible, "ier-proofs");
        assert_eq!(cursor, 10);
    }

    #[test]
    fn worker_model_badge_shortens_alias() {
        let worker = WorkerSnapshot {
            model_name: "GPT-5.4-HIGH".to_string(),
            reasoning_effort: "high".to_string(),
            ..Default::default()
        };
        assert_eq!(worker_model_badge(&worker), "5.4-high");
        assert_eq!(worker_reasoning_badge(&worker), "high");
    }
}
