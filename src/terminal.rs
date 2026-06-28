// ponytail: pipe-based console, no PTY yet — proper TTY support needs forkpty (Linux-only)
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_escape = false;
    for ch in s.chars() {
        if in_escape {
            if ch == 'm' || ch.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else if ch == '\u{1b}' || ch == '\u{9b}' {
            in_escape = true;
        } else {
            out.push(ch);
        }
    }
    out
}
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

pub struct TerminalConsole {
    child: Child,
    output: Arc<std::sync::Mutex<VecDeque<String>>>,
    input_tx: mpsc::Sender<String>,
    running: Arc<AtomicBool>,
}

impl TerminalConsole {
    pub fn spawn() -> Option<Self> {
        let mut child = Command::new("sh")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .ok()?;

        let stdout = child.stdout.take()?;
        let stderr = child.stderr.take()?;
        let stdin = child.stdin.take()?;
        let output = Arc::new(std::sync::Mutex::new(VecDeque::new()));
        let running = Arc::new(AtomicBool::new(true));
        let (input_tx, input_rx) = mpsc::channel::<String>();

        // Reader thread: merge stdout + stderr
        let out = output.clone();
        let run = running.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if !run.load(Ordering::Relaxed) {
                    break;
                }
                if let Ok(line) = line {
                    if let Ok(mut g) = out.lock() {
                        let clean = strip_ansi(&line);
                        if g.len() >= 1000 {
                            g.pop_front();
                        }
                        g.push_back(clean);
                    }
                }
            }
        });

        let out2 = output.clone();
        let run2 = running.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if !run2.load(Ordering::Relaxed) {
                    break;
                }
                if let Ok(line) = line {
                    if let Ok(mut g) = out2.lock() {
                        let clean = strip_ansi(&line);
                        if g.len() >= 1000 {
                            g.pop_front();
                        }
                        g.push_back(format!("err: {clean}"));
                    }
                }
            }
        });

        // Writer thread: send stdin to shell
        let run3 = running.clone();
        thread::spawn(move || {
            let mut stdin = stdin;
            while run3.load(Ordering::Relaxed) {
                if let Ok(line) = input_rx.recv() {
                    if writeln!(stdin, "{line}").is_err() {
                        break;
                    }
                } else {
                    break;
                }
            }
        });

        Some(TerminalConsole {
            child,
            output,
            input_tx,
            running,
        })
    }

    pub fn write(&self, line: &str) {
        let _ = self.input_tx.send(line.to_string());
    }

    #[allow(dead_code)]
    pub fn is_alive(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn output(&self) -> Vec<String> {
        self.output
            .lock()
            .ok()
            .map(|g| g.iter().cloned().collect())
            .unwrap_or_default()
    }

    #[allow(dead_code)]
    pub fn clear(&self) {
        if let Ok(mut g) = self.output.lock() {
            g.clear();
        }
    }
}

impl Drop for TerminalConsole {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
