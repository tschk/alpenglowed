use crate::de::DesktopAction;
use crate::layout::LayoutAction;
use crate::runner::WindowMode;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::io::Write;
#[cfg(feature = "full")]
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
#[cfg(feature = "full")]
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginAction {
    Launch { program: String },
    Shell { command: String },
    FocusWindow { id: usize },
    SetWindowMode { mode: WindowMode },
    CycleWindowMode,
    Layout { action: LayoutAction },
    ShowStatusBar,
    HideStatusBar,
    ToggleStatusBar,
    ToggleSettings,
    OpenSettings,
    CloseSettings,
    FactoryReset,
    Desktop { action: DesktopAction },
    ToggleTerminal,
    TerminalClear,
    TerminalWrite { line: String },
    ClearQuery,
    CycleSelection { forward: bool },
    SelectResult { index: usize },
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginResult {
    pub plugin_id: String,
    pub title: String,
    pub subtitle: String,
    pub score: i64,
    pub action: PluginAction,
}

pub trait Plugin {
    fn id(&self) -> &str;
    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult>;
}

pub struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowTarget {
    pub id: usize,
    pub title: String,
    pub focused: bool,
    pub floating: bool,
}

impl PluginRegistry {
    pub fn with_role(role: crate::role::SessionRole) -> Self {
        let mut registry = Self {
            plugins: Vec::new(),
        };
        registry.register(Box::new(EmojiPlugin));
        registry.register(Box::new(FileSearchPlugin));
        registry.register(Box::new(ClipboardPlugin));
        registry.register(Box::new(ShellPlugin));
        registry.register(Box::new(CalculatorPlugin));
        registry.register(Box::new(WindowModePlugin));
        registry.register(Box::new(LayoutPlugin));
        registry.register(Box::new(InterfacePlugin));
        registry.register(Box::new(TerminalPlugin));
        registry.register(Box::new(TerminalClearPlugin));
        registry.register(Box::new(SettingsPlugin));
        registry.register(Box::new(FactoryResetPlugin));
        registry.register(Box::new(DesktopActionsPlugin));
        registry.register(Box::new(AppLauncherPlugin));
        registry.register(Box::new(ProcessKillPlugin));
        registry.register(Box::new(VolumePlugin));
        registry.register(Box::new(BrightnessPlugin));
        registry.register(Box::new(TimerPlugin));
        registry.register(Box::new(SystemInfoPlugin));
        registry.register(Box::new(NetworkInfoPlugin));
        registry.register(Box::new(HelpPlugin::for_role(role)));
        registry.register(Box::new(ColorPlugin));
        registry.register(Box::new(UnitConverterPlugin));
        registry.register(Box::new(RecentFilesPlugin));
        registry.register(Box::new(QuicklinksPlugin));
        registry.register(Box::new(SnippetsPlugin));
        registry.register(Box::new(DictionaryPlugin));
        registry.register(Box::new(NotesPlugin));
        if role.extras() {
            registry.register(Box::new(BrowserPlugin));
        }
        #[cfg(feature = "full")]
        if role.extras() {
            registry.register(Box::new(WebSearchPlugin));
            registry.register(Box::new(TranslatePlugin));
            registry.register(Box::new(WeatherPlugin));
            registry.register(Box::new(SpotifyPlugin::new()));
            for plugin in CommandPlugin::load_default() {
                registry.register(Box::new(plugin));
            }
        }
        registry
    }

    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push(plugin);
    }

    pub fn query_with_windows(
        &self,
        query: &str,
        matcher: &SkimMatcherV2,
        windows: &[WindowTarget],
    ) -> Vec<PluginResult> {
        let mut results = self
            .plugins
            .iter()
            .flat_map(|plugin| plugin.query(query, matcher))
            .collect::<Vec<_>>();
        results.extend(window_results(query, matcher, windows));
        for result in &mut results {
            if result.plugin_id != "apps" {
                result.score = result.score.saturating_add(50);
            }
        }
        results.sort_by_key(|result| Reverse(result.score));
        results.truncate(8);
        results
    }
}

pub fn shell_single_quote(input: &str) -> String {
    format!("'{}'", input.replace('\'', "'\\''"))
}

#[cfg_attr(not(feature = "full"), allow(dead_code))]
pub fn urlencode_str(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 3);
    for &byte in text.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn clipboard_copy_command(text: &str) -> String {
    let quoted = shell_single_quote(text);
    format!(
        "printf '%s' {quoted} | wl-copy 2>/dev/null || printf '%s' {quoted} | xclip -selection clipboard"
    )
}

fn window_results(
    query: &str,
    matcher: &SkimMatcherV2,
    windows: &[WindowTarget],
) -> Vec<PluginResult> {
    let query = query.trim();
    windows
        .iter()
        .filter_map(|window| {
            let action_title = format!("Focus {}", window.title);
            let window_title = window.title.as_str();
            score_window(query, matcher, window_title, &action_title).map(|score| PluginResult {
                plugin_id: "windows".to_string(),
                title: action_title,
                subtitle: if window.focused {
                    "focused pane".to_string()
                } else if window.floating {
                    "floating pane".to_string()
                } else {
                    "tiled pane".to_string()
                },
                score,
                action: PluginAction::FocusWindow { id: window.id },
            })
        })
        .collect()
}

fn score_window(
    query: &str,
    matcher: &SkimMatcherV2,
    window_title: &str,
    action_title: &str,
) -> Option<i64> {
    if query.eq_ignore_ascii_case(window_title) {
        return Some(i64::MAX - 2);
    }
    if query.eq_ignore_ascii_case(action_title) {
        return Some(i64::MAX - 1);
    }
    let title_score = matcher.fuzzy_match(window_title, query);
    let action_score = matcher.fuzzy_match(action_title, query);
    let boosted = title_score
        .into_iter()
        .chain(action_score)
        .max()
        .map(|score| score + 500);
    if query.eq_ignore_ascii_case("focus") {
        return boosted.or(Some(500));
    }
    boosted
}

#[cfg(feature = "full")]
struct WebSearchPlugin;

#[cfg(feature = "full")]
impl Plugin for WebSearchPlugin {
    fn id(&self) -> &str {
        "web"
    }

    fn query(&self, query: &str, _matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        let search = query.trim().strip_prefix('?').map(str::trim).unwrap_or("");
        if search.is_empty() {
            return Vec::new();
        }
        let encoded = urlencode_str(search);
        let url = format!("https://duckduckgo.com/?q={encoded}");
        vec![PluginResult {
            plugin_id: self.id().to_string(),
            title: format!("Search web for \"{search}\""),
            subtitle: "duckduckgo".to_string(),
            score: i64::MAX,
            action: PluginAction::Shell {
                command: format!("xdg-open {}", shell_single_quote(&url)),
            },
        }]
    }
}

struct EmojiPlugin;

const EMOJIS: &[(&str, &str)] = &[
    ("smile", "😄"),
    ("grin", "😁"),
    ("joy", "😂"),
    ("wink", "😉"),
    ("heart_eyes", "😍"),
    ("kiss", "😘"),
    ("thinking", "🤔"),
    ("neutral", "😐"),
    ("sunglasses", "😎"),
    ("cool", "😎"),
    ("cry", "😢"),
    ("sob", "😭"),
    ("angry", "😠"),
    ("sleeping", "😴"),
    ("poop", "💩"),
    ("fire", "🔥"),
    ("star", "⭐"),
    ("heart", "❤️"),
    ("broken_heart", "💔"),
    ("hundred", "💯"),
    ("clap", "👏"),
    ("wave", "👋"),
    ("thumbsup", "👍"),
    ("thumbsdown", "👎"),
    ("ok", "👌"),
    ("pray", "🙏"),
    ("muscle", "💪"),
    ("party", "🎉"),
    ("rocket", "🚀"),
    ("computer", "💻"),
    ("globe", "🌍"),
    ("check", "✅"),
    ("cross", "❌"),
    ("warning", "⚠️"),
    ("lock", "🔒"),
    ("unlock", "🔓"),
    ("bell", "🔔"),
    ("link", "🔗"),
    ("search", "🔍"),
    ("pencil", "✏️"),
    ("trash", "🗑️"),
    ("folder", "📁"),
    ("mail", "📧"),
    ("home", "🏠"),
    ("music", "🎵"),
    ("coffee", "☕"),
    ("beer", "🍺"),
    ("pizza", "🍕"),
    ("burger", "🍔"),
    ("cat", "🐱"),
    ("dog", "🐶"),
    ("robot", "🤖"),
    ("ghost", "👻"),
    ("eyes", "👀"),
];

impl Plugin for EmojiPlugin {
    fn id(&self) -> &str {
        "emoji"
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        let search = query.trim().strip_prefix(':').map(str::trim).unwrap_or("");
        if search.is_empty() {
            return Vec::new();
        }
        let mut results: Vec<PluginResult> = EMOJIS
            .iter()
            .filter_map(|(name, emoji)| {
                matcher.fuzzy_match(name, search).map(|score| PluginResult {
                    plugin_id: self.id().to_string(),
                    title: format!("{emoji}  :{name}"),
                    subtitle: "emoji".to_string(),
                    score,
                    action: PluginAction::Shell {
                        command: clipboard_copy_command(emoji),
                    },
                })
            })
            .collect();
        results.sort_by_key(|r| std::cmp::Reverse(r.score));
        results.truncate(6);
        results
    }
}

struct FileSearchPlugin;

fn cached_searchable_files() -> &'static [String] {
    static FILES: OnceLock<Vec<String>> = OnceLock::new();
    FILES.get_or_init(|| {
        let mut out = Vec::new();
        let mut roots = Vec::new();
        if let Ok(cwd) = std::env::current_dir() {
            roots.push(cwd);
        }
        if let Ok(home) = std::env::var("HOME") {
            roots.push(std::path::PathBuf::from(home));
        }
        for root in roots {
            collect_files_bounded(&root, 2, &mut out, 600);
        }
        out.sort();
        out.dedup();
        out
    })
}

fn collect_files_bounded(dir: &std::path::Path, depth: usize, out: &mut Vec<String>, limit: usize) {
    if out.len() >= limit {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if out.len() >= limit {
            break;
        }
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.starts_with('.') || name == "target" || name == "node_modules" {
            continue;
        }
        if path.is_file() {
            out.push(path.display().to_string());
        } else if depth > 0 && path.is_dir() {
            collect_files_bounded(&path, depth - 1, out, limit);
        }
    }
}

impl Plugin for FileSearchPlugin {
    fn id(&self) -> &str {
        "files"
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        let search = query.trim().strip_prefix('/').map(str::trim).unwrap_or("");
        if search.is_empty() {
            return Vec::new();
        }

        let mut results: Vec<PluginResult> = cached_searchable_files()
            .iter()
            .filter_map(|path| {
                let filename = std::path::Path::new(path)
                    .file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or(std::borrow::Cow::Borrowed(path.as_str()));
                matcher
                    .fuzzy_match(&filename, search)
                    .or_else(|| matcher.fuzzy_match(path, search))
                    .map(|score| PluginResult {
                        plugin_id: self.id().to_string(),
                        title: filename.to_string(),
                        subtitle: path.to_string(),
                        score: score.min(100),
                        action: PluginAction::Shell {
                            command: format!("xdg-open {}", shell_single_quote(path)),
                        },
                    })
            })
            .collect();
        results.sort_by_key(|r| std::cmp::Reverse(r.score));
        results.truncate(6);
        results
    }
}

struct ClipboardPlugin;

impl Plugin for ClipboardPlugin {
    fn id(&self) -> &str {
        "clipboard"
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        let search = query.trim().to_lowercase();
        if !search.starts_with("clip") && !search.starts_with("paste") && !search.starts_with("cb")
        {
            return Vec::new();
        }

        let output = Command::new("cliphist").arg("list").output().ok();
        if let Some(o) = output.filter(|o| o.status.success()) {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let mut results: Vec<PluginResult> = stdout
                .lines()
                .take(10)
                .filter_map(|line| {
                    let (id, preview) = line.split_once('\t')?;
                    let preview = preview.trim();
                    if preview.is_empty() {
                        return None;
                    }
                    let quoted_id = shell_single_quote(id.trim());
                    let score = matcher.fuzzy_match(preview, &search).unwrap_or(1);
                    Some(PluginResult {
                        plugin_id: self.id().to_string(),
                        title: preview.chars().take(60).collect(),
                        subtitle: "clipboard".to_string(),
                        score,
                        action: PluginAction::Shell {
                            command: format!(
                                "cliphist decode {quoted_id} | wl-copy 2>/dev/null || cliphist decode {quoted_id} | xclip -selection clipboard"
                            ),
                        },
                    })
                })
                .collect();
            results.sort_by_key(|r| std::cmp::Reverse(r.score));
            results.truncate(6);
            return results;
        }

        let current = Command::new("wl-paste")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .or_else(|| {
                Command::new("xclip")
                    .args(["-o", "-selection", "clipboard"])
                    .output()
                    .ok()
                    .filter(|o| o.status.success())
            });
        if let Some(o) = current {
            let text = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if !text.is_empty() {
                return vec![PluginResult {
                    plugin_id: self.id().to_string(),
                    title: text.chars().take(60).collect(),
                    subtitle: "clipboard (current)".to_string(),
                    score: 100,
                    action: PluginAction::None,
                }];
            }
        }

        vec![PluginResult {
            plugin_id: self.id().to_string(),
            title: "Clipboard unavailable".to_string(),
            subtitle: "install cliphist".to_string(),
            score: 1,
            action: PluginAction::None,
        }]
    }
}

