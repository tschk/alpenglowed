use crate::plugin::{PluginAction, PluginRegistry, PluginResult};
use fuzzy_matcher::skim::SkimMatcherV2;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

pub struct Runner {
    pub query: String,
    pub results: Vec<PluginResult>,
    pub selected: usize,
    matcher: SkimMatcherV2,
    plugins: PluginRegistry,
}

impl Runner {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            selected: 0,
            matcher: SkimMatcherV2::default(),
            plugins: PluginRegistry::new(),
        }
    }

    pub fn update(&mut self) {
        self.results = self.plugins.query(self.query.trim(), &self.matcher);
        if self.results.is_empty() {
            self.selected = 0;
        } else {
            self.selected = self.selected.min(self.results.len() - 1);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::de::DesktopAction;

    #[test]
    fn update_should_return_shell_action_when_query_starts_with_prompt() {
        let mut runner = Runner::new();
        runner.query = "> echo ok".to_string();
        runner.update();

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
        runner.update();

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
        runner.update();

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
    fn update_should_return_os_actions() {
        let mut runner = Runner::new();
        runner.query = "lock".to_string();
        runner.update();

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
        runner.update();

        assert!(runner
            .results
            .iter()
            .any(|result| result.action == PluginAction::OpenSettings));
    }

    #[test]
    fn selection_should_wrap() {
        let mut runner = Runner::new();
        runner.query = "window".to_string();
        runner.update();
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
    fn selected_result_should_follow_selection() {
        let mut runner = Runner::new();
        runner.query = "window".to_string();
        runner.update();
        let first = runner.selected_result().map(|result| result.title.clone());
        runner.select_next();
        let second = runner.selected_result().map(|result| result.title.clone());
        assert_ne!(first, second);
    }
}
