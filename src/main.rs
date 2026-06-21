// Alpenglowed — Raycast-style bar launcher for Linux (Wayland)
// The entire desktop is one GPUI window: status pills + text bar + results.
// Summon: Super+Space. Type to launch apps, run commands, calculate.
//
// Architecture:
//   Pills row (clock, battery, cpu, wifi, weather)
//   Text bar: "> _"
//   Results: fuzzy-matched apps / shell output / calculator
//   Below: launched app windows managed by cage (Phase A) or built-in (Phase D)

mod de;
mod pills;
mod runner;

use gpui::prelude::*;
use gpui::*;
use runner::{Runner, RunnerAction, WindowMode};

actions!(alpenglowed, [Quit, FocusBar, DefocusBar, Confirm]);

struct Alpenglowed {
    query: SharedString,
    pills: Entity<pills::Pills>,
    focused: bool,
    mode: WindowMode,
    runner: Runner,
}

impl Alpenglowed {
    fn new(cx: &mut Context<Self>) -> Self {
        let mut runner = Runner::new();
        runner.query = "window".to_string();
        runner.update();

        Self {
            query: SharedString::from("window"),
            pills: cx.new(pills::Pills::new),
            focused: true,
            mode: WindowMode::Tiling,
            runner,
        }
    }

    fn set_query(&mut self, query: String, cx: &mut Context<Self>) {
        self.query = SharedString::from(query.clone());
        self.runner.query = query;
        self.runner.update();
        cx.notify();
    }

    fn apply(&mut self, action: RunnerAction, cx: &mut Context<Self>) {
        match action {
            RunnerAction::SetWindowMode(mode) => self.mode = mode,
            RunnerAction::Desktop(action) => de::run(&action),
            RunnerAction::Launch(_) | RunnerAction::Shell(_) | RunnerAction::Calculator(_) => {}
        }
        cx.notify();
    }

    fn render_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .h(px(48.))
            .bg(rgb(0x222222))
            .flex()
            .items_center()
            .px(px(16.))
            .gap(px(8.))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if !this.focused {
                    return;
                }
                let key = event.keystroke.key.as_str();
                if key == "backspace" {
                    let mut query = String::from(this.query.as_ref());
                    query.pop();
                    this.set_query(query, cx);
                    cx.stop_propagation();
                    return;
                }
                if key == "enter" {
                    if let Some(action) = this.runner.confirm() {
                        this.apply(action, cx);
                    }
                    cx.stop_propagation();
                    return;
                }
                if event.keystroke.modifiers == Modifiers::default() {
                    if let Some(ch) = event.keystroke.key_char.as_deref() {
                        if ch.chars().count() == 1 && !ch.chars().all(|c| c.is_control()) {
                            let mut query = String::from(this.query.as_ref());
                            query.push_str(ch);
                            this.set_query(query, cx);
                            cx.stop_propagation();
                        }
                    }
                }
            }))
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

    fn render_results(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(4.))
            .p(px(12.))
            .children(self.runner.results.iter().map(|result| {
                div()
                    .rounded(px(6.))
                    .bg(rgb(0x242424))
                    .px(px(12.))
                    .py(px(8.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .text_color(rgb(0xf2f2f2))
                            .child(result.title.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(0x8d8d8d))
                            .child(result.subtitle.clone()),
                    )
            }))
    }

    fn render_workspace(&self) -> impl IntoElement {
        div().flex_1().bg(rgb(0x181818)).p(px(12.)).child(
            div()
                .w_full()
                .h_full()
                .rounded(px(6.))
                .bg(match self.mode {
                    WindowMode::Tiling => rgb(0x20262a),
                    WindowMode::Floating => rgb(0x2a2420),
                })
                .flex()
                .items_center()
                .justify_center()
                .text_color(rgb(0xb8b8b8))
                .text_size(px(14.))
                .child(format!("{} mode", self.mode.label())),
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
            .key_context("alpenglowed")
            .on_action(cx.listener(|this, _: &FocusBar, _, cx| {
                this.focused = true;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &DefocusBar, _, cx| {
                this.focused = false;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &Confirm, _, cx| {
                if let Some(action) = this.runner.confirm() {
                    this.apply(action, cx);
                }
            }))
            // Pills row — always visible
            .child(self.pills.clone())
            // Bar + results
            .child(self.render_bar(cx))
            .child(self.render_results())
            .child(self.render_workspace())
    }
}

fn main() {
    if std::env::args().any(|arg| arg == "--polybar") {
        println!("{}", de::DesktopState::detect("tiling").polybar());
        return;
    }

    Application::new().run(|cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("cmd-space", FocusBar, None),
            KeyBinding::new("escape", DefocusBar, None),
            KeyBinding::new("enter", Confirm, None),
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
