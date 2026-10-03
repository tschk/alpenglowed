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
        #[cfg(feature = "full")]
        if role.extras() {
            registry.register(Box::new(WebSearchPlugin));
            registry.register(Box::new(TranslatePlugin));
            registry.register(Box::new(WeatherPlugin));
            registry.register(Box::new(SpotifyPlugin));
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
        vec![PluginResult {
            plugin_id: self.id().to_string(),
            title: format!("Search web for \"{search}\""),
            subtitle: "duckduckgo".to_string(),
            score: i64::MAX,
            action: PluginAction::Shell {
                command: format!("xdg-open 'https://duckduckgo.com/?q={search}'"),
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
                        command: format!(
                            "printf '%s' '{}' | wl-copy 2>/dev/null || printf '%s' '{}' | xclip -selection clipboard",
                            emoji, emoji
                        ),
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

impl Plugin for FileSearchPlugin {
    fn id(&self) -> &str {
        "files"
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        let search = query.trim().strip_prefix('/').map(str::trim).unwrap_or("");
        if search.is_empty() {
            return Vec::new();
        }

        let output = Command::new("sh")
            .arg("-c")
            .arg(crate::shell::file_search_command(search))
            .output()
            .ok();
        let output = match output {
            Some(o) if o.status.success() => o,
            _ => return Vec::new(),
        };
        let stdout = String::from_utf8_lossy(&output.stdout);
        let paths: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
        if paths.is_empty() {
            return Vec::new();
        }

        let mut results: Vec<PluginResult> = paths
            .iter()
            .filter_map(|path| {
                let filename = std::path::Path::new(path)
                    .file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or((*path).into());
                matcher
                    .fuzzy_match(&filename, search)
                    .or_else(|| Some(if path.contains(search) { 10 } else { 1 }))
                    .map(|score| PluginResult {
                        plugin_id: self.id().to_string(),
                        title: filename.to_string(),
                        subtitle: path.to_string(),
                        score: score.min(100),
                        action: PluginAction::Shell {
                            command: crate::shell::open_file_command(path),
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

        let output = Command::new("sh")
            .arg("-c")
            .arg("cliphist list 2>/dev/null | head -10")
            .output()
            .ok();
        if let Some(o) = output.filter(|o| o.status.success()) {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let mut results: Vec<PluginResult> = stdout
                .lines()
                .filter_map(|line| {
                    let (id, preview) = line.split_once('\t')?;
                    let preview = preview.trim();
                    if preview.is_empty() {
                        return None;
                    }
                    let score = matcher.fuzzy_match(preview, &search).unwrap_or(1);
                    Some(PluginResult {
                        plugin_id: self.id().to_string(),
                        title: preview.chars().take(60).collect(),
                        subtitle: "clipboard".to_string(),
                        score,
                        action: PluginAction::Shell {
                            command: format!(
                                "cliphist decode '{}' | wl-copy 2>/dev/null || cliphist decode '{}' | xclip -selection clipboard",
                                id, id
                            ),
                        },
                    })
                })
                .collect();
            results.sort_by_key(|r| std::cmp::Reverse(r.score));
            results.truncate(6);
            return results;
        }

        let current = Command::new("sh")
            .arg("-c")
            .arg("wl-paste 2>/dev/null || xclip -o -selection clipboard 2>/dev/null")
            .output()
            .ok();
        if let Some(o) = current.filter(|o| o.status.success()) {
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

fn run_capture(command: &str) -> Vec<PluginResult> {
    let output = Command::new("sh").arg("-c").arg(command).output().ok();
    let output = match output {
        Some(o) if o.status.success() || !o.stdout.is_empty() => o,
        _ => {
            return vec![PluginResult {
                plugin_id: "shell".to_string(),
                title: format!("{command}: no output"),
                subtitle: "shell".to_string(),
                score: i64::MAX,
                action: PluginAction::Shell {
                    command: command.to_string(),
                },
            }]
        }
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let mut results = Vec::new();
    for line in stdout.lines().chain(stderr.lines()) {
        if results.len() >= 6 {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        results.push(PluginResult {
            plugin_id: "shell".to_string(),
            title: line.chars().take(80).collect(),
            subtitle: format!("$ {command}"),
            score: i64::MAX - results.len() as i64,
            action: PluginAction::Shell {
                command: command.to_string(),
            },
        });
    }
    if results.is_empty() {
        results.push(PluginResult {
            plugin_id: "shell".to_string(),
            title: "(empty output)".to_string(),
            subtitle: format!("$ {command}"),
            score: i64::MAX,
            action: PluginAction::Shell {
                command: command.to_string(),
            },
        });
    }
    results
}

struct ShellPlugin;

impl Plugin for ShellPlugin {
    fn id(&self) -> &str {
        "shell"
    }

    fn query(&self, query: &str, _matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        let trimmed = query.trim();
        if trimmed.is_empty() || !trimmed.starts_with('>') {
            return Vec::new();
        }

        if let Some(cmd) = trimmed.strip_prefix(">'").map(str::trim) {
            if !cmd.is_empty() {
                return run_capture(cmd);
            }
        }

        let command = trimmed.strip_prefix('>').map(str::trim).unwrap_or("");
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

struct CalculatorPlugin;

impl Plugin for CalculatorPlugin {
    fn id(&self) -> &str {
        "calculator"
    }

    fn query(&self, query: &str, _matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        if !is_math(query) {
            return Vec::new();
        }
        calc(query).map_or_else(Vec::new, |value| {
            vec![PluginResult {
                plugin_id: self.id().to_string(),
                title: format!("= {value}"),
                subtitle: "calculator".to_string(),
                score: i64::MAX,
                action: PluginAction::Shell {
                    command: format!(
                        "printf '%s' '{}' | wl-copy 2>/dev/null || printf '%s' '{}' | xclip -selection clipboard",
                        value, value
                    ),
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
struct SpotifyPlugin;

#[cfg(feature = "full")]
impl Plugin for SpotifyPlugin {
    fn id(&self) -> &str {
        "spotify"
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        if matcher.fuzzy_match("Spotify", query).is_some() && !program_available("playerctl") {
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
        let results = actions
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
            .collect::<Vec<_>>();
        results
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
        let output = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "ps -eo pid,comm --no-headers | grep -i '{needle}' | head -6"
            ))
            .output()
            .ok();
        let Some(output) = output.filter(|o| o.status.success() || !o.stdout.is_empty()) else {
            return Vec::new();
        };
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .lines()
            .filter_map(|line| {
                let mut parts = line.split_whitespace();
                let pid = parts.next()?;
                let name = parts.collect::<Vec<_>>().join(" ");
                if name.is_empty() {
                    return None;
                }
                let score = matcher.fuzzy_match(&name, needle).unwrap_or(10);
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
            .collect()
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
        let command = format!(
            "(sleep {seconds} && notify-send -u critical alpenglowed 'timer: {rest} done') &"
        );
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
                command: format!(
                    "printf '%s' 'rgb({r}, {g}, {b})' | wl-copy 2>/dev/null || printf '%s' 'rgb({r}, {g}, {b})' | xclip -selection clipboard",
                ),
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
                command: format!(
                    "printf '%s' '{result:.4}' | wl-copy 2>/dev/null || printf '%s' '{result:.4}' | xclip -selection clipboard",
                ),
            },
        }]
    }
}

fn convert_units(value: f64, from: &str, to: &str) -> Option<f64> {
    // Convert to a canonical base unit, then to target.
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
        // pattern: translate <text> to <lang>
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
        let url = format!(
            "https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl={lang}&dt=t&q={}",
            urlencode(text)
        );
        vec![PluginResult {
            plugin_id: self.id().to_string(),
            title: format!("Translate \"{text}\" to {lang}"),
            subtitle: "google translate".to_string(),
            score: i64::MAX,
            action: PluginAction::Shell {
                command: format!(
                    "curl -s '{url}' | jq -r '.[0][0][0]' 2>/dev/null || curl -s '{url}'"
                ),
            },
        }]
    }
}

#[cfg(feature = "full")]
fn urlencode(text: &str) -> String {
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

struct RecentFilesPlugin;

impl Plugin for RecentFilesPlugin {
    fn id(&self) -> &str {
        "recent"
    }

    fn query(&self, query: &str, _matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        if !query.trim().to_lowercase().starts_with("recent") {
            return Vec::new();
        }
        let output = Command::new("sh")
            .arg("-c")
            .arg("find ~ -type f -mmin -1440 -not -path '*/.cache/*' -not -path '*/.git/*' 2>/dev/null | head -8")
            .output()
            .ok();
        let Some(output) = output.filter(|o| o.status.success() || !o.stdout.is_empty()) else {
            return vec![PluginResult {
                plugin_id: self.id().to_string(),
                title: "Recent files unavailable".to_string(),
                subtitle: "find command failed".to_string(),
                score: 1,
                action: PluginAction::None,
            }];
        };
        let stdout = String::from_utf8_lossy(&output.stdout);
        let paths: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
        if paths.is_empty() {
            return vec![PluginResult {
                plugin_id: self.id().to_string(),
                title: "No recent files".to_string(),
                subtitle: "nothing modified in last 24h".to_string(),
                score: 1,
                action: PluginAction::None,
            }];
        }
        paths
            .iter()
            .map(|path| {
                let filename = std::path::Path::new(path)
                    .file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or((*path).into());
                PluginResult {
                    plugin_id: self.id().to_string(),
                    title: filename.to_string(),
                    subtitle: path.to_string(),
                    score: 100,
                    action: PluginAction::Shell {
                        command: crate::shell::open_file_command(path),
                    },
                }
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
            .all(|c| c.is_ascii_digit() || "+-*/() .".contains(c))
        && text.contains(|c: char| c.is_ascii_digit())
}

fn calc(expr: &str) -> Option<f64> {
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

#[cfg(feature = "full")]
fn default_timeout_ms() -> u64 {
    1000
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
        let old_path = std::env::var_os("PATH");
        std::env::set_var("PATH", test_dir("empty_path"));
        let results = SpotifyPlugin.query("spotify", &SkimMatcherV2::default());
        if let Some(path) = old_path {
            std::env::set_var("PATH", path);
        }

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
}
