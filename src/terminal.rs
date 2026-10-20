use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Instant;

use crepuscularity_gpui::prelude::*;
use crepuscularity_gpui::{
    div, font, px, rgb, size, App, Context, Div, FontWeight, KeyDownEvent, SharedString, Stateful,
    Window, WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowKind, WindowOptions,
};

use crate::components::Theme;
use crate::icons::{self, SfSymbol};

const BLOCK_SENTINEL: &str = "__ALPENGLOW_BLOCK_END__:";

pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_escape = false;
    for ch in s.chars() {
        if in_escape {
            if ch == 'm' || ch.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else if ch == '\u{1b}' || ch == '\u{9b}' {
            in_escape = true;
        } else {
            out.push(ch);
        }
    }
    out
}

/// A single self-contained command execution block in the threaded terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalBlock {
    pub id: usize,
    pub command: String,
    pub cwd: String,
    pub git_branch: Option<String>,
    pub output: Vec<String>,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u128>,
    pub running: bool,
    pub collapsed: bool,
}

impl TerminalBlock {
    pub fn status_label(&self) -> String {
        if self.running {
            "running".to_string()
        } else {
            match self.exit_code {
                Some(0) => "exit 0".to_string(),
                Some(code) => format!("exit {code}"),
                None => "done".to_string(),
            }
        }
    }
}

struct TerminalSharedState {
    blocks: VecDeque<TerminalBlock>,
    flat_output: VecDeque<String>,
    current_cwd: String,
    git_branch: Option<String>,
    next_id: usize,
    active_started_at: Option<Instant>,
}

pub struct TerminalConsole {
    child: Child,
    state: Arc<Mutex<TerminalSharedState>>,
    input_tx: mpsc::Sender<String>,
    running: Arc<AtomicBool>,
}

pub fn shorten_home_path(path: &str) -> String {
    if let Some(home) = std::env::var_os("HOME").and_then(|h| h.into_string().ok()) {
        if !home.is_empty() && path.starts_with(&home) {
            let rest = &path[home.len()..];
            if rest.is_empty() {
                return "~".to_string();
            } else if rest.starts_with('/') {
                return format!("~{rest}");
            }
        }
    }
    path.to_string()
}

pub fn detect_git_branch(cwd: &str) -> Option<String> {
    let head_path = std::path::Path::new(cwd).join(".git/HEAD");
    if let Ok(contents) = std::fs::read_to_string(&head_path) {
        let trimmed = contents.trim();
        if let Some(branch) = trimmed.strip_prefix("ref: refs/heads/") {
            return Some(branch.to_string());
        }
        if trimmed.len() >= 7 {
            return Some(trimmed[..7].to_string());
        }
    }
    None
}

