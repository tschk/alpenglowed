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
    matcher: SkimMatcherV2,
    plugins: PluginRegistry,
}

impl Runner {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            matcher: SkimMatcherV2::default(),
            plugins: PluginRegistry::new(),
        }
    }

    pub fn update(&mut self) {
        self.results = self.plugins.query(self.query.trim(), &self.matcher);
    }

    pub fn confirm(&self) -> Option<PluginAction> {
        Some(self.results.first()?.action.clone())
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
}
