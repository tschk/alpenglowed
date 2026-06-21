extern crate crepuscularity_gpui as gpui;

mod de;
mod plugin;
mod runner;
mod session;

use crepuscularity_gpui::prelude::*;
use crepuscularity_gpui::{
    actions, bounds, point, size, AnyWindowHandle, EventEmitter, KeyBinding, KeyDownEvent,
    Modifiers, WindowBounds, WindowKind, WindowOptions,
};
use plugin::PluginAction;
use runner::{Runner, WindowMode};
use std::process::Command;

actions!(alpenglowed, [Quit, FocusBar, DefocusBar, Confirm]);

#[derive(Clone, Copy)]
struct UiOptions {
    status_bar: bool,
    open_settings: bool,
}

#[derive(Clone, Copy)]
enum DesktopEvent {
    Changed,
}

struct DesktopModel {
    query: String,
    mode: WindowMode,
    status_bar: bool,
    runner: Runner,
    session_control: bool,
    launcher: Option<AnyWindowHandle>,
    settings: Option<AnyWindowHandle>,
}

impl EventEmitter<DesktopEvent> for DesktopModel {}

impl DesktopModel {
    fn new(options: UiOptions) -> Self {
        let mut runner = Runner::new();
        runner.query = "window".to_string();
        runner.update();

        Self {
            query: "window".to_string(),
            mode: WindowMode::Tiling,
            status_bar: options.status_bar,
            runner,
            session_control: std::env::var_os("ALPENGLOW_SESSION_CONTROL").is_some(),
            launcher: None,
            settings: None,
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
            PluginAction::SetWindowMode { mode } => {
                self.mode = mode.clone();
                let _ = session::dispatch(&session::SessionRequest::SetWindowMode { mode });
            }
            PluginAction::OpenSettings => {}
            PluginAction::Desktop { action } => {
                if session::dispatch(&session::SessionRequest::DesktopAction {
                    action: action.clone(),
                })
                .is_err()
                {
                    de::run(&action);
                }
            }
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

    fn toggle_status_bar(&mut self, cx: &mut Context<Self>) {
        self.status_bar = !self.status_bar;
        self.changed(cx);
    }
}

struct WorkspaceWindow {
    desktop: Entity<DesktopModel>,
    role: DesktopWindowRole,
}

impl WorkspaceWindow {
    fn new(
        desktop: Entity<DesktopModel>,
        role: DesktopWindowRole,
        options: UiOptions,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.subscribe(&desktop, |_, _, _: &DesktopEvent, cx| {
            cx.notify();
        })
        .detach();

        let _ = options;
        Self { desktop, role }
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
        let session = if desktop.session_control {
            "session"
        } else {
            "local"
        };

        crepuscularity_gpui::view! {r#"
            div absolute top-5 left-1/2 ml-[-190px] w-[380px] h-[34px] rounded-[6px] bg-[#050505] border border-[#2a2a2a] flex items-center justify-between px-3 text-[12px] text-[#cfcfcf]
                "{desktop.mode.label()}"
                "{backend}"
                "{display}"
                "{session}"
        "#}
    }

    fn render_workspace(&self, cx: &App) -> impl IntoElement {
        let desktop = self.desktop.read(cx);
        let mode_label = format!("{} mode", desktop.mode.label());
        let title = self.role.title();
        let subtitle = self.role.subtitle();
        let query = desktop.query.clone();

        crepuscularity_gpui::view! {r#"
            div flex-1 bg-[#050505] p-4
                div w-full h-full rounded-[6px] bg-[#080808] border border-[#252525]
                    div w-full h-full flex flex-col
                        div h-[28px] border-b border-[#252525] flex items-center justify-between px-3
                            div text-[#dcdcdc] text-[12px]
                                "{title}"
                            div text-[#9a9a9a] text-[11px]
                                "{mode_label}"
                        div flex-1 flex items-center justify-center
                            div flex flex-col items-center gap-3
                                div text-[#f0f0f0] text-xl
                                    "{title}"
                                div text-[#b8b8b8] text-sm
                                    "{subtitle}"
                                div text-[#f0f0f0] text-[13px]
                                    "$ {query}"
                                div text-[#8d8d8d] text-xs
                                    "{mode_label}"
        "#}
    }
}

struct CompanionWindow {
    desktop: Entity<DesktopModel>,
}

impl CompanionWindow {
    fn new(desktop: Entity<DesktopModel>, cx: &mut Context<Self>) -> Self {
        cx.subscribe(&desktop, |_, _, _: &DesktopEvent, cx| {
            cx.notify();
        })
        .detach();
        Self { desktop }
    }
}

impl Render for CompanionWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let desktop = self.desktop.read(cx);
        let session_status = if desktop.session_control {
            "compositor session"
        } else {
            "local shell"
        };

        crepuscularity_gpui::view! {r#"
            div size-full bg-[#050505] p-4
                div w-full h-full rounded-[6px] bg-[#080808] border border-[#252525] p-4 flex flex-col gap-4
                    div flex items-center justify-between
                        div text-[#f0f0f0] text-[14px]
                            "Actions"
                        div text-[#9a9a9a] text-[11px]
                            "{desktop.mode.label()}"
                    div flex flex-col gap-2 text-[12px] text-[#d0d0d0]
                        div "Enter  run result"
                        div "Esc    close bar"
                        div "Cmd-Space focus bar"
                        div "Cmd-Q  quit shell"
                    div flex flex-col gap-2 pt-2 border-t border-[#252525]
                        div text-[#b8b8b8] text-[12px]
                            "Session"
                        div text-[#8d8d8d] text-[12px]
                            "{session_status}"
                        div text-[#8d8d8d] text-[12px]
                            "Use the bar for quit, settings, shell, and desktop actions."
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
                focus_or_open_launcher(&this.desktop, cx);
            }))
            .child(self.render_workspace(cx));

        if self.desktop.read(cx).status_bar {
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
            if matches!(action, PluginAction::OpenSettings) {
                open_or_focus_settings(&self.desktop, cx);
            } else {
                self.desktop.update(cx, |desktop, cx| {
                    desktop.apply(action, cx);
                });
            }
        }
    }

    fn render_bar(&self, cx: &App) -> impl IntoElement {
        let desktop = self.desktop.read(cx);

        crepuscularity_gpui::view! {r#"
            div w-[860px] h-[60px] rounded-[6px] bg-[#050505] border border-[#2a2a2a] flex items-center px-[18px] gap-3
                div text-[13px] text-[#d0d0d0]
                    "user@alpenglowed"
                div text-[16px] text-[#8e8e8e]
                    ":"
                div text-[16px] text-[#f0f0f0]
                    "~"
                div text-[16px] text-[#f0f0f0]
                    "$"
                div flex-1 text-[18px] text-[#ffffff]
                    "{desktop.query}"
                div text-[12px] text-[#b8b8b8]
                    "{desktop.mode.label()}"
        "#}
    }

    fn render_results(&self, cx: &App) -> impl IntoElement {
        let desktop = self.desktop.read(cx);

        div()
            .w(px(860.))
            .gap(px(4.))
            .rounded(px(6.))
            .bg(rgb(0x050505))
            .border_1()
            .border_color(rgb(0x2a2a2a))
            .p(px(10.))
            .children(desktop.runner.results.iter().map(|result| {
                div()
                    .rounded(px(6.))
                    .bg(rgb(0x111111))
                    .px(px(12.))
                    .py(px(10.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(0xb8b8b8))
                            .child("$"),
                    )
                    .child(
                        div()
                            .text_size(px(14.))
                            .text_color(rgb(0xf0f0f0))
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

struct SettingsWindow {
    desktop: Entity<DesktopModel>,
}

impl SettingsWindow {
    fn new(desktop: Entity<DesktopModel>, cx: &mut Context<Self>) -> Self {
        cx.subscribe(&desktop, |_, _, _: &DesktopEvent, cx| {
            cx.notify();
        })
        .detach();

        Self { desktop }
    }

    fn mode_button(
        &self,
        label: &'static str,
        mode: WindowMode,
        active: bool,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let desktop = self.desktop.clone();
        let bg = if active { rgb(0xf0f0f0) } else { rgb(0x1a1a1a) };
        let fg = if active { rgb(0x050505) } else { rgb(0xe0e0e0) };

        div()
            .id(SharedString::from(format!("mode-{label}")))
            .px(px(12.))
            .py(px(8.))
            .rounded(px(10.))
            .bg(bg)
            .text_color(fg)
            .cursor_pointer()
            .child(label)
            .on_click(move |_, _, cx| {
                desktop.update(cx, |desktop, cx| {
                    desktop.mode = mode.clone();
                    let _ = session::dispatch(&session::SessionRequest::SetWindowMode {
                        mode: mode.clone(),
                    });
                    desktop.changed(cx);
                });
            })
    }

    fn action_button(
        &self,
        label: &'static str,
        on_click: impl Fn(&Entity<DesktopModel>, &mut App) + 'static,
    ) -> impl IntoElement {
        let desktop = self.desktop.clone();
        div()
            .id(SharedString::from(format!("settings-{label}")))
            .px(px(12.))
            .py(px(8.))
            .rounded(px(10.))
            .bg(rgb(0x1a1a1a))
            .text_color(rgb(0xe8e8e8))
            .cursor_pointer()
            .child(label)
            .on_click(move |_, _, cx| on_click(&desktop, cx))
    }

    fn desktop_action_button(
        &self,
        label: &'static str,
        action: de::DesktopAction,
    ) -> impl IntoElement {
        self.action_button(label, move |desktop, cx| {
            desktop.update(cx, |desktop, cx| {
                desktop.apply(
                    PluginAction::Desktop {
                        action: action.clone(),
                    },
                    cx,
                );
            });
        })
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let desktop = self.desktop.read(cx);
        let tiling = desktop.mode == WindowMode::Tiling;
        let floating = desktop.mode == WindowMode::Floating;
        let status_bar = if desktop.status_bar {
            "enabled"
        } else {
            "disabled"
        };
        let session_status = if desktop.session_control {
            "Connected to compositor"
        } else {
            "Running local fallbacks"
        };

        div().size_full().bg(rgb(0x080808)).child(
            crepuscularity_gpui::view! {r#"
                div size-full bg-[#050505] p-5
                    div w-full h-full rounded-[6px] bg-[#080808] border border-[#252525] p-5 flex flex-col gap-5
                        div flex items-center justify-between
                            div flex flex-col gap-1
                                div text-[#f0f0f0] text-xl
                                    "Settings"
                                div text-[#b8b8b8] text-sm
                                    "Desktop, launcher, modes, and system actions"
                            div text-[#b8b8b8] text-xs
                                "{desktop.mode.label()}"
                        div flex flex-col gap-3
                            div text-[#d0d0d0] text-sm
                                "Windows"
                        div flex flex-col gap-3
                            div text-[#d0d0d0] text-sm
                                "Interface"
                            div text-[#8d8d8d] text-xs
                                "Status bar is {status_bar}"
                        div flex flex-col gap-3
                            div text-[#d0d0d0] text-sm
                                "Launcher"
                        div flex flex-col gap-3
                            div text-[#d0d0d0] text-sm
                                "Desktop actions"
                        div flex flex-col gap-3
                            div text-[#d0d0d0] text-sm
                                "Session"
                            div text-[#8d8d8d] text-xs
                                "{session_status}"
            "#}
            .child(
                div()
                    .absolute()
                    .top(px(114.))
                    .left(px(36.))
                    .flex()
                    .gap(px(8.))
                    .child(self.mode_button("Tile windows", WindowMode::Tiling, tiling, cx))
                    .child(self.mode_button("Float windows", WindowMode::Floating, floating, cx)),
            )
            .child(
                div()
                    .absolute()
                    .top(px(204.))
                    .left(px(36.))
                    .flex()
                    .gap(px(8.))
                    .child(self.action_button("Toggle status bar", |desktop, cx| {
                        desktop.update(cx, |desktop, cx| desktop.toggle_status_bar(cx));
                    })),
            )
            .child(
                div()
                    .absolute()
                    .top(px(294.))
                    .left(px(36.))
                    .flex()
                    .gap(px(8.))
                    .child(self.action_button("Focus launcher", |desktop, cx| {
                        focus_or_open_launcher(desktop, cx);
                    }))
                    .child(self.action_button("Reset query", |desktop, cx| {
                        desktop.update(cx, |desktop, cx| {
                            desktop.set_query("window".to_string(), cx);
                        });
                    })),
            )
            .child(
                div()
                    .absolute()
                    .top(px(384.))
                    .left(px(36.))
                    .flex()
                    .gap(px(8.))
                    .child(self.desktop_action_button("Lock", de::DesktopAction::Lock))
                    .child(self.desktop_action_button("Terminal", de::DesktopAction::Terminal))
                    .child(self.desktop_action_button("Files", de::DesktopAction::Files))
                    .child(
                        self.desktop_action_button(
                            "Screenshot",
                            de::DesktopAction::Screenshot,
                        ),
                    )
                    .child(
                        self.desktop_action_button(
                            "Clipboard",
                            de::DesktopAction::Clipboard,
                        ),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .top(px(474.))
                    .left(px(36.))
                    .flex()
                    .gap(px(8.))
                    .child(self.desktop_action_button("Wi-Fi", de::DesktopAction::Wifi))
                    .child(
                        self.desktop_action_button(
                            "Notifications",
                            de::DesktopAction::Notifications,
                        ),
                    )
                    .child(self.desktop_action_button("Logout", de::DesktopAction::Logout))
                    .child(self.desktop_action_button("Suspend", de::DesktopAction::Suspend))
                    .child(self.desktop_action_button("Reboot", de::DesktopAction::Reboot))
                    .child(
                        self.desktop_action_button("Shutdown", de::DesktopAction::Shutdown),
                    ),
            ),
        )
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

fn launcher_window_options(cx: &App) -> WindowOptions {
    WindowOptions {
        app_id: Some("alpenglowed-launcher".into()),
        titlebar: None,
        window_bounds: Some(WindowBounds::centered(size(px(900.), px(340.)), cx)),
        kind: WindowKind::PopUp,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        ..Default::default()
    }
}

fn settings_window_options(cx: &App) -> WindowOptions {
    WindowOptions {
        app_id: Some("alpenglowed-settings".into()),
        titlebar: None,
        window_bounds: Some(WindowBounds::centered(size(px(900.), px(640.)), cx)),
        kind: WindowKind::PopUp,
        is_resizable: false,
        ..Default::default()
    }
}

fn companion_window_options(_cx: &App) -> WindowOptions {
    WindowOptions {
        app_id: Some("alpenglowed-companion".into()),
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(bounds(
            point(px(950.), px(150.)),
            size(px(240.), px(220.)),
        ))),
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

#[derive(Clone, Copy)]
enum DesktopWindowRole {
    Primary,
}

impl DesktopWindowRole {
    fn title(self) -> &'static str {
        match self {
            Self::Primary => "Desktop",
        }
    }

    fn subtitle(self) -> &'static str {
        match self {
            Self::Primary => "keyboard workspace",
        }
    }

    fn app_id(self) -> &'static str {
        "alpenglowed"
    }
}

fn managed_window_options(role: DesktopWindowRole, mode: WindowMode, cx: &App) -> WindowOptions {
    WindowOptions {
        app_id: Some(role.app_id().into()),
        titlebar: None,
        window_bounds: Some(managed_window_bounds(role, mode, cx)),
        is_resizable: true,
        ..Default::default()
    }
}

fn managed_window_bounds(role: DesktopWindowRole, mode: WindowMode, cx: &App) -> WindowBounds {
    match mode {
        WindowMode::Tiling => match role {
            DesktopWindowRole::Primary => {
                WindowBounds::Windowed(bounds(point(px(32.), px(56.)), size(px(880.), px(688.))))
            }
        },
        WindowMode::Floating => match role {
            DesktopWindowRole::Primary => WindowBounds::centered(size(px(860.), px(640.)), cx),
        },
    }
}

fn open_managed_window(
    desktop: &Entity<DesktopModel>,
    role: DesktopWindowRole,
    options: UiOptions,
    cx: &mut App,
) {
    let mode = desktop.read(cx).mode.clone();
    let desktop_entity = desktop.clone();
    let handle = cx
        .open_window(
            managed_window_options(role, mode, cx),
            move |_window, cx| cx.new(|cx| WorkspaceWindow::new(desktop_entity, role, options, cx)),
        )
        .unwrap();
    let _: AnyWindowHandle = handle.into();
    desktop.update(cx, |desktop, cx| desktop.changed(cx));
}

fn open_settings_window(desktop: &Entity<DesktopModel>, cx: &mut App) -> AnyWindowHandle {
    let desktop_entity = desktop.clone();
    let handle = cx
        .open_window(settings_window_options(cx), move |window, cx| {
            let view = cx.new(|cx| SettingsWindow::new(desktop_entity, cx));
            window.activate_window();
            view
        })
        .unwrap();
    let any_handle: AnyWindowHandle = handle.into();
    desktop.update(cx, |desktop, cx| {
        desktop.settings = Some(any_handle);
        desktop.changed(cx);
    });
    any_handle
}

fn open_companion_window(desktop: &Entity<DesktopModel>, cx: &mut App) {
    let desktop_entity = desktop.clone();
    let _ = cx.open_window(companion_window_options(cx), move |window, cx| {
        let view = cx.new(|cx| CompanionWindow::new(desktop_entity, cx));
        window.activate_window();
        view
    });
}

fn focus_or_open_launcher(desktop: &Entity<DesktopModel>, cx: &mut App) {
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
    open_launcher_window(desktop, cx);
}

fn open_or_focus_settings(desktop: &Entity<DesktopModel>, cx: &mut App) {
    let Some(handle) = desktop.read(cx).settings else {
        open_settings_window(desktop, cx);
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
        desktop.settings = None;
        desktop.changed(cx);
    });
    open_settings_window(desktop, cx);
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

        let desktop = cx.new(|_| DesktopModel::new(options));
        open_managed_window(&desktop, DesktopWindowRole::Primary, options, cx);
        open_companion_window(&desktop, cx);

        if options.open_settings {
            open_or_focus_settings(&desktop, cx);
        } else {
            focus_or_open_launcher(&desktop, cx);
        }
    });
}

impl UiOptions {
    fn from_env() -> Self {
        let status_bar = std::env::args().any(|arg| arg == "--status-bar")
            || matches!(
                std::env::var("ALPENGLOWED_STATUS_BAR").as_deref(),
                Ok("1" | "true" | "yes")
            );
        let open_settings = std::env::args().any(|arg| arg == "--open-settings");

        Self {
            status_bar,
            open_settings,
        }
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
