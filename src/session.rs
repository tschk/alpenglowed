use crate::de::DesktopAction;
use crate::layout::LayoutAction;
use crate::runner::WindowMode;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionRequest {
    SetWindowMode { mode: WindowMode },
    CycleWindowMode,
    FocusWindow { id: usize },
    Layout { action: LayoutAction },
    DesktopAction { action: DesktopAction },
}

/// Sends `request` to the compositor and waits for its verdict. The
/// compositor answers `ok` only when it actually handled the request, so a
/// request it does not implement comes back as an error instead of a
/// silent success.
pub fn dispatch(request: &SessionRequest) -> Result<(), String> {
    let Some(path) = std::env::var_os("ALPENGLOW_SESSION_CONTROL") else {
        return Err("session control unavailable".to_string());
    };
    let mut stream = UnixStream::connect(&path).map_err(|error| error.to_string())?;
    serde_json::to_writer(&mut stream, request).map_err(|error| error.to_string())?;
    stream.write_all(b"\n").map_err(|error| error.to_string())?;
    stream
        .shutdown(Shutdown::Write)
        .map_err(|error| error.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;
    let mut reply = String::new();
    stream
        .read_to_string(&mut reply)
        .map_err(|error| format!("no reply from compositor: {error}"))?;
    match reply.trim() {
        "ok" => Ok(()),
        "" => Err("compositor sent no reply".to_string()),
        other => Err(other.strip_prefix("error: ").unwrap_or(other).to_string()),
    }
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
    fn dispatch_should_succeed_only_when_compositor_confirms() {
        use std::os::unix::net::UnixListener;

        let path = std::env::temp_dir().join(format!(
            "alpenglowed-session-test-{}.sock",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).unwrap();
        std::env::set_var("ALPENGLOW_SESSION_CONTROL", &path);
        let serve = |reply: &'static str| {
            let listener = listener.try_clone().unwrap();
            std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = String::new();
                stream.read_to_string(&mut request).unwrap();
                let _ = stream.write_all(reply.as_bytes());
                request
            })
        };
        let lock = SessionRequest::DesktopAction {
            action: DesktopAction::Lock,
        };

        // A compositor that reads the request and replies nothing must not
        // count as having handled it.
        let server = serve("");
        assert!(dispatch(&lock).is_err());
        assert!(server.join().unwrap().contains(r#""action":"lock""#));

        let server = serve("error: alpenglowed-comp does not implement desktop action `lock`\n");
        assert_eq!(
            dispatch(&lock),
            Err("alpenglowed-comp does not implement desktop action `lock`".to_string())
        );
        server.join().unwrap();

        let server = serve("ok\n");
        assert_eq!(dispatch(&SessionRequest::CycleWindowMode), Ok(()));
        server.join().unwrap();

        std::env::remove_var("ALPENGLOW_SESSION_CONTROL");
        let _ = std::fs::remove_file(&path);
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