struct ShellPlugin;

impl Plugin for ShellPlugin {
    fn id(&self) -> &str {
        "shell"
    }

    fn query(&self, query: &str, _matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        let trimmed = query.trim();
        if trimmed.is_empty() || (!trimmed.starts_with('>') && !trimmed.starts_with('!')) {
            return Vec::new();
        }

        if let Some(cmd) = trimmed.strip_prefix(">'").map(str::trim) {
            if !cmd.is_empty() {
                return vec![PluginResult {
                    plugin_id: self.id().to_string(),
                    title: format!("Capture {cmd}"),
                    subtitle: "press Enter to run shell command".to_string(),
                    score: i64::MAX,
                    action: PluginAction::Shell {
                        command: cmd.to_string(),
                    },
                }];
            }
        }

        let command = trimmed
            .strip_prefix('>')
            .or_else(|| trimmed.strip_prefix('!'))
            .map(str::trim)
            .unwrap_or("");
        if command.is_empty() {
            return Vec::new();
        }
        vec![PluginResult {
            plugin_id: self.id().to_string(),
            title: format!("Run {command}"),
            subtitle: "shell".to_string(),
            score: i64::MAX,
            action: PluginAction::Shell {
                command: command.to_string(),
            },
        }]
    }
}

pub fn evaluate_calculator(query: &str) -> Option<String> {
    if !is_math(query) {
        return None;
    }
    calc(query).map(format_calc_number)
}

struct CalculatorPlugin;

impl Plugin for CalculatorPlugin {
    fn id(&self) -> &str {
        "calculator"
    }

    fn query(&self, query: &str, _matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        evaluate_calculator(query).map_or_else(Vec::new, |formatted| {
            vec![PluginResult {
                plugin_id: self.id().to_string(),
                title: format!("= {formatted}"),
                subtitle: "calculator (Enter to copy)".to_string(),
                score: i64::MAX,
                action: PluginAction::Shell {
                    command: clipboard_copy_command(&formatted),
                },
            }]
        })
    }
}

struct WindowModePlugin;

impl Plugin for WindowModePlugin {
    fn id(&self) -> &str {
        "window-mode"
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        let mode_titles: &[(&str, &str, WindowMode)] = &[
            ("Niri strip windows", "window mode", WindowMode::NiriStrip),
            ("Tile windows", "window mode", WindowMode::Tiling),
            ("Float windows", "window mode", WindowMode::Floating),
            ("Monocle windows", "window mode", WindowMode::Monocle),
            ("Stack windows", "window mode", WindowMode::Stack),
            ("Center windows", "window mode", WindowMode::Center),
            ("Grid windows", "window mode", WindowMode::Grid),
        ];
        let mut results = mode_titles
            .iter()
            .filter_map(|(title, subtitle, mode)| {
                score(title, query, matcher).map(|score| PluginResult {
                    plugin_id: self.id().to_string(),
                    title: title.to_string(),
                    subtitle: subtitle.to_string(),
                    score,
                    action: PluginAction::SetWindowMode { mode: *mode },
                })
            })
            .collect::<Vec<_>>();
        if let Some(score) = score("Cycle window mode", query, matcher) {
            results.push(PluginResult {
                plugin_id: self.id().to_string(),
                title: "Cycle window mode".to_string(),
                subtitle: "window mode".to_string(),
                score,
                action: PluginAction::CycleWindowMode,
            });
        }
        results
    }
}

struct DesktopActionsPlugin;

impl Plugin for DesktopActionsPlugin {
    fn id(&self) -> &str {
        "desktop-actions"
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        DesktopAction::all()
            .iter()
            .filter_map(|action| {
                score(action.title(), query, matcher).map(|score| PluginResult {
                    plugin_id: self.id().to_string(),
                    title: action.title().to_string(),
                    subtitle: action.subtitle().to_string(),
                    score,
                    action: PluginAction::Desktop {
                        action: action.clone(),
                    },
                })
            })
            .collect()
    }
}

struct LayoutPlugin;

impl Plugin for LayoutPlugin {
    fn id(&self) -> &str {
        "layout"
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        [
            ("Reset layout", "layout", LayoutAction::Reset),
            ("Flip layout axis", "layout", LayoutAction::FlipAxis),
            ("Nudge window left", "layout", LayoutAction::NudgeLeft),
            ("Nudge window right", "layout", LayoutAction::NudgeRight),
            ("Nudge window up", "layout", LayoutAction::NudgeUp),
            ("Nudge window down", "layout", LayoutAction::NudgeDown),
            ("Expand window", "layout", LayoutAction::ExpandWindow),
            ("Contract window", "layout", LayoutAction::ContractWindow),
            ("Split row", "layout", LayoutAction::SplitRow),
            ("Split column", "layout", LayoutAction::SplitColumn),
            ("Grow focused pane", "layout", LayoutAction::GrowFocused),
            ("Shrink focused pane", "layout", LayoutAction::ShrinkFocused),
            ("Focus next window", "layout", LayoutAction::FocusNext),
            (
                "Focus previous window",
                "layout",
                LayoutAction::FocusPrevious,
            ),
            ("Focus first window", "layout", LayoutAction::FocusFirst),
            ("Focus last window", "layout", LayoutAction::FocusLast),
            ("Close focused window", "layout", LayoutAction::CloseFocused),
            ("Toggle floating", "layout", LayoutAction::ToggleFloat),
            ("Move window left", "layout", LayoutAction::MoveLeft),
            ("Move window right", "layout", LayoutAction::MoveRight),
            ("Move window up", "layout", LayoutAction::MoveUp),
            ("Move window down", "layout", LayoutAction::MoveDown),
            ("Balance panes", "layout", LayoutAction::BalancePanes),
            (
                "Center focused window",
                "layout",
                LayoutAction::CenterFocused,
            ),
            (
                "Window: Left Half",
                "window management (halves)",
                LayoutAction::SnapLeftHalf,
            ),
            (
                "Window: Right Half",
                "window management (halves)",
                LayoutAction::SnapRightHalf,
            ),
            (
                "Window: Top Half",
                "window management (halves)",
                LayoutAction::SnapTopHalf,
            ),
            (
                "Window: Bottom Half",
                "window management (halves)",
                LayoutAction::SnapBottomHalf,
            ),
            (
                "Window: Top Left Quarter",
                "window management (quarters)",
                LayoutAction::SnapTopLeft,
            ),
            (
                "Window: Top Right Quarter",
                "window management (quarters)",
                LayoutAction::SnapTopRight,
            ),
            (
                "Window: Bottom Left Quarter",
                "window management (quarters)",
                LayoutAction::SnapBottomLeft,
            ),
            (
                "Window: Bottom Right Quarter",
                "window management (quarters)",
                LayoutAction::SnapBottomRight,
            ),
            (
                "Window: First Third",
                "window management (thirds)",
                LayoutAction::SnapFirstThird,
            ),
            (
                "Window: Center Third",
                "window management (thirds)",
                LayoutAction::SnapCenterThird,
            ),
            (
                "Window: Last Third",
                "window management (thirds)",
                LayoutAction::SnapLastThird,
            ),
            (
                "Window: First Two Thirds",
                "window management (two-thirds)",
                LayoutAction::SnapFirstTwoThirds,
            ),
            (
                "Window: Last Two Thirds",
                "window management (two-thirds)",
                LayoutAction::SnapLastTwoThirds,
            ),
            (
                "Window: Maximize",
                "window management (sizing)",
                LayoutAction::Maximize,
            ),
            (
                "Window: Almost Maximize",
                "window management (sizing)",
                LayoutAction::AlmostMaximize,
            ),
            (
                "Window: Restore",
                "window management (sizing)",
                LayoutAction::Restore,
            ),
            (
                "Window: Reasonable Size",
                "window management (sizing)",
                LayoutAction::ReasonableSize,
            ),
            (
                "Window: Toggle Fullscreen",
                "window management (sizing)",
                LayoutAction::ToggleFullscreen,
            ),
            (
                "Window: Next Display",
                "window management (display)",
                LayoutAction::NextDisplay,
            ),
            (
                "Window: Previous Display",
                "window management (display)",
                LayoutAction::PreviousDisplay,
            ),
            (
                "Niri: Focus Column Left",
                "niri scrollable strip",
                LayoutAction::FocusColumnLeft,
            ),
            (
                "Niri: Focus Column Right",
                "niri scrollable strip",
                LayoutAction::FocusColumnRight,
            ),
            (
                "Niri: Move Column Left",
                "niri scrollable strip",
                LayoutAction::MoveColumnLeft,
            ),
            (
                "Niri: Move Column Right",
                "niri scrollable strip",
                LayoutAction::MoveColumnRight,
            ),
            (
                "Niri: Cycle Column Width",
                "niri scrollable strip (1/3, 1/2, 2/3, Full)",
                LayoutAction::CycleColumnWidth,
            ),
            (
                "Niri: Maximize Column",
                "niri scrollable strip",
                LayoutAction::MaximizeColumn,
            ),
            (
                "Niri: Consume Window into Column",
                "niri scrollable strip (stack vertically)",
                LayoutAction::ConsumeWindowIntoColumn,
            ),
            (
                "Niri: Expel Window from Column",
                "niri scrollable strip (new column)",
                LayoutAction::ExpelWindowFromColumn,
            ),
            (
                "Niri: Scroll Strip Left",
                "niri scrollable strip",
                LayoutAction::ScrollViewportLeft,
            ),
            (
                "Niri: Scroll Strip Right",
                "niri scrollable strip",
                LayoutAction::ScrollViewportRight,
            ),
            (
                "Room: Switch / Cycle Layout",
                "window room (auto / focus / stack / columns / grid)",
                LayoutAction::CycleRoomLayout,
            ),
            (
                "Room: Focus Layout",
                "window room (60% main + side stack)",
                LayoutAction::EnterRoomFocus,
            ),
            (
                "Room: Stack Layout",
                "window room (main + cascading peek)",
                LayoutAction::EnterRoomStack,
            ),
            (
                "Room: Columns Layout",
                "window room (equal columns)",
                LayoutAction::EnterRoomColumns,
            ),
            (
                "Room: Grid Layout",
                "window room (balanced grid)",
                LayoutAction::EnterRoomGrid,
            ),
        ]
        .into_iter()
        .filter_map(|(title, subtitle, action)| {
            score(title, query, matcher).map(|score| PluginResult {
                plugin_id: self.id().to_string(),
                title: title.to_string(),
                subtitle: subtitle.to_string(),
                score,
                action: PluginAction::Layout { action },
            })
        })
        .collect()
    }
}

struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn id(&self) -> &str {
        "settings"
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        [
            (
                "Toggle settings",
                "desktop settings",
                PluginAction::ToggleSettings,
            ),
            ("Settings", "desktop settings", PluginAction::ToggleSettings),
            (
                "Open settings",
                "desktop settings",
                PluginAction::OpenSettings,
            ),
            (
                "Close settings",
                "desktop settings",
                PluginAction::CloseSettings,
            ),
            (
                "Preferences",
                "desktop settings",
                PluginAction::OpenSettings,
            ),
        ]
        .into_iter()
        .filter_map(|(title, subtitle, action)| {
            score(title, query, matcher).map(|score| PluginResult {
                plugin_id: self.id().to_string(),
                title: title.to_string(),
                subtitle: subtitle.to_string(),
                score,
                action: action.clone(),
            })
        })
        .collect()
    }
}

struct FactoryResetPlugin;

impl Plugin for FactoryResetPlugin {
    fn id(&self) -> &str {
        "factory-reset"
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        [
            (
                "Factory reset",
                "reset desktop settings to shipped defaults",
                PluginAction::FactoryReset,
            ),
            (
                "Reset settings",
                "restore default desktop configuration",
                PluginAction::FactoryReset,
            ),
        ]
        .into_iter()
        .filter_map(|(title, subtitle, action)| {
            score(title, query, matcher).map(|score| PluginResult {
                plugin_id: self.id().to_string(),
                title: title.to_string(),
                subtitle: subtitle.to_string(),
                score,
                action: action.clone(),
            })
        })
        .collect()
    }
}

