use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionRole {
    Potatoes,
    Desktop,
    Workstation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleCapabilities {
    pub weather: bool,
    pub wifi_pill: bool,
    pub network_plugins: bool,
    pub command_plugins: bool,
    pub spotify: bool,
    pub fleet: bool,
    pub compositor: bool,
    pub skinny_bar: bool,
    pub default_status_bar: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoleError {
    Unsupported { name: String },
    Unknown { name: String },
}

impl SessionRole {
    pub fn all() -> &'static [Self] {
        &[Self::Potatoes, Self::Desktop, Self::Workstation]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Potatoes => "potatoes",
            Self::Desktop => "desktop",
            Self::Workstation => "workstation",
        }
    }

    pub fn parse(name: &str) -> Result<Self, RoleError> {
        match name.trim().to_ascii_lowercase().as_str() {
            "potatoes" | "potato" | "lite" => Ok(Self::Potatoes),
            "desktop" => Ok(Self::Desktop),
            "workstation" | "fleet" => Ok(Self::Workstation),
            name @ ("kiosk" | "internet" | "sold" | "embedded" | "containers" | "container") => {
                Err(RoleError::Unsupported {
                    name: name.to_string(),
                })
            }
            other if other.is_empty() => Err(RoleError::Unknown {
                name: other.to_string(),
            }),
            other => Err(RoleError::Unknown {
                name: other.to_string(),
            }),
        }
    }

    pub fn from_edition(edition: &str) -> Option<Self> {
        match edition.trim().to_ascii_lowercase().as_str() {
            "desktop" => Some(Self::Potatoes),
            "desktop-full" => Some(Self::Desktop),
            _ => None,
        }
    }

    pub fn capabilities(self) -> RoleCapabilities {
        match self {
            Self::Potatoes => RoleCapabilities {
                weather: false,
                wifi_pill: false,
                network_plugins: false,
                command_plugins: false,
                spotify: false,
                fleet: false,
                compositor: false,
                skinny_bar: true,
                default_status_bar: true,
            },
            Self::Desktop => RoleCapabilities::desktop(),
            Self::Workstation => RoleCapabilities {
                fleet: true,
                ..RoleCapabilities::desktop()
            },
        }
    }

    pub fn resolve() -> Result<Self, RoleError> {
        if let Some(value) = cli_flag("role") {
            return Self::parse(&value);
        }
        if let Some(value) = std::env::var("ALPENGLOWED_ROLE")
            .ok()
            .filter(|value| !value.trim().is_empty())
        {
            return Self::parse(&value);
        }
        if let Some(role) =
            read_trimmed("/run/alpenglow/role").and_then(|value| Self::parse(&value).ok())
        {
            return Ok(role);
        }
        if let Some(role) =
            read_trimmed("/etc/alpenglow/role").and_then(|value| Self::parse(&value).ok())
        {
            return Ok(role);
        }
        if let Some(role) = crate::config::Config::load()
            .role
            .as_deref()
            .and_then(|value| Self::parse(value).ok())
        {
            return Ok(role);
        }
        if let Some(role) = std::env::var("ALPENGLOW_EDITION")
            .ok()
            .and_then(|value| Self::from_edition(&value))
        {
            return Ok(role);
        }
        if let Some(role) =
            read_trimmed("/run/alpenglow/edition").and_then(|value| Self::from_edition(&value))
        {
            return Ok(role);
        }
        Ok(Self::Desktop)
    }
}

impl RoleCapabilities {
    pub fn desktop() -> Self {
        Self {
            weather: cfg!(feature = "full"),
            wifi_pill: true,
            network_plugins: cfg!(feature = "full"),
            command_plugins: cfg!(feature = "full"),
            spotify: cfg!(feature = "full"),
            fleet: false,
            compositor: true,
            skinny_bar: false,
            default_status_bar: false,
        }
    }
}

impl RoleError {
    pub fn message(&self) -> String {
        match self {
            Self::Unsupported { name } => format!(
                "role '{name}' is out of scope for alpenglowed; use Cage (kiosk) or sold (internet). See docs/alpenglow-session-contract.md"
            ),
            Self::Unknown { name } => {
                format!("unknown role '{name}'; expected potatoes, desktop, or workstation")
            }
        }
    }
}

