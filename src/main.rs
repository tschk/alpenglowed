extern crate crepuscularity_gpui as gpui;

#[allow(dead_code)]
mod browser;
#[allow(dead_code)]
mod components;
#[cfg(feature = "compositor")]
mod compositor;
mod config;
mod de;
#[allow(dead_code)]
mod icons;
#[allow(dead_code)]
mod layout;
mod notifications;
#[allow(dead_code)]
mod plugin;
mod role;
mod runner;
mod session;
#[allow(dead_code)]
mod terminal;

use crepuscularity_core::context::{TemplateContext, TemplateValue};
use crepuscularity_gpui::prelude::*;
use crepuscularity_gpui::{
    actions, font, size, AnyWindowHandle, Div, EventEmitter, Font, KeyBinding, KeyDownEvent,
    WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowKind, WindowOptions,
};
use crepuscularity_web::render_component_file_to_html;
use layout::LayoutState;
use plugin::{PluginAction, WindowTarget};
use runner::{Runner, WindowMode};
use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;

const GEIST: &[u8] = include_bytes!("../assets/fonts/geist-regular.ttf");
const GEIST_MONO: &[u8] = include_bytes!("../assets/fonts/geist-mono-regular.ttf");

fn ui_font() -> Font {
    font("Geist")
}

const SURFACE: u32 = 0x161618;
const SURFACE_2: u32 = 0x1e1e22;
const SURFACE_3: u32 = 0x28282d;
const BORDER: u32 = 0x38383f;
const BORDER_SOFT: u32 = 0x26262b;
const TEXT: u32 = 0xffffff;
const TEXT_DIM: u32 = 0x9a9aa0;
const TEXT_FAINT: u32 = 0x68686e;

// Palette Theme Tokens (white alpha ramp over dark scrim)
const TC_SCRIM: u32 = 0x161618;
const TC_PANEL_STROKE: u32 = 0x26262b;
const TC_GLASS_BG: u32 = 0x242429; // controlSurface / glass capsule
const TC_BORDER: u32 = 0x3e3e46; // white 0.20 outline border
const TC_CARD_FILL: u32 = 0x1f1f24; // white 0.05 surface fill
const TC_CARD_STROKE: u32 = 0x2b2b31; // white 0.10 separator / hairline
const TC_ICON_SURFACE: u32 = 0x28282f; // white 0.09 flat symbol tile
const TC_TEXT_PRIMARY: u32 = 0xffffff; // white 1.00
const TC_TEXT_SECONDARY: u32 = 0x9a9aa0; // white 0.60
const TC_TEXT_TERTIARY: u32 = 0x68686e; // white 0.40
use std::process::Command;

const SHELL_CREPUS: &str = include_str!("views/shell.crepus");

actions!(
    alpenglowed,
    [
        Quit,
        FocusBar,
        DefocusBar,
        Confirm,
        ToggleActionsMenu,
        SplitRow,
        SplitColumn,
        FlipAxis,
        ResetLayout,
        NudgeLeft,
        NudgeRight,
        NudgeUp,
        NudgeDown,
        ExpandWindow,
        ContractWindow,
        GrowPane,
        ShrinkPane,
        FocusNextPane,
        FocusPreviousPane,
        ClosePane,
        ToggleFloatPane,
        ToggleTerminalPane,
        ToggleSettingsWindow,
        ToggleStatusBarAction,
        CycleWindowModeAction,
        ClearQueryAction,
        BalancePanesAction,
        CenterFocusedAction,
        MoveWindowLeft,
        MoveWindowRight,
        MoveWindowUp,
        MoveWindowDown,
        FocusFirstPane,
        FocusLastPane,
        SelectResult1,
        SelectResult2,
        SelectResult3,
        SelectResult4,
        SelectResult5,
        SelectResult6,
        SelectResult7,
        SelectResult8
    ]
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Main,
    WindowControls,
    Browser,
    Terminal,
    ClipboardHistory,
    FileSearch,
}

