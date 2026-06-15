// Alpenglowed — Raycast-style bar launcher for Linux (Wayland)
// The entire desktop is one GPUI window: status pills + text bar + results.
// Summon: Super+Space. Type to launch apps, run commands, calculate.
//
// Architecture:
//   Pills row (clock, battery, cpu, wifi, weather)
//   Text bar: "> _"
//   Results: fuzzy-matched apps / shell output / calculator
//   Below: launched app windows managed by cage (Phase A) or built-in (Phase D)

mod pills;
mod runner;

use gpui::prelude::*;
use gpui::*;

actions!(alpenglowed, [Quit, FocusBar, DefocusBar]);

struct Alpenglowed {
    query: SharedString,
    pills: Entity<pills::Pills>,
    focused: bool,
}

impl Alpenglowed {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            query: SharedString::default(),
            pills: cx.new(pills::Pills::new),
            focused: true,
        }
    }

    fn render_bar(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .h(px(48.))
            .bg(rgb(0x222222))
            .flex()
            .items_center()
            .px(px(16.))
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(18.))
                    .text_color(rgb(0x888888))
                    .child("> "),
            )
            .child(
                div()
                    .flex_1()
                    .text_size(px(18.))
                    .text_color(rgb(0xcccccc))
                    .child(self.query.clone()),
            )
    }
}

impl Render for Alpenglowed {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0x1a1a1a))
            .flex()
            .flex_col()
            // Pills row — always visible
            .child(self.pills.clone())
            // Bar + results
            .child(self.render_bar(cx))
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("cmd-space", FocusBar, None),
            KeyBinding::new("escape", DefocusBar, None),
            KeyBinding::new("cmd-q", Quit, None),
        ]);

        let bounds = Bounds::maximized(None, cx);
        let window_options = WindowOptions {
            app_id: Some("alpenglowed".into()),
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        };

        cx.open_window(window_options, |_window, cx| cx.new(Alpenglowed::new))
            .unwrap();
    });
}
