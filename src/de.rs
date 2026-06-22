use std::process::Command;
use wayland_client::Connection;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesktopAction {
    Lock,
    Logout,
    Reboot,
    Shutdown,
    Suspend,
    Hibernate,
    Terminal,
    Apps,
    Wifi,
    WifiOn,
    WifiOff,
    Audio,
    AudioMute,
    AudioUp,
    AudioDown,
    Display,
    Screenshot,
    Clipboard,
    Notifications,
    Processes,
    Files,
}

impl DesktopAction {
    pub fn all() -> &'static [Self] {
        &[
            Self::Lock,
            Self::Logout,
            Self::Reboot,
            Self::Shutdown,
            Self::Suspend,
            Self::Hibernate,
            Self::Terminal,
            Self::Apps,
            Self::Wifi,
            Self::WifiOn,
            Self::WifiOff,
            Self::Audio,
            Self::AudioMute,
            Self::AudioUp,
            Self::AudioDown,
            Self::Display,
            Self::Screenshot,
            Self::Clipboard,
            Self::Notifications,
            Self::Processes,
            Self::Files,
        ]
    }

    pub fn title(&self) -> &'static str {
        match self {
            Self::Lock => "Lock",
            Self::Logout => "Logout",
            Self::Reboot => "Reboot",
            Self::Shutdown => "Shutdown",
            Self::Suspend => "Suspend",
            Self::Hibernate => "Hibernate",
            Self::Terminal => "Terminal",
            Self::Apps => "Apps",
            Self::Wifi => "Wi-Fi",
            Self::WifiOn => "Wi-Fi On",
            Self::WifiOff => "Wi-Fi Off",
            Self::Audio => "Audio",
            Self::AudioMute => "Mute Audio",
            Self::AudioUp => "Audio Up",
            Self::AudioDown => "Audio Down",
            Self::Display => "Display",
            Self::Screenshot => "Screenshot",
            Self::Clipboard => "Clipboard",
            Self::Notifications => "Notifications",
            Self::Processes => "Processes",
            Self::Files => "Files",
        }
    }

    pub fn subtitle(&self) -> &'static str {
        match self {
            Self::Lock
            | Self::Logout
            | Self::Reboot
            | Self::Shutdown
            | Self::Suspend
            | Self::Hibernate => "os action",
            _ => "desktop action",
        }
    }

    pub fn commands(&self) -> &'static [(&'static str, &'static [&'static str])] {
        match self {
            Self::Lock => &[("loginctl", &["lock-session"])],
            Self::Logout => &[("loginctl", &["terminate-user", "$USER"])],
            Self::Reboot => &[("loginctl", &["reboot"]), ("reboot", &[])],
            Self::Shutdown => &[("loginctl", &["poweroff"]), ("poweroff", &[])],
            Self::Suspend => &[("loginctl", &["suspend"]), ("zzz", &[])],
            Self::Hibernate => &[("loginctl", &["hibernate"])],
            Self::Terminal => &[("foot", &[]), ("alacritty", &[]), ("xterm", &[])],
            Self::Apps => &[
                ("alpenglowed", &["--polybar"]),
                ("xdg-open", &["/usr/share/applications"]),
                ("gio", &["open", "/usr/share/applications"]),
            ],
            Self::Wifi => &[("iwctl", &[]), ("nmtui", &[])],
            Self::WifiOn => &[
                (
                    "iwctl",
                    &["adapter", "phy0", "set-property", "Powered", "on"],
                ),
                ("rfkill", &["unblock", "wifi"]),
            ],
            Self::WifiOff => &[
                (
                    "iwctl",
                    &["adapter", "phy0", "set-property", "Powered", "off"],
                ),
                ("rfkill", &["block", "wifi"]),
            ],
            Self::Audio => &[
                ("wpctl", &["status"]),
                ("pactl", &["info"]),
                ("alsamixer", &[]),
                ("pavucontrol", &[]),
            ],
            Self::AudioMute => &[("wpctl", &["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])],
            Self::AudioUp => &[("wpctl", &["set-volume", "@DEFAULT_AUDIO_SINK@", "5%+"])],
            Self::AudioDown => &[("wpctl", &["set-volume", "@DEFAULT_AUDIO_SINK@", "5%-"])],
            Self::Display => &[("wlr-randr", &[]), ("arandr", &[])],
            Self::Screenshot => &[("grim", &["$HOME/Pictures/alpenglow-screenshot.png"])],
            Self::Clipboard => &[("cliphist", &["list"])],
            Self::Notifications => &[
                ("makoctl", &["mode", "-t", "do-not-disturb"]),
                ("notify-send", &["alpenglowed", "notifications check"]),
            ],
            Self::Processes => &[("top", &[])],
            Self::Files => &[
                ("nnn", &[]),
                ("vifm", &[]),
                ("xdg-open", &["$HOME"]),
                ("gio", &["open", "$HOME"]),
            ],
        }
    }

    pub fn resolve(&self) -> Option<ResolvedCommand> {
        self.resolve_with(available)
    }

    pub fn resolve_with(&self, available: impl Fn(&str) -> bool) -> Option<ResolvedCommand> {
        self.commands()
            .iter()
            .find(|(program, _)| available(program))
            .map(|(program, args)| ResolvedCommand {
                program: (*program).to_string(),
                args: args.iter().map(expand_arg).collect(),
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCommand {
    pub program: String,
    pub args: Vec<String>,
}

impl ResolvedCommand {
    pub fn display(&self) -> String {
        if self.args.is_empty() {
            self.program.clone()
        } else {
            format!("{} {}", self.program, self.args.join(" "))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopState {
    pub mode: &'static str,
    pub wayland: bool,
    pub display: Option<String>,
}

impl DesktopState {
    pub fn detect(mode: &'static str) -> Self {
        Self {
            mode,
            wayland: Connection::connect_to_env().is_ok(),
            display: std::env::var("WAYLAND_DISPLAY").ok(),
        }
    }

    pub fn polybar(&self) -> String {
        let display = self.display.as_deref().unwrap_or("no-display");
        let wayland = if self.wayland {
            "wayland"
        } else {
            "no-wayland"
        };
        format!("alpenglowed {} {} {}", self.mode, wayland, display)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunResult {
    Spawned(ResolvedCommand),
    MissingCommand,
}

pub fn run(action: &DesktopAction) -> RunResult {
    let Some(command) = action.resolve() else {
        return RunResult::MissingCommand;
    };
    let _ = Command::new(&command.program).args(&command.args).spawn();
    RunResult::Spawned(command)
}

pub fn probe_actions() -> Vec<String> {
    DesktopAction::all()
        .iter()
        .map(|action| {
            let resolved = action
                .resolve()
                .map(|command| command.display())
                .unwrap_or_else(|| "unavailable".to_string());
            format!("{}\t{}\t{}", action.title(), action.subtitle(), resolved)
        })
        .collect()
}

pub fn smoke_wayland() -> Result<(), String> {
    std::env::var("WAYLAND_DISPLAY").map_err(|_| "WAYLAND_DISPLAY is not set".to_string())?;
    Connection::connect_to_env()
        .map(|_| ())
        .map_err(|error| format!("wayland connection failed: {error}"))
}

fn available(program: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| dir.join(program).is_file())
}

fn expand_arg(arg: &&'static str) -> String {
    if *arg == "$USER" {
        std::env::var("USER").unwrap_or_default()
    } else if let Some(rest) = arg.strip_prefix("$HOME/") {
        std::env::var("HOME")
            .map(|home| format!("{home}/{rest}"))
            .unwrap_or_else(|_| (*arg).to_string())
    } else {
        (*arg).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polybar_should_include_mode_and_display_state() {
        let state = DesktopState {
            mode: "tiling",
            wayland: false,
            display: Some("wayland-1".to_string()),
        };

        assert_eq!(state.polybar(), "alpenglowed tiling no-wayland wayland-1");
    }

    #[test]
    fn desktop_actions_should_offer_alpenglow_first_fallbacks() {
        assert_eq!(DesktopAction::Reboot.commands()[0].0, "loginctl");
        assert_eq!(DesktopAction::Reboot.commands()[1].0, "reboot");
        assert_eq!(DesktopAction::Terminal.commands()[0].0, "foot");
    }

    #[test]
    fn every_desktop_action_should_resolve_with_first_available_command() {
        for action in DesktopAction::all() {
            let first = action.commands()[0].0;
            assert_eq!(
                action.resolve_with(|program| program == first),
                Some(ResolvedCommand {
                    program: first.to_string(),
                    args: action.commands()[0].1.iter().map(expand_arg).collect(),
                })
            );
        }
    }

    #[test]
    fn desktop_action_should_report_missing_commands_without_spawning() {
        assert_eq!(DesktopAction::Shutdown.resolve_with(|_| false), None);
    }

    #[test]
    fn probe_actions_should_emit_one_line_per_action() {
        let lines = probe_actions();
        assert_eq!(lines.len(), DesktopAction::all().len());
        assert!(lines.iter().all(|line| line.split('\t').count() == 3));
    }
}
