use std::process::Command;

#[test]
fn cli_reports_failed_actions() {
    for args in [
        ["open", "example.com"],
        ["search", "hello"],
        ["launch", "nonexistent-alpenglowed-test-app"],
        ["niri", "cycle-column-width"],
        ["mode", "niri"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_alpenglowed"))
            .args(args)
            .env("PATH", "/nonexistent-alpenglowed-test-path")
            .env_remove("ALPENGLOW_SESSION_CONTROL")
            .output()
            .unwrap();
        assert!(!output.status.success(), "{args:?}: {output:?}");
        assert!(output.stdout.is_empty(), "{args:?}: {output:?}");
        assert!(!output.stderr.is_empty(), "{args:?}: {output:?}");
    }
}

#[test]
fn cli_reports_success_after_actions_are_accepted() {
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixListener;

    let dir = std::env::temp_dir().join(format!("alpenglowed-cli-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let opener = dir.join("xdg-open");
    std::fs::write(&opener, "#!/bin/sh\nexit 0\n").unwrap();
    std::fs::set_permissions(&opener, std::fs::Permissions::from_mode(0o755)).unwrap();
    let socket = dir.join("session.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = std::thread::spawn(move || {
        use std::io::{Read, Write};
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = String::new();
            stream.read_to_string(&mut request).unwrap();
            stream.write_all(b"ok\n").unwrap();
        }
    });
    for args in [
        ["open", "example.com"],
        ["search", "hello"],
        ["launch", "/usr/bin/true"],
        ["niri", "cycle-column-width"],
        ["mode", "niri"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_alpenglowed"))
            .args(args)
            .env("PATH", &dir)
            .env("ALPENGLOW_SESSION_CONTROL", &socket)
            .output()
            .unwrap();
        assert!(output.status.success(), "{args:?}: {output:?}");
        assert!(!output.stdout.is_empty(), "{args:?}: {output:?}");
    }
    server.join().unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}
