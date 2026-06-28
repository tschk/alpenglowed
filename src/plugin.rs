use crate::de::DesktopAction;
use crate::layout::LayoutAction;
use crate::runner::WindowMode;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginAction {
    Launch { program: String },
    Shell { command: String },
    FocusWindow { id: usize },
    SetWindowMode { mode: WindowMode },
    Layout { action: LayoutAction },
    ShowStatusBar,
    HideStatusBar,
    ToggleStatusBar,
    ToggleSettings,
    OpenSettings,
    CloseSettings,
    Desktop { action: DesktopAction },
    ToggleTerminal,
    TerminalClear,
    TerminalWrite { line: String },
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
    pub fn new() -> Self {
        let mut registry = Self {
            plugins: Vec::new(),
        };
        registry.register(Box::new(WebSearchPlugin));
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
        registry.register(Box::new(DesktopActionsPlugin));
        registry.register(Box::new(AppLauncherPlugin));
        registry.register(Box::new(SpotifyPlugin));
        for plugin in CommandPlugin::load_default() {
            registry.register(Box::new(plugin));
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
        results.sort_by_key(|result| Reverse(result.score));
        results.truncate(6);
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

struct WebSearchPlugin;

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
            .args(["-c", &format!("locate -i -l 8 '{}' 2>/dev/null || fd -t f -l 8 '{}' 2>/dev/null || find ~ -maxdepth 4 -iname '*{}*' -type f 2>/dev/null | head -8", search, search, search)])
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
                            command: format!("xdg-open '{}'", path),
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
        [
            ("Tile windows", "window mode", WindowMode::Tiling),
            ("Float windows", "window mode", WindowMode::Floating),
        ]
        .into_iter()
        .filter_map(|(title, subtitle, mode)| {
            score(title, query, matcher).map(|score| PluginResult {
                plugin_id: self.id().to_string(),
                title: title.to_string(),
                subtitle: subtitle.to_string(),
                score,
                action: PluginAction::SetWindowMode { mode },
            })
        })
        .collect()
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
            ("Close focused window", "layout", LayoutAction::CloseFocused),
            ("Toggle floating", "layout", LayoutAction::ToggleFloat),
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

struct SpotifyPlugin;

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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginKind {
    Command,
    Crepus,
    Rust,
    Webcode,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchMode {
    #[default]
    Always,
    Prefix,
    Fuzzy,
}

pub struct CommandPlugin {
    manifest: CommandPluginManifest,
    base_dir: PathBuf,
}

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

#[derive(Debug, Serialize)]
struct PluginRequest<'a> {
    r#type: &'a str,
    query: &'a str,
}

#[derive(Debug, Deserialize)]
struct PluginResponse {
    results: Vec<PluginResponseResult>,
}

#[derive(Debug, Deserialize)]
struct PluginResponseResult {
    title: String,
    subtitle: String,
    score: i64,
    action: PluginAction,
}

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

fn default_timeout_ms() -> u64 {
    1000
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    #[test]
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
    fn spotify_reports_unavailable_without_playerctl() {
        let old_path = std::env::var_os("PATH");
        std::env::set_var("PATH", test_dir("empty_path"));
        let results = SpotifyPlugin.query("spotify", &SkimMatcherV2::default());
        if let Some(path) = old_path {
            std::env::set_var("PATH", path);
        }

        assert_eq!(results[0].title, "Spotify unavailable");
    }

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("alpenglowed-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }
}
