use crate::de::DesktopAction;
use crate::layout::LayoutAction;
use crate::runner::WindowMode;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::os::unix::net::UnixStream;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionRequest {
    SetWindowMode { mode: WindowMode },
    CycleWindowMode,
    FocusWindow { id: usize },
    Layout { action: LayoutAction },
    DesktopAction { action: DesktopAction },
}

pub fn dispatch(request: &SessionRequest) -> Result<(), String> {
    let Some(path) = std::env::var_os("ALPENGLOW_SESSION_CONTROL") else {
        return Err("session control unavailable".to_string());
    };
    let mut stream = UnixStream::connect(&path).map_err(|error| error.to_string())?;
    serde_json::to_writer(&mut stream, request).map_err(|error| error.to_string())?;
    stream.write_all(b"\n").map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_window_mode_request_should_serialize() {
        let payload = serde_json::to_string(&SessionRequest::SetWindowMode {
            mode: WindowMode::Tiling,
        })
        .unwrap();

        assert_eq!(payload, r#"{"type":"set_window_mode","mode":"tiling"}"#);
        let parsed: SessionRequest = serde_json::from_str(&payload).unwrap();
        assert_eq!(
            parsed,
            SessionRequest::SetWindowMode {
                mode: WindowMode::Tiling
            }
        );
    }

    #[test]
    fn desktop_action_request_should_serialize() {
        let payload = serde_json::to_string(&SessionRequest::DesktopAction {
            action: DesktopAction::Lock,
        })
        .unwrap();

        assert_eq!(payload, r#"{"type":"desktop_action","action":"lock"}"#);
        let parsed: SessionRequest = serde_json::from_str(&payload).unwrap();
        assert_eq!(
            parsed,
            SessionRequest::DesktopAction {
                action: DesktopAction::Lock
            }
        );
    }

    #[test]
    fn layout_request_should_serialize() {
        let payload = serde_json::to_string(&SessionRequest::Layout {
            action: LayoutAction::SplitRow,
        })
        .unwrap();

        assert_eq!(payload, r#"{"type":"layout","action":"split_row"}"#);
        let parsed: SessionRequest = serde_json::from_str(&payload).unwrap();
        assert_eq!(
            parsed,
            SessionRequest::Layout {
                action: LayoutAction::SplitRow
            }
        );
    }
}
