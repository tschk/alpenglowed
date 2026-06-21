extern crate crepuscularity_gpui as gpui;

mod de;
mod plugin;
mod runner;

use crepuscularity_gpui::prelude::*;
use crepuscularity_gpui::{
    actions, size, AnyWindowHandle, Bounds, EventEmitter, KeyBinding, KeyDownEvent, Modifiers,
    TitlebarOptions, WindowBounds, WindowKind, WindowOptions,
};
use plugin::PluginAction;
use runner::{Runner, WindowMode};
use std::process::Command;

actions!(alpenglowed, [Quit, FocusBar, DefocusBar, Confirm]);

#[derive(Clone, Copy)]
struct UiOptions {
    status_bar: bool,
}

#[derive(Clone, Copy)]
enum DesktopEvent {
    Changed,
}

struct DesktopModel {
    query: String,
    mode: WindowMode,
    runner: Runner,
    launcher: Option<AnyWindowHandle>,
}

impl EventEmitter<DesktopEvent> for DesktopModel {}

impl DesktopModel {
    fn new() -> Self {
        let mut runner = Runner::new();
        runner.query = "window".to_string();
        runner.update();

        Self {
            query: "window".to_string(),
            mode: WindowMode::Tiling,
            runner,
            launcher: None,
        }
    }

    fn set_query(&mut self, query: String, cx: &mut Context<Self>) {
        self.query = query.clone();
        self.runner.query = query;
        self.runner.update();
        self.changed(cx);
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
        self.changed(cx);
    }

    fn changed(&mut self, cx: &mut Context<Self>) {
        cx.notify();
        cx.emit(DesktopEvent::Changed);
    }
}

struct WorkspaceWindow {
    desktop: Entity<DesktopModel>,
    status_bar: bool,
    options: UiOptions,
}

impl WorkspaceWindow {
    fn new(desktop: Entity<DesktopModel>, options: UiOptions, cx: &mut Context<Self>) -> Self {
        cx.subscribe(&desktop, |_, _, _: &DesktopEvent, cx| {
            cx.notify();
        })
        .detach();

        Self {
            desktop,
            status_bar: options.status_bar,
            options,
        }
    }

    fn render_status_bar(&self, cx: &App) -> impl IntoElement {
        let desktop = self.desktop.read(cx);
        let desktop_state = de::DesktopState::detect(desktop.mode.label());
        let display = desktop_state
            .display
            .unwrap_or_else(|| "no-display".to_string());
        let backend = if desktop_state.wayland {
            "wayland"
        } else {
            "offline"
        };

        crepuscularity_gpui::view! {r#"
            div absolute top-5 left-1/2 ml-[-160px] w-[320px] h-[34px] rounded-[17px] bg-neutral-950 border border-neutral-800 flex items-center justify-between px-3 text-[12px] text-neutral-300
                "{desktop.mode.label()}"
                "{backend}"
                "{display}"
        "#}
    }

    fn render_workspace(&self, cx: &App) -> impl IntoElement {
        let desktop = self.desktop.read(cx);
        let mode_label = format!("{} mode", desktop.mode.label());

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

impl Render for WorkspaceWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut root = div()
            .size_full()
            .bg(rgb(0x0f0f0f))
            .relative()
            .key_context("alpenglowed")
            .on_action(cx.listener(|this, _: &FocusBar, _, cx| {
                focus_or_open_launcher(&this.desktop, this.options, cx);
            }))
            .child(self.render_workspace(cx));

        if self.status_bar {
            root = root.child(self.render_status_bar(cx));
        }

        root
    }
}

struct LauncherWindow {
    desktop: Entity<DesktopModel>,
}

impl LauncherWindow {
    fn new(desktop: Entity<DesktopModel>, cx: &mut Context<Self>) -> Self {
        cx.subscribe(&desktop, |_, _, _: &DesktopEvent, cx| {
            cx.notify();
        })
        .detach();

        Self { desktop }
    }

    fn backspace(&mut self, cx: &mut Context<Self>) {
        let query = self.desktop.read(cx).query.clone();
        let mut next = query;
        next.pop();
        self.desktop.update(cx, |desktop, cx| {
            desktop.set_query(next, cx);
        });
    }

    fn append(&mut self, ch: &str, cx: &mut Context<Self>) {
        let query = self.desktop.read(cx).query.clone();
        let mut next = query;
        next.push_str(ch);
        self.desktop.update(cx, |desktop, cx| {
            desktop.set_query(next, cx);
        });
    }

    fn confirm(&mut self, cx: &mut Context<Self>) {
        let action = self.desktop.read(cx).runner.confirm();
        if let Some(action) = action {
            self.desktop.update(cx, |desktop, cx| {
                desktop.apply(action, cx);
            });
        }
    }