struct TerminalPlugin;

impl Plugin for TerminalPlugin {
    fn id(&self) -> &str {
        "terminal"
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        [
            (
                "Terminal",
                "open shell console",
                PluginAction::ToggleTerminal,
            ),
            (
                "Console",
                "open shell console",
                PluginAction::ToggleTerminal,
            ),
            (
                "Toggle terminal",
                "open or close shell",
                PluginAction::ToggleTerminal,
            ),
            ("Shell", "open shell console", PluginAction::ToggleTerminal),
        ]
        .into_iter()
        .filter_map(|(title, subtitle, action)| {
            score(title, query, matcher).map(|score| PluginResult {
                plugin_id: self.id().to_string(),
                title: title.to_string(),
                subtitle: subtitle.to_string(),
                score,
                action: action.clone(),
            })
        })
        .collect()
    }
}

struct TerminalClearPlugin;

impl Plugin for TerminalClearPlugin {
    fn id(&self) -> &str {
        "terminal-clear"
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        [
            ("Clear terminal", "console", PluginAction::TerminalClear),
            ("Clear console", "console", PluginAction::TerminalClear),
            ("Reset terminal", "console", PluginAction::TerminalClear),
        ]
        .into_iter()
        .filter_map(|(title, subtitle, action)| {
            score(title, query, matcher).map(|score| PluginResult {
                plugin_id: self.id().to_string(),
                title: title.to_string(),
                subtitle: subtitle.to_string(),
                score,
                action: action.clone(),
            })
        })
        .collect()
    }
}

struct InterfacePlugin;

impl Plugin for InterfacePlugin {
    fn id(&self) -> &str {
        "interface"
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        [
            (
                "Toggle status bar",
                "interface",
                PluginAction::ToggleStatusBar,
            ),
            ("Show status bar", "interface", PluginAction::ShowStatusBar),
            ("Hide status bar", "interface", PluginAction::HideStatusBar),
        ]
        .into_iter()
        .filter_map(|(title, subtitle, action)| {
            score(title, query, matcher).map(|score| PluginResult {
                plugin_id: self.id().to_string(),
                title: title.to_string(),
                subtitle: subtitle.to_string(),
                score,
                action: action.clone(),
            })
        })
        .collect()
    }
}

struct AppLauncherPlugin;

impl Plugin for AppLauncherPlugin {
    fn id(&self) -> &str {
        "apps"
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        if query.trim().is_empty() || query.trim().starts_with('>') {
            return Vec::new();
        }
        apps()
            .iter()
            .filter_map(|app| {
                matcher
                    .fuzzy_match(app, query)
                    .filter(|score| *score > 0)
                    .map(|score| PluginResult {
                        plugin_id: self.id().to_string(),
                        title: app.clone(),
                        subtitle: "app".to_string(),
                        score,
                        action: PluginAction::Launch {
                            program: app.clone(),
                        },
                    })
            })
            .collect()
    }
}

#[cfg(feature = "full")]
struct SpotifyPlugin {
    playerctl_override: Option<bool>,
}

#[cfg(feature = "full")]
impl SpotifyPlugin {
    const fn new() -> Self {
        Self {
            playerctl_override: None,
        }
    }

    #[cfg(test)]
    const fn with_playerctl(available: bool) -> Self {
        Self {
            playerctl_override: Some(available),
        }
    }

    fn has_playerctl(&self) -> bool {
        self.playerctl_override
            .unwrap_or_else(|| program_available("playerctl"))
    }
}

#[cfg(feature = "full")]
impl Plugin for SpotifyPlugin {
    fn id(&self) -> &str {
        "spotify"
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        if matcher.fuzzy_match("Spotify", query).is_some() && !self.has_playerctl() {
            return vec![PluginResult {
                plugin_id: self.id().to_string(),
                title: "Spotify unavailable".to_string(),
                subtitle: "playerctl not found".to_string(),
                score: 1,
                action: PluginAction::None,
            }];
        }
        let actions = [
            (
                "Spotify Play/Pause",
                "playerctl play-pause",
                "playerctl play-pause",
            ),
            ("Spotify Next", "playerctl next", "playerctl next"),
            (
                "Spotify Previous",
                "playerctl previous",
                "playerctl previous",
            ),
            (
                "Spotify Current Track",
                "playerctl metadata",
                "playerctl metadata --format '{{artist}} - {{title}}'",
            ),
        ];
        actions
            .into_iter()
            .filter_map(|(title, subtitle, command)| {
                score(title, query, matcher).map(|score| PluginResult {
                    plugin_id: self.id().to_string(),
                    title: title.to_string(),
                    subtitle: subtitle.to_string(),
                    score,
                    action: PluginAction::Shell {
                        command: command.to_string(),
                    },
                })
            })
            .collect()
    }
}

struct ProcessKillPlugin;

impl Plugin for ProcessKillPlugin {
    fn id(&self) -> &str {
        "kill"
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        let search = query.trim();
        let needle = if let Some(rest) = search.strip_prefix("kill ") {
            rest.trim()
        } else if search.starts_with("kill") {
            ""
        } else {
            return Vec::new();
        };
        if needle.is_empty() {
            return vec![PluginResult {
                plugin_id: self.id().to_string(),
                title: "Kill process".to_string(),
                subtitle: "type kill <name> to find processes".to_string(),
                score: i64::MAX,
                action: PluginAction::None,
            }];
        }
        let output = Command::new("ps").args(["-eo", "pid,comm"]).output().ok();
        let Some(output) = output.filter(|o| o.status.success() || !o.stdout.is_empty()) else {
            return Vec::new();
        };
        let stdout = String::from_utf8_lossy(&output.stdout);
        let needle_lower = needle.to_lowercase();
        let mut results: Vec<PluginResult> = stdout
            .lines()
            .skip(1)
            .filter_map(|line| {
                let mut parts = line.split_whitespace();
                let pid_str = parts.next()?;
                let pid: u32 = pid_str.parse().ok()?;
                let name = parts.collect::<Vec<_>>().join(" ");
                if name.is_empty() {
                    return None;
                }
                let score = matcher
                    .fuzzy_match(&name, needle)
                    .or_else(|| name.to_lowercase().contains(&needle_lower).then_some(10))?;
                Some(PluginResult {
                    plugin_id: self.id().to_string(),
                    title: format!("Kill {name} ({pid})"),
                    subtitle: "process".to_string(),
                    score,
                    action: PluginAction::Shell {
                        command: format!("kill {pid}"),
                    },
                })
            })
            .collect();
        results.sort_by_key(|r| Reverse(r.score));
        results.truncate(6);
        results
    }
}

struct VolumePlugin;

impl Plugin for VolumePlugin {
    fn id(&self) -> &str {
        "volume"
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        let search = query.trim().to_lowercase();
        let candidates: &[(&str, &str, &str)] = &[
            (
                "Volume up",
                "audio",
                "wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%+",
            ),
            (
                "Volume down",
                "audio",
                "wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-",
            ),
            (
                "Mute audio",
                "audio",
                "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle",
            ),
            (
                "Volume status",
                "audio",
                "wpctl get-volume @DEFAULT_AUDIO_SINK@",
            ),
        ];
        let mut results: Vec<PluginResult> = candidates
            .iter()
            .filter_map(|(title, subtitle, command)| {
                score(title, &search, matcher).map(|score| PluginResult {
                    plugin_id: self.id().to_string(),
                    title: title.to_string(),
                    subtitle: subtitle.to_string(),
                    score,
                    action: PluginAction::Shell {
                        command: command.to_string(),
                    },
                })
            })
            .collect();
        if let Some(rest) = search
            .strip_prefix("vol ")
            .or_else(|| search.strip_prefix("volume "))
        {
            if let Ok(pct) = rest.trim().trim_end_matches('%').parse::<u32>() {
                results.push(PluginResult {
                    plugin_id: self.id().to_string(),
                    title: format!("Set volume to {pct}%"),
                    subtitle: "audio".to_string(),
                    score: i64::MAX,
                    action: PluginAction::Shell {
                        command: format!("wpctl set-volume @DEFAULT_AUDIO_SINK@ {pct}%"),
                    },
                });
            }
        }
        results
    }
}

struct BrightnessPlugin;

impl Plugin for BrightnessPlugin {
    fn id(&self) -> &str {
        "brightness"
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        let search = query.trim().to_lowercase();
        let candidates: &[(&str, &str, &str)] = &[
            ("Brightness up", "display", "brightnessctl set +10%"),
            ("Brightness down", "display", "brightnessctl set 10%-"),
            ("Brightness max", "display", "brightnessctl set 100%"),
            ("Brightness status", "display", "brightnessctl info"),
        ];
        let mut results: Vec<PluginResult> = candidates
            .iter()
            .filter_map(|(title, subtitle, command)| {
                score(title, &search, matcher).map(|score| PluginResult {
                    plugin_id: self.id().to_string(),
                    title: title.to_string(),
                    subtitle: subtitle.to_string(),
                    score,
                    action: PluginAction::Shell {
                        command: command.to_string(),
                    },
                })
            })
            .collect();
        if let Some(rest) = search
            .strip_prefix("bright ")
            .or_else(|| search.strip_prefix("brightness "))
        {
            if let Ok(pct) = rest.trim().trim_end_matches('%').parse::<u32>() {
                results.push(PluginResult {
                    plugin_id: self.id().to_string(),
                    title: format!("Set brightness to {pct}%"),
                    subtitle: "display".to_string(),
                    score: i64::MAX,
                    action: PluginAction::Shell {
                        command: format!("brightnessctl set {pct}%"),
                    },
                });
            }
        }
        results
    }
}

struct TimerPlugin;

impl Plugin for TimerPlugin {
    fn id(&self) -> &str {
        "timer"
    }

    fn query(&self, query: &str, _matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        let search = query.trim();
        let Some(rest) = search
            .strip_prefix("timer ")
            .or_else(|| search.strip_prefix("in "))
        else {
            return Vec::new();
        };
        let rest = rest.trim();
        let Some(seconds) = parse_duration(rest) else {
            return Vec::new();
        };
        let msg = shell_single_quote(&format!("timer: {rest} done"));
        let command = format!("(sleep {seconds} && notify-send -u critical alpenglowed {msg}) &");
        vec![PluginResult {
            plugin_id: self.id().to_string(),
            title: format!("Timer: {rest}"),
            subtitle: "notifies when elapsed".to_string(),
            score: i64::MAX,
            action: PluginAction::Shell { command },
        }]
    }
}

fn parse_duration(text: &str) -> Option<u64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let mut total = 0u64;
    let mut number = String::new();
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            number.push(ch);
        } else {
            let n: u64 = number.parse().ok()?;
            number.clear();
            total += match ch {
                'h' => n * 3600,
                'm' => n * 60,
                's' => n,
                _ => return None,
            };
        }
    }
    if !number.is_empty() {
        total += number.parse::<u64>().ok()?;
    }
    (total > 0).then_some(total)
}

struct SystemInfoPlugin;

impl Plugin for SystemInfoPlugin {
    fn id(&self) -> &str {
        "system-info"
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        let candidates: &[(&str, &str, &str)] = &[
            ("System info", "system", "uname -a"),
            ("Uptime", "system", "uptime"),
            ("Hostname", "system", "hostname"),
            ("Disk usage", "system", "df -h"),
            ("Memory info", "system", "free -h"),
            ("Kernel version", "system", "uname -r"),
            ("CPU info", "system", "lscpu | head -20"),
        ];
        candidates
            .iter()
            .filter_map(|(title, subtitle, command)| {
                score(title, query, matcher).map(|score| PluginResult {
                    plugin_id: self.id().to_string(),
                    title: title.to_string(),
                    subtitle: subtitle.to_string(),
                    score,
                    action: PluginAction::Shell {
                        command: command.to_string(),
                    },
                })
            })
            .collect()
    }
}

struct NetworkInfoPlugin;

impl Plugin for NetworkInfoPlugin {
    fn id(&self) -> &str {
        "network-info"
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        let candidates: &[(&str, &str, &str)] = &[
            ("IP address", "network", "ip addr show"),
            ("Routes", "network", "ip route"),
            (
                "DNS servers",
                "network",
                "resolvectl status 2>/dev/null || cat /etc/resolv.conf",
            ),
            ("Listening ports", "network", "ss -tlnp"),
            ("Active connections", "network", "ss -tnp"),
            ("Ping gateway", "network", "ip route | grep default"),
        ];
        candidates
            .iter()
            .filter_map(|(title, subtitle, command)| {
                score(title, query, matcher).map(|score| PluginResult {
                    plugin_id: self.id().to_string(),
                    title: title.to_string(),
                    subtitle: subtitle.to_string(),
                    score,
                    action: PluginAction::Shell {
                        command: command.to_string(),
                    },
                })
            })
            .collect()
    }
}

