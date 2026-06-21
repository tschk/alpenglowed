use serde_json::{json, Value};
use std::io::{self, Read};
use std::process::Command;

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let request: Value = serde_json::from_str(&input).unwrap_or_else(|_| json!({}));
    let query = request["query"].as_str().unwrap_or("").trim();
    let tail = query
        .strip_prefix("spotify-rust")
        .unwrap_or(query)
        .trim()
        .to_lowercase();

    let results = if !has_playerctl() {
        vec![result(
            "unavailable",
            "Spotify unavailable",
            "playerctl not found",
            None,
            1,
        )]
    } else if tail.is_empty() || tail.contains("play") || tail.contains("pause") {
        vec![
            result(
                "play-pause",
                "Spotify Play/Pause",
                "playerctl play-pause",
                Some("playerctl play-pause"),
                100,
            ),
            result(
                "next",
                "Spotify Next",
                "playerctl next",
                Some("playerctl next"),
                90,
            ),
            result(
                "previous",
                "Spotify Previous",
                "playerctl previous",
                Some("playerctl previous"),
                80,
            ),
            result(
                "current",
                "Spotify Current Track",
                "playerctl metadata",
                Some("playerctl metadata --format '{{artist}} - {{title}}'"),
                70,
            ),
        ]
    } else if tail.contains("next") {
        vec![result(
            "next",
            "Spotify Next",
            "playerctl next",
            Some("playerctl next"),
            100,
        )]
    } else if tail.contains("prev") {
        vec![result(
            "previous",
            "Spotify Previous",
            "playerctl previous",
            Some("playerctl previous"),
            100,
        )]
    } else if tail.contains("current") || tail.contains("track") || tail.contains("song") {
        vec![result(
            "current",
            "Spotify Current Track",
            "playerctl metadata",
            Some("playerctl metadata --format '{{artist}} - {{title}}'"),
            100,
        )]
    } else {
        Vec::new()
    };

    println!("{}", json!({ "results": results }));
}

fn result(id: &str, title: &str, subtitle: &str, command: Option<&str>, score: i64) -> Value {
    json!({
        "id": id,
        "title": title,
        "subtitle": subtitle,
        "score": score,
        "action": command.map_or_else(
            || json!({ "type": "none" }),
            |command| json!({ "type": "shell", "command": command }),
        ),
    })
}

fn has_playerctl() -> bool {
    Command::new("sh")
        .arg("-c")
        .arg("command -v playerctl >/dev/null 2>&1")
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