impl ViewMode {
    pub fn in_submenu(self) -> bool {
        !matches!(self, Self::Main)
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Main => "Launcher",
            Self::WindowControls => "Window Management",
            Self::Browser => "Quicklinks",
            Self::Terminal => "Terminal",
            Self::ClipboardHistory => "Clipboard History",
            Self::FileSearch => "Search Files",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RowOffsets {
    pub has_terminal: bool,
    pub has_macro: bool,
    pub has_url: bool,
    pub has_calc: bool,
    pub has_win: bool,
    pub has_help: bool,
    pub command_offset: usize,
}

impl RowOffsets {
    pub fn compute(query: &str, view_mode: ViewMode, extras: bool) -> Self {
        let trimmed = query.trim();
        if !matches!(view_mode, ViewMode::Main) {
            return Self::default();
        }
        let has_terminal = trimmed.starts_with('>') || trimmed.starts_with("! ");
        if has_terminal {
            return Self {
                has_terminal: true,
                has_macro: false,
                has_url: false,
                has_calc: false,
                has_win: false,
                has_help: false,
                command_offset: 1,
            };
        }
        let has_macro = plugin::resolve_text_macro(trimmed).is_some();
        let has_url = !has_macro && extras && plugin::parse_browser_command(trimmed).is_some();
        let has_calc = plugin::evaluate_calculator(trimmed).is_some();
        let lower = trimmed.to_lowercase();
        let has_win = !trimmed.is_empty()
            && (lower.contains("niri")
                || lower.contains("room")
                || lower.contains("column")
                || lower.contains("strip")
                || lower.contains("mode")
                || lower.contains("window")
                || lower.contains("tile")
                || lower.contains("float"));
        let has_help = plugin::help_answer_for(trimmed).is_some();
        let command_offset = usize::from(has_macro)
            + usize::from(has_url)
            + usize::from(has_calc)
            + usize::from(has_win)
            + usize::from(has_help);
        Self {
            has_terminal: false,
            has_macro,
            has_url,
            has_calc,
            has_win,
            has_help,
            command_offset,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct QueryHistory {
    entries: Vec<String>,
    cursor: Option<usize>,
    saved_prefix: String,
}

impl QueryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, query: &str) {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return;
        }
        self.entries.retain(|q| q != trimmed);
        self.entries.insert(0, trimmed.to_string());
        if self.entries.len() > 64 {
            self.entries.truncate(64);
        }
        self.cursor = None;
    }

    pub fn reset_cursor(&mut self) {
        self.cursor = None;
    }

    pub fn navigate_up(&mut self, current_query: &str) -> Option<String> {
        let start = match self.cursor {
            None => {
                self.saved_prefix = current_query.to_string();
                0
            }
            Some(idx) => idx + 1,
        };
        let prefix = self.saved_prefix.as_str();
        for (idx, entry) in self.entries.iter().enumerate().skip(start) {
            if prefix.is_empty() || entry.starts_with(prefix) {
                self.cursor = Some(idx);
                return Some(entry.clone());
            }
        }
        None
    }

    pub fn navigate_down(&mut self) -> Option<String> {
        let current = self.cursor?;
        if current == 0 {
            self.cursor = None;
            return Some(self.saved_prefix.clone());
        }
        let prefix = self.saved_prefix.as_str();
        for idx in (0..current).rev() {
            if let Some(entry) = self.entries.get(idx) {
                if prefix.is_empty() || entry.starts_with(prefix) {
                    self.cursor = Some(idx);
                    return Some(entry.clone());
                }
            }
        }
        self.cursor = None;
        Some(self.saved_prefix.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemAction {
    pub id: &'static str,
    pub label: &'static str,
    pub shortcut: &'static str,
    pub keycaps: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoverBadge {
    Terminal,
    Url,
    Calc,
    Window,
    Help,
    Submenu,
    Command,
}

impl RoverBadge {
    pub fn label(self) -> Option<&'static str> {
        match self {
            Self::Terminal => Some("$"),
            Self::Url => Some("URL"),
            Self::Calc => Some("="),
            Self::Window => Some("WIN"),
            Self::Help => Some("?"),
            Self::Submenu => Some("›"),
            Self::Command => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoverPillAction {
    Plugin(PluginAction),
    EnterSubmenu(ViewMode),
    Browser(plugin::BrowserCommand),
    RunInlineTerminal(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoverPill {
    pub badge: RoverBadge,
    pub title: String,
    pub subtitle: String,
    pub section: &'static str,
    pub kind_label: &'static str,
    pub icon_glyph: &'static str,
    pub icon_bg: u32,
    pub running: bool,
    pub keycaps: Vec<&'static str>,
    pub action: RoverPillAction,
}

impl RoverPill {
    pub fn primary_verb(&self) -> &'static str {
        match self.kind_label {
            "Application" => "Open Application",
            "Snippet" => "Paste Snippet",
            "Quicklink" => "Open in Browser",
            "Calculator" => "Copy Answer",
            _ => "Open Command",
        }
    }
}

fn clean_display_title(raw: &str) -> String {
    let trimmed = raw
        .strip_prefix("Window: ")
        .or_else(|| raw.strip_prefix("Niri: "))
        .or_else(|| raw.strip_prefix("Room: "))
        .or_else(|| raw.strip_prefix("Quicklink: "))
        .or_else(|| raw.strip_prefix("Snippet: "))
        .unwrap_or(raw);
    match trimmed {
        "Switch / Cycle Layout" => "Switch Room".to_string(),
        "Niri strip windows" => "Column Strip Layout".to_string(),
        other => other.to_string(),
    }
}

fn classify_plugin_row(
    title: &str,
    action: &PluginAction,
    active_mode: WindowMode,
) -> (
    &'static str,
    &'static str,
    &'static str,
    u32,
    bool,
    Vec<&'static str>,
) {
    match action {
        PluginAction::Layout { action: act } => match act {
            layout::LayoutAction::CycleRoomLayout => (
                "Favorites",
                "Window",
                "⊞",
                TC_ICON_SURFACE,
                false,
                vec!["Super", "Space"],
            ),
            layout::LayoutAction::EnterRoomFocus
            | layout::LayoutAction::EnterRoomStack
            | layout::LayoutAction::EnterRoomColumns
            | layout::LayoutAction::EnterRoomGrid => (
                "Commands",
                "Window",
                "⊞",
                TC_ICON_SURFACE,
                false,
                Vec::new(),
            ),
            layout::LayoutAction::CycleColumnWidth => (
                "Favorites",
                "Window",
                "↔",
                TC_ICON_SURFACE,
                false,
                vec!["Super", "R"],
            ),
            layout::LayoutAction::FocusColumnLeft => (
                "Commands",
                "Window",
                "←",
                TC_ICON_SURFACE,
                false,
                vec!["Super", "H"],
            ),
            layout::LayoutAction::FocusColumnRight => (
                "Commands",
                "Window",
                "→",
                TC_ICON_SURFACE,
                false,
                vec!["Super", "L"],
            ),
            layout::LayoutAction::SnapLeftHalf => (
                "Favorites",
                "Window",
                "◧",
                TC_ICON_SURFACE,
                false,
                vec!["Ctrl", "Alt", "←"],
            ),
            layout::LayoutAction::SnapRightHalf => (
                "Favorites",
                "Window",
                "◨",
                TC_ICON_SURFACE,
                false,
                vec!["Ctrl", "Alt", "→"],
            ),
            layout::LayoutAction::SnapTopHalf => (
                "Commands",
                "Window",
                "⬒",
                TC_ICON_SURFACE,
                false,
                vec!["Ctrl", "Alt", "↑"],
            ),
            layout::LayoutAction::SnapBottomHalf => (
                "Commands",
                "Window",
                "⬓",
                TC_ICON_SURFACE,
                false,
                vec!["Ctrl", "Alt", "↓"],
            ),
            layout::LayoutAction::Maximize => (
                "Favorites",
                "Window",
                "⤢",
                TC_ICON_SURFACE,
                false,
                vec!["Ctrl", "Alt", "↵"],
            ),
            layout::LayoutAction::AlmostMaximize => (
                "Commands",
                "Window",
                "▣",
                TC_ICON_SURFACE,
                false,
                Vec::new(),
            ),
            layout::LayoutAction::ReasonableSize => (
                "Commands",
                "Window",
                "⊡",
                TC_ICON_SURFACE,
                false,
                Vec::new(),
            ),
            layout::LayoutAction::Restore => (
                "Commands",
                "Window",
                "↺",
                TC_ICON_SURFACE,
                false,
                vec!["Ctrl", "Alt", "⌫"],
            ),
            layout::LayoutAction::CenterFocused => (
                "Commands",
                "Window",
                "⌖",
                TC_ICON_SURFACE,
                false,
                vec!["Ctrl", "Alt", "C"],
            ),
            _ => (
                "Commands",
                "Window",
                "⊞",
                TC_ICON_SURFACE,
                false,
                Vec::new(),
            ),
        },
        PluginAction::SetWindowMode { mode } => {
            let is_active = *mode == active_mode;
            (
                "Commands",
                "Window",
                "▥",
                TC_ICON_SURFACE,
                is_active,
                Vec::new(),
            )
        }
        PluginAction::CycleWindowMode => (
            "Commands",
            "Window",
            "↻",
            TC_ICON_SURFACE,
            false,
            Vec::new(),
        ),
        PluginAction::FocusWindow { .. } => {
            ("Commands", "Window", "▣", TC_ICON_SURFACE, true, Vec::new())
        }
        PluginAction::Desktop { action: de_act } => match de_act {
            de::DesktopAction::Terminal => (
                "Applications",
                "Application",
                ">_",
                0x1f2937,
                true,
                Vec::new(),
            ),
            de::DesktopAction::Files => (
                "Applications",
                "Application",
                "📁",
                0x2563eb,
                false,
                Vec::new(),
            ),
            de::DesktopAction::Apps => (
                "Applications",
                "Application",
                "✦",
                0x7c3aed,
                true,
                Vec::new(),
            ),
            _ => (
                "Commands",
                "Command",
                "⚙",
                TC_ICON_SURFACE,
                false,
                Vec::new(),
            ),
        },
        PluginAction::Launch { .. } => (
            "Applications",
            "Application",
            "◆",
            0x3b82f6,
            true,
            Vec::new(),
        ),
        PluginAction::Shell { .. } => {
            if title.starts_with('=') {
                ("Results", "Calculator", "=", 0xf59e0b, false, Vec::new())
            } else if title.starts_with("Snippet:") {
                (
                    "Snippets",
                    "Snippet",
                    "{}",
                    TC_ICON_SURFACE,
                    false,
                    Vec::new(),
                )
            } else if title.starts_with("Quicklink:") || title.starts_with("Search ") {
                (
                    "Quicklinks",
                    "Quicklink",
                    "🔗",
                    TC_ICON_SURFACE,
                    false,
                    Vec::new(),
                )
            } else {
                (
                    "Commands",
                    "Command",
                    ">_",
                    TC_ICON_SURFACE,
                    false,
                    Vec::new(),
                )
            }
        }
        _ => (
            "Commands",
            "Command",
            "⌘",
            TC_ICON_SURFACE,
            false,
            Vec::new(),
        ),
    }
}

#[derive(Clone)]
struct UiOptions {
    status_bar: bool,
    external_polybar: bool,
    open_settings: bool,
    initial_query: String,
    mode: WindowMode,
    demo_layout: bool,
    role: role::SessionRole,
    initial_view_mode: ViewMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BarCategory {
    All,
    Windows,
    Modes,
    System,
    Clipboard,
    Notes,
    Terminal,
}

impl BarCategory {
    fn all() -> &'static [Self] {
        &[
            Self::All,
            Self::Windows,
            Self::Modes,
            Self::System,
            Self::Clipboard,
            Self::Notes,
            Self::Terminal,
        ]
    }

    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Windows => "Windows",
            Self::Modes => "Modes",
            Self::System => "System",
            Self::Clipboard => "Clipboard",
            Self::Notes => "Notes",
            Self::Terminal => "Terminal",
        }
    }

    fn next(self) -> Self {
        let all = Self::all();
        let idx = all.iter().position(|c| *c == self).unwrap_or(0);
        all[(idx + 1) % all.len()]
    }
}

#[derive(Clone, Copy)]
enum DesktopEvent {
    Changed,
}

struct DesktopModel {
    query: String,
    mode: WindowMode,
    layout: LayoutState,
    status_bar: bool,
    external_polybar: bool,
    last_action: String,
    runner: Runner,
    #[allow(dead_code)]
    session_control: bool,
    launcher: Option<AnyWindowHandle>,
    active_category: BarCategory,
    view_mode: ViewMode,
    offsets: RowOffsets,
    pills: Vec<RoverPill>,
    selected_pill: usize,
    show_actions: bool,
    show_app_menu: bool,
    selected_action: usize,
    history: QueryHistory,
    browser: plugin::BrowserTransportState,
    browser_page: browser::InternalBrowserPage,
    browser_engine: browser::BrowserEngineStatus,
    notifications: notifications::NotificationState,
    terminal: Option<terminal::TerminalConsole>,
    terminal_open: bool,
    role: role::SessionRole,
    #[cfg(feature = "compositor")]
    compositor_events: Option<std::sync::mpsc::Receiver<compositor::CompositorEvent>>,
    #[cfg(feature = "compositor")]
    compositor_surfaces: std::collections::HashMap<u32, usize>,
}

impl EventEmitter<DesktopEvent> for DesktopModel {}

#[derive(serde::Serialize, serde::Deserialize)]
struct ExternalBarState {
    focused_title: String,
    layout: String,
    mode: String,
    last_action: String,
}

impl DesktopModel {
    fn new(
        options: UiOptions,
        #[cfg(feature = "compositor")] compositor_rx: Option<
            std::sync::mpsc::Receiver<compositor::CompositorEvent>,
        >,
        #[cfg(feature = "compositor")] _compositor_tx: Option<
            std::sync::mpsc::Sender<compositor::CompositorCommand>,
        >,
    ) -> Self {
        let mut notifications = notifications::NotificationState::new();
        notifications.start();
        let initial_category = if options.open_settings {
            BarCategory::System
        } else {
            BarCategory::All
        };
        let mut browser = plugin::BrowserTransportState::new();
        let initial_url = "https://docs.rs".to_string();
        let _ = browser.execute(plugin::BrowserCommand::Navigate(initial_url.clone()));
        let mut desktop = Self {
            query: options.initial_query,
            mode: options.mode,
            layout: {
                let mut layout = if options.demo_layout {
                    LayoutState::demo()
                } else {
                    LayoutState::new()
                };
                layout.set_workarea(1440.0, 900.0);
                layout.set_window_mode(&options.mode);
                if options.demo_layout && matches!(options.mode, WindowMode::Tiling) {
                    layout.set_window_floating(4, true);
                }
                layout
            },
            status_bar: options.status_bar,
            external_polybar: options.external_polybar,
            last_action: "Ready: Alpenglow Wayland Desktop".to_string(),
            runner: Runner::with_role(options.role),
            session_control: std::env::var_os("ALPENGLOW_SESSION_CONTROL").is_some(),
            role: options.role,
            launcher: None,
            active_category: initial_category,
            view_mode: options.initial_view_mode,
            offsets: RowOffsets::default(),
            pills: Vec::new(),
            selected_pill: 0,
            show_actions: matches!(
                std::env::var("ALPENGLOWED_SHOW_ACTIONS").as_deref(),
                Ok("1" | "true")
            ),
            show_app_menu: matches!(
                std::env::var("ALPENGLOWED_SHOW_APP_MENU").as_deref(),
                Ok("1" | "true")
            ),
            selected_action: 0,
            history: QueryHistory::new(),
            browser,
            browser_page: browser::load_internal_page(&initial_url),
            browser_engine: browser::detect_browser_engine(),
            notifications,
            terminal: None,
            terminal_open: matches!(options.initial_view_mode, ViewMode::Terminal),
            #[cfg(feature = "compositor")]
            compositor_events: compositor_rx,
            #[cfg(feature = "compositor")]
            compositor_surfaces: std::collections::HashMap::new(),
        };
        if desktop.terminal_open {
            desktop.terminal = terminal::TerminalConsole::spawn();
            if let Some(ref term) = desktop.terminal {
                if term.blocks().is_empty() {
                    term.seed_demo_blocks();
                }
            }
        }
        desktop.runner.query = desktop.query.clone();
        desktop.refresh_runner();
        desktop
    }

    fn set_query(&mut self, query: String, cx: &mut Context<Self>) {
        self.query = query.clone();
        self.history.reset_cursor();
        self.runner.query = match self.view_mode {
            ViewMode::ClipboardHistory if query.trim().is_empty() => "clipboard".to_string(),
            ViewMode::FileSearch if !query.trim().starts_with('/') => format!("/{}", query.trim()),
            _ => query,
        };
        self.refresh_runner();
        self.changed(cx);
    }

    fn set_view_mode(&mut self, mode: ViewMode, cx: &mut Context<Self>) {
        self.view_mode = mode;
        self.show_actions = false;
        self.show_app_menu = false;
        self.selected_action = 0;
        self.active_category = match mode {
            ViewMode::Main => BarCategory::All,
            ViewMode::WindowControls => BarCategory::Windows,
            ViewMode::Browser => BarCategory::All,
            ViewMode::Terminal => BarCategory::Terminal,
            ViewMode::ClipboardHistory => BarCategory::Clipboard,
            ViewMode::FileSearch => BarCategory::All,
        };
        self.terminal_open = matches!(mode, ViewMode::Terminal);
        if self.terminal_open && self.terminal.is_none() {
            self.terminal = terminal::TerminalConsole::spawn();
            if let Some(ref term) = self.terminal {
                if term.blocks().is_empty() {
                    term.seed_demo_blocks();
                }
            }
        }
        if matches!(mode, ViewMode::Browser) {
            self.browser_page = browser::load_internal_page(self.browser.active_url());
        } else {
            browser::detach_native_webview();
        }
        self.query.clear();
        self.runner.query = match mode {
            ViewMode::ClipboardHistory => "clipboard".to_string(),
            ViewMode::FileSearch => "/".to_string(),
            _ => String::new(),
        };
        self.refresh_runner();
        self.changed(cx);
    }

    fn set_category(&mut self, category: BarCategory, cx: &mut Context<Self>) {
        self.active_category = category;
        self.view_mode = match category {
            BarCategory::Windows => ViewMode::WindowControls,
            BarCategory::Terminal => ViewMode::Terminal,
            BarCategory::Clipboard => ViewMode::ClipboardHistory,
            _ => ViewMode::Main,
        };
        self.terminal_open = matches!(category, BarCategory::Terminal);
        if self.terminal_open && self.terminal.is_none() {
            self.terminal = terminal::TerminalConsole::spawn();
            if let Some(ref term) = self.terminal {
                if term.blocks().is_empty() {
                    term.seed_demo_blocks();
                }
            }
        }
        let preset_query = match category {
            BarCategory::Clipboard => Some("clipboard".to_string()),
            BarCategory::Notes => Some("note".to_string()),
            _ => None,
        };
        if let Some(q) = preset_query {
            self.query = q.clone();
            self.runner.query = q;
        }
        self.last_action = format!("Category: {}", category.label());
        self.refresh_runner();
        self.changed(cx);
    }

    fn set_last_action(&mut self, title: impl Into<String>, detail: impl Into<String>) {
        let title = title.into();
        let detail = detail.into();
        self.last_action = format!("{title}: {detail}");
        self.layout.set_focused_window_content(title, detail);
    }

    fn set_action_log(&mut self, title: impl Into<String>, detail: impl Into<String>) {
        self.last_action = format!("{}: {}", title.into(), detail.into());
    }

    fn focus_targets(&self) -> Vec<WindowTarget> {
        self.layout
            .windows()
            .into_iter()
            .map(|window| WindowTarget {
                id: window.id,
                title: window.title,
                focused: window.focused,
                floating: window.floating,
            })
            .collect()
    }

    fn refresh_runner(&mut self) {
        let windows = self.focus_targets();
        self.runner.update_with_windows(&windows);
        self.rebuild_pills();
    }

    fn rebuild_pills(&mut self) {
        let trimmed = self.query.trim();
        self.offsets = RowOffsets::compute(trimmed, self.view_mode, self.role.extras());
        let mut pills = Vec::new();
        let is_filtering = !trimmed.is_empty();

        if matches!(self.view_mode, ViewMode::WindowControls) && trimmed.is_empty() {
            let win_rows: &[(&str, &str, &str, Vec<&str>, layout::LayoutAction)] = &[
                (
                    "Left Half",
                    "Favorites",
                    "◧",
                    vec!["Ctrl", "Alt", "←"],
                    layout::LayoutAction::SnapLeftHalf,
                ),
                (
                    "Right Half",
                    "Favorites",
                    "◨",
                    vec!["Ctrl", "Alt", "→"],
                    layout::LayoutAction::SnapRightHalf,
                ),
                (
                    "Maximize",
                    "Favorites",
                    "⤢",
                    vec!["Ctrl", "Alt", "↵"],
                    layout::LayoutAction::Maximize,
                ),
                (
                    "Center",
                    "Window Management",
                    "⌖",
                    vec!["Ctrl", "Alt", "C"],
                    layout::LayoutAction::CenterFocused,
                ),
                (
                    "Restore Window",
                    "Window Management",
                    "↺",
                    vec!["Ctrl", "Alt", "⌫"],
                    layout::LayoutAction::Restore,
                ),
                (
                    "Switch Room",
                    "Window Management",
                    "⊞",
                    vec!["Super", "Space"],
                    layout::LayoutAction::CycleRoomLayout,
                ),
                (
                    "Cycle Column Width",
                    "Window Management",
                    "↔",
                    vec!["Super", "R"],
                    layout::LayoutAction::CycleColumnWidth,
                ),
                (
                    "Focus Left Column",
                    "Window Management",
                    "←",
                    vec!["Super", "H"],
                    layout::LayoutAction::FocusColumnLeft,
                ),
            ];
            for (title, sec, glyph, keys, act) in win_rows {
                pills.push(RoverPill {
                    badge: RoverBadge::Command,
                    title: (*title).to_string(),
                    subtitle: String::new(),
                    section: sec,
                    kind_label: "Window",
                    icon_glyph: glyph,
                    icon_bg: TC_ICON_SURFACE,
                    running: false,
                    keycaps: keys.clone(),
                    action: RoverPillAction::Plugin(PluginAction::Layout {
                        action: act.clone(),
                    }),
                });
            }
            self.pills = pills;
            self.selected_pill = 0;
            return;
        }

        if matches!(self.view_mode, ViewMode::Browser) && trimmed.is_empty() {
            let bang_rows: &[(&str, &str, &str)] = &[
                ("Search DuckDuckGo", "!ddg", "https://duckduckgo.com"),
                ("Search GitHub", "!gh", "https://github.com"),
                ("Search Rust Docs", "!rs", "https://docs.rs"),
                ("Search Crates", "!crates", "https://crates.io"),
                ("Search Wikipedia", "!wiki", "https://en.wikipedia.org"),
                ("Search YouTube", "!yt", "https://www.youtube.com"),
                ("Search Arch Wiki", "!arch", "https://wiki.archlinux.org"),
            ];
            for (title, bang, url) in bang_rows {
                pills.push(RoverPill {
                    badge: RoverBadge::Url,
                    title: (*title).to_string(),
                    subtitle: (*bang).to_string(),
                    section: "Quicklinks",
                    kind_label: "Quicklink",
                    icon_glyph: "🔗",
                    icon_bg: TC_ICON_SURFACE,
                    running: false,
                    keycaps: Vec::new(),
                    action: RoverPillAction::Browser(plugin::BrowserCommand::Navigate(
                        (*url).to_string(),
                    )),
                });
            }
            for m in plugin::TEXT_MACROS.iter().take(4) {
                let expanded = plugin::expand_macro_template(m.template, "");
                let preview = expanded.replace('\n', " ");
                let copy_cmd = format!("printf %s {}", plugin::shell_single_quote(&expanded));
                pills.push(RoverPill {
                    badge: RoverBadge::Command,
                    title: m.name.to_string(),
                    subtitle: format!("{}  {preview}", m.bang),
                    section: "Snippets",
                    kind_label: "Snippet",
                    icon_glyph: "{}",
                    icon_bg: TC_ICON_SURFACE,
                    running: false,
                    keycaps: Vec::new(),
                    action: RoverPillAction::Plugin(PluginAction::Shell { command: copy_cmd }),
                });
            }
            self.pills = pills;
            self.selected_pill = 0;
            return;
        }

        if self.offsets.has_terminal {
            let cmd = trimmed
                .strip_prefix('>')
                .or_else(|| trimmed.strip_prefix('!'))
                .map(str::trim)
                .unwrap_or("");
            let display_cmd = if cmd.is_empty() {
                "Run Shell Command".to_string()
            } else {
                cmd.to_string()
            };
            pills.push(RoverPill {
                badge: RoverBadge::Terminal,
                title: display_cmd,
                subtitle: String::new(),
                section: "Results",
                kind_label: "Command",
                icon_glyph: ">_",
                icon_bg: TC_ICON_SURFACE,
                running: false,
                keycaps: Vec::new(),
                action: RoverPillAction::RunInlineTerminal(cmd.to_string()),
            });
            self.pills = pills;
            self.selected_pill = 0;
            return;
        }

        if self.offsets.has_macro {
            if let Some((name, bang, expanded)) = plugin::resolve_text_macro(trimmed) {
                let preview = expanded.replace('\n', " ");
                let copy_cmd = format!("printf %s {}", plugin::shell_single_quote(&expanded));
                pills.push(RoverPill {
                    badge: RoverBadge::Command,
                    title: name,
                    subtitle: format!("{bang}  {preview}"),
                    section: "Snippets",
                    kind_label: "Snippet",
                    icon_glyph: "{}",
                    icon_bg: TC_ICON_SURFACE,
                    running: false,
                    keycaps: Vec::new(),
                    action: RoverPillAction::Plugin(PluginAction::Shell { command: copy_cmd }),
                });
            }
        } else if trimmed.starts_with('!') || trimmed.starts_with(';') {
            let prefix = trimmed.trim_start_matches(['!', ';']).to_lowercase();
            for m in plugin::TEXT_MACROS {
                let key = m.bang.trim_start_matches('!');
                if prefix.is_empty()
                    || key.starts_with(&prefix)
                    || m.name.to_lowercase().contains(&prefix)
                {
                    let expanded = plugin::expand_macro_template(m.template, "");
                    let preview = expanded.replace('\n', " ");
                    let copy_cmd = format!("printf %s {}", plugin::shell_single_quote(&expanded));
                    pills.push(RoverPill {
                        badge: RoverBadge::Command,
                        title: m.name.to_string(),
                        subtitle: format!("{}  {preview}", m.bang),
                        section: "Snippets",
                        kind_label: "Snippet",
                        icon_glyph: "{}",
                        icon_bg: TC_ICON_SURFACE,
                        running: false,
                        keycaps: Vec::new(),
                        action: RoverPillAction::Plugin(PluginAction::Shell { command: copy_cmd }),
                    });
                }
            }
            let bang_rows: &[(&str, &str, &str)] = &[
                ("Search Google", "!g", "https://www.google.com"),
                ("Search DuckDuckGo", "!ddg", "https://duckduckgo.com"),
                ("Search GitHub", "!gh", "https://github.com"),
                ("Search Rust Docs", "!rs", "https://docs.rs"),
                ("Search Crates", "!crates", "https://crates.io"),
                ("Search Wikipedia", "!wiki", "https://en.wikipedia.org"),
                ("Search YouTube", "!yt", "https://www.youtube.com"),
                ("Search Arch Wiki", "!arch", "https://wiki.archlinux.org"),
            ];
            for (title, bang, url) in bang_rows {
                let key = bang.trim_start_matches('!');
                if prefix.is_empty() || key.starts_with(&prefix) {
                    pills.push(RoverPill {
                        badge: RoverBadge::Url,
                        title: (*title).to_string(),
                        subtitle: (*bang).to_string(),
                        section: "Quicklinks",
                        kind_label: "Quicklink",
                        icon_glyph: "🔗",
                        icon_bg: TC_ICON_SURFACE,
                        running: false,
                        keycaps: Vec::new(),
                        action: RoverPillAction::Browser(plugin::BrowserCommand::Navigate(
                            (*url).to_string(),
                        )),
                    });
                }
            }
            if !pills.is_empty() && !self.offsets.has_url {
                self.pills = pills;
                self.selected_pill = 0;
                return;
            }
        }

        if self.offsets.has_url {
            if let Some(cmd) = plugin::parse_browser_command(trimmed) {
                let (title, subtitle) = match &cmd {
                    plugin::BrowserCommand::Navigate(url) => {
                        ("Open in Browser".to_string(), url.clone())
                    }
                    plugin::BrowserCommand::Search { engine_name, query } => {
                        (format!("Search {engine_name}"), query.clone())
                    }
                    _ => ("Open in Browser".to_string(), String::new()),
                };
                pills.push(RoverPill {
                    badge: RoverBadge::Url,
                    title,
                    subtitle,
                    section: "Quicklinks",
                    kind_label: "Quicklink",
                    icon_glyph: "🔗",
                    icon_bg: TC_ICON_SURFACE,
                    running: false,
                    keycaps: Vec::new(),
                    action: RoverPillAction::Browser(cmd),
                });
            }
        }

        if self.offsets.has_calc {
            if let Some(eval) = plugin::evaluate_calculator(trimmed) {
                let copy_cmd = format!("printf %s {}", plugin::shell_single_quote(&eval));
                pills.push(RoverPill {
                    badge: RoverBadge::Calc,
                    title: eval,
                    subtitle: trimmed.to_string(),
                    section: "Calculator",
                    kind_label: "Calculator",
                    icon_glyph: "=",
                    icon_bg: TC_ICON_SURFACE,
                    running: false,
                    keycaps: Vec::new(),
                    action: RoverPillAction::Plugin(PluginAction::Shell { command: copy_cmd }),
                });
            }
        }

        if self.offsets.has_win {
            pills.push(RoverPill {
                badge: RoverBadge::Window,
                title: "Window Management".to_string(),
                subtitle: String::new(),
                section: "Results",
                kind_label: "Command",
                icon_glyph: "⊞",
                icon_bg: TC_ICON_SURFACE,
                running: false,
                keycaps: vec!["Super", "W"],
                action: RoverPillAction::EnterSubmenu(ViewMode::WindowControls),
            });
        }

        if self.offsets.has_help {
            if let Some((answer, subtitle)) = plugin::help_answer_for(trimmed) {
                pills.push(RoverPill {
                    badge: RoverBadge::Help,
                    title: answer,
                    subtitle,
                    section: "Results",
                    kind_label: "Command",
                    icon_glyph: "?",
                    icon_bg: TC_ICON_SURFACE,
                    running: false,
                    keycaps: Vec::new(),
                    action: RoverPillAction::Plugin(PluginAction::None),
                });
            }
        }

        if matches!(self.view_mode, ViewMode::Main) && trimmed.is_empty() {
            pills.push(RoverPill {
                badge: RoverBadge::Command,
                title: "Left Half".to_string(),
                subtitle: String::new(),
                section: "Favorites",
                kind_label: "Window",
                icon_glyph: "◧",
                icon_bg: TC_ICON_SURFACE,
                running: false,
                keycaps: vec!["Ctrl", "Alt", "←"],
                action: RoverPillAction::Plugin(PluginAction::Layout {
                    action: layout::LayoutAction::SnapLeftHalf,
                }),
            });
            pills.push(RoverPill {
                badge: RoverBadge::Command,
                title: "Right Half".to_string(),
                subtitle: String::new(),
                section: "Favorites",
                kind_label: "Window",
                icon_glyph: "◨",
                icon_bg: TC_ICON_SURFACE,
                running: false,
                keycaps: vec!["Ctrl", "Alt", "→"],
                action: RoverPillAction::Plugin(PluginAction::Layout {
                    action: layout::LayoutAction::SnapRightHalf,
                }),
            });
            pills.push(RoverPill {
                badge: RoverBadge::Command,
                title: "Maximize".to_string(),
                subtitle: String::new(),
                section: "Favorites",
                kind_label: "Window",
                icon_glyph: "⤢",
                icon_bg: TC_ICON_SURFACE,
                running: false,
                keycaps: vec!["Ctrl", "Alt", "↵"],
                action: RoverPillAction::Plugin(PluginAction::Layout {
                    action: layout::LayoutAction::Maximize,
                }),
            });
            pills.push(RoverPill {
                badge: RoverBadge::Command,
                title: "Switch Room".to_string(),
                subtitle: String::new(),
                section: "Favorites",
                kind_label: "Window",
                icon_glyph: "⊞",
                icon_bg: TC_ICON_SURFACE,
                running: false,
                keycaps: vec!["Super", "Space"],
                action: RoverPillAction::Plugin(PluginAction::Layout {
                    action: layout::LayoutAction::CycleRoomLayout,
                }),
            });
            pills.push(RoverPill {
                badge: RoverBadge::Submenu,
                title: "Clipboard History".to_string(),
                subtitle: String::new(),
                section: "Suggestions",
                kind_label: "Command",
                icon_glyph: "📋",
                icon_bg: TC_ICON_SURFACE,
                running: false,
                keycaps: vec!["Super", "V"],
                action: RoverPillAction::EnterSubmenu(ViewMode::ClipboardHistory),
            });
            if self.role.extras() {
                pills.push(RoverPill {
                    badge: RoverBadge::Submenu,
                    title: "Search Quicklinks".to_string(),
                    subtitle: String::new(),
                    section: "Suggestions",
                    kind_label: "Command",
                    icon_glyph: "🔗",
                    icon_bg: TC_ICON_SURFACE,
                    running: false,
                    keycaps: vec!["Super", "B"],
                    action: RoverPillAction::EnterSubmenu(ViewMode::Browser),
                });
            }
            pills.push(RoverPill {
                badge: RoverBadge::Submenu,
                title: "Terminal".to_string(),
                subtitle: String::new(),
                section: "Suggestions",
                kind_label: "Application",
                icon_glyph: ">_",
                icon_bg: 0x1f2937,
                running: true,
                keycaps: vec!["Super", "J"],
                action: RoverPillAction::EnterSubmenu(ViewMode::Terminal),
            });
            pills.push(RoverPill {
                badge: RoverBadge::Submenu,
                title: "Search Files".to_string(),
                subtitle: String::new(),
                section: "Suggestions",
                kind_label: "Command",
                icon_glyph: "📁",
                icon_bg: TC_ICON_SURFACE,
                running: false,
                keycaps: vec!["Super", "F"],
                action: RoverPillAction::EnterSubmenu(ViewMode::FileSearch),
            });
            self.pills = pills;
            self.selected_pill = 0;
            return;
        }

        for result in self.runner.results.iter().take(8) {
            let (sec, kind_label, icon_glyph, icon_bg, running, keycaps) =
                classify_plugin_row(&result.title, &result.action, self.mode);
            let clean_title = clean_display_title(&result.title);
            let show_sub = if result.plugin_id == "snippets"
                || result.plugin_id == "quicklinks"
                || result.plugin_id == "dictionary"
            {
                result.subtitle.clone()
            } else {
                String::new()
            };
            pills.push(RoverPill {
                badge: RoverBadge::Command,
                title: clean_title,
                subtitle: show_sub,
                section: if is_filtering { "Results" } else { sec },
                kind_label: if kind_label == "Window Management" || kind_label == "Room" {
                    "Window"
                } else {
                    kind_label
                },
                icon_glyph,
                icon_bg,
                running,
                keycaps,
                action: RoverPillAction::Plugin(result.action.clone()),
            });
        }

        if self.selected_pill >= pills.len() {
            self.selected_pill = 0;
        }
        self.pills = pills;
    }

    fn live_hint(&self) -> String {
        let trimmed = self.query.trim();
        if let Some(pill) = self.pills.get(self.selected_pill) {
            return pill.title.clone();
        }
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
        self.mode.label().to_string()
    }

    pub fn actions_for_selected(&self) -> Vec<ItemAction> {
        if self.show_app_menu {
            return vec![
                ItemAction {
                    id: "open-windows",
                    label: "Window Management",
                    shortcut: "Super+W",
                    keycaps: vec!["Super", "W"],
                },
                ItemAction {
                    id: "room-cycle",
                    label: "Switch Room",
                    shortcut: "Super+Space",
                    keycaps: vec!["Super", "Space"],
                },
                ItemAction {
                    id: "open-browser",
                    label: "Search Quicklinks",
                    shortcut: "Super+B",
                    keycaps: vec!["Super", "B"],
                },
                ItemAction {
                    id: "open-terminal",
                    label: "Open Terminal",
                    shortcut: "Super+J",
                    keycaps: vec!["Super", "J"],
                },
            ];
        }

        vec![
            ItemAction {
                id: "execute",
                label: "Open",
                shortcut: "↵",
                keycaps: vec!["↵"],
            },
            ItemAction {
                id: "room-cycle",
                label: "Switch Room",
                shortcut: "Super+Space",
                keycaps: vec!["Super", "Space"],
            },
            ItemAction {
                id: "niri-cycle-width",
                label: "Cycle Column Width",
                shortcut: "Super+R",
                keycaps: vec!["Super", "R"],
            },
            ItemAction {
                id: "toggle-float",
                label: "Toggle Floating",
                shortcut: "Super+V",
                keycaps: vec!["Super", "V"],
            },
            ItemAction {
                id: "open-terminal",
                label: "Open Terminal",
                shortcut: "Super+J",
                keycaps: vec!["Super", "J"],
            },
        ]
    }

    fn execute_item_action(&mut self, action_id: &str, cx: &mut Context<Self>) {
        self.show_actions = false;
        self.show_app_menu = false;
        self.selected_action = 0;
        match action_id {
            "room-cycle" => self.apply(
                PluginAction::Layout {
                    action: layout::LayoutAction::CycleRoomLayout,
                },
                cx,
            ),
            "mode-niri" => self.apply(
                PluginAction::SetWindowMode {
                    mode: WindowMode::NiriStrip,
                },
                cx,
            ),
            "mode-floating" => self.apply(
                PluginAction::SetWindowMode {
                    mode: WindowMode::Floating,
                },
                cx,
            ),
            "niri-cycle-width" => self.apply(
                PluginAction::Layout {
                    action: layout::LayoutAction::CycleColumnWidth,
                },
                cx,
            ),
            "niri-consume" => self.apply(
                PluginAction::Layout {
                    action: layout::LayoutAction::ConsumeWindowIntoColumn,
                },
                cx,
            ),
            "niri-expel" => self.apply(
                PluginAction::Layout {
                    action: layout::LayoutAction::ExpelWindowFromColumn,
                },
                cx,
            ),
            "toggle-float" => self.apply(
                PluginAction::Layout {
                    action: layout::LayoutAction::ToggleFloat,
                },
                cx,
            ),
            "open-windows" => self.set_view_mode(ViewMode::WindowControls, cx),
            "open-browser" => self.set_view_mode(ViewMode::Browser, cx),
            "open-terminal" => self.set_view_mode(ViewMode::Terminal, cx),
            "browser-new-tab" => {
                self.browser.execute(plugin::BrowserCommand::NewTab);
                self.set_view_mode(ViewMode::Browser, cx);
            }
            "browser-bookmark" => {
                self.browser.execute(plugin::BrowserCommand::AddBookmark);
                self.last_action = format!("Bookmarked {}", self.browser.active_url());
                self.changed(cx);
            }
            "terminal-clear" => self.apply(PluginAction::TerminalClear, cx),
            "run-background" => {
                if let Some(RoverPill {
                    action: RoverPillAction::RunInlineTerminal(cmd),
                    ..
                }) = self.pills.get(self.selected_pill).cloned()
                {
                    if !cmd.is_empty() {
                        self.apply(PluginAction::Shell { command: cmd }, cx);
                    }
                }
            }
            _ => {
                if let Some(pill) = self.pills.get(self.selected_pill).cloned() {
                    self.execute_pill(pill, cx);
                }
            }
        }
    }

    fn execute_pill(&mut self, pill: RoverPill, cx: &mut Context<Self>) {
        if !self.query.trim().is_empty() {
            self.history.record(&self.query);
        }
        self.runner.record_recent(&pill.title);
        match pill.action {
            RoverPillAction::EnterSubmenu(mode) => {
                self.set_view_mode(mode, cx);
            }
            RoverPillAction::Browser(cmd) => {
                let _ = self.browser.execute(cmd);
                let active_url = self.browser.active_url().to_string();
                self.browser_page = browser::load_internal_page(&active_url);
                self.last_action = format!("Browser: {active_url}");
                self.set_view_mode(ViewMode::Browser, cx);
            }
            RoverPillAction::RunInlineTerminal(cmd) => {
                self.set_view_mode(ViewMode::Terminal, cx);
                if !cmd.is_empty() {
                    self.terminal_write(&cmd);
                    self.changed(cx);
                }
            }
            RoverPillAction::Plugin(action) => {
                self.apply(action, cx);
            }
        }
    }

    fn apply(&mut self, action: PluginAction, cx: &mut Context<Self>) {
        match action {
            PluginAction::FocusWindow { id } => {
                self.layout.focus_window(id);
                let focused = self.layout.focused_title().to_string();
                self.set_action_log("Focus window", focused);
                let _ = session::dispatch(&session::SessionRequest::FocusWindow { id });
            }
            PluginAction::SetWindowMode { mode } => {
                self.mode = mode;
                self.layout.set_window_mode(&self.mode);
                self.set_last_action("Window mode", self.mode.label());
                let _ =
                    session::dispatch(&session::SessionRequest::SetWindowMode { mode: self.mode });
            }
            PluginAction::CycleWindowMode => {
                self.mode = self.mode.next();
                self.layout.set_window_mode(&self.mode);
                self.set_last_action("Window mode", self.mode.label());
                let _ = session::dispatch(&session::SessionRequest::CycleWindowMode);
            }
            PluginAction::Layout { action } => {
                if let Some(parsed) = layout::LayoutAction::from_label(action.label()) {
                    self.layout.apply(&parsed);
                } else {
                    self.layout.apply(&action);
                }
                self.set_last_action(action.title(), self.layout.summary());
                let _ = session::dispatch(&session::SessionRequest::Layout { action });
            }
            PluginAction::ShowStatusBar => {
                self.status_bar = true;
                self.set_last_action("Status bar", "enabled");
            }
            PluginAction::HideStatusBar => {
                self.status_bar = false;
                self.set_last_action("Status bar", "disabled");
            }
            PluginAction::ToggleStatusBar => {
                self.toggle_status_bar(cx);
                self.set_last_action(
                    "Status bar",
                    if self.status_bar {
                        "enabled"
                    } else {
                        "disabled"
                    },
                );
            }
            PluginAction::ToggleSettings => {
                if self.active_category == BarCategory::System {
                    self.active_category = BarCategory::All;
                    self.set_last_action("Settings", "inline view closed");
                } else {
                    self.active_category = BarCategory::System;
                    self.set_last_action("Settings", "inline view opened");
                }
                focus_or_open_launcher(&cx.entity(), cx);
            }
            PluginAction::OpenSettings => {
                self.active_category = BarCategory::System;
                self.set_last_action("Settings", "inline view opened");
                focus_or_open_launcher(&cx.entity(), cx);
            }
            PluginAction::CloseSettings => {
                self.active_category = BarCategory::All;
                self.set_last_action("Settings", "inline view closed");
            }
            PluginAction::FactoryReset => {
                crate::config::Config::factory_reset();
                self.set_last_action(
                    "Factory reset",
                    "settings cleared; restart to restore defaults",
                );
            }
            PluginAction::Desktop { action } => {
                let resolved = action
                    .resolve()
                    .map(|command| command.display())
                    .unwrap_or_else(|| "no command available".to_string());
                if session::dispatch(&session::SessionRequest::DesktopAction {
                    action: action.clone(),
                })
                .is_ok()
                {
                    self.set_last_action(action.title(), format!("session {resolved}"));
                } else {
                    match de::run(&action) {
                        de::RunResult::Spawned(command) => self.set_last_action(
                            action.title(),
                            format!("local {}", command.display()),
                        ),
                        de::RunResult::MissingCommand => {
                            self.set_last_action(action.title(), "no command available")
                        }
                    }
                }
            }
            PluginAction::Launch { program } => {
                self.set_last_action(program.clone(), "app launch");
                let _ = Command::new(program).spawn();
            }
            PluginAction::Shell { command } => {
                if self.terminal_open {
                    self.terminal_write(&command);
                } else {
                    self.set_last_action("Shell", command.clone());
                    let _ = Command::new("sh").arg("-c").arg(command).spawn();
                }
            }
            PluginAction::TerminalClear => {
                if let Some(ref term) = self.terminal {
                    term.clear();
                    self.last_action = "Terminal: cleared".to_string();
                }
                self.changed(cx);
            }
            PluginAction::ToggleTerminal => {
                self.toggle_terminal(cx);
            }
            PluginAction::TerminalWrite { line } => {
                self.terminal_write(&line);
                self.changed(cx);
            }
            PluginAction::ClearQuery => {
                self.set_query(String::new(), cx);
                self.set_action_log("Clear query", "cleared");
            }
            PluginAction::CycleSelection { forward } => {
                if forward {
                    self.runner.select_next();
                } else {
                    self.runner.select_previous();
                }
                self.set_action_log("Selection", self.runner.selection_label());
            }
            PluginAction::SelectResult { index } => {
                self.runner.select(index);
                self.set_action_log("Selection", self.runner.selection_label());
            }
            PluginAction::None => {}
        }
        self.refresh_runner();
        self.changed(cx);
    }

    fn changed(&mut self, cx: &mut Context<Self>) {
        write_external_bar_state(self);
        cx.notify();
        cx.emit(DesktopEvent::Changed);
    }

    #[cfg(feature = "compositor")]
    fn poll_compositor(&mut self) {
        let Some(ref events) = self.compositor_events else {
            return;
        };
        while let Ok(event) = events.try_recv() {
            match event {
                compositor::CompositorEvent::Created {
                    id,
                    title,
                    app_id: _,
                } => {
                    let window_id = self.layout.split_next(title.clone());
                    self.compositor_surfaces.insert(id, window_id);
                    self.last_action = format!("Compositor: {title} connected");
                }
                compositor::CompositorEvent::Updated { id } => {
                    if self.compositor_surfaces.contains_key(&id) {
                        self.layout.set_focused_window_content("Surface", "updated");
                    }
                }
                compositor::CompositorEvent::Closed { id } => {
                    if let Some(window_id) = self.compositor_surfaces.remove(&id) {
                        if !self.layout.remove_window_by_id(window_id) {
                            self.layout.set_window_floating(window_id, true);
                        }
                        self.last_action = format!("Compositor: surface {id} closed");
                    }
                }
            }
        }
    }

    fn toggle_status_bar(&mut self, cx: &mut Context<Self>) {
        self.status_bar = !self.status_bar;
        self.changed(cx);
    }

    fn toggle_terminal(&mut self, cx: &mut Context<Self>) {
        self.terminal_open = !self.terminal_open;
        if self.terminal_open {
            self.active_category = BarCategory::Terminal;
            if self.terminal.is_none() {
                self.terminal = terminal::TerminalConsole::spawn();
            }
            if self.terminal.is_some() {
                self.last_action = "Terminal: opened in bar".to_string();
            } else {
                self.last_action = "Terminal: spawn failed".to_string();
                self.terminal_open = false;
                self.active_category = BarCategory::All;
            }
        } else {
            self.terminal = None;
            if self.active_category == BarCategory::Terminal {
                self.active_category = BarCategory::All;
            }
            self.last_action = "Terminal: closed".to_string();
        }
        self.changed(cx);
    }

    fn terminal_write(&mut self, line: &str) {
        if let Some(ref term) = self.terminal {
            let trimmed = line.trim().to_string();
            if trimmed == "exit" || trimmed == "quit" {
                self.terminal = None;
                self.terminal_open = false;
                if self.active_category == BarCategory::Terminal {
                    self.active_category = BarCategory::All;
                }
                self.last_action = "Terminal: closed".to_string();
            } else {
                term.write(&trimmed);
                self.last_action = format!("$ {trimmed}");
            }
        }
    }
}

struct InstallerWindow {
    source: PathBuf,
    target: PathBuf,
    status: String,
}

impl InstallerWindow {
    fn new(source: PathBuf, target: PathBuf) -> Self {
        let status = format!("Ready to install to {}", target.display());
        Self {
            source,
            target,
            status,
        }
    }

    fn install(&mut self, _: &gpui::ClickEvent, _: &mut gpui::Window, cx: &mut Context<Self>) {
        let result = copy_install_image(&self.source, &self.target);
        self.status = match result {
            Ok(bytes) => format!("Wrote {bytes} bytes to {}", self.target.display()),
            Err(error) => format!("Install failed: {error}"),
        };
        cx.notify();
    }
}

fn copy_install_image(source: &PathBuf, target: &PathBuf) -> std::io::Result<u64> {
    std::fs::File::open(source).and_then(|mut input| {
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(target)
            .and_then(|mut output| std::io::copy(&mut input, &mut output))
    })
}

impl Render for InstallerWindow {
    fn render(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) -> impl IntoElement {
        let source = self.source.display().to_string();
        let target = self.target.display().to_string();
        let status = self.status.clone();

        div()
            .size_full()
            .bg(rgb(0x050505))
            .text_color(rgb(TEXT))
            .font(ui_font())
            .p(px(24.))
            .flex()
            .flex_col()
            .gap(px(18.))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        div()
                            .w(px(14.))
                            .h(px(14.))
                            .rounded(px(7.))
                            .bg(rgb(0xffffff)),
                    )
                    .child(
                        div()
                            .text_size(px(18.))
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("Alpenglow Installer"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(10.))
                    .child(step_pill("1 Choose disk", true))
                    .child(step_pill("2 Review", false))
                    .child(step_pill("3 Install", false)),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_end()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(30.))
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("Select disk"),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(rgb(TEXT_DIM))
                            .child("This will overwrite the selected disk."),
                    ),
            )
            .child(installer_field("Source", source))
            .child(installer_field("Target", target))
            .child(installer_field("Status", status))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .pt(px(16.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(TEXT_DIM))
                            .child("No changes are made until Install is clicked."),
                    )
                    .child(
                        div()
                            .id("alpenglowed-installer-install")
                            .px(px(22.))
                            .py(px(12.))
                            .rounded(px(8.))
                            .bg(rgb(0xffffff))
                            .cursor_pointer()
                            .on_click(cx.listener(Self::install))
                            .child(
                                div()
                                    .text_color(rgb(0x000000))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child("Install"),
                            ),
                    ),
            )
    }
}

fn step_pill(label: &'static str, active: bool) -> Div {
    div()
        .px(px(14.))
        .py(px(9.))
        .rounded(px(8.))
        .border_1()
        .border_color(rgb(if active { 0xffffff } else { BORDER }))
        .bg(rgb(if active { 0xffffff } else { SURFACE_2 }))
        .text_color(rgb(if active { 0x000000 } else { TEXT_DIM }))
        .text_size(px(13.))
        .font_weight(if active {
            gpui::FontWeight::BOLD
        } else {
            gpui::FontWeight::NORMAL
        })
        .child(label)
}

fn installer_field(label: &'static str, value: String) -> Div {
    div()
        .rounded(px(8.))
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE_2))
        .p(px(14.))
        .flex()
        .flex_col()
        .gap(px(6.))
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(TEXT_DIM))
                .child(label),
        )
        .child(div().text_size(px(14.)).text_color(rgb(TEXT)).child(value))
}

