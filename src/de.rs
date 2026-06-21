use std::process::Command;
use wayland_client::Connection;

#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub fn commands(&self) -> &'static [(&'static str, &'static [&'static str])] {
        match self {
            Self::Lock => &[("loginctl", &["lock-session"])],
            Self::Logout => &[("loginctl", &["terminate-user", "$USER"])],
            Self::Reboot => &[("loginctl", &["reboot"]), ("reboot", &[])],
            Self::Shutdown => &[("loginctl", &["poweroff"]), ("poweroff", &[])],
            Self::Suspend => &[("loginctl", &["suspend"]), ("zzz", &[])],
            Self::Hibernate => &[("loginctl", &["hibernate"])],
            Self::Terminal => &[("foot", &[]), ("alacritty", &[]), ("xterm", &[])],
            Self::Apps => &[("alpenglowed", &["--polybar"])],
            Self::Wifi => &[("iwctl", &[]), ("nmtui", &[])],
            Self::WifiOn => &[(
                "iwctl",
                &["adapter", "phy0", "set-property", "Powered", "on"],
            )],
            Self::WifiOff => &[(
                "iwctl",
                &["adapter", "phy0", "set-property", "Powered", "off"],
            )],
            Self::Audio => &[("alsamixer", &[]), ("pavucontrol", &[])],
            Self::AudioMute => &[("wpctl", &["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])],
            Self::AudioUp => &[("wpctl", &["set-volume", "@DEFAULT_AUDIO_SINK@", "5%+"])],
            Self::AudioDown => &[("wpctl", &["set-volume", "@DEFAULT_AUDIO_SINK@", "5%-"])],
            Self::Display => &[("wlr-randr", &[]), ("arandr", &[])],
            Self::Screenshot => &[("grim", &["$HOME/Pictures/alpenglow-screenshot.png"])],
            Self::Clipboard => &[("cliphist", &["list"])],
            Self::Notifications => &[("makoctl", &["mode", "-t", "do-not-disturb"])],
            Self::Processes => &[("top", &[])],
            Self::Files => &[("nnn", &[]), ("vifm", &[])],
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

pub fn run(action: &DesktopAction) {
    for (program, args) in action.commands() {
        if !available(program) {
            continue;
        }
        let args = args.iter().map(expand_arg);
        let _ = Command::new(program).args(args).spawn();
        return;
    }
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
}
