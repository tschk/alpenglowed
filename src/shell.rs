//! Shell command construction for literal file paths and search text.

fn quote_word(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub(crate) fn open_file_command(path: &str) -> String {
    format!("xdg-open {}", quote_word(path))
}

pub(crate) fn file_search_command(search: &str) -> String {
    let word = quote_word(search);
    let pattern = quote_word(&format!("*{search}*"));
    format!("locate -i -l 8 {word} 2>/dev/null || fd -t f -l 8 {word} 2>/dev/null || find ~ -maxdepth 4 -iname {pattern} -type f 2>/dev/null | head -8")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output, Stdio};
    use std::sync::atomic::{AtomicU64, Ordering};

    struct OpenerFixture(PathBuf);

    impl OpenerFixture {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let dir = std::env::temp_dir().join(format!(
                "alpenglowed-opener-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&dir).unwrap();
            let opener = dir.join("xdg-open");
            fs::write(&opener, "#!/bin/sh\nprintf '%s\\0' \"$#\" \"$@\"\n").unwrap();
            fs::set_permissions(opener, fs::Permissions::from_mode(0o700)).unwrap();
            Self(dir)
        }
    }

    impl Drop for OpenerFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn shell_output(prologue: &str, command: &str, terminal: bool, path: Option<&Path>) -> Output {
        let script = format!("{prologue}\n{command}\n");
        let mut shell = Command::new("/bin/sh");
        shell.env("HOME", "/synthetic-home");
        if let Some(path) = path {
            shell.env("PATH", path);
        }
        if terminal {
            let mut child = shell
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            child
                .stdin
                .take()
                .unwrap()
                .write_all(script.as_bytes())
                .unwrap();
            child.wait_with_output().unwrap()
        } else {
            shell.args(["-c", &script]).output().unwrap()
        }
    }

    #[test]
    fn file_paths_remain_one_literal_argument_in_both_shell_modes() {
        let fixture = OpenerFixture::new();
        for path in [
            "/tmp/report.txt",
            "/tmp/report with spaces.txt",
            "/tmp/report'; printf INJECTED; #.txt",
            "/tmp/report'$(printf INJECTED)'`printf MORE`.txt",
            "/tmp/O'Brien 雪.txt",
            "/tmp/$name;back\\slash\nnext.txt ",
        ] {
            for terminal in [false, true] {
                let output = shell_output("", &open_file_command(path), terminal, Some(&fixture.0));
                assert!(output.status.success(), "{output:?}");
                assert_eq!(output.stdout, format!("1\0{path}\0").as_bytes());
                assert!(output.stderr.is_empty(), "{output:?}");
            }
        }
    }

    #[test]
    fn search_text_remains_literal_in_each_discovery_fallback() {
        for search in [
            "report",
            "'; printf INJECTED; #",
            "'$(printf INJECTED)'`printf MORE`",
            "O'Brien 雪",
            "space $name;back\\slash\nnext",
        ] {
            for backend in ["locate", "fd", "find"] {
                let prologue = format!(
                    r#"
locate() {{ [ '{backend}' = locate ] || return 1; printf '%s\0' "$@"; }}
fd() {{ [ '{backend}' = fd ] || return 1; printf '%s\0' "$@"; }}
find() {{ printf '%s\0' "$@"; }}
head() {{ /bin/cat; }}
"#
                );
                let output = shell_output(&prologue, &file_search_command(search), false, None);
                assert!(output.status.success(), "{output:?}");
                let expected = match backend {
                    "locate" => format!("-i\0-l\08\0{search}\0"),
                    "fd" => format!("-t\0f\0-l\08\0{search}\0"),
                    _ => format!("/synthetic-home\0-maxdepth\04\0-iname\0*{search}*\0-type\0f\0"),
                };
                assert_eq!(output.stdout, expected.as_bytes());
                assert!(output.stderr.is_empty(), "{output:?}");
            }
        }
    }
}
