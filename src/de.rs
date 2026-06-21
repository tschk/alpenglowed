use std::process::Command;
use wayland_client::Connection;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopAction {
    Lock,
    Logout,
    Reboot,
    Shutdown,
    Suspend,
    Terminal,
    Apps,
    Wifi,
    Audio,
    Clipboard,
    Notifications,
}

impl DesktopAction {
    pub fn command(&self) -> Option<(&'static str, &'static [&'static str])> {
        match self {
            Self::Lock => Some(("loginctl", &["lock-session"])),
            Self::Logout => Some(("loginctl", &["terminate-user", "$USER"])),
            Self::Reboot => Some(("systemctl", &["reboot"])),
            Self::Shutdown => Some(("systemctl", &["poweroff"])),
            Self::Suspend => Some(("systemctl", &["suspend"])),
            Self::Terminal => Some(("foot", &[])),
            Self::Apps | Self::Wifi | Self::Audio | Self::Clipboard | Self::Notifications => None,
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
    let Some((program, args)) = action.command() else {
        return;
    };
    let args = args.iter().map(|arg| {
        if *arg == "$USER" {
            std::env::var("USER").unwrap_or_default()
        } else {
            (*arg).to_string()
        }
    });
    let _ = Command::new(program).args(args).spawn();
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
}