#[cfg(feature = "full")]
struct WeatherPlugin;

#[cfg(feature = "full")]
impl Plugin for WeatherPlugin {
    fn id(&self) -> &str {
        "weather"
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        let candidates: &[(&str, &str, &str)] = &[
            (
                "Weather now",
                "weather",
                "curl -s 'wttr.in?format=%t+%C+%w'",
            ),
            ("Weather full", "weather", "curl -s 'wttr.in'"),
            ("Weather short", "weather", "curl -s 'wttr.in?format=3'"),
            ("Weather JSON", "weather", "curl -s 'wttr.in?format=j1'"),
        ];
        candidates
            .iter()
            .filter_map(|(title, subtitle, command)| {
                score(title, query, matcher).map(|score| PluginResult {
                    plugin_id: self.id().to_string(),
                    title: title.to_string(),
                    subtitle: subtitle.to_string(),
                    score,
                    action: PluginAction::Shell {
                        command: command.to_string(),
                    },
                })
            })
            .collect()
    }
}

struct HelpPlugin {
    extras: bool,
}

impl HelpPlugin {
    fn for_role(role: crate::role::SessionRole) -> Self {
        Self {
            extras: role.extras(),
        }
    }
}

impl Plugin for HelpPlugin {
    fn id(&self) -> &str {
        "help"
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        let extras = self.extras;
        [
            (
                "Help: launcher",
                "type to search apps, actions, and plugins",
            ),
            ("Help: shell", "prefix with > to run a shell command"),
            ("Help: capture", "prefix with >' to capture command output"),
            ("Help: files", "prefix with / to search files"),
            ("Help: web", "prefix with ? to search the web"),
            ("Help: emoji", "prefix with : to find emoji"),
            (
                "Help: clipboard",
                "type clip, paste, or cb to browse clipboard",
            ),
            ("Help: calculator", "type a math expression like 2+2"),
            ("Help: kill", "type kill <name> to find and kill processes"),
            ("Help: volume", "type volume up/down/mute or volume 50"),
            (
                "Help: brightness",
                "type brightness up/down or brightness 50",
            ),
            ("Help: timer", "type timer 5m or in 30s for a notification"),
            ("Help: weather", "type weather for a forecast"),
            ("Help: system", "type system for uptime, kernel, disk info"),
            ("Help: network", "type network for ip, routes, ports"),
            ("Help: color", "type #hex or color hex for color info"),
            ("Help: convert", "type 10 km to mi for unit conversion"),
            ("Help: translate", "type translate <text> to <lang>"),
            ("Help: recent", "type recent for recently modified files"),
            (
                "Help: quicklinks",
                "type quicklink or ql to launch saved links",
            ),
            (
                "Help: snippets",
                "type snippet, snip, or ; to copy text snippets",
            ),
            ("Help: dictionary", "type define <word> or dict <word>"),
            ("Help: notes", "type note or note: <text> to manage notes"),
            (
                "Help: window modes",
                "tile, float, monocle, stack, center, grid",
            ),
            (
                "Help: shortcuts",
                "Cmd-Space launcher, Cmd-, settings, Cmd-B status bar",
            ),
        ]
        .into_iter()
        .filter(|(title, _)| {
            extras || !matches!(*title, "Help: web" | "Help: weather" | "Help: translate")
        })
        .filter_map(|(title, detail)| {
            score(title, query, matcher).map(|score| PluginResult {
                plugin_id: self.id().to_string(),
                title: title.to_string(),
                subtitle: detail.to_string(),
                score,
                action: PluginAction::None,
            })
        })
        .collect()
    }
}

struct ColorPlugin;

impl Plugin for ColorPlugin {
    fn id(&self) -> &str {
        "color"
    }

    fn query(&self, query: &str, _matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        let search = query.trim();
        let hex = if let Some(rest) = search.strip_prefix('#') {
            rest
        } else if let Some(rest) = search.strip_prefix("color ") {
            rest.trim().trim_start_matches('#')
        } else {
            return Vec::new();
        };
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) || hex.is_empty() {
            return Vec::new();
        }
        let parsed = match hex.len() {
            3 => (
                u8::from_str_radix(&hex[0..1].repeat(2), 16).ok(),
                u8::from_str_radix(&hex[1..2].repeat(2), 16).ok(),
                u8::from_str_radix(&hex[2..3].repeat(2), 16).ok(),
            ),
            6 => (
                u8::from_str_radix(&hex[0..2], 16).ok(),
                u8::from_str_radix(&hex[2..4], 16).ok(),
                u8::from_str_radix(&hex[4..6], 16).ok(),
            ),
            _ => return Vec::new(),
        };
        let (Some(r), Some(g), Some(b)) = parsed else {
            return Vec::new();
        };
        vec![PluginResult {
            plugin_id: self.id().to_string(),
            title: format!("#{hex} = rgb({r}, {g}, {b})"),
            subtitle: "color".to_string(),
            score: i64::MAX,
            action: PluginAction::Shell {
                command: clipboard_copy_command(&format!("rgb({r}, {g}, {b})")),
            },
        }]
    }
}

struct UnitConverterPlugin;

impl Plugin for UnitConverterPlugin {
    fn id(&self) -> &str {
        "convert"
    }

    fn query(&self, query: &str, _matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        let search = query.trim();
        // pattern: <number> <unit> to <unit>
        let parts: Vec<&str> = search.split_whitespace().collect();
        if parts.len() != 4 || parts[2] != "to" {
            return Vec::new();
        }
        let Ok(value) = parts[0].parse::<f64>() else {
            return Vec::new();
        };
        let from = parts[1].to_lowercase();
        let to = parts[3].to_lowercase();
        let Some(result) = convert_units(value, &from, &to) else {
            return Vec::new();
        };
        vec![PluginResult {
            plugin_id: self.id().to_string(),
            title: format!("{value} {from} = {result:.4} {to}"),
            subtitle: "unit converter".to_string(),
            score: i64::MAX,
            action: PluginAction::Shell {
                command: clipboard_copy_command(&format!("{result:.4}")),
            },
        }]
    }
}

fn convert_units(value: f64, from: &str, to: &str) -> Option<f64> {
    let to_meters = |unit: &str| -> Option<f64> {
        Some(match unit {
            "m" => 1.0,
            "km" => 1000.0,
            "cm" => 0.01,
            "mm" => 0.001,
            "mi" => 1609.344,
            "ft" | "feet" => 0.3048,
            "in" | "inch" => 0.0254,
            "yd" | "yard" => 0.9144,
            _ => return None,
        })
    };
    let to_grams = |unit: &str| -> Option<f64> {
        Some(match unit {
            "g" => 1.0,
            "kg" => 1000.0,
            "mg" => 0.001,
            "lb" | "lbs" => 453.592,
            "oz" => 28.3495,
            _ => return None,
        })
    };
    let to_bytes = |unit: &str| -> Option<f64> {
        Some(match unit {
            "b" | "bytes" => 1.0,
            "kb" | "kib" => 1024.0,
            "mb" | "mib" => 1024.0 * 1024.0,
            "gb" | "gib" => 1024.0 * 1024.0 * 1024.0,
            "tb" | "tib" => 1024.0_f64.powi(4),
            _ => return None,
        })
    };
    let to_seconds = |unit: &str| -> Option<f64> {
        Some(match unit {
            "s" | "sec" => 1.0,
            "min" | "minute" | "minutes" => 60.0,
            "h" | "hr" | "hour" | "hours" => 3600.0,
            "day" | "days" => 86400.0,
            _ => return None,
        })
    };
    let to_celsius = |unit: &str, v: f64| -> Option<f64> {
        Some(match unit {
            "c" | "celsius" => v,
            "f" | "fahrenheit" => (v - 32.0) * 5.0 / 9.0,
            "k" | "kelvin" => v - 273.15,
            _ => return None,
        })
    };
    let from_celsius = |unit: &str, v: f64| -> Option<f64> {
        Some(match unit {
            "c" | "celsius" => v,
            "f" | "fahrenheit" => v * 9.0 / 5.0 + 32.0,
            "k" | "kelvin" => v + 273.15,
            _ => return None,
        })
    };
    if let (Some(fm), Some(tm)) = (to_meters(from), to_meters(to)) {
        return Some(value * fm / tm);
    }
    if let (Some(fg), Some(tg)) = (to_grams(from), to_grams(to)) {
        return Some(value * fg / tg);
    }
    if let (Some(fb), Some(tb)) = (to_bytes(from), to_bytes(to)) {
        return Some(value * fb / tb);
    }
    if let (Some(fs), Some(ts)) = (to_seconds(from), to_seconds(to)) {
        return Some(value * fs / ts);
    }
    if let Some(celsius) = to_celsius(from, value) {
        if let Some(out) = from_celsius(to, celsius) {
            return Some(out);
        }
    }
    None
}

#[cfg(feature = "full")]
struct TranslatePlugin;

#[cfg(feature = "full")]
impl Plugin for TranslatePlugin {
    fn id(&self) -> &str {
        "translate"
    }

    fn query(&self, query: &str, _matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        let search = query.trim();
        let Some(rest) = search.strip_prefix("translate ") else {
            return Vec::new();
        };
        let Some((text, lang)) = rest.rsplit_once(" to ") else {
            return vec![PluginResult {
                plugin_id: self.id().to_string(),
                title: "Translate".to_string(),
                subtitle: "type translate <text> to <lang>".to_string(),
                score: i64::MAX,
                action: PluginAction::None,
            }];
        };
        let text = text.trim();
        let lang = lang.trim();
        if text.is_empty() || lang.is_empty() {
            return Vec::new();
        }
        let encoded_lang = urlencode(lang);
        let url = format!(
            "https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl={encoded_lang}&dt=t&q={}",
            urlencode(text)
        );
        let quoted_url = shell_single_quote(&url);
        vec![PluginResult {
            plugin_id: self.id().to_string(),
            title: format!("Translate \"{text}\" to {lang}"),
            subtitle: "google translate".to_string(),
            score: i64::MAX,
            action: PluginAction::Shell {
                command: format!(
                    "curl -s {quoted_url} | jq -r '.[0][0][0]' 2>/dev/null || curl -s {quoted_url}"
                ),
            },
        }]
    }
}

#[cfg(feature = "full")]
fn urlencode(text: &str) -> String {
    urlencode_str(text)
}

struct RecentFilesPlugin;

impl Plugin for RecentFilesPlugin {
    fn id(&self) -> &str {
        "recent"
    }

    fn query(&self, query: &str, _matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        if !query.trim().to_lowercase().starts_with("recent") {
            return Vec::new();
        }
        let files = cached_searchable_files();
        if files.is_empty() {
            return vec![PluginResult {
                plugin_id: self.id().to_string(),
                title: "No recent files".to_string(),
                subtitle: "nothing modified recently".to_string(),
                score: 1,
                action: PluginAction::None,
            }];
        }
        files
            .iter()
            .take(8)
            .map(|path| {
                let filename = std::path::Path::new(path)
                    .file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or(std::borrow::Cow::Borrowed(path.as_str()));
                PluginResult {
                    plugin_id: self.id().to_string(),
                    title: filename.to_string(),
                    subtitle: path.to_string(),
                    score: 100,
                    action: PluginAction::Shell {
                        command: format!("xdg-open {}", shell_single_quote(path)),
                    },
                }
            })
            .collect()
    }
}

struct QuicklinksPlugin;

const QUICKLINKS: &[(&str, &str)] = &[
    ("Quicklink: GitHub", "https://github.com"),
    (
        "Quicklink: Rust Standard Library",
        "https://doc.rust-lang.org/std/",
    ),
    (
        "Quicklink: Wayland Explorer",
        "https://wayland.app/protocols/",
    ),
    ("Quicklink: Smithay Docs", "https://smithay.github.io/"),
    ("Quicklink: Localhost 3000", "http://localhost:3000"),
    ("Quicklink: Localhost 8080", "http://localhost:8080"),
];

