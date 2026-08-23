use crate::plugin::{PluginAction, PluginRegistry, PluginResult, WindowTarget};
use fuzzy_matcher::skim::SkimMatcherV2;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowMode {
    Tiling,
    Floating,
    Monocle,
    Stack,
    Center,
    Grid,
}

impl WindowMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Tiling => "tiling",
            Self::Floating => "floating",
            Self::Monocle => "monocle",
            Self::Stack => "stack",
            Self::Center => "center",
            Self::Grid => "grid",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Tiling,
            Self::Floating,
            Self::Monocle,
            Self::Stack,
            Self::Center,
            Self::Grid,
        ]
    }

    pub fn next(self) -> Self {
        let modes = Self::all();
        let index = modes.iter().position(|mode| *mode == self).unwrap_or(0);
        modes[(index + 1) % modes.len()]
    }

    pub fn from_label(label: &str) -> Option<Self> {
        Self::all()
            .iter()
            .copied()
            .find(|mode| mode.label() == label)
    }
}

const RECENT_ACTION_LIMIT: usize = 16;

pub struct Runner {
    pub query: String,
    pub results: Vec<PluginResult>,
    pub selected: usize,
    matcher: SkimMatcherV2,
    plugins: PluginRegistry,
    recent_titles: HashMap<String, u32>,
}