fn shell_text_component(component: &str, props: &[(&str, &str)]) -> String {
    let mut ctx = TemplateContext::new();
    for (key, value) in props {
        ctx.set(*key, TemplateValue::Str((*value).to_string()));
    }
    render_component_file_to_html(SHELL_CREPUS, component, &ctx)
        .map(|html| html_text_content(&html))
        .unwrap_or_default()
}

#[allow(dead_code)]
fn shell_header_component(component: &str) -> (String, String) {
    render_component_file_to_html(SHELL_CREPUS, component, &TemplateContext::new())
        .map(|html| {
            let texts = html_tag_texts(&html, &["h1", "p"]);
            let title = texts
                .first()
                .cloned()
                .unwrap_or_else(|| "Settings".to_string());
            let subtitle = texts
                .get(1)
                .cloned()
                .unwrap_or_else(|| "Desktop, launcher, session, and system controls".to_string());
            (title, subtitle)
        })
        .unwrap_or_else(|_| {
            (
                "Settings".to_string(),
                "Desktop, launcher, session, and system controls".to_string(),
            )
        })
}

#[allow(dead_code)]
fn shell_list_component(component: &str) -> Vec<String> {
    render_component_file_to_html(SHELL_CREPUS, component, &TemplateContext::new())
        .map(|html| html_list_items(&html))
        .unwrap_or_default()
}