pub fn session_contract() -> serde_json::Value {
    serde_json::json!({
        "name": "alpenglowed",
        "binaries": {
            "alpenglowed": {
                "path": "/usr/bin/alpenglowed",
                "build": "cargo build --release",
                "roles": ["desktop", "workstation"]
            },
            "alpenglowed-lite": {
                "path": "/usr/bin/alpenglowed-lite",
                "build": "cargo build --release --no-default-features",
                "roles": ["potatoes"]
            },
            "alpenglow-greeter": {
                "path": "/usr/bin/alpenglow-greeter",
                "build": "cargo build --release -p alpenglow-greeter"
            },
            "alpenglowed-comp": {
                "path": "/usr/bin/alpenglowed-comp",
                "build": "cargo build --release -p alpenglowed-comp",
                "status": "nested winit prototype; not the session compositor yet"
            }
        },
        "roles": {
            "potatoes": {
                "bar": "skinny",
                "plugins": "core only",
                "weather": false,
                "compositor_flag": false,
                "suggested_edition": "desktop"
            },
            "desktop": {
                "bar": "full",
                "plugins": "full",
                "weather": true,
                "compositor_flag": "experimental",
                "suggested_edition": "desktop-full"
            },
            "workstation": {
                "bar": "full",
                "plugins": "desktop plus /run/alpenglow fleet status",
                "weather": true,
                "compositor_flag": "experimental",
                "suggested_edition": "desktop-full"
            }
        },
        "out_of_scope": {
            "kiosk": "Cage + a single app; do not start alpenglowed",
            "internet": "sold from tschk/soliloquy; do not start alpenglowed",
            "embedded": "headless; no graphical session",
            "containers": "headless; no graphical session"
        },
        "start": {
            "session_wrapper": "/usr/local/bin/alpenglow-session-start",
            "flags": ["--role=potatoes|desktop|workstation", "--session-contract", "--status-bar"],
            "env": [
                "ALPENGLOWED_ROLE",
                "ALPENGLOW_EDITION",
                "ALPENGLOWED_MODE",
                "ALPENGLOWED_STATUS_BAR",
                "ALPENGLOWED_PLUGIN_DIR",
                "ALPENGLOW_SESSION_CONTROL",
                "ALPENGLOWED_INSTALLER_SOURCE",
                "ALPENGLOWED_INSTALLER_TARGET",
                "XDG_RUNTIME_DIR",
                "WAYLAND_DISPLAY"
            ],
            "dinit": {
                "depends_on": ["seatd"],
                "depends_on_desktop_full": ["pipewire", "wireplumber"],
                "compositor": "velox or cage must own the seat; alpenglowed is a client"
            }
        },
        "run_alpenglow": {
            "reads": [
                "/run/alpenglow/role",
                "/run/alpenglow/edition",
                "/run/alpenglow/alpenglow.img.zst",
                "/run/alpenglow/pressurectl/state.json",
                "/run/alpenglow/netd/interfaces.json",
                "/run/alpenglow/netd/runtime-state.env",
                "/run/alpenglow/runtime-state.env",
                "/run/alpenglow/rootfs.env"
            ],
            "writes": []
        },
        "image": {
            "required": ["seatd", "wayland", "mesa or llvmpipe", "font-dejavu"],
            "compositor": ["velox", "cage"],
            "session": ["greetd (desktop-full / login)", "elogind"],
            "optional": ["pipewire", "wireplumber", "foot", "iwd"],
            "libc": "glibc dynamic for GPUI (dlopen Wayland/Vulkan); musl static scripts exist but are not the image path"
        }
    })
}

fn cli_flag(name: &str) -> Option<String> {
    let prefix = format!("--{name}=");
    std::env::args().find_map(|arg| arg.strip_prefix(&prefix).map(ToString::to_string))
}

fn read_trimmed(path: impl AsRef<Path>) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_should_accept_role_aliases() {
        assert_eq!(SessionRole::parse("lite").unwrap(), SessionRole::Potatoes);
        assert_eq!(
            SessionRole::parse("fleet").unwrap(),
            SessionRole::Workstation
        );
        assert_eq!(SessionRole::parse("desktop").unwrap(), SessionRole::Desktop);
    }

    #[test]
    fn parse_should_reject_kiosk_and_internet() {
        assert!(matches!(
            SessionRole::parse("kiosk"),
            Err(RoleError::Unsupported { .. })
        ));
        assert!(matches!(
            SessionRole::parse("internet"),
            Err(RoleError::Unsupported { .. })
        ));
        assert!(matches!(
            SessionRole::parse("sold"),
            Err(RoleError::Unsupported { .. })
        ));
    }

    #[test]
    fn edition_desktop_maps_to_potatoes() {
        assert_eq!(
            SessionRole::from_edition("desktop"),
            Some(SessionRole::Potatoes)
        );
        assert_eq!(
            SessionRole::from_edition("desktop-full"),
            Some(SessionRole::Desktop)
        );
        assert_eq!(SessionRole::from_edition("minimal"), None);
    }

    #[test]
    fn potatoes_should_disable_weather_and_compositor() {
        let caps = SessionRole::Potatoes.capabilities();
        assert!(!caps.weather);
        assert!(!caps.compositor);
        assert!(caps.skinny_bar);
        assert!(caps.default_status_bar);
        assert!(!caps.command_plugins);
        assert!(!caps.fleet);
    }

    #[test]
    fn workstation_should_enable_fleet_on_desktop_caps() {
        let caps = SessionRole::Workstation.capabilities();
        assert!(caps.fleet);
        assert!(!caps.skinny_bar);
        assert_eq!(caps.wifi_pill, RoleCapabilities::desktop().wifi_pill);
    }

    #[test]
    fn session_contract_should_name_binaries_and_out_of_scope() {
        let contract = session_contract();
        assert_eq!(contract["name"], "alpenglowed");
        assert_eq!(
            contract["binaries"]["alpenglowed-lite"]["path"],
            "/usr/bin/alpenglowed-lite"
        );
        assert!(contract["out_of_scope"]["kiosk"]
            .as_str()
            .unwrap()
            .contains("Cage"));
        assert!(contract["out_of_scope"]["internet"]
            .as_str()
            .unwrap()
            .contains("sold"));
        assert!(contract["run_alpenglow"]["reads"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "/run/alpenglow/role"));
    }
}
