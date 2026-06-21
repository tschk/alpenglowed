// Alpenglowed Runner — fuzzy app launcher + shell runner + calculator

use crate::de::DesktopAction;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::cmp::Reverse;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

static APP_CACHE: OnceLock<Vec<String>> = OnceLock::new();

fn apps() -> &'static Vec<String> {
    APP_CACHE.get_or_init(|| {
        let mut a = Vec::new();
        if let Ok(p) = std::env::var("PATH") {
            for d in p.split(':') {
                if let Ok(e) = std::fs::read_dir(d) {
                    for entry in e.flatten() {
                        if let Some(n) = entry.file_name().to_str() {
                            if !n.starts_with('.') {
                                a.push(n.to_owned());
                            }
                        }
                    }
                }
            }
        }
        a.sort();
        a.dedup();
        a
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowMode {
    Tiling,
    Floating,
}

impl WindowMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Tiling => "tiling",
            Self::Floating => "floating",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RunnerAction {
    Launch(String),
    Shell(String),
    Calculator(f64),
    SetWindowMode(WindowMode),
    Desktop(DesktopAction),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunnerResult {
    pub title: String,
    pub subtitle: String,
    pub score: i64,
    pub action: RunnerAction,
}

pub struct Runner {
    pub query: String,
    pub results: Vec<RunnerResult>,
    matcher: SkimMatcherV2,
}

impl Runner {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            matcher: SkimMatcherV2::default(),
        }
    }

    pub fn update(&mut self) {
        self.results.clear();
        let q = self.query.trim();

        if q.starts_with('>') {
            let cmd = q.trim_start_matches('>').trim();
            if !cmd.is_empty() {
                self.results.push(RunnerResult {
                    title: format!("Run {cmd}"),
                    subtitle: "shell".to_string(),
                    score: i64::MAX,
                    action: RunnerAction::Shell(cmd.to_string()),
                });
            }
            return;
        }

        if is_math(q) {
            if let Some(v) = calc(q) {
                self.results.push(RunnerResult {
                    title: format!("= {v}"),
                    subtitle: "calculator".to_string(),
                    score: i64::MAX,
                    action: RunnerAction::Calculator(v),
                });
            }
        }

        for result in action_results(q, &self.matcher) {
            self.results.push(result);
        }

        for app in apps() {
            if let Some(s) = self.matcher.fuzzy_match(app, q) {
                if s > 0 {
                    self.results.push(RunnerResult {
                        title: app.clone(),
                        subtitle: "app".to_string(),
                        score: s,
                        action: RunnerAction::Launch(app.clone()),
                    });
                }
            }
        }
        self.results.sort_by_key(|result| Reverse(result.score));
        self.results.truncate(15);
    }

    pub fn confirm(&self) -> Option<RunnerAction> {
        let action = self.results.first()?.action.clone();
        match &action {
            RunnerAction::Shell(command) => {
                let _ = Command::new("sh").arg("-c").arg(command).spawn();
            }
            RunnerAction::Launch(app) => {
                let _ = Command::new(app).spawn();
            }
            RunnerAction::Calculator(_)
            | RunnerAction::SetWindowMode(_)
            | RunnerAction::Desktop(_) => {}
        }
        Some(action)
    }
}

fn action_results(q: &str, matcher: &SkimMatcherV2) -> Vec<RunnerResult> {
    [
        (
            "Tile windows",
            "window mode",
            RunnerAction::SetWindowMode(WindowMode::Tiling),
        ),
        (
            "Float windows",
            "window mode",
            RunnerAction::SetWindowMode(WindowMode::Floating),
        ),
        (
            "Lock",
            "os action",
            RunnerAction::Desktop(DesktopAction::Lock),
        ),
        (
            "Logout",
            "os action",
            RunnerAction::Desktop(DesktopAction::Logout),
        ),
        (
            "Reboot",
            "os action",
            RunnerAction::Desktop(DesktopAction::Reboot),
        ),
        (
            "Shutdown",
            "os action",
            RunnerAction::Desktop(DesktopAction::Shutdown),
        ),
        (
            "Suspend",
            "os action",
            RunnerAction::Desktop(DesktopAction::Suspend),
        ),
        (
            "Terminal",
            "desktop action",
            RunnerAction::Desktop(DesktopAction::Terminal),
        ),
        (
            "Apps",
            "desktop action",
            RunnerAction::Desktop(DesktopAction::Apps),
        ),
        (
            "Wi-Fi",
            "desktop action",
            RunnerAction::Desktop(DesktopAction::Wifi),
        ),
        (
            "Audio",
            "desktop action",
            RunnerAction::Desktop(DesktopAction::Audio),
        ),
        (
            "Clipboard",
            "desktop action",
            RunnerAction::Desktop(DesktopAction::Clipboard),
        ),
        (
            "Notifications",
            "desktop action",
            RunnerAction::Desktop(DesktopAction::Notifications),
        ),
    ]
    .into_iter()
    .filter_map(move |(title, subtitle, action)| {
        let score = if title.eq_ignore_ascii_case(q) {
            Some(i64::MAX - 1)
        } else {
            matcher.fuzzy_match(title, q)
        };
        score.map(|score| RunnerResult {
            title: title.to_string(),
            subtitle: subtitle.to_string(),
            score,
            action,
        })
    })
    .collect()
}

fn is_math(s: &str) -> bool {
    let t = s.trim();
    !t.is_empty()
        && t.chars()
            .all(|c| c.is_ascii_digit() || "+-*/() .".contains(c))
        && t.contains(|c: char| c.is_ascii_digit())
}

fn calc(expr: &str) -> Option<f64> {
    let mut child = Command::new("bc")
        .arg("-ql")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .ok()?;
    child.stdin.as_mut()?.write_all(expr.as_bytes()).ok()?;
    let o = child.wait_with_output().ok()?;
    String::from_utf8_lossy(&o.stdout).trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_should_return_shell_action_when_query_starts_with_prompt() {
        let mut runner = Runner::new();
        runner.query = "> echo ok".to_string();
        runner.update();

        assert_eq!(
            runner.results.first().map(|result| &result.action),
            Some(&RunnerAction::Shell("echo ok".to_string()))
        );
    }

    #[test]
    fn update_should_return_tiling_action_for_window_mode_query() {
        let mut runner = Runner::new();
        runner.query = "tile".to_string();
        runner.update();

        assert!(runner
            .results
            .iter()
            .any(|result| { result.action == RunnerAction::SetWindowMode(WindowMode::Tiling) }));
    }

    #[test]
    fn update_should_return_os_actions() {
        let mut runner = Runner::new();
        runner.query = "lock".to_string();
        runner.update();

        assert_eq!(
            runner.results.first().map(|result| &result.action),
            Some(&RunnerAction::Desktop(DesktopAction::Lock))
        );
    }
}