fn shell_row_component(marker: &str, title: &str, subtitle: &str) -> (String, String, String) {
    let mut ctx = TemplateContext::new();
    ctx.set("marker", TemplateValue::Str(marker.to_string()));
    ctx.set("title", TemplateValue::Str(title.to_string()));
    ctx.set("subtitle", TemplateValue::Str(subtitle.to_string()));
    render_component_file_to_html(SHELL_CREPUS, "LauncherRow", &ctx)
        .map(|html| {
            let texts = html_tag_texts(&html, &["strong", "h2", "p"]);
            let marker = texts.first().cloned().unwrap_or_else(|| marker.to_string());
            let title = texts.get(1).cloned().unwrap_or_else(|| title.to_string());
            let subtitle = texts
                .get(2)
                .cloned()
                .unwrap_or_else(|| subtitle.to_string());
            (marker, title, subtitle)
        })
        .unwrap_or_else(|_| (marker.to_string(), title.to_string(), subtitle.to_string()))
}

fn shell_bar_component(query: &str, meta: &str) -> (String, String) {
    let mut ctx = TemplateContext::new();
    ctx.set("query", TemplateValue::Str(query.to_string()));
    ctx.set("meta", TemplateValue::Str(meta.to_string()));
    render_component_file_to_html(SHELL_CREPUS, "LauncherBar", &ctx)
        .map(|html| {
            let texts = html_tag_texts(&html, &["h1", "p"]);
            let query = texts.first().cloned().unwrap_or_else(|| query.to_string());
            let meta = texts.get(1).cloned().unwrap_or_else(|| meta.to_string());
            (query, meta)
        })
        .unwrap_or_else(|_| (query.to_string(), meta.to_string()))
}

#[allow(dead_code)]
fn shell_section_header_component(title: &str, detail: &str) -> (String, String) {
    let mut ctx = TemplateContext::new();
    ctx.set("title", TemplateValue::Str(title.to_string()));
    ctx.set("detail", TemplateValue::Str(detail.to_string()));
    render_component_file_to_html(SHELL_CREPUS, "SettingsSectionHeader", &ctx)
        .map(|html| {
            let texts = html_tag_texts(&html, &["h2", "p"]);
            let title = texts.first().cloned().unwrap_or_else(|| title.to_string());
            let detail = texts.get(1).cloned().unwrap_or_else(|| detail.to_string());
            (title, detail)
        })
        .unwrap_or_else(|_| (title.to_string(), detail.to_string()))
}

fn shell_top_bar_title_component(title: &str, subtitle: &str) -> (String, String) {
    let mut ctx = TemplateContext::new();
    ctx.set("title", TemplateValue::Str(title.to_string()));
    ctx.set("subtitle", TemplateValue::Str(subtitle.to_string()));
    render_component_file_to_html(SHELL_CREPUS, "TopBarTitle", &ctx)
        .map(|html| {
            let texts = html_tag_texts(&html, &["h1", "p"]);
            let title = texts.first().cloned().unwrap_or_else(|| title.to_string());
            let subtitle = texts
                .get(1)
                .cloned()
                .unwrap_or_else(|| subtitle.to_string());
            (title, subtitle)
        })
        .unwrap_or_else(|_| (title.to_string(), subtitle.to_string()))
}

fn shell_top_bar_metric_component(label: &str, value: &str) -> (String, String) {
    let mut ctx = TemplateContext::new();
    ctx.set("label", TemplateValue::Str(label.to_string()));
    ctx.set("value", TemplateValue::Str(value.to_string()));
    render_component_file_to_html(SHELL_CREPUS, "TopBarMetric", &ctx)
        .map(|html| {
            let texts = html_tag_texts(&html, &["strong", "p"]);
            let label = texts.first().cloned().unwrap_or_else(|| label.to_string());
            let value = texts.get(1).cloned().unwrap_or_else(|| value.to_string());
            (label, value)
        })
        .unwrap_or_else(|_| (label.to_string(), value.to_string()))
}

#[allow(dead_code)]
fn shell_status_row_component(mode: &str, layout: &str, focused: &str) -> Vec<String> {
    let mut ctx = TemplateContext::new();
    ctx.set("mode", TemplateValue::Str(mode.to_string()));
    ctx.set("layout", TemplateValue::Str(layout.to_string()));
    ctx.set("focused", TemplateValue::Str(focused.to_string()));
    render_component_file_to_html(SHELL_CREPUS, "SettingsStatusRow", &ctx)
        .map(|html| html_list_items(&html))
        .unwrap_or_else(|_| vec![mode.to_string(), layout.to_string(), focused.to_string()])
}

#[derive(Clone)]
struct TopBarMetrics {
    clock: String,
    date: String,
    battery: String,
    temp: String,
    wifi: String,
    load: String,
    memory: String,
    backend: String,
}

