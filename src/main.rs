extern crate crepuscularity_gpui as gpui;

mod de;
mod plugin;
mod runner;

use crepuscularity_gpui::prelude::*;
use crepuscularity_gpui::{actions, Bounds, KeyBinding, KeyDownEvent, Modifiers, WindowBounds};
use plugin::PluginAction;
use runner::{Runner, WindowMode};
use std::process::Command;

actions!(alpenglowed, [Quit, FocusBar, DefocusBar, Confirm]);

struct UiOptions {
    status_bar: bool,
}

struct Alpenglowed {
    query: SharedString,
    focused: bool,
    mode: WindowMode,
    runner: Runner,
    status_bar: bool,
}

impl Alpenglowed {
    fn new(status_bar: bool, _cx: &mut Context<Self>) -> Self {
        let mut runner = Runner::new();
        runner.query = "window".to_string();
        runner.update();

        Self {
            query: SharedString::from("window"),
            focused: true,
            mode: WindowMode::Tiling,
            runner,
            status_bar,
        }
    }

    fn set_query(&mut self, query: String, cx: &mut Context<Self>) {
        self.query = SharedString::from(query.clone());
        self.runner.query = query;
        self.runner.update();
        cx.notify();
    }

    fn apply(&mut self, action: PluginAction, cx: &mut Context<Self>) {
        match action {
            PluginAction::SetWindowMode { mode } => self.mode = mode,
            PluginAction::Desktop { action } => de::run(&action),
            PluginAction::Launch { program } => {
                let _ = Command::new(program).spawn();
            }
            PluginAction::Shell { command } => {
                let _ = Command::new("sh").arg("-c").arg(command).spawn();
            }
            PluginAction::None => {}
        }
        cx.notify();
    }

    fn render_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let prompt = if self.focused { ">" } else { "." };

        div()
            .w(px(860.))
            .h(px(60.))
            .rounded(px(14.))
            .bg(rgb(0x161616))
            .border_1()
            .border_color(rgb(0x2a2a2a))
            .flex()
            .items_center()
            .px(px(18.))
            .gap(px(12.))
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
                    .text_size(px(16.))
                    .text_color(rgb(0x8f8f8f))
                    .child(prompt),
            )
            .child(
                div()
                    .flex_1()
                    .text_size(px(18.))
                    .text_color(rgb(0xf1f1f1))
                    .child(self.query.clone()),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(0x767676))
                    .child(self.mode.label()),
            )
    }

    fn render_results(&self) -> impl IntoElement {
        div()
            .w(px(860.))
            .gap(px(4.))
            .rounded(px(14.))
            .bg(rgb(0x141414))
            .border_1()
            .border_color(rgb(0x242424))
            .p(px(10.))
            .children(self.runner.results.iter().map(|result| {
                div()
                    .rounded(px(10.))
                    .bg(rgb(0x1d1d1d))
                    .px(px(12.))
                    .py(px(10.))
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

    fn render_status_bar(&self) -> impl IntoElement {
        let desktop = de::DesktopState::detect(self.mode.label());
        let display = desktop.display.unwrap_or_else(|| "no-display".to_string());
        let backend = if desktop.wayland {
            "wayland"
        } else {
            "offline"
        };

        div()
            .absolute()
            .top(px(18.))
            .left_1_2()
            .ml(px(-160.))
            .w(px(320.))
            .h(px(34.))
            .rounded(px(17.))
            .bg(rgb(0x121212))
            .border_1()
            .border_color(rgb(0x242424))
            .flex()
            .items_center()
            .justify_between()
            .px(px(12.))
            .text_size(px(12.))
            .text_color(rgb(0xbcbcbc))
            .child(self.mode.label())
            .child(backend)
            .child(display)
    }

    fn render_workspace(&self) -> impl IntoElement {
        let mode_label = format!("{} mode", self.mode.label());
        crepuscularity_gpui::view! {r#"
            div flex-1 bg-neutral-950 p-6
                div w-full h-full rounded bg-neutral-900 border border-neutral-800
                    div w-full h-full flex items-center justify-center
                        div flex flex-col items-center gap-3
                            div text-neutral-100 text-xl
                                "alpenglowed"
                            div text-neutral-400 text-sm
                                "{mode_label}"
        "#}
    }
}

impl Render for Alpenglowed {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut root = div()
            .size_full()
            .bg(rgb(0x0f0f0f))
            .flex()
            .flex_col()
            .relative()
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
            .child(self.render_workspace())
            .child(
                div()
                    .absolute()
                    .top_1_2()
                    .left_1_2()
                    .ml(px(-430.))
                    .mt(px(-180.))
                    .w(px(860.))
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(self.render_bar(cx))
                    .child(self.render_results()),
            );

        if self.status_bar {
            root = root.child(self.render_status_bar());
        }

        root
    }
}

fn main() {
    let options = UiOptions::from_env();

    if std::env::args().any(|arg| arg == "--polybar") {
        println!("{}", de::DesktopState::detect("tiling").polybar());
        return;
    }
    if std::env::args().any(|arg| arg == "--smoke-wayland") {
        match de::smoke_wayland() {
            Ok(()) => println!("wayland ok"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return;
    }

    ensure_wayland_display();

    Application::new().run(move |cx: &mut App| {
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

        let status_bar = options.status_bar;
        cx.open_window(window_options, move |_window, cx| {
            cx.new(|cx| Alpenglowed::new(status_bar, cx))
        })
        .unwrap();
    });
}

impl UiOptions {
    fn from_env() -> Self {
        let status_bar = std::env::args().any(|arg| arg == "--status-bar")
            || matches!(
                std::env::var("ALPENGLOWED_STATUS_BAR").as_deref(),
                Ok("1" | "true" | "yes")
            );

        Self { status_bar }
    }
}

fn ensure_wayland_display() {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() || std::env::var_os("DISPLAY").is_some() {
        return;
    }

    let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR") else {
        return;
    };

    let wayland_socket = std::path::Path::new(&runtime_dir).join("wayland-0");
    if wayland_socket.exists() {
        std::env::set_var("WAYLAND_DISPLAY", "wayland-0");
    }
}
