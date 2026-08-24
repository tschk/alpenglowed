use serde::{Deserialize, Serialize};

const SYSTEM_DEFAULTS: &str = "/usr/share/defaults/alpenglowed/config.toml";
const USER_CONFIG: &str = "/etc/alpenglowed/config.toml";
const FACTORY_RESET_MARKER: &str = "/var/lib/alpenglow/.factory-reset";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub status_bar: Option<bool>,
    pub external_polybar: Option<bool>,
    pub open_settings: Option<bool>,
    pub initial_query: Option<String>,
    pub mode: Option<String>,
    pub demo_layout: Option<bool>,
}

impl Config {
    pub fn load() -> Self {
        Self::load_from_path(USER_CONFIG)
            .or_else(|_| Self::load_from_path(SYSTEM_DEFAULTS))
            .unwrap_or_default()
    }

    fn load_from_path(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }

    pub fn factory_reset() {
        let _ = std::fs::remove_dir_all("/etc/alpenglowed");
        let _ = std::fs::write(FACTORY_RESET_MARKER, b"");
    }
}
