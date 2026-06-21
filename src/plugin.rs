use crate::de::DesktopAction;
use crate::runner::WindowMode;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginAction {
    Launch { program: String },
    Shell { command: String },
    SetWindowMode { mode: WindowMode },
    OpenSettings,
    Desktop { action: DesktopAction },
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

impl PluginRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            plugins: Vec::new(),
        };
        registry.register(Box::new(ShellPlugin));
        registry.register(Box::new(CalculatorPlugin));
        registry.register(Box::new(WindowModePlugin));
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

    pub fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        let mut results = self
            .plugins
            .iter()
            .flat_map(|plugin| plugin.query(query, matcher))
            .collect::<Vec<_>>();
        results.sort_by_key(|result| Reverse(result.score));
        results.truncate(15);
        results
    }
}

struct ShellPlugin;

impl Plugin for ShellPlugin {
    fn id(&self) -> &str {
        "shell"
    }

    fn query(&self, query: &str, _matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        let command = query.trim().strip_prefix('>').map(str::trim).unwrap_or("");
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
                action: PluginAction::None,
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

struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn id(&self) -> &str {
        "settings"
    }

    fn query(&self, query: &str, matcher: &SkimMatcherV2) -> Vec<PluginResult> {
        [
            ("Settings", "desktop settings"),
            ("Open settings", "desktop settings"),
            ("Preferences", "desktop settings"),
        ]
        .into_iter()
        .filter_map(|(title, subtitle)| {
            score(title, query, matcher).map(|score| PluginResult {
                plugin_id: self.id().to_string(),
                title: title.to_string(),
                subtitle: subtitle.to_string(),
                score,
                action: PluginAction::OpenSettings,
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
        let _ = Command::new("chmod").arg("+x").arg(&script).status();
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
