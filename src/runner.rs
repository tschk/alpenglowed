// Alpenglowed Runner — fuzzy app launcher + shell runner + calculator

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::process::Command;
use std::sync::OnceLock;

static APP_CACHE: OnceLock<Vec<String>> = OnceLock::new();

fn apps() -> &'static Vec<String> {
    APP_CACHE.get_or_init(|| {
        let mut a = Vec::new();
        if let Ok(p) = std::env::var("PATH") {
            for d in p.split(':') {
                if let Ok(e) = std::fs::read_dir(d) {
                    for entry in e.flatten() {
                        if let Some(n) = entry.file_name().to_str() {
                            if !n.starts_with('.') {
                                a.push(n.to_owned());
                            }
                        }
                    }
                }
            }
        }
        a.sort();
        a.dedup();
        a
    })
}

pub struct Runner {
    pub query: String,
    pub results: Vec<(String, i64)>,
    matcher: SkimMatcherV2,
}

impl Runner {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            matcher: SkimMatcherV2::default(),
        }
    }

    pub fn update(&mut self) {
        self.results.clear();
        let q = self.query.trim();

        // Shell
        if q.starts_with('>') {
            let cmd = q[1..].trim();
            if !cmd.is_empty() {
                self.results.push((format!("▶ {}", cmd), i64::MAX));
            }
            return;
        }

        // Calculator
        if is_math(q) {
            if let Some(v) = calc(q) {
                self.results.push((format!("= {}", v), i64::MAX));
            }
        }

        // Fuzzy apps
        for app in apps() {
            if let Some(s) = self.matcher.fuzzy_match(app, q) {
                if s > 0 {
                    self.results.push((app.clone(), s));
                }
            }
        }
        self.results.sort_by(|a, b| b.1.cmp(&a.1));
        self.results.truncate(15);
    }

    pub fn confirm(&self) {
        if self.results.is_empty() {
            return;
        }
        let (t, _) = &self.results[0];
        if let Some(c) = t.strip_prefix("▶ ") {
            let _ = Command::new("sh").arg("-c").arg(c).spawn();
        } else if !t.starts_with("= ") {
            let _ = Command::new(t).spawn();
        }
    }
}

fn is_math(s: &str) -> bool {
    let t = s.trim();
    !t.is_empty()
        && t.chars().all(|c| c.is_ascii_digit() || "+-*/() .".contains(c))
        && t.contains(|c: char| c.is_ascii_digit())
}

fn calc(expr: &str) -> Option<f64> {
    let o = Command::new("bc").arg("-ql").arg(expr).output().ok()?;
    String::from_utf8_lossy(&o.stdout).trim().parse().ok()
}