impl Runner {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            selected: 0,
            matcher: SkimMatcherV2::default(),
            plugins: PluginRegistry::new(),
            recent_titles: HashMap::new(),
        }
    }

    pub fn with_role(role: crate::role::SessionRole) -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            selected: 0,
            matcher: SkimMatcherV2::default(),
            plugins: PluginRegistry::with_role(role),
            recent_titles: HashMap::new(),
        }
    }

    pub fn update_with_windows(&mut self, windows: &[WindowTarget]) {
        let query = self.query.trim();
        if query.is_empty() {
            self.results.clear();
            self.selected = 0;
            return;
        }
        self.results = self
            .plugins
            .query_with_windows(query, &self.matcher, windows);
        for result in &mut self.results {
            if let Some(boost) = self.recent_titles.get(&result.title) {
                result.score = result.score.saturating_add(100 + *boost as i64);
            }
        }
        self.results.sort_by_key(|result| Reverse(result.score));
        self.results.truncate(8);
        if self.results.is_empty() {
            self.selected = 0;
        } else {
            self.selected = self.selected.min(self.results.len() - 1);
        }
    }

    pub fn record_recent(&mut self, title: &str) {
        let count = self.recent_titles.entry(title.to_string()).or_insert(0);
        *count += 1;
        if self.recent_titles.len() > RECENT_ACTION_LIMIT {
            let threshold = self.recent_titles.values().copied().min().unwrap_or(0);
            self.recent_titles.retain(|_, count| *count > threshold);
        }
    }

    pub fn confirm(&self) -> Option<PluginAction> {
        Some(self.results.get(self.selected)?.action.clone())
    }

    pub fn selected_result(&self) -> Option<&PluginResult> {
        self.results.get(self.selected)
    }

    pub fn selection_label(&self) -> String {
        if self.results.is_empty() {
            "0 results".to_string()
        } else {
            format!("{}/{}", self.selected + 1, self.results.len())
        }
    }

    pub fn select_next(&mut self) {
        if self.results.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.results.len();
    }

    pub fn select_previous(&mut self) {
        if self.results.is_empty() {
            return;
        }
        self.selected = if self.selected == 0 {
            self.results.len() - 1
        } else {
            self.selected - 1
        };
    }

    pub fn select(&mut self, index: usize) {
        if self.results.is_empty() {
            self.selected = 0;
        } else {
            self.selected = index.min(self.results.len() - 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::de::DesktopAction;
    use crate::plugin::WindowTarget;

    #[test]
    fn update_should_return_shell_action_when_query_starts_with_prompt() {
        let mut runner = Runner::new();
        runner.query = "> echo ok".to_string();
        runner.update_with_windows(&[]);

        assert_eq!(
            runner.results.first().map(|result| &result.action),
            Some(&PluginAction::Shell {
                command: "echo ok".to_string()
            })
        );
    }

    #[test]
    fn update_should_return_tiling_action_for_window_mode_query() {
        let mut runner = Runner::new();
        runner.query = "tile".to_string();
        runner.update_with_windows(&[]);

        assert!(runner.results.iter().any(|result| {
            result.action
                == PluginAction::SetWindowMode {
                    mode: WindowMode::Tiling,
                }
        }));
    }

    #[test]
    fn update_should_return_layout_action_for_split_query() {
        let mut runner = Runner::new();
        runner.query = "split row".to_string();
        runner.update_with_windows(&[]);

        assert!(runner.results.iter().any(|result| {
            matches!(
                result.action,
                PluginAction::Layout {
                    action: crate::layout::LayoutAction::SplitRow
                }
            )
        }));
    }

    #[test]
    fn update_should_return_layout_action_for_flip_axis_query() {
        let mut runner = Runner::new();
        runner.query = "flip axis".to_string();
        runner.update_with_windows(&[]);

        assert!(runner.results.iter().any(|result| {
            matches!(
                result.action,
                PluginAction::Layout {
                    action: crate::layout::LayoutAction::FlipAxis
                }
            )
        }));
    }

    #[test]
    fn update_should_return_focus_window_action_for_named_pane() {
        let mut runner = Runner::new();
        runner.query = "focus workspace".to_string();
        runner.update_with_windows(&[WindowTarget {
            id: 1,
            title: "Workspace".to_string(),
            focused: false,
            floating: false,
        }]);

        assert!(runner
            .results
            .iter()
            .any(|result| { result.action == PluginAction::FocusWindow { id: 1 } }));
    }

    #[test]
    fn update_should_rank_window_targets_for_plain_focus_query() {
        let mut runner = Runner::new();
        runner.query = "focus".to_string();
        runner.update_with_windows(&[
            WindowTarget {
                id: 1,
                title: "Workspace".to_string(),
                focused: true,
                floating: false,
            },
            WindowTarget {
                id: 2,
                title: "Scratch".to_string(),
                focused: false,
                floating: false,
            },
        ]);

        assert!(matches!(
            runner.results.first().map(|result| &result.action),
            Some(PluginAction::FocusWindow { .. })
        ));
    }

    #[test]
    fn update_should_return_os_actions() {
        let mut runner = Runner::new();
        runner.query = "lock".to_string();
        runner.update_with_windows(&[]);

        assert_eq!(
            runner.results.first().map(|result| &result.action),
            Some(&PluginAction::Desktop {
                action: DesktopAction::Lock
            })
        );
    }

    #[test]
    fn update_should_return_settings_action() {
        let mut runner = Runner::new();
        runner.query = "settings".to_string();
        runner.update_with_windows(&[]);

        assert!(runner
            .results
            .iter()
            .any(|result| result.action == PluginAction::ToggleSettings));
    }

    #[test]
    fn update_should_return_status_bar_action() {
        let mut runner = Runner::new();
        runner.query = "status".to_string();
        runner.update_with_windows(&[]);

        assert!(runner
            .results
            .iter()
            .any(|result| result.action == PluginAction::ToggleStatusBar));
    }

    #[test]
    fn update_should_return_open_settings_action() {
        let mut runner = Runner::new();
        runner.query = "open settings".to_string();
        runner.update_with_windows(&[]);

        assert!(runner
            .results
            .iter()
            .any(|result| result.action == PluginAction::OpenSettings));
    }

    #[test]
    fn update_should_return_show_status_bar_action() {
        let mut runner = Runner::new();
        runner.query = "show status bar".to_string();
        runner.update_with_windows(&[]);

        assert!(runner
            .results
            .iter()
            .any(|result| result.action == PluginAction::ShowStatusBar));
    }

    #[test]
    fn update_should_return_close_settings_action() {
        let mut runner = Runner::new();
        runner.query = "close settings".to_string();
        runner.update_with_windows(&[]);

        assert!(runner
            .results
            .iter()
            .any(|result| result.action == PluginAction::CloseSettings));
    }

    #[test]
    fn selection_should_wrap() {
        let mut runner = Runner::new();
        runner.query = "window".to_string();
        runner.update_with_windows(&[]);
        let len = runner.results.len();
        runner.select_previous();
        assert_eq!(runner.selected, len - 1);
        runner.select_next();
        assert_eq!(runner.selected, 0);
    }

    #[test]
    fn selection_label_should_report_empty_state() {
        let runner = Runner::new();
        assert_eq!(runner.selection_label(), "0 results");
    }

    #[test]
    fn update_should_clear_results_when_query_empty() {
        let mut runner = Runner::new();
        runner.query = "window".to_string();
        runner.update_with_windows(&[]);
        assert!(!runner.results.is_empty());

        runner.query.clear();
        runner.update_with_windows(&[]);

        assert!(runner.results.is_empty());
    }

    #[test]
    fn selected_result_should_follow_selection() {
        let mut runner = Runner::new();
        runner.query = "window".to_string();
        runner.update_with_windows(&[]);
        let first = runner.selected_result().map(|result| result.title.clone());
        runner.select_next();
        let second = runner.selected_result().map(|result| result.title.clone());
        assert_ne!(first, second);
    }

    #[test]
    fn select_should_clamp_to_available_results() {
        let mut runner = Runner::new();
        runner.query = "window".to_string();
        runner.update_with_windows(&[]);
        runner.select(999);
        assert_eq!(runner.selected, runner.results.len() - 1);
    }

    #[test]
    fn update_should_limit_results_to_eight() {
        let mut runner = Runner::new();
        runner.query = "o".to_string();
        runner.update_with_windows(&[]);
        assert!(runner.results.len() <= 8);
    }

    #[test]
    fn record_recent_should_boost_previously_confirmed_titles() {
        let mut runner = Runner::new();
        runner.query = "tile".to_string();
        runner.update_with_windows(&[]);
        let tile_result = runner
            .results
            .iter()
            .find(|r| matches!(r.action, PluginAction::SetWindowMode { .. }))
            .map(|r| r.title.clone());
        if let Some(title) = tile_result {
            runner.record_recent(&title);
            runner.update_with_windows(&[]);
            assert!(runner
                .results
                .iter()
                .any(|r| r.title == title && r.score > 100));
        }
    }

    #[test]
    fn record_recent_should_evict_least_frequent() {
        let mut runner = Runner::new();
        for i in 0..20 {
            runner.record_recent(&format!("action-{i}"));
        }
        runner.record_recent("action-5");
        assert!(runner.recent_titles.len() <= 16);
    }
}