    fn render_bar(&self, cx: &App) -> impl IntoElement {
        let desktop = self.desktop.read(cx);

        crepuscularity_gpui::view! {r#"
            div w-[860px] h-[60px] rounded-[14px] bg-neutral-950 border border-neutral-800 flex items-center px-[18px] gap-3
                div text-[16px] text-neutral-500
                    ">"
                div flex-1 text-[18px] text-neutral-100
                    "{desktop.query}"
                div text-[12px] text-neutral-500
                    "{desktop.mode.label()}"
        "#}
    }

    fn render_results(&self, cx: &App) -> impl IntoElement {
        let desktop = self.desktop.read(cx);

        div()
            .w(px(860.))
            .gap(px(4.))
            .rounded(px(14.))
            .bg(rgb(0x141414))
            .border_1()
            .border_color(rgb(0x242424))
            .p(px(10.))
            .children(desktop.runner.results.iter().map(|result| {
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
}

impl Render for LauncherWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0x0f0f0f))
            .key_context("alpenglowed")
            .on_action(cx.listener(|this, _: &Confirm, _, cx| {
                this.confirm(cx);
            }))
            .on_action(cx.listener(|this, _: &DefocusBar, window, cx| {
                let id = window.window_handle().window_id();
                this.desktop.update(cx, move |desktop, cx| {
                    if desktop
                        .launcher
                        .is_some_and(|handle| handle.window_id() == id)
                    {
                        desktop.launcher = None;
                        desktop.changed(cx);
                    }
                });
                window.remove_window();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                let key = event.keystroke.key.as_str();
                if key == "backspace" {
                    this.backspace(cx);
                    cx.stop_propagation();
                    return;
                }
                if key == "enter" {
                    this.confirm(cx);
                    cx.stop_propagation();
                    return;
                }
                if event.keystroke.modifiers == Modifiers::default() {
                    if let Some(ch) = event.keystroke.key_char.as_deref() {
                        if ch.chars().count() == 1 && !ch.chars().all(|c| c.is_control()) {
                            this.append(ch, cx);
                            cx.stop_propagation();
                        }
                    }
                }
            }))
            .child(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .w(px(860.))
                            .flex()
                            .flex_col()
                            .gap(px(10.))
                            .child(self.render_bar(cx))
                            .child(self.render_results(cx)),
                    ),
            )
    }
}

fn workspace_window_options(cx: &App) -> WindowOptions {
    WindowOptions {
        app_id: Some("alpenglowed".into()),
        titlebar: None,
        window_bounds: Some(WindowBounds::Maximized(Bounds::maximized(None, cx))),
        ..Default::default()
    }
}

fn launcher_window_options(cx: &App) -> WindowOptions {
    WindowOptions {
        app_id: Some("alpenglowed-launcher".into()),
        titlebar: Some(TitlebarOptions::default()),
        window_bounds: Some(WindowBounds::centered(size(px(900.), px(340.)), cx)),
        kind: WindowKind::PopUp,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        ..Default::default()
    }
}

fn open_launcher_window(desktop: &Entity<DesktopModel>, cx: &mut App) -> AnyWindowHandle {
    let desktop_entity = desktop.clone();
    let handle = cx
        .open_window(launcher_window_options(cx), move |window, cx| {
            let view = cx.new(|cx| LauncherWindow::new(desktop_entity, cx));
            window.activate_window();
            view
        })
        .unwrap();
    let any_handle: AnyWindowHandle = handle.into();
    desktop.update(cx, |desktop, cx| {
        desktop.launcher = Some(any_handle);
        desktop.changed(cx);
    });
    any_handle
}

fn focus_or_open_launcher(desktop: &Entity<DesktopModel>, options: UiOptions, cx: &mut App) {
    let Some(handle) = desktop.read(cx).launcher else {
        open_launcher_window(desktop, cx);
        return;
    };

    if handle
        .update(cx, |_, window, _| {
            window.activate_window();
        })
        .is_ok()
    {
        return;
    }

    desktop.update(cx, |desktop, cx| {
        desktop.launcher = None;
        desktop.changed(cx);
    });
    let _ = options;
    open_launcher_window(desktop, cx);
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

        let desktop = cx.new(|_| DesktopModel::new());
        let workspace = desktop.clone();
        cx.open_window(workspace_window_options(cx), move |_window, cx| {
            cx.new(|cx| WorkspaceWindow::new(workspace, options, cx))
        })
        .unwrap();

        focus_or_open_launcher(&desktop, options, cx);
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