impl TopBarMetrics {
    fn detect(desktop: &DesktopModel) -> Self {
        use std::sync::{Mutex, OnceLock};
        use std::time::{Duration, Instant};

        static METRICS_CACHE: OnceLock<Mutex<Option<(Instant, TopBarMetrics)>>> = OnceLock::new();
        let cache = METRICS_CACHE.get_or_init(|| Mutex::new(None));
        if let Ok(guard) = cache.lock() {
            if let Some((when, ref cached)) = *guard {
                if when.elapsed() < Duration::from_secs(2) {
                    let mut out = cached.clone();
                    out.backend = top_bar_backend(desktop);
                    return out;
                }
            }
        }

        let fresh = Self {
            clock: date_value("+%H:%M").unwrap_or_else(|| "--:--".to_string()),
            date: date_value("+%a %b %e").unwrap_or_else(|| "date unavailable".to_string()),
            battery: battery_value().unwrap_or_else(|| "battery unavailable".to_string()),
            temp: if desktop.role.extras() {
                temp_value().unwrap_or_else(|| "--°".to_string())
            } else {
                String::new()
            },
            wifi: if !desktop.role.skinny() {
                wifi_value().unwrap_or_else(|| "wifi unavailable".to_string())
            } else {
                String::new()
            },
            load: cpu_value().unwrap_or_else(|| "cpu unavailable".to_string()),
            memory: memory_value().unwrap_or_else(|| "memory unavailable".to_string()),
            backend: top_bar_backend(desktop),
        };
        if let Ok(mut guard) = cache.lock() {
            *guard = Some((Instant::now(), fresh.clone()));
        }
        fresh
    }
}

fn date_value(format: &str) -> Option<String> {
    let output = Command::new("date").arg(format).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

fn battery_value() -> Option<String> {
    let entries = fs::read_dir("/sys/class/power_supply").ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let kind = fs::read_to_string(path.join("type")).ok()?;
        if kind.trim() != "Battery" {
            continue;
        }
        let capacity = fs::read_to_string(path.join("capacity")).ok()?;
        let status = fs::read_to_string(path.join("status")).ok()?;
        return Some(format!(
            "{}% {}",
            capacity.trim(),
            status.trim().to_lowercase()
        ));
    }
    None
}

#[cfg(feature = "full")]
fn temp_value() -> Option<String> {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    static CACHE: std::sync::OnceLock<Mutex<(Instant, String)>> = std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new((Instant::now(), String::new())));
    let mut guard = cache.lock().ok()?;
    if guard.0.elapsed() < Duration::from_secs(300) {
        return Some(guard.1.clone()).filter(|s| !s.is_empty());
    }
    let output = Command::new("sh")
        .arg("-c")
        .arg("curl -s 'wttr.in?format=%t' 2>/dev/null")
        .output()
        .ok()?;
    let temp = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if temp.is_empty() {
        return None;
    }
    *guard = (Instant::now(), temp.clone());
    Some(temp)
}

#[cfg(not(feature = "full"))]
fn temp_value() -> Option<String> {
    None
}

fn wifi_value() -> Option<String> {
    let text = fs::read_to_string("/proc/net/wireless").ok()?;
    for line in text.lines().skip(2) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }
        let iface = parts.first()?.trim_end_matches(':');
        if iface == "lo" {
            continue;
        }
        let link_quality = parts.get(2).filter(|q| **q != "0").or(parts.get(3))?;
        let signal = parts.get(3).or(parts.get(2))?;
        return Some(format!("{iface} {link_quality}/{signal} db"));
    }
    None
}

fn load_value() -> Option<String> {
    let text = fs::read_to_string("/proc/loadavg").ok()?;
    let first = text.split_whitespace().next()?;
    Some(first.to_string())
}

fn cpu_value() -> Option<String> {
    let text = fs::read_to_string("/proc/stat").ok()?;
    let line = text.lines().next()?;
    let vals: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .filter_map(|v| v.parse().ok())
        .collect();
    if vals.len() < 4 {
        return None;
    }
    let total: u64 = vals.iter().sum();
    let idle = vals[3];
    if total == 0 {
        return None;
    }
    Some(format!("{}%", ((total - idle) * 100) / total))
}

fn memory_value() -> Option<String> {
    let text = fs::read_to_string("/proc/meminfo").ok()?;
    let mut total_kb = None;
    let mut available_kb = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            total_kb = rest
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<u64>().ok());
        } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
            available_kb = rest
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<u64>().ok());
        }
    }
    let total_kb = total_kb?;
    let available_kb = available_kb?;
    if total_kb == 0 {
        return None;
    }
    let used_pct = ((total_kb.saturating_sub(available_kb)) * 100) / total_kb;
    Some(format!("{used_pct}% used"))
}

fn top_bar_backend(desktop: &DesktopModel) -> String {
    let state = de::DesktopState::detect(desktop.mode.label());
    polybar_backend(&state)
}

fn polybar_backend(state: &de::DesktopState) -> String {
    let display = state.display.as_deref().unwrap_or("no-display").to_string();
    let backend = if state.wayland { "wayland" } else { "offline" };
    format!("{backend} {display}")
}

#[allow(dead_code)]
fn html_list_items(html: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cursor = 0;
    while let Some(open_rel) = html[cursor..].find("<li") {
        let open = cursor + open_rel;
        let Some(content_start_rel) = html[open..].find('>') else {
            break;
        };
        let content_start = open + content_start_rel + 1;
        let Some(close_rel) = html[content_start..].find("</li>") else {
            break;
        };
        let content_end = content_start + close_rel;
        let text = html[content_start..content_end]
            .replace("&quot;", "\"")
            .replace("&amp;", "&")
            .replace("&#39;", "'")
            .replace("&lt;", "<")
            .replace("&gt;", ">");
        let text = text.trim();
        if !text.is_empty() {
            lines.push(text.to_string());
        }
        cursor = content_end + "</li>".len();
    }
    lines
}

fn html_text_content(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(ch),
            _ => {}
        }
    }
    text.replace("&quot;", "\"")
        .replace("&amp;", "&")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .trim()
        .to_string()
}

fn html_tag_texts(html: &str, tags: &[&str]) -> Vec<String> {
    tags.iter()
        .filter_map(|tag| {
            let open = format!("<{tag}>");
            let close = format!("</{tag}>");
            let start = html.find(&open)? + open.len();
            let end = html[start..].find(&close)? + start;
            Some(html_text_content(&html[start..end]))
        })
        .collect()
}

struct LauncherWindow {
    desktop: Entity<DesktopModel>,
}

impl LauncherWindow {
    fn new(desktop: Entity<DesktopModel>, cx: &mut Context<Self>) -> Self {
        cx.subscribe(&desktop, |_, _, _: &DesktopEvent, cx| {
            cx.notify();
        })
        .detach();

        Self { desktop }
    }

    fn backspace(&mut self, cx: &mut Context<Self>) {
        let (query, in_sub) = {
            let d = self.desktop.read(cx);
            (d.query.clone(), d.view_mode.in_submenu())
        };
        if query.is_empty() && in_sub {
            self.desktop.update(cx, |desktop, cx| {
                desktop.set_view_mode(ViewMode::Main, cx);
            });
            return;
        }
        let mut next = query;
        next.pop();
        self.desktop.update(cx, |desktop, cx| {
            desktop.set_query(next, cx);
        });
    }

    fn append(&mut self, ch: &str, cx: &mut Context<Self>) {
        let query = self.desktop.read(cx).query.clone();
        let mut next = query;
        next.push_str(ch);
        self.desktop.update(cx, |desktop, cx| {
            desktop.set_query(next, cx);
        });
    }

    fn confirm(&mut self, cx: &mut Context<Self>) {
        if self.desktop.read(cx).show_actions {
            let action_id = {
                let d = self.desktop.read(cx);
                let actions = d.actions_for_selected();
                actions.get(d.selected_action).map(|a| a.id)
            };
            if let Some(action_id) = action_id {
                self.desktop.update(cx, |desktop, cx| {
                    desktop.execute_item_action(action_id, cx);
                });
            }
            return;
        }
        if self.desktop.read(cx).view_mode == ViewMode::Terminal {
            let line = self.desktop.read(cx).query.clone();
            if !line.trim().is_empty() {
                self.desktop.update(cx, |desktop, cx| {
                    desktop.history.record(&line);
                    desktop.set_query(String::new(), cx);
                    desktop.apply(PluginAction::TerminalWrite { line }, cx);
                });
            }
            return;
        }
        if self.desktop.read(cx).view_mode == ViewMode::Browser {
            let query = self.desktop.read(cx).query.clone();
            if !query.trim().is_empty() {
                let cmd = plugin::parse_browser_command(&query).unwrap_or_else(|| {
                    plugin::BrowserCommand::Search {
                        engine_name: "duckduckgo".to_string(),
                        query: query.trim().to_string(),
                    }
                });
                self.desktop.update(cx, |desktop, cx| {
                    desktop.history.record(&query);
                    desktop.set_query(String::new(), cx);
                    let _ = desktop.browser.execute(cmd);
                    let active_url = desktop.browser.active_url().to_string();
                    desktop.browser_page = browser::load_internal_page(&active_url);
                    desktop.changed(cx);
                });
                return;
            }
        }
        let selected_pill = {
            let d = self.desktop.read(cx);
            d.pills.get(d.selected_pill).cloned()
        };
        if let Some(pill) = selected_pill {
            self.desktop.update(cx, |desktop, cx| {
                desktop.execute_pill(pill, cx);
            });
            return;
        }
        let action = self.desktop.read(cx).runner.confirm();
        if let Some(action) = action {
            if !matches!(action, PluginAction::None) {
                let title = self
                    .desktop
                    .read(cx)
                    .runner
                    .selected_result()
                    .map(|r| r.title.clone());
                self.desktop.update(cx, |desktop, cx| {
                    if let Some(title) = &title {
                        desktop.runner.record_recent(title);
                    }
                    desktop.apply(action, cx);
                });
            }
        }
    }

    fn select_and_confirm(&mut self, index: usize, cx: &mut Context<Self>) {
        let pill = self.desktop.read(cx).pills.get(index).cloned();
        if let Some(pill) = pill {
            self.desktop.update(cx, |desktop, cx| {
                desktop.selected_pill = index;
                desktop.execute_pill(pill, cx);
            });
            return;
        }
        let count = self.desktop.read(cx).runner.results.len();
        if count == 0 || index >= count {
            return;
        }
        self.desktop.update(cx, |desktop, cx| {
            desktop.runner.select(index);
            desktop.changed(cx);
        });
        self.confirm(cx);
    }

    fn delete_word(&mut self, cx: &mut Context<Self>) {
        let query = self.desktop.read(cx).query.clone();
        let trimmed_end = query.trim_end();
        let mut next = trimmed_end.to_string();
        if let Some(idx) = next.rfind(|c: char| c.is_whitespace()) {
            next.truncate(idx);
        } else {
            next.clear();
        }
        self.desktop.update(cx, |desktop, cx| {
            desktop.set_query(next, cx);
        });
    }

    fn select_next(&mut self, cx: &mut Context<Self>) {
        self.desktop.update(cx, |desktop, cx| {
            if desktop.show_actions {
                let len = desktop.actions_for_selected().len();
                if len > 0 {
                    desktop.selected_action = (desktop.selected_action + 1) % len;
                }
                desktop.changed(cx);
                return;
            }
            if let Some(next_q) = desktop.history.navigate_down() {
                desktop.query = next_q.clone();
                desktop.runner.query = next_q;
                desktop.refresh_runner();
                desktop.changed(cx);
                return;
            }
            let len = desktop.pills.len();
            if len > 0 {
                desktop.selected_pill = (desktop.selected_pill + 1) % len;
            }
            desktop.runner.select_next();
            desktop.changed(cx);
        });
    }

    fn select_previous(&mut self, cx: &mut Context<Self>) {
        self.desktop.update(cx, |desktop, cx| {
            if desktop.show_actions {
                let len = desktop.actions_for_selected().len();
                if len > 0 {
                    desktop.selected_action = if desktop.selected_action == 0 {
                        len - 1
                    } else {
                        desktop.selected_action - 1
                    };
                }
                desktop.changed(cx);
                return;
            }
            if desktop.selected_pill == 0 {
                let current_q = desktop.query.clone();
                if let Some(prev_q) = desktop.history.navigate_up(&current_q) {
                    desktop.query = prev_q.clone();
                    desktop.runner.query = prev_q;
                    desktop.refresh_runner();
                    desktop.changed(cx);
                    return;
                }
            }
            let len = desktop.pills.len();
            if len > 0 {
                desktop.selected_pill = if desktop.selected_pill == 0 {
                    len - 1
                } else {
                    desktop.selected_pill - 1
                };
            }
            desktop.runner.select_previous();
            desktop.changed(cx);
        });
    }

