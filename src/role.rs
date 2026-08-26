use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionRole {
    Potato,
    Desktop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoleError {
    Rejected { name: String },
}

impl SessionRole {
    pub fn label(self) -> &'static str {
        match self {
            Self::Potato => "potato",
            Self::Desktop => "desktop",
        }
    }

    pub fn skinny(self) -> bool {
        matches!(self, Self::Potato)
    }

    pub fn extras(self) -> bool {
        matches!(self, Self::Desktop) && cfg!(feature = "full")
    }

    pub fn parse(name: &str) -> Result<Self, RoleError> {
        match name.trim().to_ascii_lowercase().as_str() {
            "potato" | "potatoes" => Ok(Self::Potato),
            "desktop" => Ok(Self::Desktop),
            other => Err(RoleError::Rejected {
                name: other.to_string(),
            }),
        }
    }

    pub fn resolve() -> Result<Self, RoleError> {
        if let Ok(value) = std::env::var("ALPENGLOWED_ROLE") {
            let value = value.trim();
            if !value.is_empty() {
                return Self::parse(value);
            }
        }
        if let Some(value) = read_trimmed("/run/alpenglow/role") {
            return Self::parse(&value);
        }
        Ok(Self::Desktop)
    }
}

impl RoleError {
    pub fn message(&self) -> String {
        match self {
            Self::Rejected { name } => {
                format!("unknown role '{name}'; expected potato or desktop")
            }
        }
    }
}

pub fn session_contract() -> serde_json::Value {
    serde_json::json!({
        "name": "alpenglowed",
        "roles": ["potato", "desktop"],
        "ALPENGLOWED_ROLE": "potato|desktop",
        "alias": { "potatoes": "potato" },
        "build": {
            "potato": "cargo build --release --no-default-features",
            "desktop": "cargo build --release"
        },
        "client": "cage"
    })
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
    fn parse_should_map_potatoes_to_potato() {
        assert_eq!(SessionRole::parse("potato").unwrap(), SessionRole::Potato);
        assert_eq!(SessionRole::parse("potatoes").unwrap(), SessionRole::Potato);
        assert_eq!(SessionRole::parse("desktop").unwrap(), SessionRole::Desktop);
        assert_eq!(SessionRole::Potato.label(), "potato");
    }

    #[test]
    fn parse_should_reject_foreign_roles() {
        assert!(SessionRole::parse("kiosk").is_err());
        assert!(SessionRole::parse("internet").is_err());
        assert!(SessionRole::parse("sold").is_err());
        assert!(SessionRole::parse("workstation").is_err());
    }

    #[test]
    fn potato_is_skinny_without_extras() {
        assert!(SessionRole::Potato.skinny());
        assert!(!SessionRole::Potato.extras());
        assert!(!SessionRole::Desktop.skinny());
        assert_eq!(SessionRole::Desktop.extras(), cfg!(feature = "full"));
    }

    #[test]
    fn session_contract_should_be_short() {
        let contract = session_contract();
        assert_eq!(contract["roles"], serde_json::json!(["potato", "desktop"]));
        assert_eq!(contract["ALPENGLOWED_ROLE"], "potato|desktop");
        assert_eq!(contract["alias"]["potatoes"], "potato");
        assert_eq!(
            contract["build"]["potato"],
            "cargo build --release --no-default-features"
        );
        assert_eq!(contract["build"]["desktop"], "cargo build --release");
        assert_eq!(contract["client"], "cage");
    }
}