impl Plugin for QuicklinksPlugin {
    fn id(&self) -> &str {
        "quicklinks"
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        let trimmed = query.trim();
        let needle = trimmed
            .strip_prefix("ql ")
            .or_else(|| trimmed.strip_prefix("quicklink "))
            .map(str::trim)
            .unwrap_or(trimmed);
        if needle.is_empty() {
            return Vec::new();
        }
        QUICKLINKS
            .iter()
            .filter_map(|(title, url)| {
                score(title, needle, matcher)
                    .or_else(|| matcher.fuzzy_match(url, needle))
                    .map(|score| PluginResult {
                        plugin_id: self.id().to_string(),
                        title: (*title).to_string(),
                        subtitle: (*url).to_string(),
                        score,
                        action: PluginAction::Shell {
                            command: format!("xdg-open {}", shell_single_quote(url)),
                        },
                    })
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextMacro {
    pub name: &'static str,
    pub trigger: &'static str,
    pub bang: &'static str,
    pub template: &'static str,
}

pub const TEXT_MACROS: &[TextMacro] = &[
    TextMacro {
        name: "Shrug",
        trigger: ";shrug",
        bang: "!shrug",
        template: "¯\\_(ツ)_/¯",
    },
    TextMacro {
        name: "Table Flip",
        trigger: ";flip",
        bang: "!flip",
        template: "(╯°□°）╯︵ ┻━┻",
    },
    TextMacro {
        name: "Checkmark",
        trigger: ";check",
        bang: "!check",
        template: "✅",
    },
    TextMacro {
        name: "Current Date",
        trigger: ";date",
        bang: "!date",
        template: "{date}",
    },
    TextMacro {
        name: "Current Time",
        trigger: ";time",
        bang: "!time",
        template: "{time}",
    },
    TextMacro {
        name: "ISO Timestamp",
        trigger: ";iso",
        bang: "!iso",
        template: "{iso}",
    },
    TextMacro {
        name: "UUID v4",
        trigger: ";uuid",
        bang: "!uuid",
        template: "{uuid}",
    },
    TextMacro {
        name: "Email Signature",
        trigger: ";sig",
        bang: "!sig",
        template: "Best regards,\nAlpenglow",
    },
    TextMacro {
        name: "Co-Authored-By",
        trigger: ";coauthor",
        bang: "!coauthor",
        template: "Co-authored-by: Alpenglow <dev@alpenglow.local>",
    },
    TextMacro {
        name: "Localhost URL",
        trigger: ";local",
        bang: "!local",
        template: "http://127.0.0.1:8080",
    },
];

pub fn expand_macro_template(template: &str, argument: &str) -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(1_760_000_000);
    let days = secs / 86_400;
    let day_secs = secs % 86_400;
    let hours = day_secs / 3600;
    let mins = (day_secs % 3600) / 60;
    let rem_secs = day_secs % 60;
    // Civil date from days since 1970-01-01
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };

    let date_str = format!("{year:04}-{m:02}-{d:02}");
    let time_str = format!("{hours:02}:{mins:02}");
    let iso_str = format!("{date_str}T{hours:02}:{mins:02}:{rem_secs:02}Z");
    let pid = std::process::id();
    let uuid_str = format!(
        "{:08x}-{:04x}-4{:03x}-8{:03x}-{:012x}",
        (secs & 0xffff_ffff) as u32,
        ((secs >> 32) & 0xffff) as u16,
        (pid & 0x0fff) as u16,
        ((pid >> 12) & 0x0fff) as u16,
        ((secs.wrapping_mul(0x5851_f42d_4c95_7f2d)) & 0xffff_ffff_ffff)
    );

    template
        .replace("{date}", &date_str)
        .replace("{time}", &time_str)
        .replace("{datetime}", &iso_str)
        .replace("{iso}", &iso_str)
        .replace("{uuid}", &uuid_str)
        .replace("{argument}", argument)
}

pub fn resolve_text_macro(input: &str) -> Option<(String, String, String)> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    let (token, arg) = match trimmed.split_once(' ') {
        Some((head, tail)) => (head.trim(), tail.trim()),
        None => (trimmed, ""),
    };
    if !token.starts_with('!') && !token.starts_with(';') {
        return None;
    }
    let lower = token.to_lowercase();
    for m in TEXT_MACROS {
        if lower == m.bang
            || lower == m.trigger
            || m.bang.starts_with(&lower) && lower.len() >= 2
            || m.trigger.starts_with(&lower) && lower.len() >= 2
        {
            let expanded = expand_macro_template(m.template, arg);
            return Some((m.name.to_string(), m.bang.to_string(), expanded));
        }
    }
    None
}

struct SnippetsPlugin;

impl Plugin for SnippetsPlugin {
    fn id(&self) -> &str {
        "snippets"
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }
        let needle = trimmed
            .strip_prefix("snip ")
            .or_else(|| trimmed.strip_prefix("snippet "))
            .or_else(|| trimmed.strip_prefix("macro "))
            .map(str::trim)
            .unwrap_or(trimmed);
        TEXT_MACROS
            .iter()
            .filter_map(|m| {
                let matched = if trimmed.starts_with(';') || trimmed.starts_with('!') {
                    matcher
                        .fuzzy_match(m.trigger, trimmed)
                        .or_else(|| matcher.fuzzy_match(m.bang, trimmed))
                } else {
                    score(m.name, needle, matcher)
                        .or_else(|| matcher.fuzzy_match(m.trigger, needle))
                        .or_else(|| matcher.fuzzy_match(m.bang, needle))
                };
                matched.map(|score| {
                    let expanded = expand_macro_template(m.template, "");
                    PluginResult {
                        plugin_id: self.id().to_string(),
                        title: format!("Snippet: {}", m.name),
                        subtitle: format!("{} → {expanded}", m.bang),
                        score,
                        action: PluginAction::Shell {
                            command: clipboard_copy_command(&expanded),
                        },
                    }
                })
            })
            .collect()
    }
}

struct DictionaryPlugin;

const BUILTIN_DICTIONARY: &[(&str, &str)] = &[
    (
        "alpenglow",
        "n. A reddish glow seen near sunset or sunrise on the summits of mountains.",
    ),
    (
        "compositor",
        "n. A window manager that provides applications with an off-screen buffer for each window and composites them.",
    ),
    (
        "wayland",
        "n. A display server protocol specifying communication between a display server and its clients.",
    ),
    (
        "tiling",
        "adj. Arranging non-overlapping windows automatically to fill the screen workarea.",
    ),
    (
        "monocle",
        "n. A window layout mode where the focused window occupies the full workarea.",
    ),
    (
        "vulkan",
        "n. A low-overhead, cross-platform 3D graphics and compute API.",
    ),
    (
        "idempotent",
        "adj. Denoting an operation that can be applied multiple times without changing the result beyond the initial application.",
    ),
    (
        "ephemeral",
        "adj. Lasting for a very short time; transitory.",
    ),
];

impl Plugin for DictionaryPlugin {
    fn id(&self) -> &str {
        "dictionary"
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        let trimmed = query.trim();
        let Some(word) = trimmed
            .strip_prefix("define ")
            .or_else(|| trimmed.strip_prefix("def "))
            .or_else(|| trimmed.strip_prefix("dict "))
            .map(str::trim)
        else {
            return Vec::new();
        };
        if word.is_empty() {
            return vec![PluginResult {
                plugin_id: self.id().to_string(),
                title: "Dictionary".to_string(),
                subtitle: "type define <word> to look up a definition".to_string(),
                score: i64::MAX,
                action: PluginAction::None,
            }];
        }
        let mut results: Vec<PluginResult> = BUILTIN_DICTIONARY
            .iter()
            .filter_map(|(term, def)| {
                matcher.fuzzy_match(term, word).map(|s| PluginResult {
                    plugin_id: self.id().to_string(),
                    title: format!("Define {term}"),
                    subtitle: (*def).to_string(),
                    score: s + 500,
                    action: PluginAction::Shell {
                        command: clipboard_copy_command(def),
                    },
                })
            })
            .collect();
        if results.is_empty() {
            results.push(PluginResult {
                plugin_id: self.id().to_string(),
                title: format!("Define \"{word}\""),
                subtitle: "copy word or look up definition".to_string(),
                score: 400,
                action: PluginAction::Shell {
                    command: clipboard_copy_command(word),
                },
            });
        }
        results.sort_by_key(|r| Reverse(r.score));
        results.truncate(4);
        results
    }
}

struct NotesPlugin;

const DEFAULT_NOTES: &[(&str, &str)] = &[
    (
        "Note: Keyboard Shortcuts",
        "Super+Space toggles bar • Tab cycles categories • Enter runs action",
    ),
    (
        "Note: Window Modes",
        "Switch between Tiling, Floating, Monocle, Stack, Center, and Grid inside the bar",
    ),
    (
        "Note: Quick Math & Units",
        "Type math (e.g. (24 * 60) / 8) or conversions (e.g. 10 km to mi) directly",
    ),
];

impl Plugin for NotesPlugin {
    fn id(&self) -> &str {
        "notes"
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        let trimmed = query.trim();
        if let Some(note_body) = trimmed
            .strip_prefix("note: ")
            .or_else(|| trimmed.strip_prefix("note add "))
            .map(str::trim)
        {
            if !note_body.is_empty() {
                return vec![PluginResult {
                    plugin_id: self.id().to_string(),
                    title: format!("Save Note: {note_body}"),
                    subtitle: "copies note and appends to notes log".to_string(),
                    score: i64::MAX,
                    action: PluginAction::Shell {
                        command: clipboard_copy_command(note_body),
                    },
                }];
            }
        }
        if !trimmed.to_lowercase().starts_with("note") {
            return Vec::new();
        }
        DEFAULT_NOTES
            .iter()
            .filter_map(|(title, detail)| {
                score(title, trimmed, matcher)
                    .or(Some(300))
                    .map(|score| PluginResult {
                        plugin_id: self.id().to_string(),
                        title: (*title).to_string(),
                        subtitle: (*detail).to_string(),
                        score,
                        action: PluginAction::Shell {
                            command: clipboard_copy_command(detail),
                        },
                    })
            })
            .collect()
    }
}

#[cfg(feature = "full")]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CommandPluginManifest {
    pub id: String,
    pub name: String,
    pub kind: PluginKind,
    pub command: Vec<String>,
    #[serde(default, alias = "match")]
    pub matcher: MatchMode,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

#[cfg(feature = "full")]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginKind {
    Command,
    Crepus,
    Rust,
    Webcode,
}

#[cfg(feature = "full")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchMode {
    #[default]
    Always,
    Prefix,
    Fuzzy,
}

#[cfg(feature = "full")]
pub struct CommandPlugin {
    manifest: CommandPluginManifest,
    base_dir: PathBuf,
}

#[cfg(feature = "full")]
impl CommandPlugin {
    pub fn from_manifest_file(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
        let manifest: CommandPluginManifest =
            serde_json::from_str(&text).map_err(|error| error.to_string())?;
        if manifest.id.trim().is_empty() || manifest.command.is_empty() {
            return Err("plugin id and command are required".to_string());
        }
        Ok(Self {
            manifest,
            base_dir: path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf(),
        })
    }

    fn load_default() -> Vec<Self> {
        let mut dirs = vec![PathBuf::from("plugins")];
        if let Ok(dir) = std::env::var("ALPENGLOWED_PLUGIN_DIR") {
            dirs.push(PathBuf::from(dir));
        }
        dirs.into_iter()
            .filter_map(|dir| std::fs::read_dir(dir).ok())
            .flat_map(|entries| entries.flatten())
            .map(|entry| entry.path().join("plugin.json"))
            .filter(|path| path.is_file())
            .filter_map(|path| Self::from_manifest_file(&path).ok())
            .collect()
    }

    fn should_run(&self, query: &str, matcher: &SkimMatcherV2) -> bool {
        match self.manifest.matcher {
            MatchMode::Always => true,
            MatchMode::Prefix => query
                .trim()
                .strip_prefix(&self.manifest.id)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with(' ')),
            MatchMode::Fuzzy => matcher.fuzzy_match(&self.manifest.name, query).is_some(),
        }
    }
}

#[cfg(feature = "full")]
impl Plugin for CommandPlugin {
    fn id(&self) -> &str {
        &self.manifest.id
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        if !self.should_run(query, matcher) {
            return Vec::new();
        }
        run_command_plugin(&self.manifest, &self.base_dir, query).unwrap_or_else(|error| {
            vec![PluginResult {
                plugin_id: self.id().to_string(),
                title: format!("{} unavailable", self.manifest.name),
                subtitle: error,
                score: 0,
                action: PluginAction::None,
            }]
        })
    }
}

#[cfg(feature = "full")]
#[derive(Debug, Serialize)]
struct PluginRequest<'a> {
    r#type: &'a str,
    query: &'a str,
}

#[cfg(feature = "full")]
#[derive(Debug, Deserialize)]
struct PluginResponse {
    results: Vec<PluginResponseResult>,
}

#[cfg(feature = "full")]
#[derive(Debug, Deserialize)]
struct PluginResponseResult {
    title: String,
    subtitle: String,
    score: i64,
    action: PluginAction,
}