    fn status_pill(label: String, value: String) -> Div {
        let metric = shell_top_bar_metric_component(&label, &value);
        div()
            .h(px(26.))
            .px(px(10.))
            .rounded(px(13.))
            .bg(rgb(SURFACE_3))
            .border_1()
            .border_color(rgb(BORDER_SOFT))
            .flex()
            .items_center()
            .gap(px(5.))
            .child(
                div()
                    .text_size(px(10.))
                    .text_color(rgb(TEXT_FAINT))
                    .child(metric.0),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(TEXT))
                    .child(metric.1),
            )
    }

    fn render_status_strip(&self, desktop: &DesktopModel) -> Div {
        let metrics = TopBarMetrics::detect(desktop);
        let focused = desktop.layout.focused_title().to_string();
        let detail = desktop.layout.summary();
        let header = shell_top_bar_title_component(&focused, &detail);
        let skinny = desktop.role.skinny();
        let mut pills = Vec::new();
        pills.push(("mode".to_string(), desktop.mode.label().to_string()));
        if !skinny {
            pills.push(("layout".to_string(), desktop.layout.summary()));
        }
        pills.push(("time".to_string(), metrics.clock));
        pills.push(("date".to_string(), metrics.date));
        if desktop.role.extras() && !metrics.temp.is_empty() {
            pills.push(("temp".to_string(), metrics.temp));
        }
        pills.push(("power".to_string(), metrics.battery));
        if !skinny {
            pills.push(("wifi".to_string(), metrics.wifi));
        }
        pills.push(("cpu".to_string(), metrics.load));
        pills.push(("mem".to_string(), metrics.memory));
        if !skinny {
            pills.push(("wl".to_string(), metrics.backend));
        }

        div()
            .w(px(760.))
            .rounded_t(px(8.))
            .bg(rgb(SURFACE))
            .border_1()
            .border_color(rgb(BORDER))
            .border_b_0()
            .px(px(16.))
            .py(px(8.))
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(TEXT))
                            .child(header.0),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(TEXT_DIM))
                            .child(header.1),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(6.))
                    .children(pills.into_iter().map(|(l, v)| Self::status_pill(l, v))),
            )
    }

    fn section_header_row(label: &'static str, is_first: bool) -> Div {
        components::section_header(label, is_first)
    }

    fn render_bar(&self, cx: &App) -> impl IntoElement {
        let desktop = self.desktop.read(cx);
        let in_submenu = desktop.view_mode.in_submenu();
        let back_entity = self.desktop.clone();

        if desktop.view_mode == ViewMode::Terminal {
            let cwd = desktop
                .terminal
                .as_ref()
                .map(|t| t.current_cwd())
                .unwrap_or_else(|| "~".to_string());
            let branch = desktop
                .terminal
                .as_ref()
                .and_then(|t| t.current_git_branch());
            return terminal::render_terminal_starship_header(
                &cwd,
                branch.as_deref(),
                &desktop.query,
                in_submenu,
                move |_, cx| {
                    back_entity.update(cx, |d, cx| {
                        d.set_view_mode(ViewMode::Main, cx);
                    });
                },
            );
        }

        if desktop.view_mode == ViewMode::Browser {
            let url_display = if !desktop.query.is_empty() {
                desktop.query.clone()
            } else {
                desktop.browser_page.url.clone()
            };
            return div()
                .w_full()
                .h(px(components::Theme::HEADER_HEIGHT))
                .px(px(16.))
                .flex()
                .items_center()
                .gap(px(12.))
                .child(
                    div()
                        .id("browser-back-btn")
                        .cursor_pointer()
                        .on_click(move |_, _, cx| {
                            back_entity.update(cx, |d, cx| {
                                d.set_view_mode(ViewMode::Main, cx);
                            });
                        })
                        .child(icons::sf_icon(
                            icons::SfSymbol::ChevronLeft,
                            15.0,
                            TC_TEXT_SECONDARY,
                        )),
                )
                .child(icons::sf_icon(icons::SfSymbol::LockFill, 13.0, 0x34d399))
                .child(
                    div()
                        .flex_1()
                        .font(font("Geist Mono"))
                        .text_size(px(15.))
                        .text_color(rgb(if desktop.query.is_empty() {
                            TC_TEXT_PRIMARY
                        } else {
                            0x60a5fa
                        }))
                        .child(url_display),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(TC_TEXT_TERTIARY))
                        .child("↵ to load"),
                );
        }

        let selection = desktop.runner.selection_label();
        let subtitle = if desktop.query.trim().is_empty() {
            shell_text_component(
                "LauncherEmpty",
                &[("message", "Search for apps and commands...")],
            )
        } else {
            desktop.live_hint()
        };
        let meta = shell_text_component(
            "LauncherMeta",
            &[("selection", &selection), ("subtitle", &subtitle)],
        );
        let placeholder = match desktop.view_mode {
            ViewMode::Main => "Search for apps and commands...",
            ViewMode::WindowControls => "Search windows and layout commands...",
            ViewMode::Browser => "Enter URL or search bang (!g, !gh, !yt, !wiki, !rs)...",
            ViewMode::Terminal => "Type a command... (↵ to run)",
            ViewMode::ClipboardHistory => "Search clipboard history...",
            ViewMode::FileSearch => "Search files (/path)...",
        };
        let query = if desktop.query.is_empty() {
            placeholder.to_string()
        } else {
            desktop.query.clone()
        };
        let bar = shell_bar_component(&query, &meta);

        div()
            .w_full()
            .h(px(components::Theme::HEADER_HEIGHT))
            .px(px(16.))
            .flex()
            .items_center()
            .gap(px(12.))
            .child(if in_submenu {
                div()
                    .id("rover-submenu-back")
                    .w(px(22.))
                    .h(px(22.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .on_click(move |_, _, cx| {
                        back_entity.update(cx, |d, cx| {
                            d.set_view_mode(ViewMode::Main, cx);
                        });
                    })
                    .child(icons::sf_icon(
                        icons::SfSymbol::ChevronLeft,
                        15.0,
                        TC_TEXT_SECONDARY,
                    ))
            } else {
                div()
                    .id("tinycast-search-icon")
                    .w(px(22.))
                    .h(px(22.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(icons::sf_icon(
                        icons::SfSymbol::MagnifyingGlass,
                        15.0,
                        TC_TEXT_SECONDARY,
                    ))
            })
            .child(
                div()
                    .flex_1()
                    .text_size(px(20.))
                    .text_color(rgb(if desktop.query.is_empty() {
                        TC_TEXT_TERTIARY
                    } else {
                        TC_TEXT_PRIMARY
                    }))
                    .child(bar.0),
            )
    }

    fn render_terminal_inline(&self, desktop: &DesktopModel) -> Div {
        terminal::render_warp_terminal(desktop.terminal.as_ref(), 4)
    }

    fn render_browser_inline(&self, desktop: &DesktopModel) -> Div {
        browser::render_internal_browser(&desktop.browser_page, &desktop.browser_engine)
    }

    fn render_results(&self, cx: &App) -> impl IntoElement {
        let desktop = self.desktop.read(cx);

        if desktop.view_mode == ViewMode::Terminal {
            return self.render_terminal_inline(desktop);
        }

        if desktop.view_mode == ViewMode::Browser {
            return self.render_browser_inline(desktop);
        }

        if desktop.pills.is_empty() {
            return div()
                .w_full()
                .px(px(16.))
                .py(px(16.))
                .text_size(px(13.))
                .text_color(rgb(TC_TEXT_SECONDARY))
                .child(shell_text_component(
                    "LauncherNoResults",
                    &[("message", "No matching applications or commands")],
                ));
        }

        let mut list = div()
            .w_full()
            .px(px(8.))
            .pt(px(4.))
            .pb(px(8.))
            .flex()
            .flex_col()
            .gap(px(0.));

        let mut current_section: Option<&'static str> = None;
        for (index, pill) in desktop.pills.iter().enumerate().take(8) {
            if current_section != Some(pill.section) {
                let is_first = current_section.is_none();
                current_section = Some(pill.section);
                list = list.child(Self::section_header_row(pill.section, is_first));
            }

            let selected = index == desktop.selected_pill;
            let desktop_entity = self.desktop.clone();
            let pill_clone = pill.clone();
            let row = shell_row_component(
                pill.badge.label().unwrap_or(""),
                &pill.title,
                &pill.subtitle,
            );
            let inline_preview = (!pill.subtitle.is_empty()
                && matches!(pill.kind_label, "Snippet" | "Quicklink" | "Calculator"))
            .then(|| SharedString::from(pill.subtitle.clone()));

            let row_el = components::list_row(
                SharedString::from(format!("rover-pill-{index}")),
                selected,
                icons::SfSymbol::from_glyph(pill.icon_glyph),
                pill.icon_bg,
                pill.running,
                row.1,
                inline_preview,
                &pill.keycaps,
                pill.kind_label,
            )
            .on_click(move |_, _, cx| {
                let p = pill_clone.clone();
                desktop_entity.update(cx, |desktop, cx| {
                    desktop.selected_pill = index;
                    desktop.execute_pill(p, cx);
                });
            });

            list = list.child(row_el);
        }

        list
    }

    fn render_actions_overlay(&self, desktop: &DesktopModel) -> Div {
        let actions = desktop.actions_for_selected();
        let selected_action = desktop.selected_action;
        let is_app_menu = desktop.show_app_menu && !desktop.show_actions;

        div()
            .absolute()
            .bottom(px(52.))
            .left(px(0.))
            .right(px(0.))
            .px(px(8.))
            .pb(px(4.))
            .flex()
            .when(!is_app_menu, |row| row.justify_end())
            .when(is_app_menu, |row| row.justify_start())
            .child(
                div()
                    .w(px(272.))
                    .rounded(px(18.))
                    .bg(rgb(components::Theme::POPOVER_BG))
                    .border_1()
                    .border_color(rgb(TC_BORDER))
                    .p(px(6.))
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .children(actions.into_iter().enumerate().map(|(idx, act)| {
                        let is_sel = idx == selected_action;
                        let entity = self.desktop.clone();
                        let action_id = act.id;
                        components::popover_row(
                            SharedString::from(format!("rover-item-action-{idx}")),
                            act.label,
                            is_sel,
                            &act.keycaps,
                        )
                        .on_click(move |_, _, cx| {
                            entity.update(cx, |d, cx| {
                                d.execute_item_action(action_id, cx);
                            });
                        })
                    }))
                    .child(div().my(px(4.)).h(px(1.)).w_full().bg(rgb(TC_CARD_STROKE)))
                    .child(
                        div()
                            .h(px(26.))
                            .px(px(10.))
                            .flex()
                            .items_center()
                            .text_size(px(12.5))
                            .text_color(rgb(TC_TEXT_TERTIARY))
                            .child("Search for actions..."),
                    ),
            )
    }

    fn render_footer(&self, desktop: &DesktopModel) -> Div {
        let primary_label = desktop
            .pills
            .get(desktop.selected_pill)
            .map(RoverPill::primary_verb)
            .unwrap_or("Open");
        let app_menu_entity = self.desktop.clone();
        let primary_entity = self.desktop.clone();
        let actions_entity = self.desktop.clone();

        div()
            .w_full()
            .h(px(components::Theme::BOTTOM_BAR_HEIGHT))
            .px(px(8.))
            .flex()
            .items_center()
            .justify_between()
            .child(
                components::circle_icon_button(
                    "tinycast-app-menu-btn",
                    icons::SfSymbol::Equal,
                    desktop.show_app_menu,
                )
                .on_click(move |_, _, cx| {
                    app_menu_entity.update(cx, |d, cx| {
                        d.show_app_menu = !d.show_app_menu;
                        if d.show_app_menu {
                            d.show_actions = false;
                        }
                        d.selected_action = 0;
                        d.changed(cx);
                    });
                }),
            )
            .child(
                div()
                    .h(px(36.))
                    .rounded(px(18.))
                    .bg(rgb(TC_GLASS_BG))
                    .p(px(4.))
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .child(
                        components::bar_button(
                            "tinycast-primary-action-btn",
                            primary_label,
                            false,
                            true,
                            &["↵"],
                        )
                        .on_click(move |_, _, cx| {
                            primary_entity.update(cx, |d, cx| {
                                if let Some(pill) = d.pills.get(d.selected_pill).cloned() {
                                    d.execute_pill(pill, cx);
                                }
                            });
                        }),
                    )
                    .child(
                        components::bar_button(
                            "rover-footer-actions-btn",
                            "Actions",
                            desktop.show_actions,
                            false,
                            &["Super", "K"],
                        )
                        .on_click(move |_, _, cx| {
                            actions_entity.update(cx, |d, cx| {
                                d.show_actions = !d.show_actions;
                                if d.show_actions {
                                    d.show_app_menu = false;
                                }
                                d.selected_action = 0;
                                d.changed(cx);
                            });
                        }),
                    ),
            )
    }

    fn render_toast(
        notif: &notifications::Notification,
        desktop: Entity<DesktopModel>,
        index: usize,
    ) -> impl IntoElement {
        let border = match notif.urgency.as_str() {
            "critical" => 0xff4444,
            "low" => 0x444466,
            _ => 0x4488ff,
        };
        div()
            .id(SharedString::from(format!("toast-{index}")))
            .w(px(750.))
            .rounded(px(12.))
            .bg(rgb(TC_CARD_FILL))
            .border_1()
            .border_color(rgb(border))
            .px(px(12.))
            .py(px(6.))
            .flex()
            .items_center()
            .justify_between()
            .cursor_pointer()
            .on_click(move |_, _, cx| {
                desktop.update(cx, |d, _| d.notifications.dismiss(index));
            })
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(TC_TEXT_PRIMARY))
                    .child(format!("{}: {}", notif.title, notif.body)),
            )
            .child(
                div()
                    .text_size(px(10.))
                    .text_color(rgb(TC_TEXT_TERTIARY))
                    .child("dismiss"),
            )
    }
}