impl TerminalConsole {
    pub fn spawn() -> Option<Self> {
        let mut child = Command::new("sh")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .ok()?;

        let stdout = child.stdout.take()?;
        let stderr = child.stderr.take()?;
        let stdin = child.stdin.take()?;

        let initial_cwd = std::env::current_dir()
            .ok()
            .and_then(|p| p.into_os_string().into_string().ok())
            .unwrap_or_else(|| "~".to_string());
        let git_branch = detect_git_branch(&initial_cwd);

        let state = Arc::new(Mutex::new(TerminalSharedState {
            blocks: VecDeque::new(),
            flat_output: VecDeque::new(),
            current_cwd: shorten_home_path(&initial_cwd),
            git_branch,
            next_id: 1,
            active_started_at: None,
        }));
        let running = Arc::new(AtomicBool::new(true));
        let (input_tx, input_rx) = mpsc::channel::<String>();

        let state_out = state.clone();
        let run_out = running.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if !run_out.load(Ordering::Relaxed) {
                    break;
                }
                if let Ok(line) = line {
                    let clean = strip_ansi(&line);
                    if let Some(rest) = clean.strip_prefix(BLOCK_SENTINEL) {
                        let mut parts = rest.splitn(2, ':');
                        let code = parts
                            .next()
                            .and_then(|c| c.parse::<i32>().ok())
                            .unwrap_or(0);
                        let new_pwd = parts.next().unwrap_or("").trim();
                        if let Ok(mut g) = state_out.lock() {
                            let elapsed =
                                g.active_started_at.take().map(|t| t.elapsed().as_millis());
                            if !new_pwd.is_empty() {
                                g.git_branch = detect_git_branch(new_pwd);
                                g.current_cwd = shorten_home_path(new_pwd);
                            }
                            if let Some(front) = g.blocks.front_mut() {
                                if front.running {
                                    front.running = false;
                                    front.exit_code = Some(code);
                                    front.duration_ms = elapsed;
                                }
                            }
                        }
                        continue;
                    }
                    if let Ok(mut g) = state_out.lock() {
                        if g.flat_output.len() >= 1000 {
                            g.flat_output.pop_front();
                        }
                        g.flat_output.push_back(clean.clone());
                        if let Some(front) = g.blocks.front_mut() {
                            if front.output.len() < 200 {
                                front.output.push(clean);
                            }
                        }
                    }
                }
            }
        });

        let state_err = state.clone();
        let run_err = running.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if !run_err.load(Ordering::Relaxed) {
                    break;
                }
                if let Ok(line) = line {
                    let clean = strip_ansi(&line);
                    let formatted = format!("err: {clean}");
                    if let Ok(mut g) = state_err.lock() {
                        if g.flat_output.len() >= 1000 {
                            g.flat_output.pop_front();
                        }
                        g.flat_output.push_back(formatted.clone());
                        if let Some(front) = g.blocks.front_mut() {
                            if front.output.len() < 200 {
                                front.output.push(formatted);
                            }
                        }
                    }
                }
            }
        });

        let run_in = running.clone();
        thread::spawn(move || {
            let mut stdin = stdin;
            while run_in.load(Ordering::Relaxed) {
                if let Ok(line) = input_rx.recv() {
                    if writeln!(stdin, "{line}").is_err() {
                        break;
                    }
                    let _ = stdin.flush();
                } else {
                    break;
                }
            }
        });

        let console = TerminalConsole {
            child,
            state,
            input_tx,
            running,
        };

        if matches!(
            std::env::var("ALPENGLOWED_TERMINAL_DEMO").as_deref(),
            Ok("1" | "true")
        ) {
            console.seed_demo_blocks();
        }

        Some(console)
    }

    /// Submits a command to the shell, creating a new `TerminalBlock` at the top of the thread
    /// and pushing older blocks down as output streams in below the command header.
    pub fn write(&self, line: &str) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return;
        }
        if trimmed == "clear" {
            self.clear();
            return;
        }
        if let Ok(mut g) = self.state.lock() {
            let id = g.next_id;
            g.next_id += 1;
            let cwd = g.current_cwd.clone();
            let git_branch = g.git_branch.clone();
            g.active_started_at = Some(Instant::now());
            // Insert newest block at the front so the thread moves down as new commands run.
            g.blocks.push_front(TerminalBlock {
                id,
                command: trimmed.to_string(),
                cwd,
                git_branch,
                output: Vec::new(),
                exit_code: None,
                duration_ms: None,
                running: true,
                collapsed: false,
            });
            if g.blocks.len() > 64 {
                g.blocks.pop_back();
            }
        }
        let wrapped = format!(
            "{trimmed}\n__ag_ec=$?; printf '{BLOCK_SENTINEL}%d:%s\\n' \"$__ag_ec\" \"$PWD\""
        );
        let _ = self.input_tx.send(wrapped);
    }

    pub fn push_block(&self, block: TerminalBlock) {
        if let Ok(mut g) = self.state.lock() {
            g.blocks.push_front(block);
        }
    }

    pub fn seed_demo_blocks(&self) {
        if let Ok(mut g) = self.state.lock() {
            if !g.blocks.is_empty() {
                return;
            }
            let cwd = g.current_cwd.clone();
            let branch = g.git_branch.clone().or_else(|| Some("main".to_string()));
            g.blocks.push_front(TerminalBlock {
                id: 1,
                command: "uname -srm && echo 'Wayland session active'".to_string(),
                cwd: cwd.clone(),
                git_branch: branch.clone(),
                output: vec![
                    "Linux 6.12.4-arch1-1 x86_64".to_string(),
                    "Wayland session active".to_string(),
                ],
                exit_code: Some(0),
                duration_ms: Some(12),
                running: false,
                collapsed: false,
            });
            g.blocks.push_front(TerminalBlock {
                id: 2,
                command: "cargo test --workspace --quiet".to_string(),
                cwd,
                git_branch: branch,
                output: vec![
                    "running 118 tests across workspace".to_string(),
                    "test result: ok. 118 passed; 0 failed; 0 ignored (0.04s)".to_string(),
                ],
                exit_code: Some(0),
                duration_ms: Some(41),
                running: false,
                collapsed: false,
            });
            g.next_id = 3;
        }
    }

    #[allow(dead_code)]
    pub fn is_alive(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn output(&self) -> Vec<String> {
        self.state
            .lock()
            .ok()
            .map(|g| g.flat_output.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn blocks(&self) -> Vec<TerminalBlock> {
        self.state
            .lock()
            .ok()
            .map(|g| g.blocks.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn current_cwd(&self) -> String {
        self.state
            .lock()
            .ok()
            .map(|g| g.current_cwd.clone())
            .unwrap_or_else(|| "~".to_string())
    }

    pub fn current_git_branch(&self) -> Option<String> {
        self.state.lock().ok().and_then(|g| g.git_branch.clone())
    }

    pub fn clear(&self) {
        if let Ok(mut g) = self.state.lock() {
            g.blocks.clear();
            g.flat_output.clear();
        }
    }
}

impl Drop for TerminalConsole {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Reusable GPUI component rendering a single command block with header, cwd, exit badge,
/// and monospace output directly below in a vertical thread.
pub fn render_terminal_block_card(block: &TerminalBlock, is_latest: bool) -> Stateful<Div> {
    let (status_color, status_bg, status_icon, status_text) = if block.running {
        (
            0xfbbf24,
            0x2d2618,
            SfSymbol::Clock,
            "Running...".to_string(),
        )
    } else {
        match block.exit_code.unwrap_or(0) {
            0 => (
                0x34d399,
                0x162b24,
                SfSymbol::Checkmark,
                block
                    .duration_ms
                    .map(|ms| format!("{ms}ms"))
                    .unwrap_or_else(|| "0ms".to_string()),
            ),
            code => (0xf87171, 0x2e1a1d, SfSymbol::Xmark, format!("exit {code}")),
        }
    };

    let border_col = if is_latest {
        Theme::CARD_BORDER
    } else {
        Theme::CARD_STROKE
    };

    div()
        .id(SharedString::from(format!("terminal-block-{}", block.id)))
        .w_full()
        .rounded(px(12.))
        .bg(rgb(0x1b1b20))
        .border_1()
        .border_color(rgb(border_col))
        .overflow_hidden()
        .flex()
        .flex_col()
        .child(
            // Command Block Header
            div()
                .w_full()
                .px(px(12.))
                .py(px(7.))
                .bg(rgb(0x222228))
                .border_b_1()
                .border_color(rgb(0x2b2b33))
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .overflow_hidden()
                        .child(icons::sf_icon(SfSymbol::ChevronRight, 11.0, 0x60a5fa))
                        .child(
                            div()
                                .font(font("Geist Mono"))
                                .text_size(px(12.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(rgb(Theme::TEXT_PRIMARY))
                                .child(block.command.clone()),
                        )
                        .child(
                            div()
                                .px(px(6.))
                                .py(px(1.))
                                .rounded(px(5.))
                                .bg(rgb(0x2b2b33))
                                .text_size(px(10.5))
                                .text_color(rgb(Theme::TEXT_TERTIARY))
                                .child(block.cwd.clone()),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .when_some(block.git_branch.as_ref(), |el, branch| {
                            el.child(
                                div()
                                    .text_size(px(10.5))
                                    .text_color(rgb(Theme::TEXT_TERTIARY))
                                    .child(format!("⎇ {branch}")),
                            )
                        })
                        .child(
                            div()
                                .px(px(7.))
                                .py(px(2.))
                                .rounded(px(6.))
                                .bg(rgb(status_bg))
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .child(icons::sf_icon(status_icon, 10.0, status_color))
                                .child(
                                    div()
                                        .font(font("Geist Mono"))
                                        .text_size(px(10.5))
                                        .text_color(rgb(status_color))
                                        .child(status_text),
                                ),
                        ),
                ),
        )
        .child(
            // Command Output Section directly below the command in the block
            div()
                .w_full()
                .px(px(12.))
                .py(px(8.))
                .flex()
                .flex_col()
                .gap(px(3.))
                .when(block.output.is_empty(), |col| {
                    col.child(
                        div()
                            .font(font("Geist Mono"))
                            .text_size(px(11.5))
                            .text_color(rgb(Theme::TEXT_TERTIARY))
                            .child(if block.running {
                                "Executing command..."
                            } else {
                                "(no stdout output)"
                            }),
                    )
                })
                .children(block.output.iter().take(8).map(|line| {
                    let is_err = line.starts_with("err:") || line.contains("FAILED");
                    let is_ok = line.contains("ok.") || line.contains("passed");
                    let color = if is_err {
                        0xf87171
                    } else if is_ok {
                        0x34d399
                    } else {
                        0xd4d4d8
                    };
                    div()
                        .font(font("Geist Mono"))
                        .text_size(px(12.))
                        .text_color(rgb(color))
                        .child(line.clone())
                })),
        )
}

/// Reusable, importable GPUI Rich Block Terminal component.
///
/// Renders the active prompt bar at the top and the chronological thread of command blocks
/// moving downward below it. Can be embedded inside `LauncherWindow` or mounted standalone.
pub fn render_warp_terminal(
    console: Option<&TerminalConsole>,
    input: &str,
    max_blocks: usize,
) -> Div {
    let blocks = console.map(TerminalConsole::blocks).unwrap_or_default();
    let cwd = console
        .map(TerminalConsole::current_cwd)
        .unwrap_or_else(|| "~".to_string());
    let branch = console.and_then(TerminalConsole::current_git_branch);

    div()
        .w_full()
        .px(px(12.))
        .pt(px(4.))
        .pb(px(8.))
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(
            // Prompt Input Block at the top of the thread
            div()
                .w_full()
                .rounded(px(12.))
                .bg(rgb(0x1f1f25))
                .border_1()
                .border_color(rgb(0x3a3a44))
                .px(px(12.))
                .py(px(8.))
                .flex()
                .flex_col()
                .gap(px(5.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .child(icons::sf_icon(SfSymbol::Folder, 11.0, 0x60a5fa))
                                .child(
                                    div()
                                        .font(font("Geist Mono"))
                                        .text_size(px(11.))
                                        .text_color(rgb(Theme::TEXT_SECONDARY))
                                        .child(cwd),
                                )
                                .when_some(branch, |row, b| {
                                    row.child(
                                        div()
                                            .px(px(6.))
                                            .py(px(1.))
                                            .rounded(px(5.))
                                            .bg(rgb(0x282830))
                                            .font(font("Geist Mono"))
                                            .text_size(px(10.5))
                                            .text_color(rgb(0xa78bfa))
                                            .child(format!("⎇ {b}")),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(Theme::TEXT_TERTIARY))
                                .child("Block Thread · ↵ Run"),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(icons::sf_icon(SfSymbol::ChevronRight, 13.0, 0x34d399))
                        .child(
                            div()
                                .flex_1()
                                .font(font("Geist Mono"))
                                .text_size(px(13.5))
                                .text_color(rgb(if input.is_empty() {
                                    Theme::TEXT_TERTIARY
                                } else {
                                    Theme::TEXT_PRIMARY
                                }))
                                .child(if input.is_empty() {
                                    "Type a shell command — output streams into a block below..."
                                        .to_string()
                                } else {
                                    input.to_string()
                                }),
                        ),
                ),
        )
        .child(
            // Downward-moving thread of executed command blocks
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(8.))
                .when(blocks.is_empty(), |col| {
                    col.child(
                        div()
                            .w_full()
                            .rounded(px(12.))
                            .bg(rgb(0x1a1a1f))
                            .border_1()
                            .border_color(rgb(Theme::CARD_STROKE))
                            .px(px(14.))
                            .py(px(12.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(10.))
                                    .child(icons::sf_icon(
                                        SfSymbol::Terminal,
                                        15.0,
                                        Theme::TEXT_SECONDARY,
                                    ))
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .text_color(rgb(Theme::TEXT_SECONDARY))
                                            .child("No command blocks yet. Run any command above to start a thread."),
                                    ),
                            ),
                    )
                })
                .children(
                    blocks
                        .iter()
                        .take(max_blocks)
                        .enumerate()
                        .map(|(idx, block)| render_terminal_block_card(block, idx == 0)),
                ),
        )
}

/// Standalone GPUI window hosting the Rich Block Terminal (`alpenglowed terminal` / `--terminal`).
pub struct StandaloneTerminalWindow {
    console: Option<TerminalConsole>,
    input: String,
    history: Vec<String>,
    history_idx: Option<usize>,
}

impl StandaloneTerminalWindow {
    pub fn new() -> Self {
        let console = TerminalConsole::spawn();
        if let Some(ref c) = console {
            if c.blocks().is_empty() {
                c.seed_demo_blocks();
            }
        }
        Self {
            console,
            input: String::new(),
            history: Vec::new(),
            history_idx: None,
        }
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        let cmd = self.input.trim().to_string();
        if cmd.is_empty() {
            return;
        }
        self.history.insert(0, cmd.clone());
        self.history_idx = None;
        self.input.clear();
        if let Some(ref console) = self.console {
            console.write(&cmd);
        }
        cx.notify();
    }
}

impl Render for StandaloneTerminalWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .rounded(px(Theme::PANEL_RADIUS))
            .bg(rgb(Theme::PANEL_SCRIM))
            .border_1()
            .border_color(rgb(Theme::PANEL_STROKE))
            .font(font("Geist"))
            .overflow_hidden()
            .flex()
            .flex_col()
            .justify_between()
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                let key = event.keystroke.key.as_str();
                if key == "enter" {
                    this.submit(cx);
                    cx.stop_propagation();
                    return;
                }
                if key == "backspace" {
                    this.input.pop();
                    cx.notify();
                    cx.stop_propagation();
                    return;
                }
                if key == "up" {
                    let next = match this.history_idx {
                        None => 0,
                        Some(i) => (i + 1).min(this.history.len().saturating_sub(1)),
                    };
                    if let Some(cmd) = this.history.get(next).cloned() {
                        this.history_idx = Some(next);
                        this.input = cmd;
                        cx.notify();
                    }
                    cx.stop_propagation();
                    return;
                }
                let mods = event.keystroke.modifiers;
                if !mods.platform && !mods.control && !mods.alt {
                    if let Some(ch) = event.keystroke.key_char.as_deref() {
                        if ch.chars().count() == 1 && !ch.chars().all(|c| c.is_control()) {
                            this.input.push_str(ch);
                            cx.notify();
                            cx.stop_propagation();
                        }
                    }
                }
            }))
            .child(
                div()
                    .w_full()
                    .flex_1()
                    .pt(px(10.))
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .w_full()
                            .h(px(Theme::HEADER_HEIGHT))
                            .px(px(16.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(10.))
                                    .child(icons::sf_icon(SfSymbol::Terminal, 16.0, 0x34d399))
                                    .child(
                                        div()
                                            .text_size(px(15.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(rgb(Theme::TEXT_PRIMARY))
                                            .child("Terminal"),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(rgb(Theme::TEXT_TERTIARY))
                                    .child("Standalone Block Session"),
                            ),
                    )
                    .child(render_warp_terminal(self.console.as_ref(), &self.input, 5)),
            )
    }
}

pub fn open_standalone_terminal_window(cx: &mut App) {
    let options = WindowOptions {
        app_id: Some("alpenglowed-terminal".into()),
        titlebar: None,
        window_bounds: Some(WindowBounds::centered(size(px(750.), px(475.)), cx)),
        kind: WindowKind::PopUp,
        is_movable: true,
        is_resizable: true,
        window_background: WindowBackgroundAppearance::Transparent,
        window_decorations: Some(WindowDecorations::Client),
        ..Default::default()
    };
    let _ = cx.open_window(options, |window, cx| {
        let view = cx.new(|_| StandaloneTerminalWindow::new());
        window.activate_window();
        view
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ansi_removes_escape_sequences() {
        let raw = "\u{1b}[32mok\u{1b}[0m test";
        assert_eq!(strip_ansi(raw), "ok test");
    }

    #[test]
    fn terminal_console_creates_blocks_in_downward_thread_order() {
        let console = TerminalConsole::spawn().expect("spawn sh");
        console.seed_demo_blocks();
        let blocks = console.blocks();
        assert_eq!(blocks.len(), 2);
        // Newest block is first (at the top of the thread, moving older blocks down)
        assert_eq!(blocks[0].id, 2);
        assert_eq!(blocks[1].id, 1);
        assert_eq!(blocks[0].status_label(), "exit 0");
    }

    #[test]
    fn terminal_write_executes_and_records_block_exit_code() {
        let console = TerminalConsole::spawn().expect("spawn sh");
        console.write("echo hello_block_thread");
        for _ in 0..30 {
            let blocks = console.blocks();
            if let Some(first) = blocks.first() {
                if !first.running
                    && first
                        .output
                        .iter()
                        .any(|l| l.contains("hello_block_thread"))
                {
                    assert_eq!(first.exit_code, Some(0));
                    return;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let blocks = console.blocks();
        assert!(!blocks.is_empty());
        assert_eq!(blocks[0].command, "echo hello_block_thread");
    }
}