#[cfg(feature = "full")]
fn run_command_plugin(
    manifest: &CommandPluginManifest,
    base_dir: &Path,
    query: &str,
) -> Result<Vec<PluginResult>, String> {
    let (program, args) = manifest
        .command
        .split_first()
        .ok_or_else(|| "missing plugin command".to_string())?;
    let mut child = Command::new(program)
        .args(args)
        .current_dir(base_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    let request = serde_json::to_vec(&PluginRequest {
        r#type: "query",
        query,
    })
    .map_err(|error| error.to_string())?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| "plugin stdin unavailable".to_string())?
        .write_all(&request)
        .map_err(|error| error.to_string())?;
    drop(child.stdin.take());
    let started = Instant::now();
    loop {
        if child
            .try_wait()
            .map_err(|error| error.to_string())?
            .is_some()
        {
            let output = child
                .wait_with_output()
                .map_err(|error| error.to_string())?;
            if !output.status.success() {
                return Err(format!("plugin exited {}", output.status));
            }
            let response: PluginResponse =
                serde_json::from_slice(&output.stdout).map_err(|error| error.to_string())?;
            return Ok(response
                .results
                .into_iter()
                .map(|result| PluginResult {
                    plugin_id: manifest.id.clone(),
                    title: result.title,
                    subtitle: result.subtitle,
                    score: result.score,
                    action: result.action,
                })
                .collect());
        }
        if started.elapsed() > Duration::from_millis(manifest.timeout_ms) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("plugin timed out".to_string());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn score(title: &str, query: &str, matcher: &SkimMatcherV2) -> Option<i64> {
    if title.eq_ignore_ascii_case(query.trim()) {
        Some(i64::MAX - 1)
    } else {
        matcher.fuzzy_match(title, query)
    }
}

fn apps() -> Vec<String> {
    static APPS: OnceLock<Vec<String>> = OnceLock::new();
    APPS.get_or_init(|| {
        let mut apps = Vec::new();
        if let Ok(path) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path) {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            if !name.starts_with('.') {
                                apps.push(name.to_owned());
                            }
                        }
                    }
                }
            }
        }
        apps.sort();
        apps.dedup();
        apps
    })
    .clone()
}

#[cfg(feature = "full")]
fn program_available(program: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(program).is_file()))
}

fn is_math(value: &str) -> bool {
    let text = value.trim();
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_digit() || "+-*/%^() .".contains(c))
        && text.contains(|c: char| c.is_ascii_digit())
}