impl Render for LauncherWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.desktop.update(cx, |desktop, _| {
            desktop.notifications.poll();
            #[cfg(feature = "compositor")]
            desktop.poll_compositor();
        });

        let desktop_entity = self.desktop.clone();
        let desktop = self.desktop.read(cx);
        let show_status = desktop.status_bar && !desktop.external_polybar;
        let show_popover = desktop.show_actions || desktop.show_app_menu;
        let active_notifs: Vec<_> = desktop
            .notifications
            .active
            .iter()
            .enumerate()
            .map(|(i, n)| Self::render_toast(&n.notification, desktop_entity.clone(), i))
            .collect();

        div()
            .size_full()
            .font(ui_font())
            .key_context("alpenglowed")
            .on_action(cx.listener(|_, _: &Quit, _, cx| {
                cx.quit();
            }))
            .on_action(cx.listener(|this, _: &FocusBar, _, cx| {
                focus_or_open_launcher(&this.desktop, cx);
            }))
            .on_action(cx.listener(|this, _: &Confirm, _, cx| {
                this.confirm(cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleActionsMenu, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.show_actions = !desktop.show_actions;
                    if desktop.show_actions {
                        desktop.show_app_menu = false;
                    }
                    desktop.selected_action = 0;
                    desktop.changed(cx);
                });
            }))
            .on_action(cx.listener(|this, _: &ClearQueryAction, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.set_query(String::new(), cx);
                });
            }))
            .on_action(cx.listener(|this, _: &ToggleSettingsWindow, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.apply(PluginAction::ToggleSettings, cx);
                });
            }))
            .on_action(cx.listener(|this, _: &ToggleStatusBarAction, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.toggle_status_bar(cx);
                });
            }))
            .on_action(cx.listener(|this, _: &CycleWindowModeAction, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.apply(PluginAction::CycleWindowMode, cx);
                });
            }))
            .on_action(cx.listener(|this, _: &ToggleTerminalPane, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.apply(PluginAction::ToggleTerminal, cx);
                });
            }))
            .on_action(cx.listener(|this, _: &SplitRow, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.apply(
                        PluginAction::Layout {
                            action: layout::LayoutAction::SplitRow,
                        },
                        cx,
                    );
                });
            }))
            .on_action(cx.listener(|this, _: &SplitColumn, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.apply(
                        PluginAction::Layout {
                            action: layout::LayoutAction::SplitColumn,
                        },
                        cx,
                    );
                });
            }))
            .on_action(cx.listener(|this, _: &FlipAxis, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.apply(
                        PluginAction::Layout {
                            action: layout::LayoutAction::FlipAxis,
                        },
                        cx,
                    );
                });
            }))
            .on_action(cx.listener(|this, _: &ResetLayout, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.apply(
                        PluginAction::Layout {
                            action: layout::LayoutAction::Reset,
                        },
                        cx,
                    );
                });
            }))
            .on_action(cx.listener(|this, _: &NudgeLeft, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.apply(
                        PluginAction::Layout {
                            action: layout::LayoutAction::NudgeLeft,
                        },
                        cx,
                    );
                });
            }))
            .on_action(cx.listener(|this, _: &NudgeRight, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.apply(
                        PluginAction::Layout {
                            action: layout::LayoutAction::NudgeRight,
                        },
                        cx,
                    );
                });
            }))
            .on_action(cx.listener(|this, _: &NudgeUp, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.apply(
                        PluginAction::Layout {
                            action: layout::LayoutAction::NudgeUp,
                        },
                        cx,
                    );
                });
            }))
            .on_action(cx.listener(|this, _: &NudgeDown, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.apply(
                        PluginAction::Layout {
                            action: layout::LayoutAction::NudgeDown,
                        },
                        cx,
                    );
                });
            }))
            .on_action(cx.listener(|this, _: &ExpandWindow, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.apply(
                        PluginAction::Layout {
                            action: layout::LayoutAction::ExpandWindow,
                        },
                        cx,
                    );
                });
            }))
            .on_action(cx.listener(|this, _: &ContractWindow, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.apply(
                        PluginAction::Layout {
                            action: layout::LayoutAction::ContractWindow,
                        },
                        cx,
                    );
                });
            }))
            .on_action(cx.listener(|this, _: &GrowPane, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.apply(
                        PluginAction::Layout {
                            action: layout::LayoutAction::GrowFocused,
                        },
                        cx,
                    );
                });
            }))
            .on_action(cx.listener(|this, _: &ShrinkPane, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.apply(
                        PluginAction::Layout {
                            action: layout::LayoutAction::ShrinkFocused,
                        },
                        cx,
                    );
                });
            }))
            .on_action(cx.listener(|this, _: &FocusNextPane, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.apply(
                        PluginAction::Layout {
                            action: layout::LayoutAction::FocusNext,
                        },
                        cx,
                    );
                });
            }))
            .on_action(cx.listener(|this, _: &FocusPreviousPane, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.apply(
                        PluginAction::Layout {
                            action: layout::LayoutAction::FocusPrevious,
                        },
                        cx,
                    );
                });
            }))
            .on_action(cx.listener(|this, _: &FocusFirstPane, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.apply(
                        PluginAction::Layout {
                            action: layout::LayoutAction::FocusFirst,
                        },
                        cx,
                    );
                });
            }))
            .on_action(cx.listener(|this, _: &FocusLastPane, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.apply(
                        PluginAction::Layout {
                            action: layout::LayoutAction::FocusLast,
                        },
                        cx,
                    );
                });
            }))
            .on_action(cx.listener(|this, _: &BalancePanesAction, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.apply(
                        PluginAction::Layout {
                            action: layout::LayoutAction::BalancePanes,
                        },
                        cx,
                    );
                });
            }))
            .on_action(cx.listener(|this, _: &CenterFocusedAction, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.apply(
                        PluginAction::Layout {
                            action: layout::LayoutAction::CenterFocused,
                        },
                        cx,
                    );
                });
            }))
            .on_action(cx.listener(|this, _: &MoveWindowLeft, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.apply(
                        PluginAction::Layout {
                            action: layout::LayoutAction::MoveLeft,
                        },
                        cx,
                    );
                });
            }))
            .on_action(cx.listener(|this, _: &MoveWindowRight, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.apply(
                        PluginAction::Layout {
                            action: layout::LayoutAction::MoveRight,
                        },
                        cx,
                    );
                });
            }))
            .on_action(cx.listener(|this, _: &MoveWindowUp, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.apply(
                        PluginAction::Layout {
                            action: layout::LayoutAction::MoveUp,
                        },
                        cx,
                    );
                });
            }))
            .on_action(cx.listener(|this, _: &MoveWindowDown, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.apply(
                        PluginAction::Layout {
                            action: layout::LayoutAction::MoveDown,
                        },
                        cx,
                    );
                });
            }))
            .on_action(cx.listener(|this, _: &ClosePane, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.apply(
                        PluginAction::Layout {
                            action: layout::LayoutAction::CloseFocused,
                        },
                        cx,
                    );
                });
            }))
            .on_action(cx.listener(|this, _: &ToggleFloatPane, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.apply(
                        PluginAction::Layout {
                            action: layout::LayoutAction::ToggleFloat,
                        },
                        cx,
                    );
                });
            }))
            .on_action(cx.listener(|this, _: &SelectResult1, _, cx| {
                this.select_and_confirm(0, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectResult2, _, cx| {
                this.select_and_confirm(1, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectResult3, _, cx| {
                this.select_and_confirm(2, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectResult4, _, cx| {
                this.select_and_confirm(3, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectResult5, _, cx| {
                this.select_and_confirm(4, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectResult6, _, cx| {
                this.select_and_confirm(5, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectResult7, _, cx| {
                this.select_and_confirm(6, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectResult8, _, cx| {
                this.select_and_confirm(7, cx);
            }))
            .on_action(cx.listener(|this, _: &DefocusBar, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    if desktop.show_actions || desktop.show_app_menu {
                        desktop.show_actions = false;
                        desktop.show_app_menu = false;
                        desktop.selected_action = 0;
                        desktop.changed(cx);
                    } else if !desktop.query.is_empty() {
                        desktop.set_query(String::new(), cx);
                    } else if desktop.view_mode.in_submenu() {
                        desktop.set_view_mode(ViewMode::Main, cx);
                    } else if desktop.active_category != BarCategory::All {
                        desktop.set_category(BarCategory::All, cx);
                    }
                });
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                let key = event.keystroke.key.as_str();
                if key == "backspace" {
                    if event.keystroke.modifiers.platform {
                        this.delete_word(cx);
                    } else {
                        this.backspace(cx);
                    }
                    cx.stop_propagation();
                    return;
                }
                if key == "up" || key == "left" {
                    this.select_previous(cx);
                    cx.stop_propagation();
                    return;
                }
                if key == "down" || key == "right" {
                    this.select_next(cx);
                    cx.stop_propagation();
                    return;
                }
                if key == "enter" {
                    this.confirm(cx);
                    cx.stop_propagation();
                    return;
                }
                if key == "tab" {
                    if event.keystroke.modifiers.shift {
                        this.desktop.update(cx, |desktop, cx| {
                            let next = desktop.active_category.next();
                            desktop.set_category(next, cx);
                        });
                        cx.stop_propagation();
                        return;
                    }
                    let title = this
                        .desktop
                        .read(cx)
                        .pills
                        .get(this.desktop.read(cx).selected_pill)
                        .map(|p| p.title.clone());
                    if let Some(title) = title {
                        this.desktop.update(cx, |desktop, cx| {
                            desktop.set_query(title, cx);
                        });
                        cx.stop_propagation();
                        return;
                    }
                }
                let mods = event.keystroke.modifiers;
                if !mods.platform && !mods.control && !mods.alt {
                    if let Some(ch) = event.keystroke.key_char.as_deref() {
                        if ch.chars().count() == 1 && !ch.chars().all(|c| c.is_control()) {
                            this.append(ch, cx);
                            cx.stop_propagation();
                        }
                    }
                }
            }))
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .when(show_status, |col| {
                        col.child(self.render_status_strip(self.desktop.read(cx)))
                    })
                    .child(
                        div()
                            .relative()
                            .w(px(750.))
                            .h(px(475.))
                            .rounded(px(26.))
                            .bg(rgb(TC_SCRIM))
                            .border_1()
                            .border_color(rgb(TC_PANEL_STROKE))
                            .overflow_hidden()
                            .flex()
                            .flex_col()
                            .justify_between()
                            .child(
                                div()
                                    .w_full()
                                    .flex_1()
                                    .pt(px(10.))
                                    .overflow_hidden()
                                    .flex()
                                    .flex_col()
                                    .child(self.render_bar(cx))
                                    .child(self.render_results(cx)),
                            )
                            .child(self.render_footer(self.desktop.read(cx)))
                            .when(show_popover, |panel| {
                                panel.child(self.render_actions_overlay(self.desktop.read(cx)))
                            }),
                    )
                    .when(!active_notifs.is_empty(), |col| {
                        col.child(
                            div()
                                .pt(px(6.))
                                .flex()
                                .flex_col()
                                .gap(px(4.))
                                .children(active_notifs),
                        )
                    }),
            )
    }
}

fn launcher_window_options(cx: &App) -> WindowOptions {
    WindowOptions {
        app_id: Some("alpenglowed-launcher".into()),
        titlebar: None,
        window_bounds: Some(WindowBounds::centered(size(px(750.), px(475.)), cx)),
        kind: WindowKind::PopUp,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        window_background: WindowBackgroundAppearance::Transparent,
        window_decorations: Some(WindowDecorations::Client),
        ..Default::default()
    }
}

fn installer_window_options(cx: &App) -> WindowOptions {
    WindowOptions {
        app_id: Some("alpenglowed-installer".into()),
        titlebar: None,
        window_bounds: Some(WindowBounds::centered(size(px(760.), px(560.)), cx)),
        kind: WindowKind::PopUp,
        is_resizable: false,
        window_background: WindowBackgroundAppearance::Opaque,
        window_decorations: Some(WindowDecorations::Client),
        ..Default::default()
    }
}

fn open_launcher_window(desktop: &Entity<DesktopModel>, cx: &mut App) -> AnyWindowHandle {
    let desktop_entity = desktop.clone();
    let handle = cx
        .open_window(launcher_window_options(cx), move |window, cx| {
            let view = cx.new(|cx| LauncherWindow::new(desktop_entity, cx));
            window.activate_window();
            view
        })
        .unwrap();
    let any_handle: AnyWindowHandle = handle.into();
    desktop.update(cx, |desktop, cx| {
        desktop.launcher = Some(any_handle);
        desktop.changed(cx);
    });
    any_handle
}

fn open_installer_window(source: PathBuf, target: PathBuf, cx: &mut App) {
    let _ = cx.open_window(installer_window_options(cx), move |window, cx| {
        let view = cx.new(|_| InstallerWindow::new(source, target));
        window.activate_window();
        view
    });
}

fn focus_or_open_launcher(desktop: &Entity<DesktopModel>, cx: &mut App) {
    let Some(handle) = desktop.read(cx).launcher else {
        open_launcher_window(desktop, cx);
        return;
    };

    if handle
        .update(cx, |_, window, _| {
            window.activate_window();
        })
        .is_ok()
    {
        return;
    }

    desktop.update(cx, |desktop, cx| {
        desktop.launcher = None;
        desktop.changed(cx);
    });
    open_launcher_window(desktop, cx);
}

fn handle_cli_subcommand(args: &[String]) -> Option<i32> {
    let sub = args.first()?.as_str();
    let rest = if args.len() > 1 {
        args[1..].join(" ")
    } else {
        String::new()
    };
    match sub {
        "open" => {
            if rest.trim().is_empty() {
                eprintln!("usage: alpenglowed open <url>");
                return Some(2);
            }
            let url = if rest.starts_with("http://") || rest.starts_with("https://") {
                rest
            } else {
                format!("https://{}", rest.trim())
            };
            let _ = Command::new("xdg-open").arg(&url).spawn();
            println!("opened {url}");
            Some(0)
        }
        "search" => {
            if rest.trim().is_empty() {
                eprintln!("usage: alpenglowed search <query>");
                return Some(2);
            }
            let url = match plugin::parse_browser_command(&rest) {
                Some(plugin::BrowserCommand::Search { engine_name, query }) => {
                    plugin::browser_search_url(&engine_name, &query)
                }
                _ => plugin::browser_search_url("google", rest.trim()),
            };
            let _ = Command::new("xdg-open").arg(&url).spawn();
            println!("searched {url}");
            Some(0)
        }
        "launch" => {
            if rest.trim().is_empty() {
                eprintln!("usage: alpenglowed launch <app>");
                return Some(2);
            }
            let _ = Command::new(rest.trim()).spawn();
            println!("launched {}", rest.trim());
            Some(0)
        }
        "run" => {
            if rest.trim().is_empty() {
                eprintln!("usage: alpenglowed run <command>");
                return Some(2);
            }
            let status = Command::new("sh")
                .arg("-c")
                .arg(&rest)
                .status()
                .map(|s| s.code().unwrap_or(1))
                .unwrap_or(1);
            Some(status)
        }
        "niri" => {
            let Some(action) = layout::LayoutAction::from_label(rest.trim()) else {
                eprintln!("unknown niri action: {}", rest.trim());
                return Some(2);
            };
            let _ = session::dispatch(&session::SessionRequest::Layout {
                action: action.clone(),
            });
            println!("niri {}", action.label());
            Some(0)
        }
        "mode" => {
            let Some(mode) = WindowMode::from_label(rest.trim()) else {
                eprintln!("unknown window mode: {}", rest.trim());
                return Some(2);
            };
            let _ = session::dispatch(&session::SessionRequest::SetWindowMode { mode });
            println!("mode {}", mode.label());
            Some(0)
        }
        _ => None,
    }
}

fn main() {
    let raw_args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(code) = handle_cli_subcommand(&raw_args) {
        if code != 0 {
            std::process::exit(code);
        }
        return;
    }

    let options = UiOptions::from_env();

    if std::env::args().any(|arg| arg == "--help") {
        eprintln!("alpenglowed — Alpenglow desktop launcher & window shell");
        eprintln!("Subcommands: open <url> | search <q> | browser | launch <app> | run <cmd> | niri <action> | mode <mode>");
        eprintln!("Flags:");
        eprintln!("  ALPENGLOWED_ROLE  potato|desktop (potatoes is a deprecated alias for potato)");
        eprintln!("  --session-contract  Print the Alpenglow session contract as JSON");
        #[cfg(feature = "compositor")]
        eprintln!(
            "  --compositor      Enable embedded smithay compositor (experimental; desktop only)"
        );
        eprintln!("  --polybar         Emit polybar status line");
        eprintln!("  --polybar-module=  Emit a single polybar module");
        eprintln!("  --probe-actions   List available desktop actions");
        eprintln!("  --notify=msg      Send notification to daemon");
        eprintln!("  --smoke-wayland   Test Wayland connection");
        eprintln!("  --open-settings   Start launcher in System view");
        eprintln!("  --floating        Start in floating mode");
        eprintln!("  --demo-layout     Use demo layout with 4 panes");
        return;
    }
    if std::env::args().any(|arg| arg == "--session-contract") {
        println!("{}", role::session_contract());
        return;
    }
    if std::env::args().any(|arg| arg == "--polybar") {
        let mode = std::env::var("ALPENGLOWED_MODE")
            .ok()
            .and_then(|m| WindowMode::from_label(&m))
            .unwrap_or(WindowMode::Tiling);
        if let Err(error) = role::SessionRole::resolve() {
            eprintln!("{}", error.message());
            std::process::exit(2);
        }
        println!("{}", de::DesktopState::detect(mode.label()).polybar());
        return;
    }
    if let Some(module) = std::env::args().find_map(|arg| {
        arg.strip_prefix("--polybar-module=")
            .map(ToString::to_string)
    }) {
        print!("{}", polybar_module(&module));
        return;
    }
    if std::env::args().any(|arg| arg == "--probe-actions") {
        for line in de::probe_actions() {
            println!("{line}");
        }
        return;
    }
    if std::env::args().any(|arg| arg == "--smoke-actions-safe") {
        for line in de::smoke_safe_actions() {
            println!("{line}");
        }
        return;
    }
    if let Some(notif) =
        std::env::args().find_map(|arg| arg.strip_prefix("--notify=").map(ToString::to_string))
    {
        let path = std::env::var_os("XDG_RUNTIME_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("alpenglowed")
            .join("notifications");
        if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(&path) {
            let payload = serde_json::json!({
                "title": "alpenglowed",
                "body": notif,
                "urgency": "normal"
            });
            let _ = serde_json::to_writer(&mut stream, &payload);
        } else {
            eprintln!("notification daemon not running");
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().any(|arg| arg == "--smoke-wayland") {
        match de::smoke_wayland() {
            Ok(()) => println!("wayland ok"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return;
    }

    let compositor_requested = std::env::args().any(|arg| arg == "--compositor");
    let start_compositor = compositor_requested
        && matches!(options.role, role::SessionRole::Desktop)
        && cfg!(feature = "compositor");

    #[cfg(feature = "compositor")]
    let (_compositor_tx, _compositor_rx) = if start_compositor {
        let (cmd, rx) = compositor::start();
        std::env::set_var("ALPENGLOW_COMPOSITOR", "1");
        (Some(cmd), Some(rx))
    } else {
        (None, None)
    };

    #[cfg(not(feature = "compositor"))]
    let (_compositor_tx, _compositor_rx): (
        Option<std::sync::mpsc::Sender<()>>,
        Option<std::sync::mpsc::Receiver<()>>,
    ) = (None, None);

    if !start_compositor {
        ensure_wayland_display();
    }

    Application::new()
        .with_assets(icons::IconAssets)
        .run(move |cx: &mut App| {
            let _ = cx
                .text_system()
                .add_fonts(vec![Cow::Borrowed(GEIST), Cow::Borrowed(GEIST_MONO)]);

            cx.bind_keys([
                KeyBinding::new("cmd-space", FocusBar, None),
                KeyBinding::new("escape", DefocusBar, None),
                KeyBinding::new("enter", Confirm, None),
                KeyBinding::new("cmd-k", ToggleActionsMenu, None),
                KeyBinding::new("ctrl-k", ToggleActionsMenu, None),
                KeyBinding::new("cmd-shift-k", ClearQueryAction, None),
                KeyBinding::new("cmd-q", Quit, None),
                KeyBinding::new("cmd-shift-]", FocusNextPane, None),
                KeyBinding::new("cmd-shift-[", FocusPreviousPane, None),
                KeyBinding::new("cmd-shift--", ClosePane, None),
                KeyBinding::new("cmd-alt-f", ToggleFloatPane, None),
                KeyBinding::new("cmd-alt-h", SplitRow, None),
                KeyBinding::new("cmd-alt-v", SplitColumn, None),
                KeyBinding::new("cmd-alt-x", FlipAxis, None),
                KeyBinding::new("cmd-alt-r", ResetLayout, None),
                KeyBinding::new("cmd-alt-left", NudgeLeft, None),
                KeyBinding::new("cmd-alt-right", NudgeRight, None),
                KeyBinding::new("cmd-alt-up", NudgeUp, None),
                KeyBinding::new("cmd-alt-down", NudgeDown, None),
                KeyBinding::new("cmd-alt-=", ExpandWindow, None),
                KeyBinding::new("cmd-alt--", ContractWindow, None),
                KeyBinding::new("cmd-alt-l", GrowPane, None),
                KeyBinding::new("cmd-alt-j", ShrinkPane, None),
                KeyBinding::new("cmd-alt-t", ToggleTerminalPane, None),
                KeyBinding::new("cmd-,", ToggleSettingsWindow, None),
                KeyBinding::new("cmd-b", ToggleStatusBarAction, None),
                KeyBinding::new("cmd-alt-m", CycleWindowModeAction, None),
                KeyBinding::new("cmd-alt-b", BalancePanesAction, None),
                KeyBinding::new("cmd-alt-c", CenterFocusedAction, None),
                KeyBinding::new("cmd-shift-left", MoveWindowLeft, None),
                KeyBinding::new("cmd-shift-right", MoveWindowRight, None),
                KeyBinding::new("cmd-shift-up", MoveWindowUp, None),
                KeyBinding::new("cmd-shift-down", MoveWindowDown, None),
                KeyBinding::new("cmd-alt-1", FocusFirstPane, None),
                KeyBinding::new("cmd-alt-9", FocusLastPane, None),
                KeyBinding::new("cmd-1", SelectResult1, None),
                KeyBinding::new("cmd-2", SelectResult2, None),
                KeyBinding::new("cmd-3", SelectResult3, None),
                KeyBinding::new("cmd-4", SelectResult4, None),
                KeyBinding::new("cmd-5", SelectResult5, None),
                KeyBinding::new("cmd-6", SelectResult6, None),
                KeyBinding::new("cmd-7", SelectResult7, None),
                KeyBinding::new("cmd-8", SelectResult8, None),
            ]);

            let desktop_options = options.clone();
            #[cfg(feature = "compositor")]
            let desktop =
                cx.new(|_| DesktopModel::new(desktop_options, _compositor_rx, _compositor_tx));
            #[cfg(not(feature = "compositor"))]
            let desktop = cx.new(|_| DesktopModel::new(desktop_options));

            let installer = installer_paths();
            if let Some((source, target)) = installer {
                open_installer_window(source, target, cx);
            } else if std::env::args().any(|arg| arg == "--standalone-terminal") {
                terminal::open_standalone_terminal_window(cx);
            } else {
                focus_or_open_launcher(&desktop, cx);
            }
        });
}

impl UiOptions {
    fn from_env() -> Self {
        let cfg = config::Config::load();

        let role = match role::SessionRole::resolve() {
            Ok(role) => role,
            Err(error) => {
                eprintln!("{}", error.message());
                std::process::exit(2);
            }
        };
        let status_bar = std::env::args().any(|arg| arg == "--status-bar")
            || matches!(
                std::env::var("ALPENGLOWED_STATUS_BAR").as_deref(),
                Ok("1" | "true" | "yes")
            )
            || cfg.status_bar.unwrap_or(role.skinny());
        let external_polybar = std::env::args().any(|arg| arg == "--external-polybar")
            || matches!(
                std::env::var("ALPENGLOWED_EXTERNAL_BAR").as_deref(),
                Ok("polybar")
            )
            || cfg.external_polybar.unwrap_or(false);
        let open_settings = std::env::args().any(|arg| arg == "--open-settings")
            || cfg.open_settings.unwrap_or(false);
        let initial_view_mode = if std::env::args().any(|arg| arg == "browser") {
            ViewMode::Browser
        } else if std::env::args().any(|arg| arg == "terminal" || arg == "--terminal") {
            ViewMode::Terminal
        } else {
            match std::env::var("ALPENGLOWED_VIEW").as_deref() {
                Ok("windows" | "niri") => ViewMode::WindowControls,
                Ok("browser") => ViewMode::Browser,
                Ok("terminal") => ViewMode::Terminal,
                Ok("clipboard") => ViewMode::ClipboardHistory,
                Ok("files") => ViewMode::FileSearch,
                _ => ViewMode::Main,
            }
        };
        let initial_query = std::env::args()
            .find_map(|arg| arg.strip_prefix("--query=").map(ToString::to_string))
            .or_else(|| std::env::var("ALPENGLOWED_QUERY").ok())
            .or_else(|| cfg.initial_query.clone())
            .unwrap_or_default();
        let mode = if std::env::args().any(|arg| arg == "--floating") {
            WindowMode::Floating
        } else if let Some(env_mode) = std::env::var("ALPENGLOWED_MODE")
            .ok()
            .and_then(|m| WindowMode::from_label(&m))
        {
            env_mode
        } else if let Some(cfg_mode) = cfg.mode.as_deref().and_then(WindowMode::from_label) {
            cfg_mode
        } else {
            WindowMode::Tiling
        };
        let demo_layout = std::env::args().any(|arg| arg == "--demo-layout")
            || matches!(
                std::env::var("ALPENGLOWED_DEMO_LAYOUT").as_deref(),
                Ok("1" | "true" | "yes")
            )
            || cfg.demo_layout.unwrap_or(false);

        Self {
            status_bar,
            external_polybar,
            open_settings,
            initial_query,
            mode,
            demo_layout,
            role,
            initial_view_mode,
        }
    }
}

fn polybar_module(name: &str) -> String {
    match name {
        "title" => external_bar_state()
            .map(|state| state.focused_title)
            .filter(|title| !title.trim().is_empty())
            .unwrap_or_else(|| "alpenglowed".to_string()),
        "mode" => external_bar_state()
            .map(|state| state.mode)
            .filter(|mode| !mode.trim().is_empty())
            .unwrap_or_else(|| "launcher".to_string()),
        "role" => role::SessionRole::resolve()
            .map(|role| role.label().to_string())
            .unwrap_or_else(|_| "desktop".to_string()),
        "backend" => polybar_backend(&de::DesktopState::detect("tiling")),
        "battery" => battery_value().unwrap_or_else(|| "battery unavailable".to_string()),
        "load" => load_value().unwrap_or_else(|| "load unavailable".to_string()),
        "memory" => memory_value().unwrap_or_else(|| "memory unavailable".to_string()),
        _ => String::new(),
    }
}

fn installer_paths() -> Option<(PathBuf, PathBuf)> {
    let source = std::env::var_os("ALPENGLOWED_INSTALLER_SOURCE")
        .map(PathBuf::from)
        .or_else(|| {
            let path = PathBuf::from("/run/alpenglow/alpenglow.img.zst");
            path.is_file().then_some(path)
        })?;
    let target = std::env::var_os("ALPENGLOWED_INSTALLER_TARGET").map(PathBuf::from)?;
    Some((source, target))
}

fn ensure_wayland_display() {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() || std::env::var_os("DISPLAY").is_some() {
        return;
    }

    let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR") else {
        return;
    };

    let wayland_socket = std::path::Path::new(&runtime_dir).join("wayland-0");
    if wayland_socket.exists() {
        std::env::set_var("WAYLAND_DISPLAY", "wayland-0");
    }
}

fn external_bar_state_path() -> PathBuf {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("alpenglowed-polybar.json")
}

fn write_external_bar_state(desktop: &DesktopModel) {
    let state = ExternalBarState {
        focused_title: desktop.layout.focused_title().to_string(),
        layout: desktop.layout.summary(),
        mode: desktop.mode.label().to_string(),
        last_action: desktop.last_action.clone(),
    };
    let _ = fs::write(
        external_bar_state_path(),
        serde_json::to_vec(&state).unwrap_or_default(),
    );
}

fn external_bar_state() -> Option<ExternalBarState> {
    #[cfg(test)]
    {
        None
    }
    #[cfg(not(test))]
    {
        let data = fs::read(external_bar_state_path()).ok()?;
        serde_json::from_slice(&data).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_list_items_should_extract_li_text() {
        assert_eq!(
            html_list_items("<ul><li>one</li><li>two &amp; three</li></ul>"),
            vec!["one".to_string(), "two & three".to_string()]
        );
    }

    #[test]
    fn shell_text_component_should_render_launcher_meta() {
        let text = shell_text_component(
            "LauncherMeta",
            &[("selection", "1/6"), ("subtitle", "window mode")],
        );
        assert_eq!(text, "1/6 window mode");
    }

    #[test]
    fn shell_list_component_should_render_shortcuts() {
        let shortcuts = shell_list_component("SettingsShortcuts");
        assert!(shortcuts
            .iter()
            .any(|line| line.contains("Cmd-Space launcher")));
        assert!(shortcuts
            .iter()
            .any(|line| line.contains("Cmd-Alt-F toggle float")));
    }

    #[test]
    fn shell_row_component_should_render_launcher_row() {
        let row = shell_row_component(">", "Tile windows", "window mode");
        assert_eq!(row.0, ">");
        assert_eq!(row.1, "Tile windows");
        assert_eq!(row.2, "window mode");
    }

    #[test]
    fn shell_bar_component_should_render_launcher_bar() {
        let bar = shell_bar_component("window", "1/6 window mode");
        assert_eq!(bar.0, "window");
        assert_eq!(bar.1, "1/6 window mode");
    }

    #[test]
    fn shell_top_bar_title_component_should_render_header() {
        let header = shell_top_bar_title_component("alpenglowed", "focused pane detail");
        assert_eq!(header.0, "alpenglowed");
        assert_eq!(header.1, "focused pane detail");
    }

    #[test]
    fn shell_top_bar_metric_component_should_render_value() {
        let metric = shell_top_bar_metric_component("clock", "11:27");
        assert_eq!(metric.0, "clock");
        assert_eq!(metric.1, "11:27");
    }

    #[test]
    fn polybar_module_should_return_mode() {
        assert_eq!(polybar_module("mode"), "launcher");
    }

    #[test]
    fn polybar_module_should_return_title_fallback() {
        assert_eq!(polybar_module("title"), "alpenglowed");
    }

    #[test]
    fn polybar_module_should_return_empty_for_unknown_name() {
        assert_eq!(polybar_module("unknown"), "");
    }

    #[test]
    fn polybar_module_should_return_a_known_role() {
        let role = polybar_module("role");
        assert!(role == "potato" || role == "desktop");
    }

    #[test]
    fn shell_section_header_component_should_render_settings_header() {
        let header = shell_section_header_component("Windows", "root row");
        assert_eq!(header.0, "Windows");
        assert_eq!(header.1, "root row");
    }

    #[test]
    fn shell_status_row_component_should_render_settings_pills() {
        let row = shell_status_row_component("tiling", "3 tiled 1 floating", "Workspace");
        assert_eq!(
            row,
            vec![
                "tiling".to_string(),
                "3 tiled 1 floating".to_string(),
                "Workspace".to_string()
            ]
        );
    }

    #[test]
    fn installer_copy_should_write_source_to_target() {
        let dir = std::env::temp_dir();
        let suffix = std::process::id();
        let source = dir.join(format!("alpenglowed-installer-source-{suffix}"));
        let target = dir.join(format!("alpenglowed-installer-target-{suffix}"));
        let source_bytes = b"ALPENGLOWED-INSTALL\n";
        std::fs::write(&source, source_bytes).unwrap();
        std::fs::write(&target, b"....................").unwrap();

        let bytes = copy_install_image(&source, &target).unwrap();

        assert_eq!(bytes, source_bytes.len() as u64);
        assert_eq!(std::fs::read(&target).unwrap(), source_bytes);
        let _ = std::fs::remove_file(source);
        let _ = std::fs::remove_file(target);
    }

    #[test]
    fn row_offsets_priority_pipeline_orders_terminal_url_calc_win_and_help() {
        let term = RowOffsets::compute("> cargo test", ViewMode::Main, true);
        assert!(term.has_terminal);
        assert!(!term.has_url);
        assert_eq!(term.command_offset, 1);

        let bang = RowOffsets::compute("! ls -la", ViewMode::Main, true);
        assert!(bang.has_terminal);

        let macro_row = RowOffsets::compute("!shrug", ViewMode::Main, true);
        assert!(macro_row.has_macro);
        assert!(!macro_row.has_terminal);

        let url = RowOffsets::compute("github.com/smithay", ViewMode::Main, true);
        assert!(url.has_url);
        assert!(url.command_offset >= 1);

        let search_bang = RowOffsets::compute("!gh smithay", ViewMode::Main, true);
        assert!(search_bang.has_url);
        assert!(!search_bang.has_terminal);

        let calc = RowOffsets::compute("24 * 7", ViewMode::Main, true);
        assert!(calc.has_calc);

        let win = RowOffsets::compute("niri cycle", ViewMode::Main, true);
        assert!(win.has_win);

        let help = RowOffsets::compute("help niri", ViewMode::Main, true);
        assert!(help.has_help);
    }

    #[test]
    fn query_history_navigates_with_prefix_matching() {
        let mut hist = QueryHistory::new();
        hist.record("g rust");
        hist.record("niri cycle-width");
        hist.record("niri focus-column-right");

        assert_eq!(
            hist.navigate_up("niri"),
            Some("niri focus-column-right".to_string())
        );
        assert_eq!(
            hist.navigate_up("niri"),
            Some("niri cycle-width".to_string())
        );
        assert_eq!(hist.navigate_up("niri"), None);
        assert_eq!(
            hist.navigate_down(),
            Some("niri focus-column-right".to_string())
        );
        assert_eq!(hist.navigate_down(), Some("niri".to_string()));
    }

    #[test]
    fn view_mode_and_cli_subcommands_handle_niri_and_modes() {
        assert!(!ViewMode::Main.in_submenu());
        assert!(ViewMode::WindowControls.in_submenu());
        assert!(ViewMode::Browser.in_submenu());
        assert!(ViewMode::Terminal.in_submenu());

        assert_eq!(
            handle_cli_subcommand(&["niri".to_string(), "cycle-column-width".to_string()]),
            Some(0)
        );
        assert_eq!(
            handle_cli_subcommand(&["niri".to_string(), "room-focus".to_string()]),
            Some(0)
        );
        assert_eq!(
            handle_cli_subcommand(&["mode".to_string(), "niri".to_string()]),
            Some(0)
        );
        assert_eq!(
            handle_cli_subcommand(&["niri".to_string(), "invalid-action".to_string()]),
            Some(2)
        );
    }

    #[test]
    fn tinycast_row_classifier_assigns_linux_keycaps_and_sections() {
        let (sec, kind, glyph, _, _, keys) = classify_plugin_row(
            "Switch Room",
            &PluginAction::Layout {
                action: layout::LayoutAction::CycleRoomLayout,
            },
            WindowMode::NiriStrip,
        );
        assert_eq!(sec, "Favorites");
        assert_eq!(kind, "Window");
        assert_eq!(glyph, "⊞");
        assert_eq!(keys, vec!["Super", "Space"]);

        let (sec_snap, kind_snap, _, _, _, keys_snap) = classify_plugin_row(
            "Left Half",
            &PluginAction::Layout {
                action: layout::LayoutAction::SnapLeftHalf,
            },
            WindowMode::Floating,
        );
        assert_eq!(sec_snap, "Favorites");
        assert_eq!(kind_snap, "Window");
        assert_eq!(keys_snap, vec!["Ctrl", "Alt", "←"]);
    }
}
