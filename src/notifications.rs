use serde::Deserialize;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Deserialize)]
pub struct Notification {
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub urgency: String,
}

pub struct NotificationDaemon {
    pub receiver: mpsc::Receiver<Notification>,
}

impl NotificationDaemon {
    pub fn start() -> Self {
        let (tx, rx) = mpsc::channel();
        if let Some(path) = socket_path() {
            let _ = std::fs::remove_file(&path);
            std::thread::spawn(move || {
                let listener = match UnixListener::bind(&path) {
                    Ok(l) => l,
                    Err(e) => {
                        eprintln!("notifications: socket bind failed: {e}");
                        return;
                    }
                };
                let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o666));
                for stream in listener.incoming() {
                    match stream {
                        Ok(mut stream) => {
                            let mut buf = Vec::new();
                            if stream.read_to_end(&mut buf).is_ok() && !buf.is_empty() {
                                if let Ok(notif) = serde_json::from_slice::<Notification>(&buf) {
                                    let _ = tx.send(notif);
                                }
                            }
                        }
                        Err(e) => eprintln!("notifications: accept: {e}"),
                    }
                }
            });
            NotificationDaemon { receiver: rx }
        } else {
            NotificationDaemon { receiver: rx }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActiveNotification {
    pub notification: Notification,
    pub received_at: Instant,
}

pub struct NotificationState {
    pub active: Vec<ActiveNotification>,
    daemon: Option<NotificationDaemon>,
}

impl NotificationState {
    pub fn new() -> Self {
        Self {
            active: Vec::new(),
            daemon: None,
        }
    }

    pub fn start(&mut self) {
        self.daemon = Some(NotificationDaemon::start());
    }

    pub fn poll(&mut self) {
        let Some(ref daemon) = self.daemon else {
            return;
        };
        let now = Instant::now();
        self.active
            .retain(|n| now.duration_since(n.received_at) < Duration::from_secs(6));
        while let Ok(notif) = daemon.receiver.try_recv() {
            self.active.push(ActiveNotification {
                notification: notif,
                received_at: now,
            });
        }
        // show at most 3
        if self.active.len() > 3 {
            self.active.sort_by_key(|n| n.received_at);
            self.active.drain(..self.active.len() - 3);
        }
    }

    #[allow(dead_code)]
    pub fn dismiss(&mut self, index: usize) {
        if index < self.active.len() {
            self.active.remove(index);
        }
    }
}

fn socket_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let dir = base.join("alpenglowed");
    let _ = std::fs::create_dir_all(&dir);
    Some(dir.join("notifications"))
}