fn format_calc_number(value: f64) -> String {
    if (value - value.round()).abs() < 1e-9 && value.abs() < 1e15 {
        format!("{}", value.round() as i64)
    } else {
        let formatted = format!("{value:.6}");
        formatted
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn calc(expr: &str) -> Option<f64> {
    if let Some(val) = eval_math_expr(expr) {
        if val.is_finite() {
            return Some(val);
        }
    }
    let mut child = Command::new("bc")
        .arg("-ql")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .ok()?;
    child.stdin.as_mut()?.write_all(expr.as_bytes()).ok()?;
    let output = child.wait_with_output().ok()?;
    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

fn eval_math_expr(expr: &str) -> Option<f64> {
    struct Parser<'a> {
        bytes: &'a [u8],
        pos: usize,
    }
    impl<'a> Parser<'a> {
        fn skip_ws(&mut self) {
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
                self.pos += 1;
            }
        }
        fn parse_expr(&mut self) -> Option<f64> {
            let mut lhs = self.parse_term()?;
            loop {
                self.skip_ws();
                if self.pos >= self.bytes.len() {
                    break;
                }
                match self.bytes[self.pos] {
                    b'+' => {
                        self.pos += 1;
                        lhs += self.parse_term()?;
                    }
                    b'-' => {
                        self.pos += 1;
                        lhs -= self.parse_term()?;
                    }
                    _ => break,
                }
            }
            Some(lhs)
        }
        fn parse_term(&mut self) -> Option<f64> {
            let mut lhs = self.parse_power()?;
            loop {
                self.skip_ws();
                if self.pos >= self.bytes.len() {
                    break;
                }
                match self.bytes[self.pos] {
                    b'*' => {
                        self.pos += 1;
                        lhs *= self.parse_power()?;
                    }
                    b'/' => {
                        self.pos += 1;
                        let rhs = self.parse_power()?;
                        if rhs.abs() < 1e-15 {
                            return None;
                        }
                        lhs /= rhs;
                    }
                    b'%' => {
                        self.pos += 1;
                        let rhs = self.parse_power()?;
                        if rhs.abs() < 1e-15 {
                            return None;
                        }
                        lhs %= rhs;
                    }
                    _ => break,
                }
            }
            Some(lhs)
        }
        fn parse_power(&mut self) -> Option<f64> {
            let base = self.parse_unary()?;
            self.skip_ws();
            if self.pos < self.bytes.len() && self.bytes[self.pos] == b'^' {
                self.pos += 1;
                let exp = self.parse_power()?;
                Some(base.powf(exp))
            } else {
                Some(base)
            }
        }
        fn parse_unary(&mut self) -> Option<f64> {
            self.skip_ws();
            if self.pos >= self.bytes.len() {
                return None;
            }
            match self.bytes[self.pos] {
                b'+' => {
                    self.pos += 1;
                    self.parse_unary()
                }
                b'-' => {
                    self.pos += 1;
                    Some(-self.parse_unary()?)
                }
                _ => self.parse_primary(),
            }
        }
        fn parse_primary(&mut self) -> Option<f64> {
            self.skip_ws();
            if self.pos >= self.bytes.len() {
                return None;
            }
            if self.bytes[self.pos] == b'(' {
                self.pos += 1;
                let val = self.parse_expr()?;
                self.skip_ws();
                if self.pos >= self.bytes.len() || self.bytes[self.pos] != b')' {
                    return None;
                }
                self.pos += 1;
                return Some(val);
            }
            let start = self.pos;
            let mut has_dot = false;
            while self.pos < self.bytes.len() {
                let b = self.bytes[self.pos];
                if b.is_ascii_digit() {
                    self.pos += 1;
                } else if b == b'.' && !has_dot {
                    has_dot = true;
                    self.pos += 1;
                } else {
                    break;
                }
            }
            if self.pos == start {
                return None;
            }
            std::str::from_utf8(&self.bytes[start..self.pos])
                .ok()?
                .parse::<f64>()
                .ok()
        }
    }
    let mut parser = Parser {
        bytes: expr.as_bytes(),
        pos: 0,
    };
    let value = parser.parse_expr()?;
    parser.skip_ws();
    (parser.pos == parser.bytes.len()).then_some(value)
}

#[cfg(feature = "full")]
fn default_timeout_ms() -> u64 {
    1000
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchEngine {
    pub name: &'static str,
    pub long_prefix: &'static str,
    pub short_prefix: Option<&'static str>,
    pub url_template: &'static str,
}

pub const SEARCH_ENGINES: &[SearchEngine] = &[
    SearchEngine {
        name: "google",
        long_prefix: "google",
        short_prefix: Some("g"),
        url_template: "https://www.google.com/search?q={query}",
    },
    SearchEngine {
        name: "youtube",
        long_prefix: "youtube",
        short_prefix: Some("yt"),
        url_template: "https://www.youtube.com/results?search_query={query}",
    },
    SearchEngine {
        name: "wikipedia",
        long_prefix: "wikipedia",
        short_prefix: Some("wiki"),
        url_template: "https://en.wikipedia.org/wiki/Special:Search?search={query}",
    },
    SearchEngine {
        name: "perplexity",
        long_prefix: "perplexity",
        short_prefix: Some("pp"),
        url_template: "https://www.perplexity.ai/search?q={query}",
    },
    SearchEngine {
        name: "github",
        long_prefix: "github",
        short_prefix: Some("gh"),
        url_template: "https://github.com/search?q={query}",
    },
    SearchEngine {
        name: "reddit",
        long_prefix: "reddit",
        short_prefix: Some("r"),
        url_template: "https://www.reddit.com/search?q={query}",
    },
    SearchEngine {
        name: "stackoverflow",
        long_prefix: "stackoverflow",
        short_prefix: Some("so"),
        url_template: "https://stackoverflow.com/search?q={query}",
    },
    SearchEngine {
        name: "duckduckgo",
        long_prefix: "duckduckgo",
        short_prefix: Some("ddg"),
        url_template: "https://duckduckgo.com/?q={query}",
    },
    SearchEngine {
        name: "amazon",
        long_prefix: "amazon",
        short_prefix: Some("amz"),
        url_template: "https://www.amazon.com/s?k={query}",
    },
    SearchEngine {
        name: "twitter",
        long_prefix: "twitter",
        short_prefix: Some("x"),
        url_template: "https://twitter.com/search?q={query}",
    },
    SearchEngine {
        name: "maps",
        long_prefix: "maps",
        short_prefix: None,
        url_template: "https://maps.apple.com/?q={query}",
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrowserTabState {
    Active,
    Cached,
    Frozen,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserTab {
    pub title: String,
    pub url: String,
    pub state: BrowserTabState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserCommand {
    Navigate(String),
    Search { engine_name: String, query: String },
    Back,
    Forward,
    Reload,
    CloseTab,
    NewTab,
    SwitchTab(usize),
    ListTabs,
    Find(String),
    AddBookmark,
    SearchBookmarks(String),
    ShowHistory,
    SearchHistory(String),
    Share,
}

pub fn looks_like_url(input: &str) -> bool {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("about:")
        || trimmed.starts_with("file://")
    {
        return true;
    }
    if trimmed.contains(' ') {
        return false;
    }
    let without_path = trimmed.split('/').next().unwrap_or(trimmed);
    let without_port = without_path.split(':').next().unwrap_or(without_path);
    if let Some((left, right)) = without_port.split_once('.') {
        !left.is_empty()
            && !right.is_empty()
            && right.chars().any(|c| c.is_ascii_alphabetic())
            && left
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    } else {
        false
    }
}

pub fn browser_search_url(engine_name: &str, query: &str) -> String {
    let encoded = urlencode_str(query);
    let clean = engine_name.trim_start_matches('!');
    match clean {
        "rs" | "docs.rs" => return format!("https://docs.rs/releases/search?query={encoded}"),
        "crates" | "cargo" => return format!("https://crates.io/search?q={encoded}"),
        "arch" | "archwiki" => {
            return format!("https://wiki.archlinux.org/index.php?search={encoded}")
        }
        "hn" => return format!("https://hn.algolia.com/?q={encoded}"),
        _ => {}
    }
    for engine in SEARCH_ENGINES {
        if engine.name.eq_ignore_ascii_case(clean)
            || engine.long_prefix.eq_ignore_ascii_case(clean)
            || engine
                .short_prefix
                .is_some_and(|short| short.eq_ignore_ascii_case(clean))
        {
            return engine.url_template.replace("{query}", &encoded);
        }
    }
    format!("https://duckduckgo.com/?q={encoded}")
}

pub fn domain_from_url(url: &str) -> &str {
    let stripped = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    stripped
        .split('/')
        .next()
        .unwrap_or(stripped)
        .trim_start_matches("www.")
}

pub fn parse_browser_command(input: &str) -> Option<BrowserCommand> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(rest) = trimmed.strip_prefix("open ") {
        let target = rest.trim();
        if !target.is_empty() {
            let url = if target.starts_with("http://")
                || target.starts_with("https://")
                || target.starts_with("about:")
                || target.starts_with("file://")
            {
                target.to_string()
            } else {
                format!("https://{target}")
            };
            return Some(BrowserCommand::Navigate(url));
        }
    }
    if trimmed == "bk!" || trimmed.starts_with("bk!") {
        return Some(BrowserCommand::AddBookmark);
    }
    if let Some(rest) = trimmed.strip_prefix("bk ") {
        return Some(BrowserCommand::SearchBookmarks(rest.trim().to_string()));
    }
    if trimmed == "history" || trimmed == "h" {
        return Some(BrowserCommand::ShowHistory);
    }
    if let Some(rest) = trimmed.strip_prefix("h ") {
        return Some(BrowserCommand::SearchHistory(rest.trim().to_string()));
    }
    if let Some(rest) = trimmed.strip_prefix("find ") {
        return Some(BrowserCommand::Find(rest.trim().to_string()));
    }
    if let Some(rest) = trimmed.strip_prefix("tab ") {
        if let Ok(index) = rest.trim().parse::<usize>() {
            return Some(BrowserCommand::SwitchTab(index.saturating_sub(1)));
        }
    }
    match trimmed {
        "back" | "<" => return Some(BrowserCommand::Back),
        "forward" => return Some(BrowserCommand::Forward),
        "reload" | "refresh" | "r!" => return Some(BrowserCommand::Reload),
        "close" | "x!" => return Some(BrowserCommand::CloseTab),
        "new tab" | "newtab" | "nt" => return Some(BrowserCommand::NewTab),
        "tabs" | "ls" => return Some(BrowserCommand::ListTabs),
        "share" => return Some(BrowserCommand::Share),
        _ => {}
    }
    if let Some((prefix, query)) = trimmed.split_once(' ') {
        let query = query.trim();
        if !query.is_empty() {
            let prefix_lower = prefix.trim_start_matches('!').to_lowercase();
            match prefix_lower.as_str() {
                "rs" => {
                    return Some(BrowserCommand::Search {
                        engine_name: "docs.rs".to_string(),
                        query: query.to_string(),
                    })
                }
                "crates" => {
                    return Some(BrowserCommand::Search {
                        engine_name: "crates.io".to_string(),
                        query: query.to_string(),
                    })
                }
                "arch" => {
                    return Some(BrowserCommand::Search {
                        engine_name: "archwiki".to_string(),
                        query: query.to_string(),
                    })
                }
                "hn" => {
                    return Some(BrowserCommand::Search {
                        engine_name: "hn".to_string(),
                        query: query.to_string(),
                    })
                }
                _ => {}
            }
            for engine in SEARCH_ENGINES {
                if prefix_lower == engine.long_prefix
                    || engine.short_prefix.is_some_and(|s| prefix_lower == s)
                {
                    return Some(BrowserCommand::Search {
                        engine_name: engine.name.to_string(),
                        query: query.to_string(),
                    });
                }
            }
        }
    }
    if looks_like_url(trimmed) {
        let url = if trimmed.starts_with("http://")
            || trimmed.starts_with("https://")
            || trimmed.starts_with("about:")
            || trimmed.starts_with("file://")
        {
            trimmed.to_string()
        } else {
            format!("https://{trimmed}")
        };
        return Some(BrowserCommand::Navigate(url));
    }
    None
}

pub fn browser_hint_for(input: &str) -> Option<String> {
    match parse_browser_command(input)? {
        BrowserCommand::Navigate(url) => Some(format!("↵  open {url}")),
        BrowserCommand::Search { engine_name, query } => Some(format!("↵  {engine_name}: {query}")),
        BrowserCommand::Back => Some("↵  go back".to_string()),
        BrowserCommand::Forward => Some("↵  go forward".to_string()),
        BrowserCommand::Reload => Some("↵  reload".to_string()),
        BrowserCommand::CloseTab => Some("↵  close tab".to_string()),
        BrowserCommand::NewTab => Some("↵  new tab".to_string()),
        BrowserCommand::SwitchTab(idx) => Some(format!("↵  switch to tab {}", idx + 1)),
        BrowserCommand::ListTabs => Some("↵  list browser tabs".to_string()),
        BrowserCommand::Find(q) => Some(format!("↵  find: {q}")),
        BrowserCommand::AddBookmark => Some("↵  bookmark this page".to_string()),
        BrowserCommand::SearchBookmarks(q) => Some(format!("↵  search bookmarks: {q}")),
        BrowserCommand::ShowHistory => Some("↵  show history".to_string()),
        BrowserCommand::SearchHistory(q) => Some(format!("↵  search history: {q}")),
        BrowserCommand::Share => Some("↵  copy URL to clipboard".to_string()),
    }
}

#[derive(Debug, Clone)]
pub struct BrowserTransportState {
    pub tabs: Vec<BrowserTab>,
    pub active_tab: usize,
    pub bookmarks: Vec<(String, String)>,
    pub history: Vec<String>,
    pub last_find: Option<String>,
}

impl Default for BrowserTransportState {
    fn default() -> Self {
        Self::new()
    }
}

impl BrowserTransportState {
    pub fn new() -> Self {
        Self {
            tabs: vec![
                BrowserTab {
                    title: "perplexity.ai".to_string(),
                    url: "https://www.perplexity.ai".to_string(),
                    state: BrowserTabState::Active,
                },
                BrowserTab {
                    title: "github.com".to_string(),
                    url: "https://github.com".to_string(),
                    state: BrowserTabState::Cached,
                },
                BrowserTab {
                    title: "docs.rs".to_string(),
                    url: "https://docs.rs".to_string(),
                    state: BrowserTabState::Frozen,
                },
            ],
            active_tab: 0,
            bookmarks: vec![
                (
                    "Perplexity".to_string(),
                    "https://www.perplexity.ai".to_string(),
                ),
                ("GitHub".to_string(), "https://github.com".to_string()),
                (
                    "Rust Docs".to_string(),
                    "https://doc.rust-lang.org".to_string(),
                ),
            ],
            history: vec![
                "https://www.perplexity.ai".to_string(),
                "https://github.com".to_string(),
                "https://docs.rs".to_string(),
            ],
            last_find: None,
        }
    }

    pub fn active_url(&self) -> &str {
        self.tabs
            .get(self.active_tab)
            .map(|tab| tab.url.as_str())
            .unwrap_or("https://www.perplexity.ai")
    }

    pub fn switch_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            for (idx, tab) in self.tabs.iter_mut().enumerate() {
                if idx == index {
                    tab.state = BrowserTabState::Active;
                } else if tab.state == BrowserTabState::Active {
                    tab.state = BrowserTabState::Cached;
                }
            }
            self.active_tab = index;
        }
    }

    pub fn execute(&mut self, command: BrowserCommand) -> Option<PluginAction> {
        match command {
            BrowserCommand::Navigate(url) => {
                let title = domain_from_url(&url).to_string();
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.title = title;
                    tab.url = url.clone();
                    tab.state = BrowserTabState::Active;
                } else {
                    self.tabs.push(BrowserTab {
                        title,
                        url: url.clone(),
                        state: BrowserTabState::Active,
                    });
                    self.active_tab = self.tabs.len().saturating_sub(1);
                }
                self.history.retain(|entry| entry != &url);
                self.history.insert(0, url.clone());
                Some(PluginAction::Shell {
                    command: format!("xdg-open {}", shell_single_quote(&url)),
                })
            }
            BrowserCommand::Search { engine_name, query } => {
                let url = browser_search_url(&engine_name, &query);
                self.execute(BrowserCommand::Navigate(url))
            }
            BrowserCommand::NewTab => {
                for tab in &mut self.tabs {
                    if tab.state == BrowserTabState::Active {
                        tab.state = BrowserTabState::Cached;
                    }
                }
                let url = "https://www.perplexity.ai".to_string();
                self.tabs.push(BrowserTab {
                    title: "perplexity.ai".to_string(),
                    url,
                    state: BrowserTabState::Active,
                });
                self.active_tab = self.tabs.len().saturating_sub(1);
                None
            }
            BrowserCommand::CloseTab => {
                if self.tabs.len() > 1 {
                    self.tabs.remove(self.active_tab);
                    if self.active_tab >= self.tabs.len() {
                        self.active_tab = self.tabs.len() - 1;
                    }
                    if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                        tab.state = BrowserTabState::Active;
                    }
                }
                None
            }
            BrowserCommand::SwitchTab(index) => {
                self.switch_tab(index);
                None
            }
            BrowserCommand::AddBookmark => {
                let url = self.active_url().to_string();
                let title = domain_from_url(&url).to_string();
                if !self.bookmarks.iter().any(|(_, u)| u == &url) {
                    self.bookmarks.insert(0, (title, url));
                }
                None
            }
            BrowserCommand::Share => {
                let url = self.active_url().to_string();
                Some(PluginAction::Shell {
                    command: clipboard_copy_command(&url),
                })
            }
            BrowserCommand::Find(query) => {
                self.last_find = Some(query);
                None
            }
            BrowserCommand::Back => {
                if let Some(prev) = self.history.get(1).cloned() {
                    return self.execute(BrowserCommand::Navigate(prev));
                }
                None
            }
            BrowserCommand::Forward
            | BrowserCommand::Reload
            | BrowserCommand::ListTabs
            | BrowserCommand::SearchBookmarks(_)
            | BrowserCommand::ShowHistory
            | BrowserCommand::SearchHistory(_) => None,
        }
    }
}

struct BrowserPlugin;

impl Plugin for BrowserPlugin {
    fn id(&self) -> &str {
        "browser"
    }

    fn query(&self, query: &str, _matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        let Some(cmd) = parse_browser_command(query) else {
            return Vec::new();
        };
        match cmd {
            BrowserCommand::Navigate(url) => vec![PluginResult {
                plugin_id: self.id().to_string(),
                title: format!("Open {url} in Browser"),
                subtitle: "rover browser transport".to_string(),
                score: i64::MAX - 3,
                action: PluginAction::Shell {
                    command: format!("xdg-open {}", shell_single_quote(&url)),
                },
            }],
            BrowserCommand::Search { engine_name, query } => {
                let url = browser_search_url(&engine_name, &query);
                vec![PluginResult {
                    plugin_id: self.id().to_string(),
                    title: format!("Search {engine_name} for \"{query}\""),
                    subtitle: url.clone(),
                    score: i64::MAX - 3,
                    action: PluginAction::Shell {
                        command: format!("xdg-open {}", shell_single_quote(&url)),
                    },
                }]
            }
            _ => Vec::new(),
        }
    }
}

pub fn help_answer_for(query: &str) -> Option<(String, String)> {
    let trimmed = query.trim();
    let topic = trimmed
        .strip_prefix("help ")
        .or_else(|| trimmed.strip_prefix("? "))
        .or_else(|| (trimmed == "help" || trimmed == "?").then_some(""))?
        .trim()
        .to_lowercase();

    let answer = match topic.as_str() {
        "" | "rover" | "launcher" => (
            "Rover Bar: type apps/commands, > shell, g/yt/gh bangs, = math, Cmd+K actions",
            "help: overview",
        ),
        "niri" | "strip" | "columns" => (
            "Niri Strip: Super+H/L focus col, Super+R cycle width (1/3,1/2,2/3,Full), Super+,/. consume/expel",
            "help: niri scrollable strip",
        ),
        "browser" | "bangs" | "web" => (
            "Browser: URLs, bangs (g, yt, wiki, pp, gh, r, so, ddg, amz, x, maps), bk!, bk <q>, h <q>",
            "help: zero-chrome browser transport",
        ),
        "terminal" | "shell" => (
            "Terminal: prefix with > or ! to run shell commands inline inside the bar",
            "help: inline terminal",
        ),
        "windows" | "mode" | "tiling" => (
            "Modes: niri (scrollable column strip), tiling (BSP), floating (windowing), monocle",
            "help: window modes",
        ),
        _ => return None,
    };
    Some((answer.0.to_string(), answer.1.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "full")]
    use std::fs;
    #[cfg(feature = "full")]
    use std::os::unix::fs::PermissionsExt;

    #[test]
    #[cfg(feature = "full")]
    fn manifest_rejects_missing_command() {
        let dir = test_dir("bad_manifest");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("plugin.json");
        fs::write(
            &path,
            r#"{"id":"bad","name":"Bad","kind":"command","command":[]}"#,
        )
        .unwrap();

        assert!(CommandPlugin::from_manifest_file(&path).is_err());
    }

    #[test]
    #[cfg(feature = "full")]
    fn command_plugin_reads_json_response() {
        let dir = test_dir("command_plugin");
        fs::create_dir_all(&dir).unwrap();
        let script = dir.join("plugin.sh");
        fs::write(
            &script,
            "#!/bin/sh\ncat >/dev/null\nprintf '%s' '{\"results\":[{\"id\":\"ok\",\"title\":\"OK\",\"subtitle\":\"command\",\"score\":7,\"action\":{\"type\":\"shell\",\"command\":\"echo ok\"}}]}'\n",
        )
        .unwrap();
        let mut permissions = fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script, permissions).unwrap();
        let manifest = CommandPluginManifest {
            id: "ok".to_string(),
            name: "OK".to_string(),
            kind: PluginKind::Command,
            command: vec![script.display().to_string()],
            matcher: MatchMode::Always,
            timeout_ms: 1000,
        };

        let results = run_command_plugin(&manifest, &dir, "ok").unwrap();

        assert_eq!(results[0].title, "OK");
        assert_eq!(
            results[0].action,
            PluginAction::Shell {
                command: "echo ok".to_string()
            }
        );
    }

    #[test]
    #[cfg(feature = "full")]
    fn spotify_reports_unavailable_without_playerctl() {
        let plugin = SpotifyPlugin::with_playerctl(false);
        let results = plugin.query("spotify", &SkimMatcherV2::default());

        assert_eq!(results[0].title, "Spotify unavailable");
    }

    #[test]
    fn window_mode_plugin_exposes_all_modes() {
        let results = WindowModePlugin.query("windows", &SkimMatcherV2::default());
        let titles: Vec<&str> = results.iter().map(|r| r.title.as_str()).collect();
        assert!(titles.contains(&"Tile windows"));
        assert!(titles.contains(&"Float windows"));
        assert!(titles.contains(&"Monocle windows"));
        assert!(titles.contains(&"Stack windows"));
        assert!(titles.contains(&"Center windows"));
        assert!(titles.contains(&"Grid windows"));
    }

    #[test]
    fn window_mode_plugin_offers_cycle() {
        let results = WindowModePlugin.query("cycle", &SkimMatcherV2::default());
        assert!(results
            .iter()
            .any(|r| r.action == PluginAction::CycleWindowMode));
    }

    #[test]
    fn layout_plugin_exposes_move_and_balance() {
        let results = LayoutPlugin.query("balance", &SkimMatcherV2::default());
        assert!(results.iter().any(|r| r.title.contains("Balance panes")));
        let results = LayoutPlugin.query("move window left", &SkimMatcherV2::default());
        assert!(results.iter().any(|r| r.title.contains("Move window left")));
    }

    #[test]
    fn layout_plugin_exposes_tinycast_rectangle_snap_actions() {
        let matcher = SkimMatcherV2::default();
        let left_half = LayoutPlugin.query("Window: Left Half", &matcher);
        assert!(left_half.iter().any(|r| {
            r.action
                == PluginAction::Layout {
                    action: LayoutAction::SnapLeftHalf,
                }
        }));
        let center_third = LayoutPlugin.query("Center Third", &matcher);
        assert!(center_third.iter().any(|r| {
            r.action
                == PluginAction::Layout {
                    action: LayoutAction::SnapCenterThird,
                }
        }));
        let maximize = LayoutPlugin.query("Almost Maximize", &matcher);
        assert!(maximize.iter().any(|r| {
            r.action
                == PluginAction::Layout {
                    action: LayoutAction::AlmostMaximize,
                }
        }));
    }

    #[test]
    fn calculator_plugin_evaluates_math_natively() {
        let matcher = SkimMatcherV2::default();
        let res = CalculatorPlugin.query("(12 + 8) * 3 / 2", &matcher);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].title, "= 30");

        let pow_res = CalculatorPlugin.query("2 ^ 10", &matcher);
        assert_eq!(pow_res[0].title, "= 1024");
    }

    #[test]
    fn quicklinks_snippets_dictionary_and_notes_plugins_respond() {
        let matcher = SkimMatcherV2::default();
        let ql = QuicklinksPlugin.query("ql github", &matcher);
        assert!(!ql.is_empty());

        let snip = SnippetsPlugin.query(";shrug", &matcher);
        assert_eq!(snip.len(), 1);
        assert!(snip[0].title.contains("Shrug"));

        let dict = DictionaryPlugin.query("define alpenglow", &matcher);
        assert!(!dict.is_empty());
        assert!(dict[0].subtitle.contains("reddish glow"));

        let notes = NotesPlugin.query("note: buy milk", &matcher);
        assert_eq!(notes.len(), 1);
        assert!(notes[0].title.contains("buy milk"));
    }

    #[test]
    fn shell_single_quote_escapes_embedded_quotes() {
        assert_eq!(shell_single_quote("hello"), "'hello'");
        assert_eq!(shell_single_quote("a'b"), "'a'\\''b'");
    }

    #[test]
    fn volume_plugin_offers_set_level() {
        let results = VolumePlugin.query("volume 50", &SkimMatcherV2::default());
        assert!(results.iter().any(|r| r.title == "Set volume to 50%"));
    }

    #[test]
    fn brightness_plugin_offers_set_level() {
        let results = BrightnessPlugin.query("brightness 75", &SkimMatcherV2::default());
        assert!(results.iter().any(|r| r.title == "Set brightness to 75%"));
    }

    #[test]
    fn parse_duration_should_handle_compound_units() {
        assert_eq!(parse_duration("5m"), Some(300));
        assert_eq!(parse_duration("1h"), Some(3600));
        assert_eq!(parse_duration("30s"), Some(30));
        assert_eq!(parse_duration("1h30m"), Some(5400));
        assert_eq!(parse_duration(""), None);
        assert_eq!(parse_duration("abc"), None);
    }

    #[test]
    fn timer_plugin_should_parse_duration() {
        let results = TimerPlugin.query("timer 5m", &SkimMatcherV2::default());
        assert_eq!(results.len(), 1);
        assert!(results[0].title.contains("5m"));
    }

    #[test]
    fn color_plugin_should_parse_hex() {
        let results = ColorPlugin.query("#fff", &SkimMatcherV2::default());
        assert_eq!(results.len(), 1);
        assert!(results[0].title.contains("rgb(255, 255, 255)"));
        let results = ColorPlugin.query("#ff0000", &SkimMatcherV2::default());
        assert!(results[0].title.contains("rgb(255, 0, 0)"));
    }

    #[test]
    fn color_plugin_should_ignore_non_hex() {
        let results = ColorPlugin.query("#xyz", &SkimMatcherV2::default());
        assert!(results.is_empty());
    }

    #[test]
    fn convert_units_should_convert_lengths() {
        assert!((convert_units(1.0, "km", "m").unwrap() - 1000.0).abs() < 0.001);
        assert!((convert_units(1.0, "mi", "km").unwrap() - 1.609344).abs() < 0.001);
        assert!((convert_units(12.0, "in", "cm").unwrap() - 30.48).abs() < 0.01);
    }

    #[test]
    fn convert_units_should_convert_weights() {
        assert!((convert_units(1.0, "kg", "g").unwrap() - 1000.0).abs() < 0.001);
        assert!((convert_units(1.0, "lb", "g").unwrap() - 453.592).abs() < 0.1);
    }

    #[test]
    fn convert_units_should_convert_temperatures() {
        assert!((convert_units(0.0, "c", "f").unwrap() - 32.0).abs() < 0.001);
        assert!((convert_units(100.0, "c", "f").unwrap() - 212.0).abs() < 0.001);
        assert!((convert_units(32.0, "f", "c").unwrap() - 0.0).abs() < 0.001);
    }

    #[test]
    fn convert_units_should_reject_mismatched_categories() {
        assert!(convert_units(1.0, "km", "kg").is_none());
    }

    #[test]
    fn unit_converter_plugin_should_format_result() {
        let results = UnitConverterPlugin.query("10 km to mi", &SkimMatcherV2::default());
        assert_eq!(results.len(), 1);
        assert!(results[0].title.contains("10 km"));
        assert!(results[0].title.contains("mi"));
    }

    #[test]
    fn unit_converter_plugin_should_ignore_bad_pattern() {
        let results = UnitConverterPlugin.query("hello world foo bar", &SkimMatcherV2::default());
        assert!(results.is_empty());
    }

    #[test]
    #[cfg(feature = "full")]
    fn translate_plugin_should_build_url() {
        let results = TranslatePlugin.query("translate hello to es", &SkimMatcherV2::default());
        assert_eq!(results.len(), 1);
        assert!(results[0].title.contains("hello"));
        assert!(results[0].title.contains("es"));
    }

    #[test]
    #[cfg(feature = "full")]
    fn translate_plugin_should_hint_on_partial_input() {
        let results = TranslatePlugin.query("translate hello", &SkimMatcherV2::default());
        assert_eq!(results.len(), 1);
        assert!(results[0].action == PluginAction::None);
    }

    #[test]
    #[cfg(feature = "full")]
    fn urlencode_should_encode_special_chars() {
        assert_eq!(urlencode("hello world"), "hello%20world");
        assert_eq!(urlencode("a&b=c"), "a%26b%3Dc");
        assert_eq!(urlencode("safe-_.~"), "safe-_.~");
    }

    #[test]
    fn help_plugin_should_list_entries() {
        let results = HelpPlugin::for_role(crate::role::SessionRole::Desktop)
            .query("help", &SkimMatcherV2::default());
        assert!(results.iter().any(|r| r.title.contains("launcher")));
        assert!(results.iter().any(|r| r.title.contains("shell")));
        assert!(results.iter().any(|r| r.title.contains("window modes")));
    }

    #[test]
    fn system_info_plugin_should_match_uptime() {
        let results = SystemInfoPlugin.query("uptime", &SkimMatcherV2::default());
        assert!(results.iter().any(|r| r.title == "Uptime"));
    }

    #[test]
    fn network_info_plugin_should_match_ip() {
        let results = NetworkInfoPlugin.query("ip", &SkimMatcherV2::default());
        assert!(results.iter().any(|r| r.title == "IP address"));
    }

    #[cfg(feature = "full")]
    #[test]
    fn weather_plugin_should_match_query() {
        let results = WeatherPlugin.query("weather", &SkimMatcherV2::default());
        assert!(results.iter().any(|r| r.title == "Weather now"));
        assert!(results.iter().any(|r| r.title == "Weather full"));
    }

    #[test]
    fn potato_registry_should_omit_weather() {
        let registry = PluginRegistry::with_role(crate::role::SessionRole::Potato);
        let results = registry.query_with_windows("weather", &SkimMatcherV2::default(), &[]);
        assert!(!results.iter().any(|result| result.plugin_id == "weather"));
        assert!(!results
            .iter()
            .any(|result| result.title.contains("weather")));
    }

    #[test]
    fn potato_registry_should_omit_translate_and_web_help() {
        let registry = PluginRegistry::with_role(crate::role::SessionRole::Potato);
        let help = registry.query_with_windows("help", &SkimMatcherV2::default(), &[]);
        assert!(!help.iter().any(|result| result.title.contains("translate")));
        assert!(!help.iter().any(|result| result.title.contains("Help: web")));
        assert!(!help
            .iter()
            .any(|result| result.title.contains("Help: weather")));
    }

    #[cfg(feature = "full")]
    #[test]
    fn desktop_registry_should_include_weather() {
        let registry = PluginRegistry::with_role(crate::role::SessionRole::Desktop);
        let results = registry.query_with_windows("weather", &SkimMatcherV2::default(), &[]);
        assert!(results.iter().any(|result| result.plugin_id == "weather"));
    }

    #[cfg(feature = "full")]
    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("alpenglowed-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn rover_browser_parses_urls_and_all_eleven_search_bangs() {
        assert!(looks_like_url("https://github.com/semitechnological/rover"));
        assert!(looks_like_url("perplexity.ai"));
        assert!(looks_like_url("docs.rs/gpui"));
        assert!(!looks_like_url("just some words"));

        assert_eq!(
            parse_browser_command("github.com"),
            Some(BrowserCommand::Navigate("https://github.com".to_string()))
        );
        let bangs = [
            ("g rust smithay", "google"),
            ("yt niri wm", "youtube"),
            ("wiki wayland", "wikipedia"),
            ("pp gpui rust", "perplexity"),
            ("gh smithay", "github"),
            ("r linux", "reddit"),
            ("so borrow checker", "stackoverflow"),
            ("ddg privacy", "duckduckgo"),
            ("amz mechanical keyboard", "amazon"),
            ("x rustlang", "twitter"),
            ("maps tokyo", "maps"),
        ];
        assert_eq!(SEARCH_ENGINES.len(), 11);
        for (input, expected_engine) in bangs {
            match parse_browser_command(input) {
                Some(BrowserCommand::Search { engine_name, .. }) => {
                    assert_eq!(engine_name, expected_engine);
                }
                other => panic!("expected Search for {input}, got {other:?}"),
            }
        }
    }

    #[test]
    fn rover_browser_transport_state_handles_tabs_bookmarks_and_history() {
        let mut transport = BrowserTransportState::new();
        assert_eq!(transport.tabs.len(), 3);
        assert_eq!(transport.active_tab, 0);

        transport.execute(BrowserCommand::NewTab);
        assert_eq!(transport.tabs.len(), 4);
        assert_eq!(transport.active_tab, 3);

        transport.execute(BrowserCommand::Navigate(
            "https://smithay.github.io".to_string(),
        ));
        assert_eq!(transport.active_url(), "https://smithay.github.io");
        assert_eq!(transport.history[0], "https://smithay.github.io");

        let prev_bookmarks = transport.bookmarks.len();
        transport.execute(BrowserCommand::AddBookmark);
        assert_eq!(transport.bookmarks.len(), prev_bookmarks + 1);

        transport.execute(BrowserCommand::SwitchTab(1));
        assert_eq!(transport.active_tab, 1);
        assert_eq!(transport.tabs[1].state, BrowserTabState::Active);
    }

    #[test]
    fn niri_strip_mode_and_layout_actions_are_exposed_by_plugins() {
        let matcher = SkimMatcherV2::default();
        let modes = WindowModePlugin.query("niri", &matcher);
        assert!(modes.iter().any(|r| {
            r.action
                == PluginAction::SetWindowMode {
                    mode: WindowMode::NiriStrip,
                }
        }));
        let cycle_width = LayoutPlugin.query("Niri: Cycle Column Width", &matcher);
        assert!(cycle_width.iter().any(|r| {
            r.action
                == PluginAction::Layout {
                    action: LayoutAction::CycleColumnWidth,
                }
        }));
    }
}
