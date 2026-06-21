#!/usr/bin/env bun

const input = await new Response(Bun.stdin.stream()).text();
const request = JSON.parse(input || "{}");
const query = String(request.query || "").trim();
const tail = query.replace(/^spotify-webcode\b/i, "").trim();

function result(id, title, subtitle, command, score = 100) {
  return {
    id,
    title,
    subtitle,
    score,
    action: command
      ? { type: "shell", command }
      : { type: "none" },
  };
}

const hasPlayerctl = (() => {
  try {
    return Bun.spawnSync(["sh", "-c", "command -v playerctl >/dev/null 2>&1"]).exitCode === 0;
  } catch {
    return false;
  }
})();

const results = [];

if (!hasPlayerctl) {
  results.push(result("unavailable", "Spotify unavailable", "playerctl not found", null, 1));
} else if (!tail || /play|pause|toggle/i.test(tail)) {
  results.push(result("play-pause", "Spotify Play/Pause", "playerctl play-pause", "playerctl play-pause"));
  results.push(result("next", "Spotify Next", "playerctl next", "playerctl next", 90));
  results.push(result("previous", "Spotify Previous", "playerctl previous", "playerctl previous", 80));
  results.push(result("current", "Spotify Current Track", "playerctl metadata", "playerctl metadata --format '{{artist}} - {{title}}'", 70));
} else if (/next/i.test(tail)) {
  results.push(result("next", "Spotify Next", "playerctl next", "playerctl next"));
} else if (/prev/i.test(tail)) {
  results.push(result("previous", "Spotify Previous", "playerctl previous", "playerctl previous"));
} else if (/current|track|song/i.test(tail)) {
  results.push(result("current", "Spotify Current Track", "playerctl metadata", "playerctl metadata --format '{{artist}} - {{title}}'"));
}

console.log(JSON.stringify({ results }));
