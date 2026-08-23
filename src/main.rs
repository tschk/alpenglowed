extern crate crepuscularity_gpui as gpui;

#[cfg(feature = "compositor")]
mod compositor;
mod config;
mod de;
mod layout;
mod notifications;
mod plugin;
mod role;
mod runner;
mod session;
mod terminal;

use crepuscularity_core::context::{TemplateContext, TemplateValue};
use crepuscularity_gpui::prelude::*;
use crepuscularity_gpui::{
    actions, font, size, AnyWindowHandle, Div, EventEmitter, Font, KeyBinding, KeyDownEvent,
    Modifiers, Pixels, WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowKind,
    WindowOptions,
};
use crepuscularity_web::render_component_file_to_html;
use layout::{Axis, LayoutChildView, LayoutState, LayoutView, LayoutWindowView};
use plugin::{PluginAction, WindowTarget};
use runner::{Runner, WindowMode};
use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;

const GEIST: &[u8] = include_bytes!("../assets/fonts/geist-regular.ttf");
const GEIST_MONO: &[u8] = include_bytes!("../assets/fonts/geist-mono-regular.ttf");

fn ui_font() -> Font {
    font("Geist")
}

const ACCENT: u32 = 0x33ccff;
const ACCENT_DIM: u32 = 0x123142;
const SURFACE: u32 = 0x080b10;
const SURFACE_2: u32 = 0x0d1118;
const SURFACE_3: u32 = 0x111722;
const BORDER: u32 = 0x23313d;
const BORDER_SOFT: u32 = 0x18222c;
const TEXT: u32 = 0xe6e9ef;
const TEXT_DIM: u32 = 0x9aa3b2;
const TEXT_FAINT: u32 = 0x6b7384;
use std::process::Command;

const SHELL_CREPUS: &str = include_str!("views/shell.crepus");

actions!(
    alpenglowed,
    [
        Quit,
        FocusBar,
        DefocusBar,
        Confirm,
        SplitRow,
        SplitColumn,
        FlipAxis,
        ResetLayout,
        NudgeLeft,
        NudgeRight,
        NudgeUp,
        NudgeDown,
        ExpandWindow,
        ContractWindow,
        GrowPane,
        ShrinkPane,
        FocusNextPane,
        FocusPreviousPane,
        ClosePane,
        ToggleFloatPane,
        ToggleTerminalPane,
        ToggleSettingsWindow,
        ToggleStatusBarAction,
        CycleWindowModeAction,
        ClearQueryAction,
        BalancePanesAction,
        CenterFocusedAction,
        MoveWindowLeft,
        MoveWindowRight,
        MoveWindowUp,
        MoveWindowDown,
        FocusFirstPane,
        FocusLastPane,
        SelectResult1,
        SelectResult2,
        SelectResult3,
        SelectResult4,
        SelectResult5,
        SelectResult6,
        SelectResult7,
        SelectResult8
    ]
);

#[derive(Clone)]
struct UiOptions {
    status_bar: bool,
    external_polybar: bool,
    open_settings: bool,
    initial_query: String,
    mode: WindowMode,
    demo_layout: bool,
    role: role::SessionRole,
}

#[derive(Clone, Copy)]
enum DesktopEvent {
    Changed,
}

struct DesktopModel {
    query: String,
    mode: WindowMode,
    layout: LayoutState,
    status_bar: bool,
    external_polybar: bool,
    last_action: String,
    runner: Runner,
    session_control: bool,
    launcher: Option<AnyWindowHandle>,
    settings: Option<AnyWindowHandle>,
    notifications: notifications::NotificationState,
    terminal: Option<terminal::TerminalConsole>,
    terminal_open: bool,
    terminal_input: String,
    role: role::SessionRole,
    #[cfg(feature = "compositor")]
    compositor_events: Option<std::sync::mpsc::Receiver<compositor::CompositorEvent>>,
    #[cfg(feature = "compositor")]
    compositor_surfaces: std::collections::HashMap<u32, usize>,
}

impl EventEmitter<DesktopEvent> for DesktopModel {}

#[derive(serde::Serialize, serde::Deserialize)]
struct ExternalBarState {
    focused_title: String,
    layout: String,
    mode: String,
    last_action: String,
}

impl DesktopModel {
    fn new(
        options: UiOptions,
        #[cfg(feature = "compositor")] compositor_rx: Option<
            std::sync::mpsc::Receiver<compositor::CompositorEvent>,
        >,
        #[cfg(feature = "compositor")] _compositor_tx: Option<
            std::sync::mpsc::Sender<compositor::CompositorCommand>,
        >,
    ) -> Self {
        let mut notifications = notifications::NotificationState::new();
        notifications.start();
        let mut desktop = Self {
            query: options.initial_query,
            mode: options.mode,
            layout: {
                let mut layout = if options.demo_layout {
                    LayoutState::demo()
                } else {
                    LayoutState::new()
                };
                layout.set_window_mode(&options.mode);
                if options.demo_layout && matches!(options.mode, WindowMode::Tiling) {
                    layout.set_window_floating(4, true);
                }
                layout
            },
            status_bar: options.status_bar,
            external_polybar: options.external_polybar,
            last_action: "Ready: desktop active".to_string(),
            runner: match options.role {
                role::SessionRole::Desktop => Runner::new(),
                role => Runner::with_role(role),
            },
            session_control: std::env::var_os("ALPENGLOW_SESSION_CONTROL").is_some(),
            role: options.role,
            launcher: None,
            settings: None,
            notifications,
            terminal: None,
            terminal_open: false,
            terminal_input: String::new(),
            #[cfg(feature = "compositor")]
            compositor_events: compositor_rx,
            #[cfg(feature = "compositor")]
            compositor_surfaces: std::collections::HashMap::new(),
        };
        desktop.runner.query = desktop.query.clone();
        desktop.refresh_runner();
        desktop
    }

    fn set_query(&mut self, query: String, cx: &mut Context<Self>) {
        self.query = query.clone();
        self.runner.query = query;
        self.refresh_runner();
        self.changed(cx);
    }

    fn set_last_action(&mut self, title: impl Into<String>, detail: impl Into<String>) {
        let title = title.into();
        let detail = detail.into();
        self.last_action = format!("{title}: {detail}");
        self.layout.set_focused_window_content(title, detail);
    }

    fn set_action_log(&mut self, title: impl Into<String>, detail: impl Into<String>) {
        self.last_action = format!("{}: {}", title.into(), detail.into());
    }

    fn focus_targets(&self) -> Vec<WindowTarget> {
        self.layout
            .windows()
            .into_iter()
            .map(|window| WindowTarget {
                id: window.id,
                title: window.title,
                focused: window.focused,
                floating: window.floating,
            })
            .collect()
    }

    fn refresh_runner(&mut self) {
        let windows = self.focus_targets();
        self.runner.update_with_windows(&windows);
    }

    fn apply(&mut self, action: PluginAction, cx: &mut Context<Self>) {
        match action {
            PluginAction::FocusWindow { id } => {
                self.layout.focus_window(id);
                let focused = self.layout.focused_title().to_string();
                self.set_action_log("Focus window", focused);
            }
            PluginAction::SetWindowMode { mode } => {
                self.mode = mode;
                self.layout.set_window_mode(&self.mode);
                self.set_last_action("Window mode", self.mode.label());
                let _ =
                    session::dispatch(&session::SessionRequest::SetWindowMode { mode: self.mode });
            }
            PluginAction::CycleWindowMode => {
                self.mode = self.mode.next();
                self.layout.set_window_mode(&self.mode);
                self.set_last_action("Window mode", self.mode.label());
                let _ =
                    session::dispatch(&session::SessionRequest::SetWindowMode { mode: self.mode });
            }
            PluginAction::Layout { action } => {
                self.layout.apply(&action);
                self.set_last_action(action.title(), self.layout.summary());
                let _ = session::dispatch(&session::SessionRequest::Layout { action });
            }
            PluginAction::ShowStatusBar => {
                self.status_bar = true;
                self.set_last_action("Status bar", "enabled");
            }
            PluginAction::HideStatusBar => {
                self.status_bar = false;
                self.set_last_action("Status bar", "disabled");
            }
            PluginAction::ToggleStatusBar => {
                self.toggle_status_bar(cx);
                self.set_last_action(
                    "Status bar",
                    if self.status_bar {
                        "enabled"
                    } else {
                        "disabled"
                    },
                );
            }
            PluginAction::ToggleSettings => {
                if let Some(handle) = self.settings {
                    let _ = handle.update(cx, |_, window, _| window.remove_window());
                    self.settings = None;
                    self.set_last_action("Settings", "closed");
                } else {
                    open_or_focus_settings(&cx.entity(), cx);
                    self.set_last_action("Settings", "opened");
                }
            }
            PluginAction::OpenSettings => {
                open_or_focus_settings(&cx.entity(), cx);
                self.set_last_action("Settings", "opened");
            }
            PluginAction::CloseSettings => {
                if let Some(handle) = self.settings {
                    let _ = handle.update(cx, |_, window, _| window.remove_window());
                    self.settings = None;
                }
                self.set_last_action("Settings", "closed");
            }
            PluginAction::FactoryReset => {
                crate::config::Config::factory_reset();
                self.set_last_action(
                    "Factory reset",
                    "settings cleared; restart to restore defaults",
                );
            }
            PluginAction::Desktop { action } => {
                let resolved = action
                    .resolve()
                    .map(|command| command.display())
                    .unwrap_or_else(|| "no command available".to_string());
                if session::dispatch(&session::SessionRequest::DesktopAction {
                    action: action.clone(),
                })
                .is_ok()
                {
                    self.set_last_action(action.title(), format!("session {resolved}"));
                } else {
                    match de::run(&action) {
                        de::RunResult::Spawned(command) => self.set_last_action(
                            action.title(),
                            format!("local {}", command.display()),
                        ),
                        de::RunResult::MissingCommand => {
                            self.set_last_action(action.title(), "no command available")
                        }
                    }
                }
            }
            PluginAction::Launch { program } => {
                self.set_last_action(program.clone(), "app launch");
                let _ = Command::new(program).spawn();
            }
            PluginAction::Shell { command } => {
                if self.terminal_open {
                    self.terminal_write(&command);
                } else {
                    self.set_last_action("Shell", command.clone());
                    let _ = Command::new("sh").arg("-c").arg(command).spawn();
                }
            }
            PluginAction::TerminalClear => {
                if let Some(ref term) = self.terminal {
                    term.clear();
                    self.last_action = "Terminal: cleared".to_string();
                }
                self.changed(cx);
            }
            PluginAction::ToggleTerminal => {
                self.toggle_terminal(cx);
            }
            PluginAction::TerminalWrite { line } => {
                self.terminal_write(&line);
                self.changed(cx);
            }
            PluginAction::ClearQuery => {
                self.set_query(String::new(), cx);
                self.set_action_log("Clear query", "cleared");
            }
            PluginAction::CycleSelection { forward } => {
                if forward {
                    self.runner.select_next();
                } else {
                    self.runner.select_previous();
                }
                self.set_action_log("Selection", self.runner.selection_label());
            }
            PluginAction::SelectResult { index } => {
                self.runner.select(index);
                self.set_action_log("Selection", self.runner.selection_label());
            }
            PluginAction::None => {}
        }
        self.refresh_runner();
        self.changed(cx);
    }

    fn changed(&mut self, cx: &mut Context<Self>) {
        write_external_bar_state(self);
        cx.notify();
        cx.emit(DesktopEvent::Changed);
    }

    #[cfg(feature = "compositor")]
    fn poll_compositor(&mut self) {
        let Some(ref events) = self.compositor_events else {
            return;
        };
        while let Ok(event) = events.try_recv() {
            match event {
                compositor::CompositorEvent::Created {
                    id,
                    title,
                    app_id: _,
                } => {
                    // Create a new layout pane for this surface
                    let window_id = self.layout.split_next(title.clone());
                    self.compositor_surfaces.insert(id, window_id);
                    self.last_action = format!("Compositor: {title} connected");
                }
                compositor::CompositorEvent::Updated { id } => {
                    if self.compositor_surfaces.contains_key(&id) {
                        self.layout.set_focused_window_content("Surface", "updated");
                    }
                }
                compositor::CompositorEvent::Closed { id } => {
                    if let Some(window_id) = self.compositor_surfaces.remove(&id) {
                        self.layout.set_window_floating(window_id, true);
                        self.last_action = format!("Compositor: surface {id} closed");
                    }
                }
            }
        }
    }

    fn toggle_status_bar(&mut self, cx: &mut Context<Self>) {
        self.status_bar = !self.status_bar;
        self.changed(cx);
    }

    fn toggle_terminal(&mut self, cx: &mut Context<Self>) {
        self.terminal_open = !self.terminal_open;
        if self.terminal_open && self.terminal.is_none() {
            self.terminal = terminal::TerminalConsole::spawn();
            if self.terminal.is_some() {
                self.last_action = "Terminal: opened".to_string();
            } else {
                self.last_action = "Terminal: spawn failed".to_string();
                self.terminal_open = false;
            }
        } else if !self.terminal_open {
            self.terminal = None;
            self.last_action = "Terminal: closed".to_string();
        }
        self.changed(cx);
    }

    fn terminal_write(&mut self, line: &str) {
        if let Some(ref term) = self.terminal {
            let trimmed = line.trim().to_string();
            if trimmed == "exit" || trimmed == "quit" {
                self.terminal = None;
                self.terminal_open = false;
                self.last_action = "Terminal: closed".to_string();
            } else {
                term.write(&trimmed);
                self.last_action = format!("$ {trimmed}");
            }
        }
    }
}

struct DesktopWindow {
    desktop: Entity<DesktopModel>,
}

struct InstallerWindow {
    source: PathBuf,
    target: PathBuf,
    status: String,
}

impl InstallerWindow {
    fn new(source: PathBuf, target: PathBuf) -> Self {
        let status = format!("Ready to install to {}", target.display());
        Self {
            source,
            target,
            status,
        }
    }

    fn install(&mut self, _: &gpui::ClickEvent, _: &mut gpui::Window, cx: &mut Context<Self>) {
        let result = copy_install_image(&self.source, &self.target);
        self.status = match result {
            Ok(bytes) => format!("Wrote {bytes} bytes to {}", self.target.display()),
            Err(error) => format!("Install failed: {error}"),
        };
        cx.notify();
    }
}

fn copy_install_image(source: &PathBuf, target: &PathBuf) -> std::io::Result<u64> {
    std::fs::File::open(source).and_then(|mut input| {
        std::fs::OpenOptions::new()
            .write(true)
            .open(target)
            .and_then(|mut output| std::io::copy(&mut input, &mut output))
    })
}

impl Render for InstallerWindow {
    fn render(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) -> impl IntoElement {
        let source = self.source.display().to_string();
        let target = self.target.display().to_string();
        let status = self.status.clone();

        div()
            .size_full()
            .bg(rgb(0x050505))
            .text_color(rgb(TEXT))
            .font(ui_font())
            .p(px(24.))
            .flex()
            .flex_col()
            .gap(px(18.))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        div()
                            .w(px(14.))
                            .h(px(14.))
                            .rounded(px(7.))
                            .bg(rgb(0xffffff)),
                    )
                    .child(
                        div()
                            .text_size(px(18.))
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("Alpenglow Installer"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(10.))
                    .child(step_pill("1 Choose disk", true))
                    .child(step_pill("2 Review", false))
                    .child(step_pill("3 Install", false)),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_end()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(30.))
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("Select disk"),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(rgb(TEXT_DIM))
                            .child("This will overwrite the selected disk."),
                    ),
            )
            .child(installer_field("Source", source))
            .child(installer_field("Target", target))
            .child(installer_field("Status", status))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .pt(px(16.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(TEXT_DIM))
                            .child("No changes are made until Install is clicked."),
                    )
                    .child(
                        div()
                            .id("alpenglowed-installer-install")
                            .px(px(22.))
                            .py(px(12.))
                            .rounded(px(8.))
                            .bg(rgb(0xffffff))
                            .cursor_pointer()
                            .on_click(cx.listener(Self::install))
                            .child(
                                div()
                                    .text_color(rgb(0x000000))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child("Install"),
                            ),
                    ),
            )
    }
}

fn step_pill(label: &'static str, active: bool) -> Div {
    div()
        .px(px(14.))
        .py(px(9.))
        .rounded(px(8.))
        .border_1()
        .border_color(rgb(if active { 0xffffff } else { BORDER }))
        .bg(rgb(if active { 0xffffff } else { SURFACE_2 }))
        .text_color(rgb(if active { 0x000000 } else { TEXT_DIM }))
        .text_size(px(13.))
        .font_weight(if active {
            gpui::FontWeight::BOLD
        } else {
            gpui::FontWeight::NORMAL
        })
        .child(label)
}

fn installer_field(label: &'static str, value: String) -> Div {
    div()
        .rounded(px(8.))
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE_2))
        .p(px(14.))
        .flex()
        .flex_col()
        .gap(px(6.))
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(TEXT_DIM))
                .child(label),
        )
        .child(div().text_size(px(14.)).text_color(rgb(TEXT)).child(value))
}

impl DesktopWindow {
    fn new(desktop: Entity<DesktopModel>, cx: &mut Context<Self>) -> Self {
        cx.subscribe(&desktop, |_, _, _: &DesktopEvent, cx| {
            cx.notify();
        })
        .detach();
        Self { desktop }
    }

    fn render_layout(desktop: &Entity<DesktopModel>, view: &LayoutView) -> Div {
        match view {
            LayoutView::Window(window) => Self::render_window(desktop, window, None),
            LayoutView::Container(container) => {
                let mut node = div()
                    .size_full()
                    .flex()
                    .gap(px(12.))
                    .when(matches!(container.axis, Axis::Column), |div| div.flex_col());
                for child in &container.children {
                    node = node.child(Self::render_child(desktop, child));
                }
                node
            }
        }
    }

    fn render_child(desktop: &Entity<DesktopModel>, child: &LayoutChildView) -> Div {
        let mut slot = div().min_w(px(0.)).min_h(px(0.)).child(match &child.node {
            LayoutView::Window(window) => Self::render_window(desktop, window, Some(child.grow)),
            _ => Self::render_layout(desktop, &child.node),
        });
        slot.style().flex_grow = Some(child.grow.max(0.1));
        slot.style().flex_shrink = Some(1.);
        slot
    }

    fn render_window(
        desktop: &Entity<DesktopModel>,
        window: &LayoutWindowView,
        _grow: Option<f32>,
    ) -> Div {
        let border = if window.focused { ACCENT } else { BORDER };
        let lines = Self::pane_lines(window);
        let window_id = window.id;
        let desktop = desktop.clone();
        let panel = div()
            .id(SharedString::from(format!("pane-{window_id}")))
            .size_full()
            .rounded(px(10.))
            .bg(rgb(SURFACE_2))
            .border_1()
            .border_color(rgb(border))
            .p(px(16.))
            .flex()
            .flex_col()
            .gap(px(8.))
            .cursor_pointer()
            .on_click(move |_, _, cx| {
                desktop.update(cx, |desktop, cx| {
                    desktop.layout.focus_window(window_id);
                    desktop.changed(cx);
                });
            })
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(TEXT_DIM))
                    .when(window.detail != "Ready", |label| {
                        label.child(window.detail.clone())
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .bg(rgb(SURFACE_3))
                    .rounded(px(8.))
                    .p(px(10.))
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .when(!lines.is_empty(), |panel| {
                        panel.children(lines.into_iter().map(Self::window_line))
                    }),
            );

        div().size_full().child(panel)
    }

    fn window_line(text: String) -> Div {
        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(TEXT_FAINT))
                    .child("~"),
            )
            .child(div().text_size(px(12.)).text_color(rgb(TEXT)).child(text))
    }

    fn pane_lines(window: &LayoutWindowView) -> Vec<String> {
        if window.detail.trim().is_empty() || window.detail == "Ready" {
            Vec::new()
        } else {
            vec![window.detail.clone()]
        }
    }

    fn render_floating_window(
        desktop: &Entity<DesktopModel>,
        window: &LayoutWindowView,
        _index: usize,
    ) -> Div {
        div()
            .absolute()
            .top(px(window.y))
            .left(px(window.x))
            .w(px(window.width))
            .h(px(window.height))
            .child(
                div()
                    .size_full()
                    .rounded(px(12.))
                    .bg(rgb(SURFACE_2))
                    .p(px(4.))
                    .child(Self::render_window(desktop, window, None)),
            )
    }

    fn render_floating_layer(desktop: &Entity<DesktopModel>, windows: &[LayoutWindowView]) -> Div {
        let mut layer = div()
            .absolute()
            .top(px(0.))
            .left(px(0.))
            .right(px(0.))
            .bottom(px(0.));
        for (index, window) in windows.iter().enumerate() {
            layer = layer.child(Self::render_floating_window(desktop, window, index));
        }
        layer
    }

    fn render_workspace(
        desktop: &Entity<DesktopModel>,
        layout: &LayoutView,
        mode: &WindowMode,
        cx: &App,
    ) -> Div {
        if matches!(mode, WindowMode::Monocle) {
            if let Some(window) = desktop.read(cx).layout.focused_window() {
                return div()
                    .size_full()
                    .child(Self::render_window(desktop, &window, None));
            }
        }
        let tiled = layout.tiled();
        let floating = layout.floating_windows();
        let mut root = div().size_full();
        if let Some(tiled) = tiled {
            root = root.child(Self::render_layout(desktop, &tiled));
        }
        if !floating.is_empty() {
            root = root.child(Self::render_floating_layer(desktop, &floating));
        }
        root
    }

    fn render_status_bar(desktop: &DesktopModel) -> Div {
        let metrics = TopBarMetrics::detect(desktop);
        let focused = desktop.layout.focused_title().to_string();
        let detail = desktop.layout.summary();
        let header = shell_top_bar_title_component(&focused, &detail);
        let caps = desktop.role.capabilities();
        let mut pills = Vec::new();
        if !caps.skinny_bar {
            pills.push(("mode".to_string(), desktop.mode.label().to_string()));
            pills.push(("layout".to_string(), desktop.layout.summary()));
        }
        pills.push(("time".to_string(), metrics.clock));
        pills.push(("date".to_string(), metrics.date));
        if caps.weather {
            pills.push(("temp".to_string(), metrics.temp));
        }
        pills.push(("power".to_string(), metrics.battery));
        if caps.wifi_pill {
            pills.push(("wifi".to_string(), metrics.wifi));
        }
        pills.push(("cpu".to_string(), metrics.load));
        pills.push(("mem".to_string(), metrics.memory));
        if !caps.skinny_bar {
            pills.push(("wl".to_string(), metrics.backend));
        }
        let bar_width = if caps.skinny_bar { px(720.) } else { px(1120.) };

        div()
            .absolute()
            .top(px(16.))
            .left(px(0.))
            .right(px(0.))
            .flex()
            .justify_center()
            .child(
                div()
                    .w(bar_width)
                    .rounded(px(12.))
                    .bg(rgb(SURFACE_2))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .px(px(18.))
                    .py(px(12.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(3.))
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .text_color(rgb(TEXT))
                                    .child(header.0),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(TEXT_DIM))
                                    .child(header.1),
                            ),
                    )
                    .child(
                        div().flex().gap(px(8.)).children(
                            pills
                                .into_iter()
                                .map(|(label, value)| Self::status_pill(label, value)),
                        ),
                    ),
            )
    }

    fn status_pill(label: String, value: String) -> Div {
        let metric = shell_top_bar_metric_component(&label, &value);
        div()
            .h(px(32.))
            .px(px(12.))
            .rounded(px(16.))
            .bg(rgb(SURFACE_3))
            .border_1()
            .border_color(rgb(BORDER_SOFT))
            .flex()
            .items_center()
            .gap(px(6.))
            .child(
                div()
                    .text_size(px(10.))
                    .text_color(rgb(TEXT_FAINT))
                    .child(metric.0),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(TEXT))
                    .child(metric.1),
            )
    }

    fn status_value_pill(value: String) -> Div {
        div()
            .h(px(32.))
            .px(px(12.))
            .rounded(px(16.))
            .bg(rgb(SURFACE_3))
            .border_1()
            .border_color(rgb(BORDER_SOFT))
            .flex()
            .items_center()
            .text_size(px(11.))
            .text_color(rgb(TEXT))
            .child(value)
    }
}

fn shell_text_component(component: &str, props: &[(&str, &str)]) -> String {
    let mut ctx = TemplateContext::new();
    for (key, value) in props {
        ctx.set(*key, TemplateValue::Str((*value).to_string()));
    }
    render_component_file_to_html(SHELL_CREPUS, component, &ctx)
        .map(|html| html_text_content(&html))
        .unwrap_or_default()
}

fn shell_header_component(component: &str) -> (String, String) {
    render_component_file_to_html(SHELL_CREPUS, component, &TemplateContext::new())
        .map(|html| {
            let texts = html_tag_texts(&html, &["h1", "p"]);
            let title = texts
                .first()
                .cloned()
                .unwrap_or_else(|| "Settings".to_string());
            let subtitle = texts
                .get(1)
                .cloned()
                .unwrap_or_else(|| "Desktop, launcher, session, and system controls".to_string());
            (title, subtitle)
        })
        .unwrap_or_else(|_| {
            (
                "Settings".to_string(),
                "Desktop, launcher, session, and system controls".to_string(),
            )
        })
}

fn shell_list_component(component: &str) -> Vec<String> {
    render_component_file_to_html(SHELL_CREPUS, component, &TemplateContext::new())
        .map(|html| html_list_items(&html))
        .unwrap_or_default()
}

fn shell_row_component(marker: &str, title: &str, subtitle: &str) -> (String, String, String) {
    let mut ctx = TemplateContext::new();
    ctx.set("marker", TemplateValue::Str(marker.to_string()));
    ctx.set("title", TemplateValue::Str(title.to_string()));
    ctx.set("subtitle", TemplateValue::Str(subtitle.to_string()));
    render_component_file_to_html(SHELL_CREPUS, "LauncherRow", &ctx)
        .map(|html| {
            let texts = html_tag_texts(&html, &["strong", "h2", "p"]);
            let marker = texts.first().cloned().unwrap_or_else(|| marker.to_string());
            let title = texts.get(1).cloned().unwrap_or_else(|| title.to_string());
            let subtitle = texts
                .get(2)
                .cloned()
                .unwrap_or_else(|| subtitle.to_string());
            (marker, title, subtitle)
        })
        .unwrap_or_else(|_| (marker.to_string(), title.to_string(), subtitle.to_string()))
}

fn shell_bar_component(query: &str, meta: &str) -> (String, String) {
    let mut ctx = TemplateContext::new();
    ctx.set("query", TemplateValue::Str(query.to_string()));
    ctx.set("meta", TemplateValue::Str(meta.to_string()));
    render_component_file_to_html(SHELL_CREPUS, "LauncherBar", &ctx)
        .map(|html| {
            let texts = html_tag_texts(&html, &["h1", "p"]);
            let query = texts.first().cloned().unwrap_or_else(|| query.to_string());
            let meta = texts.get(1).cloned().unwrap_or_else(|| meta.to_string());
            (query, meta)
        })
        .unwrap_or_else(|_| (query.to_string(), meta.to_string()))
}

fn shell_section_header_component(title: &str, detail: &str) -> (String, String) {
    let mut ctx = TemplateContext::new();
    ctx.set("title", TemplateValue::Str(title.to_string()));
    ctx.set("detail", TemplateValue::Str(detail.to_string()));
    render_component_file_to_html(SHELL_CREPUS, "SettingsSectionHeader", &ctx)
        .map(|html| {
            let texts = html_tag_texts(&html, &["h2", "p"]);
            let title = texts.first().cloned().unwrap_or_else(|| title.to_string());
            let detail = texts.get(1).cloned().unwrap_or_else(|| detail.to_string());
            (title, detail)
        })
        .unwrap_or_else(|_| (title.to_string(), detail.to_string()))
}

fn shell_top_bar_title_component(title: &str, subtitle: &str) -> (String, String) {
    let mut ctx = TemplateContext::new();
    ctx.set("title", TemplateValue::Str(title.to_string()));
    ctx.set("subtitle", TemplateValue::Str(subtitle.to_string()));
    render_component_file_to_html(SHELL_CREPUS, "TopBarTitle", &ctx)
        .map(|html| {
            let texts = html_tag_texts(&html, &["h1", "p"]);
            let title = texts.first().cloned().unwrap_or_else(|| title.to_string());
            let subtitle = texts
                .get(1)
                .cloned()
                .unwrap_or_else(|| subtitle.to_string());
            (title, subtitle)
        })
        .unwrap_or_else(|_| (title.to_string(), subtitle.to_string()))
}

fn shell_top_bar_metric_component(label: &str, value: &str) -> (String, String) {
    let mut ctx = TemplateContext::new();
    ctx.set("label", TemplateValue::Str(label.to_string()));
    ctx.set("value", TemplateValue::Str(value.to_string()));
    render_component_file_to_html(SHELL_CREPUS, "TopBarMetric", &ctx)
        .map(|html| {
            let texts = html_tag_texts(&html, &["strong", "p"]);
            let label = texts.first().cloned().unwrap_or_else(|| label.to_string());
            let value = texts.get(1).cloned().unwrap_or_else(|| value.to_string());
            (label, value)
        })
        .unwrap_or_else(|_| (label.to_string(), value.to_string()))
}

fn shell_status_row_component(mode: &str, layout: &str, focused: &str) -> Vec<String> {
    let mut ctx = TemplateContext::new();
    ctx.set("mode", TemplateValue::Str(mode.to_string()));
    ctx.set("layout", TemplateValue::Str(layout.to_string()));
    ctx.set("focused", TemplateValue::Str(focused.to_string()));
    render_component_file_to_html(SHELL_CREPUS, "SettingsStatusRow", &ctx)
        .map(|html| html_list_items(&html))
        .unwrap_or_else(|_| vec![mode.to_string(), layout.to_string(), focused.to_string()])
}

struct TopBarMetrics {
    clock: String,
    date: String,
    battery: String,
    temp: String,
    wifi: String,
    load: String,
    memory: String,
    backend: String,
}

impl TopBarMetrics {
    fn detect(desktop: &DesktopModel) -> Self {
        Self {
            clock: date_value("+%H:%M").unwrap_or_else(|| "--:--".to_string()),
            date: date_value("+%a %b %e").unwrap_or_else(|| "date unavailable".to_string()),
            battery: battery_value().unwrap_or_else(|| "battery unavailable".to_string()),
            temp: if desktop.role.capabilities().weather {
                temp_value().unwrap_or_else(|| "--°".to_string())
            } else {
                String::new()
            },
            wifi: if desktop.role.capabilities().wifi_pill {
                wifi_value().unwrap_or_else(|| "wifi unavailable".to_string())
            } else {
                String::new()
            },
            load: cpu_value().unwrap_or_else(|| "cpu unavailable".to_string()),
            memory: memory_value().unwrap_or_else(|| "memory unavailable".to_string()),
            backend: top_bar_backend(desktop),
        }
    }
}

fn date_value(format: &str) -> Option<String> {
    let output = Command::new("date").arg(format).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

fn battery_value() -> Option<String> {
    let entries = fs::read_dir("/sys/class/power_supply").ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let kind = fs::read_to_string(path.join("type")).ok()?;
        if kind.trim() != "Battery" {
            continue;
        }
        let capacity = fs::read_to_string(path.join("capacity")).ok()?;
        let status = fs::read_to_string(path.join("status")).ok()?;
        return Some(format!(
            "{}% {}",
            capacity.trim(),
            status.trim().to_lowercase()
        ));
    }
    None
}

fn temp_value() -> Option<String> {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    static CACHE: std::sync::OnceLock<Mutex<(Instant, String)>> = std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new((Instant::now(), String::new())));
    let mut guard = cache.lock().ok()?;
    if guard.0.elapsed() < Duration::from_secs(300) {
        return Some(guard.1.clone()).filter(|s| !s.is_empty());
    }
    let output = Command::new("sh")
        .arg("-c")
        .arg("curl -s 'wttr.in?format=%t' 2>/dev/null")
        .output()
        .ok()?;
    let temp = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if temp.is_empty() {
        return None;
    }
    *guard = (Instant::now(), temp.clone());
    Some(temp)
}

fn wifi_value() -> Option<String> {
    let text = fs::read_to_string("/proc/net/wireless").ok()?;
    for line in text.lines().skip(2) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }
        let iface = parts.first()?.trim_end_matches(':');
        if iface == "lo" {
            continue;
        }
        let link_quality = parts.get(2).filter(|q| **q != "0").or(parts.get(3))?;
        let signal = parts.get(3).or(parts.get(2))?;
        return Some(format!("{iface} {link_quality}/{signal} db"));
    }
    None
}

fn load_value() -> Option<String> {
    let text = fs::read_to_string("/proc/loadavg").ok()?;
    let first = text.split_whitespace().next()?;
    Some(first.to_string())
}

fn cpu_value() -> Option<String> {
    let text = fs::read_to_string("/proc/stat").ok()?;
    let line = text.lines().next()?;
    let vals: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .filter_map(|v| v.parse().ok())
        .collect();
    if vals.len() < 4 {
        return None;
    }
    let total: u64 = vals.iter().sum();
    let idle = vals[3];
    if total == 0 {
        return None;
    }
    // ponytail: since-boot avg, not delta between samples
    Some(format!("{}%", ((total - idle) * 100) / total))
}

fn memory_value() -> Option<String> {
    let text = fs::read_to_string("/proc/meminfo").ok()?;
    let mut total_kb = None;
    let mut available_kb = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            total_kb = rest
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<u64>().ok());
        } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
            available_kb = rest
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<u64>().ok());
        }
    }
    let total_kb = total_kb?;
    let available_kb = available_kb?;
    if total_kb == 0 {
        return None;
    }
    let used_pct = ((total_kb.saturating_sub(available_kb)) * 100) / total_kb;
    Some(format!("{used_pct}% used"))
}

fn top_bar_backend(desktop: &DesktopModel) -> String {
    let state = de::DesktopState::detect(desktop.mode.label());
    polybar_backend(&state)
}

fn polybar_backend(state: &de::DesktopState) -> String {
    let display = state.display.as_deref().unwrap_or("no-display").to_string();
    let backend = if state.wayland { "wayland" } else { "offline" };
    format!("{backend} {display}")
}

fn html_list_items(html: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cursor = 0;
    while let Some(open_rel) = html[cursor..].find("<li") {
        let open = cursor + open_rel;
        let Some(content_start_rel) = html[open..].find('>') else {
            break;
        };
        let content_start = open + content_start_rel + 1;
        let Some(close_rel) = html[content_start..].find("</li>") else {
            break;
        };
        let content_end = content_start + close_rel;
        let text = html[content_start..content_end]
            .replace("&quot;", "\"")
            .replace("&amp;", "&")
            .replace("&#39;", "'")
            .replace("&lt;", "<")
            .replace("&gt;", ">");
        let text = text.trim();
        if !text.is_empty() {
            lines.push(text.to_string());
        }
        cursor = content_end + "</li>".len();
    }
    lines
}

fn html_text_content(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(ch),
            _ => {}
        }
    }
    text.replace("&quot;", "\"")
        .replace("&amp;", "&")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .trim()
        .to_string()
}

fn html_tag_texts(html: &str, tags: &[&str]) -> Vec<String> {
    tags.iter()
        .filter_map(|tag| {
            let open = format!("<{tag}>");
            let close = format!("</{tag}>");
            let start = html.find(&open)? + open.len();
            let end = html[start..].find(&close)? + start;
            Some(html_text_content(&html[start..end]))
        })
        .collect()
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
            if !matches!(action, PluginAction::None) {
                let title = self
                    .desktop
                    .read(cx)
                    .runner
                    .selected_result()
                    .map(|r| r.title.clone());
                self.desktop.update(cx, |desktop, cx| {
                    if let Some(title) = &title {
                        desktop.runner.record_recent(title);
                    }
                    desktop.apply(action, cx);
                });
                if let Some(handle) = self.desktop.read(cx).launcher {
                    let _ = handle.update(cx, |_, window, _| window.remove_window());
                }
                self.desktop.update(cx, |desktop, cx| {
                    desktop.launcher = None;
                    desktop.changed(cx);
                });
            }
        }
    }

    fn select_and_confirm(&mut self, index: usize, cx: &mut Context<Self>) {
        let count = self.desktop.read(cx).runner.results.len();
        if count == 0 || index >= count {
            return;
        }
        self.desktop.update(cx, |desktop, cx| {
            desktop.runner.select(index);
            desktop.changed(cx);
        });
        self.confirm(cx);
    }

    fn delete_word(&mut self, cx: &mut Context<Self>) {
        let query = self.desktop.read(cx).query.clone();
        let trimmed_end = query.trim_end();
        let mut next = trimmed_end.to_string();
        if let Some(idx) = next.rfind(|c: char| c.is_whitespace()) {
            next.truncate(idx);
        } else {
            next.clear();
        }
        self.desktop.update(cx, |desktop, cx| {
            desktop.set_query(next, cx);
        });
    }

    fn select_next(&mut self, cx: &mut Context<Self>) {
        self.desktop.update(cx, |desktop, cx| {
            desktop.runner.select_next();
            desktop.changed(cx);
        });
    }

    fn select_previous(&mut self, cx: &mut Context<Self>) {
        self.desktop.update(cx, |desktop, cx| {
            desktop.runner.select_previous();
            desktop.changed(cx);
        });
    }

    fn render_bar(&self, cx: &App) -> impl IntoElement {
        let desktop = self.desktop.read(cx);
        let show_results = !desktop.query.trim().is_empty();
        let selection = desktop.runner.selection_label();
        let subtitle = if desktop.query.trim().is_empty() {
            shell_text_component(
                "LauncherEmpty",
                &[("message", "type to search desktop actions")],
            )
        } else {
            desktop
                .runner
                .selected_result()
                .map(|result| format!("{} via {}", result.subtitle, result.plugin_id))
                .unwrap_or_else(|| "no match".to_string())
        };
        let meta = shell_text_component(
            "LauncherMeta",
            &[("selection", &selection), ("subtitle", &subtitle)],
        );
        let query = if desktop.query.is_empty() {
            " ".to_string()
        } else {
            desktop.query.clone()
        };
        let bar = shell_bar_component(&query, &meta);

        div()
            .w(px(680.))
            .h(px(48.))
            .when(show_results, |bar| bar.rounded_t(px(6.)))
            .when(!show_results, |bar| bar.rounded(px(6.)))
            .bg(rgb(SURFACE_2))
            .border_1()
            .border_color(rgb(BORDER))
            .px(px(18.))
            .flex()
            .items_center()
            .gap(px(12.))
            .child(
                div()
                    .flex_1()
                    .text_size(px(16.))
                    .text_color(rgb(TEXT))
                    .child(bar.0),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(TEXT_DIM))
                    .child(bar.1),
            )
    }

    fn render_results(&self, cx: &App) -> impl IntoElement {
        let desktop = self.desktop.read(cx);

        if desktop.query.trim().is_empty() {
            return div();
        }

        if desktop.runner.results.is_empty() {
            return div()
                .w(px(680.))
                .rounded_b(px(6.))
                .bg(rgb(SURFACE_2))
                .border_1()
                .border_color(rgb(BORDER))
                .border_t_0()
                .px(px(18.))
                .py(px(12.))
                .text_size(px(12.))
                .text_color(rgb(TEXT_DIM))
                .child(shell_text_component(
                    "LauncherNoResults",
                    &[("message", "No results")],
                ));
        }

        div()
            .w(px(680.))
            .rounded_b(px(6.))
            .bg(rgb(SURFACE_2))
            .border_1()
            .border_color(rgb(BORDER))
            .border_t_0()
            .p(px(6.))
            .gap(px(2.))
            .children(
                desktop
                    .runner
                    .results
                    .iter()
                    .enumerate()
                    .map(|(index, result)| {
                        let selected = index == desktop.runner.selected;
                        let desktop = self.desktop.clone();
                        let detail = format!("{} via {}", result.subtitle, result.plugin_id);
                        let row = shell_row_component(
                            if selected { ">" } else { "$" },
                            &result.title,
                            &detail,
                        );
                        div()
                            .id(SharedString::from(format!("launcher-result-{index}")))
                            .rounded(px(4.))
                            .bg(if selected {
                                rgb(ACCENT_DIM)
                            } else {
                                rgb(SURFACE_2)
                            })
                            .px(px(12.))
                            .py(px(8.))
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .cursor_pointer()
                            .on_click(move |_, _, cx| {
                                desktop.update(cx, |desktop, cx| {
                                    desktop.runner.select(index);
                                    desktop.changed(cx);
                                });
                                let action = desktop.read(cx).runner.confirm();
                                if let Some(action) = action {
                                    if !matches!(action, PluginAction::None) {
                                        desktop.update(cx, |desktop, cx| {
                                            desktop.apply(action, cx);
                                        });
                                        if let Some(handle) = desktop.read(cx).launcher {
                                            let _ = handle
                                                .update(cx, |_, window, _| window.remove_window());
                                        }
                                        desktop.update(cx, |desktop, cx| {
                                            desktop.launcher = None;
                                            desktop.changed(cx);
                                        });
                                    }
                                }
                            })
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(if selected {
                                        rgb(ACCENT)
                                    } else {
                                        rgb(TEXT_FAINT)
                                    })
                                    .child(row.0),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(px(13.))
                                    .text_color(if selected { rgb(TEXT) } else { rgb(TEXT_DIM) })
                                    .child(row.1),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(TEXT_FAINT))
                                    .child(row.2),
                            )
                    }),
            )
    }
}

struct SettingsWindow {
    desktop: Entity<DesktopModel>,
    section: SettingsSection,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SettingsSection {
    Windows,
    System,
    Interface,
    Session,
}

impl SettingsSection {
    fn label(self) -> &'static str {
        match self {
            Self::Windows => "Windows",
            Self::System => "System",
            Self::Interface => "Interface",
            Self::Session => "Session",
        }
    }

    fn detail(self) -> &'static str {
        match self {
            Self::Windows => "layout and pane flow",
            Self::System => "desktop and os actions",
            Self::Interface => "launcher and chrome",
            Self::Session => "state and shortcuts",
        }
    }

    fn from_env() -> Self {
        let value = std::env::args()
            .find_map(|arg| arg.strip_prefix("--settings-section=").map(str::to_string))
            .or_else(|| std::env::var("ALPENGLOWED_SETTINGS_SECTION").ok());
        match value.as_deref() {
            Some("system") => Self::System,
            Some("interface") => Self::Interface,
            Some("session") => Self::Session,
            _ => Self::Windows,
        }
    }
}

impl SettingsWindow {
    fn new(desktop: Entity<DesktopModel>, cx: &mut Context<Self>) -> Self {
        cx.subscribe(&desktop, |_, _, _: &DesktopEvent, cx| {
            cx.notify();
        })
        .detach();

        Self {
            desktop,
            section: SettingsSection::from_env(),
        }
    }

    fn mode_button(&self, label: &'static str, mode: WindowMode, active: bool) -> impl IntoElement {
        let desktop = self.desktop.clone();
        self.chip_button(
            SharedString::from(format!("mode-{label}")),
            label,
            active,
            move |desktop, cx| {
                desktop.update(cx, |desktop, cx| {
                    desktop.apply(PluginAction::SetWindowMode { mode }, cx);
                });
            },
            desktop,
        )
    }

    fn action_button(
        &self,
        label: &'static str,
        on_click: impl Fn(&Entity<DesktopModel>, &mut App) + 'static,
    ) -> impl IntoElement {
        let desktop = self.desktop.clone();
        self.chip_button(
            SharedString::from(format!("settings-{label}")),
            label,
            false,
            on_click,
            desktop,
        )
    }

    fn chip_button(
        &self,
        id: SharedString,
        label: &'static str,
        active: bool,
        on_click: impl Fn(&Entity<DesktopModel>, &mut App) + 'static,
        desktop: Entity<DesktopModel>,
    ) -> impl IntoElement {
        div()
            .id(id)
            .px(px(12.))
            .py(px(8.))
            .rounded(px(4.))
            .bg(rgb(if active { ACCENT } else { SURFACE_3 }))
            .border_1()
            .border_color(rgb(if active { ACCENT } else { BORDER }))
            .text_size(px(12.))
            .text_color(rgb(if active { SURFACE } else { TEXT }))
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

    fn layout_action_button(
        &self,
        label: &'static str,
        action: layout::LayoutAction,
    ) -> impl IntoElement {
        self.action_button(label, move |desktop, cx| {
            desktop.update(cx, |desktop, cx| {
                desktop.apply(
                    PluginAction::Layout {
                        action: action.clone(),
                    },
                    cx,
                );
            });
        })
    }

    fn section_card(
        &self,
        title: &'static str,
        detail: impl Into<String>,
        content: impl IntoElement,
    ) -> Div {
        let detail = detail.into();
        let header = shell_section_header_component(title, &detail);
        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .border_t_1()
            .border_color(rgb(BORDER_SOFT))
            .pt(px(14.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(3.))
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(rgb(TEXT))
                            .child(header.0),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(TEXT_DIM))
                            .child(header.1),
                    ),
            )
            .child(content)
    }

    fn nav_button(&self, cx: &Context<Self>, section: SettingsSection) -> impl IntoElement {
        let active = self.section == section;
        div()
            .id(SharedString::from(format!(
                "settings-nav-{}",
                section.label()
            )))
            .px(px(10.))
            .py(px(9.))
            .rounded(px(4.))
            .bg(rgb(if active { ACCENT_DIM } else { SURFACE }))
            .border_1()
            .border_color(rgb(if active { ACCENT } else { BORDER_SOFT }))
            .cursor_pointer()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(if active { TEXT } else { TEXT_DIM }))
                            .child(section.label()),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(if active { TEXT_DIM } else { TEXT_FAINT }))
                            .child(section.detail()),
                    ),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.section = section;
                cx.notify();
            }))
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let desktop = self.desktop.read(cx);
        let tiling = desktop.mode == WindowMode::Tiling;
        let floating = desktop.mode == WindowMode::Floating;
        let monocle = desktop.mode == WindowMode::Monocle;
        let stack = desktop.mode == WindowMode::Stack;
        let center = desktop.mode == WindowMode::Center;
        let grid = desktop.mode == WindowMode::Grid;
        let status_bar = if desktop.status_bar {
            "enabled"
        } else {
            "disabled"
        };
        let layout_summary = desktop.layout.summary();
        let layout_axis = desktop.layout.axis();
        let focused = desktop.layout.focused_title().to_string();
        let focused_detail = desktop
            .layout
            .view()
            .focused_detail()
            .unwrap_or_else(|| "Ready".to_string());
        let session_status = if desktop.session_control {
            "Connected to compositor"
        } else {
            "Running local fallbacks"
        };
        let last_action = desktop.last_action.clone();
        let settings_header = shell_header_component("SettingsHeader");
        let status_row =
            shell_status_row_component(desktop.mode.label(), &layout_summary, &focused);
        let shortcuts = shell_list_component("SettingsShortcuts");
        div().size_full().font(ui_font()).bg(rgb(SURFACE)).key_context("alpenglowed")
            .on_action(cx.listener(|this, _: &FocusBar, _, cx| {
                focus_or_open_launcher(&this.desktop, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleSettingsWindow, window, cx| {
                let id = window.window_handle().window_id();
                this.desktop.update(cx, move |desktop, cx| {
                    if desktop.settings.is_some_and(|handle| handle.window_id() == id) {
                        desktop.settings = None;
                        desktop.changed(cx);
                    }
                });
                window.remove_window();
            }))
            .on_action(cx.listener(|this, _: &ToggleStatusBarAction, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.toggle_status_bar(cx);
                });
            }))
            .child(
            div().size_full().bg(rgb(SURFACE)).p(px(18.)).child(
                div()
                    .size_full()
                    .rounded(px(6.))
                    .bg(rgb(SURFACE_2))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .p(px(18.))
                    .flex()
                    .flex_col()
                    .gap(px(16.))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .items_start()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(5.))
                                    .child(
                                        div()
                                            .text_size(px(17.))
                                            .text_color(rgb(TEXT))
                                            .child(settings_header.0),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(rgb(TEXT_DIM))
                                            .child(settings_header.1),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap(px(8.))
                                    .children(
                                        status_row
                                            .into_iter()
                                            .map(DesktopWindow::status_value_pill),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(24.))
                            .child(
                                div()
                                    .w(px(160.))
                                    .flex()
                                    .flex_col()
                                    .gap(px(4.))
                                    .child(self.nav_button(cx, SettingsSection::Windows))
                                    .child(self.nav_button(cx, SettingsSection::System))
                                    .child(self.nav_button(cx, SettingsSection::Interface))
                                    .child(self.nav_button(cx, SettingsSection::Session)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap(px(18.))
                                    .when(self.section == SettingsSection::Windows, |panel| {
                                        panel.child(
                                            self.section_card(
                                                "Modes",
                                                format!("root {layout_axis} • detail {focused_detail}"),
                                                div()
                                                    .flex()
                                                    .flex_wrap()
                                                    .gap(px(8.))
                                                    .child(self.mode_button(
                                                        "Tile windows",
                                                        WindowMode::Tiling,
                                                        tiling,
                                                    ))
                                                    .child(self.mode_button(
                                                        "Float windows",
                                                        WindowMode::Floating,
                                                        floating,
                                                    ))
                                                    .child(self.mode_button(
                                                        "Monocle",
                                                        WindowMode::Monocle,
                                                        monocle,
                                                    ))
                                                    .child(self.mode_button(
                                                        "Stack",
                                                        WindowMode::Stack,
                                                        stack,
                                                    ))
                                                    .child(self.mode_button(
                                                        "Center",
                                                        WindowMode::Center,
                                                        center,
                                                    ))
                                                    .child(self.mode_button(
                                                        "Grid",
                                                        WindowMode::Grid,
                                                        grid,
                                                    ))
                                                    .child(self.action_button(
                                                        "Cycle mode",
                                                        |desktop, cx| {
                                                            desktop.update(cx, |desktop, cx| {
                                                                desktop.apply(
                                                                    PluginAction::CycleWindowMode,
                                                                    cx,
                                                                );
                                                            });
                                                        },
                                                    ))
                                                    .child(self.layout_action_button(
                                                        "Split row",
                                                        layout::LayoutAction::SplitRow,
                                                    ))
                                                    .child(self.layout_action_button(
                                                        "Split column",
                                                        layout::LayoutAction::SplitColumn,
                                                    ))
                                                    .child(self.layout_action_button(
                                                        "Flip layout axis",
                                                        layout::LayoutAction::FlipAxis,
                                                    ))
                                            ),
                                        )
                                        .child(
                                            self.section_card(
                                                "Flow",
                                                "focus and grouping",
                                                div()
                                                    .flex()
                                                    .flex_wrap()
                                                    .gap(px(8.))
                                                    .child(self.layout_action_button(
                                                        "Focus next",
                                                        layout::LayoutAction::FocusNext,
                                                    ))
                                                    .child(self.layout_action_button(
                                                        "Focus previous",
                                                        layout::LayoutAction::FocusPrevious,
                                                    ))
                                                    .child(self.layout_action_button(
                                                        "Focus first",
                                                        layout::LayoutAction::FocusFirst,
                                                    ))
                                                    .child(self.layout_action_button(
                                                        "Focus last",
                                                        layout::LayoutAction::FocusLast,
                                                    ))
                                                    .child(self.layout_action_button(
                                                        "Toggle float",
                                                        layout::LayoutAction::ToggleFloat,
                                                    ))
                                                    .child(self.layout_action_button(
                                                        "Close focused",
                                                        layout::LayoutAction::CloseFocused,
                                                    ))
                                                    .child(self.layout_action_button(
                                                        "Center focused",
                                                        layout::LayoutAction::CenterFocused,
                                                    )),
                                            ),
                                        )
                                        .child(
                                            self.section_card(
                                                "Move",
                                                "position, swap, and reset",
                                                div()
                                                    .flex()
                                                    .flex_wrap()
                                                    .gap(px(8.))
                                                    .child(self.layout_action_button(
                                                        "Reset layout",
                                                        layout::LayoutAction::Reset,
                                                    ))
                                                    .child(self.layout_action_button(
                                                        "Move left",
                                                        layout::LayoutAction::MoveLeft,
                                                    ))
                                                    .child(self.layout_action_button(
                                                        "Move right",
                                                        layout::LayoutAction::MoveRight,
                                                    ))
                                                    .child(self.layout_action_button(
                                                        "Move up",
                                                        layout::LayoutAction::MoveUp,
                                                    ))
                                                    .child(self.layout_action_button(
                                                        "Move down",
                                                        layout::LayoutAction::MoveDown,
                                                    ))
                                                    .child(self.layout_action_button(
                                                        "Nudge left",
                                                        layout::LayoutAction::NudgeLeft,
                                                    ))
                                                    .child(self.layout_action_button(
                                                        "Nudge right",
                                                        layout::LayoutAction::NudgeRight,
                                                    ))
                                                    .child(self.layout_action_button(
                                                        "Nudge up",
                                                        layout::LayoutAction::NudgeUp,
                                                    ))
                                                    .child(self.layout_action_button(
                                                        "Nudge down",
                                                        layout::LayoutAction::NudgeDown,
                                                    ))
                                            ),
                                        )
                                        .child(
                                            self.section_card(
                                                "Resize",
                                                "grow, contract, rebalance",
                                                div()
                                                    .flex()
                                                    .flex_wrap()
                                                    .gap(px(8.))
                                                    .child(self.layout_action_button(
                                                        "Expand window",
                                                        layout::LayoutAction::ExpandWindow,
                                                    ))
                                                    .child(self.layout_action_button(
                                                        "Contract window",
                                                        layout::LayoutAction::ContractWindow,
                                                    ))
                                                    .child(self.layout_action_button(
                                                        "Grow focused",
                                                        layout::LayoutAction::GrowFocused,
                                                    ))
                                                    .child(self.layout_action_button(
                                                        "Shrink focused",
                                                        layout::LayoutAction::ShrinkFocused,
                                                    ))
                                                    .child(self.layout_action_button(
                                                        "Balance panes",
                                                        layout::LayoutAction::BalancePanes,
                                                    )),
                                            ),
                                        )
                                    })
                                    .when(self.section == SettingsSection::System, |panel| {
                                        panel.child(
                                            self.section_card(
                                                "Access",
                                                "launcher and workspace tools",
                                                div()
                                                    .flex()
                                                    .flex_wrap()
                                                    .gap(px(8.))
                                                    .child(self.desktop_action_button(
                                                        "Apps",
                                                        de::DesktopAction::Apps,
                                                    ))
                                                    .child(self.desktop_action_button(
                                                        "Terminal",
                                                        de::DesktopAction::Terminal,
                                                    ))
                                                    .child(self.desktop_action_button(
                                                        "Files",
                                                        de::DesktopAction::Files,
                                                    )),
                                            ),
                                        )
                                        .child(
                                            self.section_card(
                                                "Network",
                                                "wireless controls",
                                                div()
                                                    .flex()
                                                    .flex_wrap()
                                                    .gap(px(8.))
                                                    .child(self.desktop_action_button(
                                                        "Wi-Fi",
                                                        de::DesktopAction::Wifi,
                                                    ))
                                                    .child(self.desktop_action_button(
                                                        "Wi-Fi on",
                                                        de::DesktopAction::WifiOn,
                                                    ))
                                                    .child(self.desktop_action_button(
                                                        "Wi-Fi off",
                                                        de::DesktopAction::WifiOff,
                                                    )),
                                            ),
                                        )
                                        .child(
                                            self.section_card(
                                                "Audio",
                                                "device and volume controls",
                                                div()
                                                    .flex()
                                                    .flex_wrap()
                                                    .gap(px(8.))
                                                    .child(self.desktop_action_button(
                                                        "Audio",
                                                        de::DesktopAction::Audio,
                                                    ))
                                                    .child(self.desktop_action_button(
                                                        "Mute",
                                                        de::DesktopAction::AudioMute,
                                                    ))
                                                    .child(self.desktop_action_button(
                                                        "Volume up",
                                                        de::DesktopAction::AudioUp,
                                                    ))
                                                    .child(self.desktop_action_button(
                                                        "Volume down",
                                                        de::DesktopAction::AudioDown,
                                                    )),
                                            ),
                                        )
                                        .child(
                                            self.section_card(
                                                "Display",
                                                "monitors and overlays",
                                                div()
                                                    .flex()
                                                    .flex_wrap()
                                                    .gap(px(8.))
                                                    .child(self.desktop_action_button(
                                                        "Display",
                                                        de::DesktopAction::Display,
                                                    ))
                                                    .child(self.desktop_action_button(
                                                        "Notifications",
                                                        de::DesktopAction::Notifications,
                                                    ))
                                                    .child(self.desktop_action_button(
                                                        "Processes",
                                                        de::DesktopAction::Processes,
                                                    )),
                                            ),
                                        )
                                        .child(
                                            self.section_card(
                                                "Capture",
                                                "clipboard and screenshots",
                                                div()
                                                    .flex()
                                                    .flex_wrap()
                                                    .gap(px(8.))
                                                    .child(self.desktop_action_button(
                                                        "Screenshot",
                                                        de::DesktopAction::Screenshot,
                                                    ))
                                                    .child(self.desktop_action_button(
                                                        "Clipboard",
                                                        de::DesktopAction::Clipboard,
                                                    )),
                                            ),
                                        )
                                        .child(
                                            self.section_card(
                                                "Session",
                                                "lock and power controls",
                                                div()
                                                    .flex()
                                                    .flex_wrap()
                                                    .gap(px(8.))
                                                    .child(self.desktop_action_button(
                                                        "Lock",
                                                        de::DesktopAction::Lock,
                                                    ))
                                                    .child(self.desktop_action_button(
                                                        "Logout",
                                                        de::DesktopAction::Logout,
                                                    ))
                                                    .child(self.desktop_action_button(
                                                        "Suspend",
                                                        de::DesktopAction::Suspend,
                                                    ))
                                                    .child(self.desktop_action_button(
                                                        "Hibernate",
                                                        de::DesktopAction::Hibernate,
                                                    ))
                                                    .child(self.desktop_action_button(
                                                        "Reboot",
                                                        de::DesktopAction::Reboot,
                                                    ))
                                                    .child(self.desktop_action_button(
                                                        "Shutdown",
                                                        de::DesktopAction::Shutdown,
                                                    )),
                                            ),
                                        )
                                    })
                                    .when(self.section == SettingsSection::Interface, |panel| {
                                        panel.child(
                                            self.section_card(
                                                "Interface",
                                                format!("status bar {status_bar}"),
                                                div()
                                                    .flex()
                                                    .flex_wrap()
                                                    .gap(px(8.))
                                                    .child(self.action_button(
                                                        "Toggle status bar",
                                                        |desktop, cx| {
                                                            desktop.update(cx, |desktop, cx| {
                                                                desktop.toggle_status_bar(cx)
                                                            });
                                                        },
                                                    ))
                                                    .child(self.action_button(
                                                        "Focus launcher",
                                                        |desktop, cx| {
                                                            focus_or_open_launcher(desktop, cx);
                                                        },
                                                    ))
                                                    .child(self.action_button(
                                                        "Open settings",
                                                        |desktop, cx| {
                                                            open_or_focus_settings(desktop, cx);
                                                        },
                                                    ))
                                                    .child(self.action_button(
                                                        "Clear query",
                                                        |desktop, cx| {
                                                            desktop.update(cx, |desktop, cx| {
                                                                desktop.set_query(String::new(), cx);
                                                            });
                                                        },
                                                    )),
                                            ),
                                        )
                                    })
                                    .when(self.section == SettingsSection::Session, |panel| {
                                        panel.child(
                                            self.section_card(
                                                "Session",
                                                session_status,
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .gap(px(8.))
                                                    .child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(rgb(TEXT_DIM))
                                                            .child(format!("Focused pane: {focused}")),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(rgb(TEXT_DIM))
                                                            .child(format!(
                                                                "Mode: {}",
                                                                desktop.mode.label()
                                                            )),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(rgb(TEXT_DIM))
                                                            .child(format!(
                                                                "Layout axis: {layout_axis}"
                                                            )),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(rgb(TEXT_DIM))
                                                            .child(format!(
                                                                "Last action: {last_action}"
                                                            )),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(rgb(TEXT_DIM))
                                                            .child(
                                                                "Desktop actions show session dispatch or local fallback command",
                                                            ),
                                                    ),
                                            ),
                                        )
                                        .child(
                                            self.section_card(
                                                "Shortcuts",
                                                "keyboard-first shell actions",
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .gap(px(6.))
                                                    .children(shortcuts.into_iter().map(|line| {
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(rgb(TEXT_FAINT))
                                                            .child(line)
                                                    })),
                                            ),
                                        )
                                    }),
                            ),
                    ),
            ),
        )
    }
}

impl Render for LauncherWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .font(ui_font())
            .key_context("alpenglowed")
            .on_action(cx.listener(|this, _: &Confirm, _, cx| {
                this.confirm(cx);
            }))
            .on_action(cx.listener(|this, _: &ClearQueryAction, _, cx| {
                this.desktop.update(cx, |desktop, cx| {
                    desktop.set_query(String::new(), cx);
                });
            }))
            .on_action(cx.listener(|this, _: &SelectResult1, _, cx| {
                this.select_and_confirm(0, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectResult2, _, cx| {
                this.select_and_confirm(1, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectResult3, _, cx| {
                this.select_and_confirm(2, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectResult4, _, cx| {
                this.select_and_confirm(3, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectResult5, _, cx| {
                this.select_and_confirm(4, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectResult6, _, cx| {
                this.select_and_confirm(5, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectResult7, _, cx| {
                this.select_and_confirm(6, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectResult8, _, cx| {
                this.select_and_confirm(7, cx);
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
                    if event.keystroke.modifiers.platform {
                        this.delete_word(cx);
                    } else {
                        this.backspace(cx);
                    }
                    cx.stop_propagation();
                    return;
                }
                if key == "up" {
                    this.select_previous(cx);
                    cx.stop_propagation();
                    return;
                }
                if key == "down" {
                    this.select_next(cx);
                    cx.stop_propagation();
                    return;
                }
                if key == "enter" {
                    this.confirm(cx);
                    cx.stop_propagation();
                    return;
                }
                if key == "tab" {
                    let title = this
                        .desktop
                        .read(cx)
                        .runner
                        .selected_result()
                        .map(|r| r.title.clone());
                    if let Some(title) = title {
                        this.desktop.update(cx, |desktop, cx| {
                            desktop.set_query(title, cx);
                        });
                        cx.stop_propagation();
                        return;
                    }
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
                            .w(px(680.))
                            .flex()
                            .flex_col()
                            .gap(px(0.))
                            .child(self.render_bar(cx))
                            .child(self.render_results(cx)),
                    ),
            )
    }
}

impl DesktopWindow {
    fn render_terminal_panel(desktop: &DesktopModel) -> Div {
        let lines = desktop
            .terminal
            .as_ref()
            .map(|t| t.output())
            .unwrap_or_default();
        div()
            .absolute()
            .bottom(px(0.))
            .left(px(0.))
            .right(px(0.))
            .h(px(240.))
            .bg(rgb(0x0a0e14))
            .border_t_1()
            .border_color(rgb(BORDER))
            .p(px(12.))
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .mb(px(6.))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(0x88ff88))
                            .child("$ terminal"),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(TEXT_FAINT))
                            .child("type directly, Ctrl-C to interrupt, 'exit' to close"),
                    ),
            )
            .child(
                div().flex_1().flex().flex_col().gap(px(2.)).children(
                    lines
                        .iter()
                        .rev()
                        .take(40)
                        .rev()
                        .map(|line| Self::render_terminal_line(line)),
                ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .px(px(4.))
                    .py(px(6.))
                    .bg(rgb(0x11151a))
                    .rounded(px(6.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(0x88ff88))
                            .child("$"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(12.))
                            .text_color(rgb(TEXT))
                            .child(desktop.terminal_input.clone()),
                    )
                    .child(div().w(px(6.)).h(px(14.)).bg(rgb(0x88ff88))),
            )
    }

    fn render_terminal_line(line: &str) -> Div {
        div()
            .text_size(px(12.))
            .text_color(rgb(0xc0c0c0))
            .font(ui_font())
            .child(line.to_string())
    }

    fn render_toast(
        notif: &notifications::Notification,
        desktop: Entity<DesktopModel>,
        index: usize,
    ) -> impl IntoElement {
        let border = match notif.urgency.as_str() {
            "critical" => 0xff4444,
            "low" => 0x444466,
            _ => 0x4488ff,
        };
        div()
            .id(SharedString::from(format!("toast-{index}")))
            .w(px(320.))
            .rounded(px(10.))
            .bg(rgb(SURFACE_2))
            .border_1()
            .border_color(rgb(border))
            .p(px(12.))
            .flex()
            .flex_col()
            .gap(px(4.))
            .cursor_pointer()
            .on_click(move |_, _, cx| {
                desktop.update(cx, |d, _| d.notifications.dismiss(index));
            })
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(rgb(TEXT))
                    .child(notif.title.clone()),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(TEXT_DIM))
                    .child(notif.body.clone()),
            )
    }
}

impl Render for DesktopWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.desktop.update(cx, |desktop, _| {
            desktop.notifications.poll();
            #[cfg(feature = "compositor")]
            desktop.poll_compositor();
        });
        let desktop_entity = self.desktop.clone();
        let desktop = self.desktop.read(cx);
        let layout = desktop.layout.view();
        let status = desktop.status_bar && !desktop.external_polybar;
        let top_inset = if status {
            72.
        } else if desktop.external_polybar {
            54.
        } else {
            24.
        };

        let mut root =
            div()
                .size_full()
                .bg(rgb(SURFACE))
                .font(ui_font())
                .key_context("alpenglowed")
                .on_action(cx.listener(|this, _: &FocusBar, _, cx| {
                    focus_or_open_launcher(&this.desktop, cx);
                }))
                .on_action(cx.listener(|this, _: &ToggleSettingsWindow, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.apply(PluginAction::ToggleSettings, cx);
                    });
                }))
                .on_action(cx.listener(|this, _: &ToggleStatusBarAction, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.toggle_status_bar(cx);
                    });
                }))
                .on_action(cx.listener(|this, _: &SplitRow, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.apply(
                            PluginAction::Layout {
                                action: layout::LayoutAction::SplitRow,
                            },
                            cx,
                        );
                    });
                }))
                .on_action(cx.listener(|this, _: &SplitColumn, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.apply(
                            PluginAction::Layout {
                                action: layout::LayoutAction::SplitColumn,
                            },
                            cx,
                        );
                    });
                }))
                .on_action(cx.listener(|this, _: &FlipAxis, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.apply(
                            PluginAction::Layout {
                                action: layout::LayoutAction::FlipAxis,
                            },
                            cx,
                        );
                    });
                }))
                .on_action(cx.listener(|this, _: &ResetLayout, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.apply(
                            PluginAction::Layout {
                                action: layout::LayoutAction::Reset,
                            },
                            cx,
                        );
                    });
                }))
                .on_action(cx.listener(|this, _: &NudgeLeft, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.apply(
                            PluginAction::Layout {
                                action: layout::LayoutAction::NudgeLeft,
                            },
                            cx,
                        );
                    });
                }))
                .on_action(cx.listener(|this, _: &NudgeRight, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.apply(
                            PluginAction::Layout {
                                action: layout::LayoutAction::NudgeRight,
                            },
                            cx,
                        );
                    });
                }))
                .on_action(cx.listener(|this, _: &NudgeUp, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.apply(
                            PluginAction::Layout {
                                action: layout::LayoutAction::NudgeUp,
                            },
                            cx,
                        );
                    });
                }))
                .on_action(cx.listener(|this, _: &NudgeDown, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.apply(
                            PluginAction::Layout {
                                action: layout::LayoutAction::NudgeDown,
                            },
                            cx,
                        );
                    });
                }))
                .on_action(cx.listener(|this, _: &ExpandWindow, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.apply(
                            PluginAction::Layout {
                                action: layout::LayoutAction::ExpandWindow,
                            },
                            cx,
                        );
                    });
                }))
                .on_action(cx.listener(|this, _: &ContractWindow, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.apply(
                            PluginAction::Layout {
                                action: layout::LayoutAction::ContractWindow,
                            },
                            cx,
                        );
                    });
                }))
                .on_action(cx.listener(|this, _: &GrowPane, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.apply(
                            PluginAction::Layout {
                                action: layout::LayoutAction::GrowFocused,
                            },
                            cx,
                        );
                    });
                }))
                .on_action(cx.listener(|this, _: &ShrinkPane, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.apply(
                            PluginAction::Layout {
                                action: layout::LayoutAction::ShrinkFocused,
                            },
                            cx,
                        );
                    });
                }))
                .on_action(cx.listener(|this, _: &FocusNextPane, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.apply(
                            PluginAction::Layout {
                                action: layout::LayoutAction::FocusNext,
                            },
                            cx,
                        );
                    });
                }))
                .on_action(cx.listener(|this, _: &FocusPreviousPane, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.apply(
                            PluginAction::Layout {
                                action: layout::LayoutAction::FocusPrevious,
                            },
                            cx,
                        );
                    });
                }))
                .on_action(cx.listener(|this, _: &FocusFirstPane, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.apply(
                            PluginAction::Layout {
                                action: layout::LayoutAction::FocusFirst,
                            },
                            cx,
                        );
                    });
                }))
                .on_action(cx.listener(|this, _: &FocusLastPane, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.apply(
                            PluginAction::Layout {
                                action: layout::LayoutAction::FocusLast,
                            },
                            cx,
                        );
                    });
                }))
                .on_action(cx.listener(|this, _: &CycleWindowModeAction, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.apply(PluginAction::CycleWindowMode, cx);
                    });
                }))
                .on_action(cx.listener(|this, _: &BalancePanesAction, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.apply(
                            PluginAction::Layout {
                                action: layout::LayoutAction::BalancePanes,
                            },
                            cx,
                        );
                    });
                }))
                .on_action(cx.listener(|this, _: &CenterFocusedAction, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.apply(
                            PluginAction::Layout {
                                action: layout::LayoutAction::CenterFocused,
                            },
                            cx,
                        );
                    });
                }))
                .on_action(cx.listener(|this, _: &MoveWindowLeft, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.apply(
                            PluginAction::Layout {
                                action: layout::LayoutAction::MoveLeft,
                            },
                            cx,
                        );
                    });
                }))
                .on_action(cx.listener(|this, _: &MoveWindowRight, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.apply(
                            PluginAction::Layout {
                                action: layout::LayoutAction::MoveRight,
                            },
                            cx,
                        );
                    });
                }))
                .on_action(cx.listener(|this, _: &MoveWindowUp, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.apply(
                            PluginAction::Layout {
                                action: layout::LayoutAction::MoveUp,
                            },
                            cx,
                        );
                    });
                }))
                .on_action(cx.listener(|this, _: &MoveWindowDown, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.apply(
                            PluginAction::Layout {
                                action: layout::LayoutAction::MoveDown,
                            },
                            cx,
                        );
                    });
                }))
                .on_action(cx.listener(|this, _: &ClosePane, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.apply(
                            PluginAction::Layout {
                                action: layout::LayoutAction::CloseFocused,
                            },
                            cx,
                        );
                    });
                }))
                .on_action(cx.listener(|this, _: &ToggleFloatPane, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.apply(
                            PluginAction::Layout {
                                action: layout::LayoutAction::ToggleFloat,
                            },
                            cx,
                        );
                    });
                }))
                .on_action(cx.listener(|this, _: &ToggleTerminalPane, _, cx| {
                    this.desktop.update(cx, |desktop, cx| {
                        desktop.apply(PluginAction::ToggleTerminal, cx);
                    });
                }))
                .child(div().size_full().p(px(24.)).pt(px(top_inset)).child(
                    Self::render_workspace(&self.desktop, &layout, &desktop.mode, cx),
                ));

        if status {
            root = root.child(Self::render_status_bar(desktop));
        }

        if !desktop.notifications.active.is_empty() {
            root = root.child(
                div()
                    .absolute()
                    .bottom(px(24.))
                    .right(px(24.))
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .children(
                        desktop
                            .notifications
                            .active
                            .iter()
                            .enumerate()
                            .map(|(i, n)| {
                                Self::render_toast(&n.notification, desktop_entity.clone(), i)
                            }),
                    ),
            );
        }

        if desktop.terminal_open {
            root = root.child(Self::render_terminal_panel(desktop));
        }

        root
    }
}

fn launcher_window_options(cx: &App) -> WindowOptions {
    WindowOptions {
        app_id: Some("alpenglowed-launcher".into()),
        titlebar: None,
        window_bounds: Some(WindowBounds::centered(size(px(760.), px(260.)), cx)),
        kind: WindowKind::PopUp,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        window_background: WindowBackgroundAppearance::Transparent,
        window_decorations: Some(WindowDecorations::Client),
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
        window_background: WindowBackgroundAppearance::Opaque,
        window_decorations: Some(WindowDecorations::Client),
        ..Default::default()
    }
}

fn installer_window_options(cx: &App) -> WindowOptions {
    WindowOptions {
        app_id: Some("alpenglowed-installer".into()),
        titlebar: None,
        window_bounds: Some(WindowBounds::centered(size(px(760.), px(560.)), cx)),
        kind: WindowKind::PopUp,
        is_resizable: false,
        window_background: WindowBackgroundAppearance::Opaque,
        window_decorations: Some(WindowDecorations::Client),
        ..Default::default()
    }
}

fn desktop_window_options(cx: &App) -> WindowOptions {
    let display_bounds = cx
        .primary_display()
        .map(|display| display.bounds())
        .unwrap_or_else(|| bounds(px(1440.), px(900.), cx));
    WindowOptions {
        app_id: Some("alpenglowed-desktop".into()),
        titlebar: None,
        window_bounds: Some(WindowBounds::Fullscreen(display_bounds)),
        kind: WindowKind::Normal,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        window_background: WindowBackgroundAppearance::Opaque,
        window_decorations: Some(WindowDecorations::Client),
        ..Default::default()
    }
}

fn bounds(width: Pixels, height: Pixels, cx: &App) -> gpui::Bounds<Pixels> {
    WindowBounds::centered(size(width, height), cx).get_bounds()
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

fn open_desktop_window(desktop: &Entity<DesktopModel>, cx: &mut App) {
    let desktop_entity = desktop.clone();
    let _ = cx.open_window(desktop_window_options(cx), move |_window, cx| {
        cx.new(|cx| DesktopWindow::new(desktop_entity, cx))
    });
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

fn open_installer_window(source: PathBuf, target: PathBuf, cx: &mut App) {
    let _ = cx.open_window(installer_window_options(cx), move |window, cx| {
        let view = cx.new(|_| InstallerWindow::new(source, target));
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

    if std::env::args().any(|arg| arg == "--help") {
        eprintln!("alpenglowed — Alpenglow desktop shell");
        eprintln!("Flags:");
        eprintln!("  --role=NAME       Session role: potatoes, desktop, workstation");
        eprintln!("  --session-contract  Print the Alpenglow session contract as JSON");
        eprintln!("  --compositor      Enable embedded smithay compositor (Linux, needs `features compositor`; not for potatoes)");
        eprintln!("  --polybar         Emit polybar status line");
        eprintln!("  --polybar-module=  Emit a single polybar module");
        eprintln!("  --probe-actions   List available desktop actions");
        eprintln!("  --notify=msg      Send notification to daemon");
        eprintln!("  --smoke-wayland   Test Wayland connection");
        eprintln!("  --open-settings   Open settings window at startup");
        eprintln!("  --floating        Start in floating mode");
        eprintln!("  --demo-layout     Use demo layout with 4 panes");
        return;
    }
    if std::env::args().any(|arg| arg == "--session-contract") {
        println!("{}", role::session_contract());
        return;
    }
    if std::env::args().any(|arg| arg == "--polybar") {
        let mode = std::env::var("ALPENGLOWED_MODE")
            .ok()
            .and_then(|m| WindowMode::from_label(&m))
            .unwrap_or(WindowMode::Tiling);
        let role = match role::SessionRole::resolve() {
            Ok(role) => role,
            Err(error) => {
                eprintln!("{}", error.message());
                std::process::exit(2);
            }
        };
        println!(
            "{}",
            de::DesktopState::detect_with_role(mode.label(), role.label()).polybar()
        );
        return;
    }
    if let Some(module) = std::env::args().find_map(|arg| {
        arg.strip_prefix("--polybar-module=")
            .map(ToString::to_string)
    }) {
        print!("{}", polybar_module(&module));
        return;
    }
    if std::env::args().any(|arg| arg == "--probe-actions") {
        for line in de::probe_actions() {
            println!("{line}");
        }
        return;
    }
    if std::env::args().any(|arg| arg == "--smoke-actions-safe") {
        for line in de::smoke_safe_actions() {
            println!("{line}");
        }
        return;
    }
    if let Some(notif) =
        std::env::args().find_map(|arg| arg.strip_prefix("--notify=").map(ToString::to_string))
    {
        let path = std::env::var_os("XDG_RUNTIME_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("alpenglowed")
            .join("notifications");
        if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(&path) {
            let payload = serde_json::json!({
                "title": "alpenglowed",
                "body": notif,
                "urgency": "normal"
            });
            let _ = serde_json::to_writer(&mut stream, &payload);
        } else {
            eprintln!("notification daemon not running");
            std::process::exit(1);
        }
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

    #[cfg(feature = "compositor")]
    let (_compositor_tx, _compositor_rx) = if std::env::args().any(|arg| arg == "--compositor")
        && options.role.capabilities().compositor
    {
        let (cmd, rx) = compositor::start();
        std::env::set_var("ALPENGLOW_COMPOSITOR", "1");
        (Some(cmd), Some(rx))
    } else {
        (None, None)
    };

    #[cfg(not(feature = "compositor"))]
    let (_compositor_tx, _compositor_rx): (
        Option<std::sync::mpsc::Sender<()>>,
        Option<std::sync::mpsc::Receiver<()>>,
    ) = (None, None);

    if !std::env::args().any(|arg| arg == "--compositor") || cfg!(not(feature = "compositor")) {
        ensure_wayland_display();
    }

    Application::new().run(move |cx: &mut App| {
        let _ = cx
            .text_system()
            .add_fonts(vec![Cow::Borrowed(GEIST), Cow::Borrowed(GEIST_MONO)]);

        cx.bind_keys([
            KeyBinding::new("cmd-space", FocusBar, None),
            KeyBinding::new("escape", DefocusBar, None),
            KeyBinding::new("enter", Confirm, None),
            KeyBinding::new("cmd-q", Quit, None),
            KeyBinding::new("cmd-shift-]", FocusNextPane, None),
            KeyBinding::new("cmd-shift-[", FocusPreviousPane, None),
            KeyBinding::new("cmd-shift--", ClosePane, None),
            KeyBinding::new("cmd-alt-f", ToggleFloatPane, None),
            KeyBinding::new("cmd-alt-h", SplitRow, None),
            KeyBinding::new("cmd-alt-v", SplitColumn, None),
            KeyBinding::new("cmd-alt-x", FlipAxis, None),
            KeyBinding::new("cmd-alt-r", ResetLayout, None),
            KeyBinding::new("cmd-alt-left", NudgeLeft, None),
            KeyBinding::new("cmd-alt-right", NudgeRight, None),
            KeyBinding::new("cmd-alt-up", NudgeUp, None),
            KeyBinding::new("cmd-alt-down", NudgeDown, None),
            KeyBinding::new("cmd-alt-=", ExpandWindow, None),
            KeyBinding::new("cmd-alt--", ContractWindow, None),
            KeyBinding::new("cmd-alt-l", GrowPane, None),
            KeyBinding::new("cmd-alt-j", ShrinkPane, None),
            KeyBinding::new("cmd-alt-t", ToggleTerminalPane, None),
            KeyBinding::new("cmd-,", ToggleSettingsWindow, None),
            KeyBinding::new("cmd-b", ToggleStatusBarAction, None),
            KeyBinding::new("cmd-alt-m", CycleWindowModeAction, None),
            KeyBinding::new("cmd-k", ClearQueryAction, None),
            KeyBinding::new("cmd-alt-b", BalancePanesAction, None),
            KeyBinding::new("cmd-alt-c", CenterFocusedAction, None),
            KeyBinding::new("cmd-shift-left", MoveWindowLeft, None),
            KeyBinding::new("cmd-shift-right", MoveWindowRight, None),
            KeyBinding::new("cmd-shift-up", MoveWindowUp, None),
            KeyBinding::new("cmd-shift-down", MoveWindowDown, None),
            KeyBinding::new("cmd-alt-1", FocusFirstPane, None),
            KeyBinding::new("cmd-alt-9", FocusLastPane, None),
            KeyBinding::new("cmd-1", SelectResult1, None),
            KeyBinding::new("cmd-2", SelectResult2, None),
            KeyBinding::new("cmd-3", SelectResult3, None),
            KeyBinding::new("cmd-4", SelectResult4, None),
            KeyBinding::new("cmd-5", SelectResult5, None),
            KeyBinding::new("cmd-6", SelectResult6, None),
            KeyBinding::new("cmd-7", SelectResult7, None),
            KeyBinding::new("cmd-8", SelectResult8, None),
        ]);

        let desktop_options = options.clone();
        #[cfg(feature = "compositor")]
        let desktop =
            cx.new(|_| DesktopModel::new(desktop_options, _compositor_rx, _compositor_tx));
        #[cfg(not(feature = "compositor"))]
        let desktop = cx.new(|_| DesktopModel::new(desktop_options));
        open_desktop_window(&desktop, cx);

        let installer = installer_paths();

        if let Some((source, target)) = installer {
            open_installer_window(source, target, cx);
        } else if options.open_settings {
            open_or_focus_settings(&desktop, cx);
        } else {
            focus_or_open_launcher(&desktop, cx);
        }
    });
}

impl UiOptions {
    fn from_env() -> Self {
        let cfg = config::Config::load();

        let role = match role::SessionRole::resolve() {
            Ok(role) => role,
            Err(error) => {
                eprintln!("{}", error.message());
                std::process::exit(2);
            }
        };
        let status_bar = std::env::args().any(|arg| arg == "--status-bar")
            || matches!(
                std::env::var("ALPENGLOWED_STATUS_BAR").as_deref(),
                Ok("1" | "true" | "yes")
            )
            || cfg
                .status_bar
                .unwrap_or(role.capabilities().default_status_bar);
        let external_polybar = std::env::args().any(|arg| arg == "--external-polybar")
            || matches!(
                std::env::var("ALPENGLOWED_EXTERNAL_BAR").as_deref(),
                Ok("polybar")
            )
            || cfg.external_polybar.unwrap_or(false);
        let open_settings = std::env::args().any(|arg| arg == "--open-settings")
            || cfg.open_settings.unwrap_or(false);
        let initial_query = std::env::args()
            .find_map(|arg| arg.strip_prefix("--query=").map(ToString::to_string))
            .or_else(|| std::env::var("ALPENGLOWED_QUERY").ok())
            .or_else(|| cfg.initial_query.clone())
            .unwrap_or_default();
        let mode = if std::env::args().any(|arg| arg == "--floating") {
            WindowMode::Floating
        } else if let Some(env_mode) = std::env::var("ALPENGLOWED_MODE")
            .ok()
            .and_then(|m| WindowMode::from_label(&m))
        {
            env_mode
        } else if let Some(cfg_mode) = cfg.mode.as_deref().and_then(WindowMode::from_label) {
            cfg_mode
        } else {
            WindowMode::Tiling
        };
        let demo_layout = std::env::args().any(|arg| arg == "--demo-layout")
            || matches!(
                std::env::var("ALPENGLOWED_DEMO_LAYOUT").as_deref(),
                Ok("1" | "true" | "yes")
            )
            || cfg.demo_layout.unwrap_or(false);

        Self {
            status_bar,
            external_polybar,
            open_settings,
            initial_query,
            mode,
            demo_layout,
            role,
        }
    }
}

fn polybar_module(name: &str) -> String {
    match name {
        "title" => external_bar_state()
            .map(|state| state.focused_title)
            .filter(|title| !title.trim().is_empty())
            .unwrap_or_else(|| "alpenglowed".to_string()),
        "mode" => external_bar_state()
            .map(|state| state.mode)
            .filter(|mode| !mode.trim().is_empty())
            .unwrap_or_else(|| "launcher".to_string()),
        "role" => role::SessionRole::resolve()
            .map(|role| role.label().to_string())
            .unwrap_or_else(|_| "desktop".to_string()),
        "backend" => polybar_backend(&de::DesktopState::detect("tiling")),
        "battery" => battery_value().unwrap_or_else(|| "battery unavailable".to_string()),
        "load" => load_value().unwrap_or_else(|| "load unavailable".to_string()),
        "memory" => memory_value().unwrap_or_else(|| "memory unavailable".to_string()),
        _ => String::new(),
    }
}

fn installer_paths() -> Option<(PathBuf, PathBuf)> {
    let source = std::env::var_os("ALPENGLOWED_INSTALLER_SOURCE")
        .map(PathBuf::from)
        .or_else(|| {
            let path = PathBuf::from("/run/alpenglow/alpenglow.img.zst");
            path.is_file().then_some(path)
        })?;
    let target = std::env::var_os("ALPENGLOWED_INSTALLER_TARGET").map(PathBuf::from)?;
    Some((source, target))
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

fn external_bar_state_path() -> PathBuf {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("alpenglowed-polybar.json")
}

fn write_external_bar_state(desktop: &DesktopModel) {
    let state = ExternalBarState {
        focused_title: desktop.layout.focused_title().to_string(),
        layout: desktop.layout.summary(),
        mode: desktop.mode.label().to_string(),
        last_action: desktop.last_action.clone(),
    };
    let _ = fs::write(
        external_bar_state_path(),
        serde_json::to_vec(&state).unwrap_or_default(),
    );
}

fn external_bar_state() -> Option<ExternalBarState> {
    let data = fs::read(external_bar_state_path()).ok()?;
    serde_json::from_slice(&data).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_list_items_should_extract_li_text() {
        assert_eq!(
            html_list_items("<ul><li>one</li><li>two &amp; three</li></ul>"),
            vec!["one".to_string(), "two & three".to_string()]
        );
    }

    #[test]
    fn shell_text_component_should_render_launcher_meta() {
        let text = shell_text_component(
            "LauncherMeta",
            &[("selection", "1/6"), ("subtitle", "window mode")],
        );
        assert_eq!(text, "1/6 window mode");
    }

    #[test]
    fn shell_list_component_should_render_shortcuts() {
        let shortcuts = shell_list_component("SettingsShortcuts");
        assert!(shortcuts
            .iter()
            .any(|line| line.contains("Cmd-Space launcher")));
        assert!(shortcuts
            .iter()
            .any(|line| line.contains("Cmd-Alt-F toggle float")));
    }

    #[test]
    fn shell_row_component_should_render_launcher_row() {
        let row = shell_row_component(">", "Tile windows", "window mode");
        assert_eq!(row.0, ">");
        assert_eq!(row.1, "Tile windows");
        assert_eq!(row.2, "window mode");
    }

    #[test]
    fn shell_bar_component_should_render_launcher_bar() {
        let bar = shell_bar_component("window", "1/6 window mode");
        assert_eq!(bar.0, "window");
        assert_eq!(bar.1, "1/6 window mode");
    }

    #[test]
    fn shell_top_bar_title_component_should_render_header() {
        let header = shell_top_bar_title_component("alpenglowed", "focused pane detail");
        assert_eq!(header.0, "alpenglowed");
        assert_eq!(header.1, "focused pane detail");
    }

    #[test]
    fn shell_top_bar_metric_component_should_render_value() {
        let metric = shell_top_bar_metric_component("clock", "11:27");
        assert_eq!(metric.0, "clock");
        assert_eq!(metric.1, "11:27");
    }

    #[test]
    fn polybar_module_should_return_mode() {
        assert_eq!(polybar_module("mode"), "launcher");
    }

    #[test]
    fn polybar_module_should_return_title_fallback() {
        assert_eq!(polybar_module("title"), "alpenglowed");
    }

    #[test]
    fn polybar_module_should_return_empty_for_unknown_name() {
        assert_eq!(polybar_module("unknown"), "");
    }

    #[test]
    fn polybar_module_should_return_a_known_role() {
        let role = polybar_module("role");
        assert!(role == "potatoes" || role == "desktop" || role == "workstation");
    }

    #[test]
    fn shell_section_header_component_should_render_settings_header() {
        let header = shell_section_header_component("Windows", "root row");
        assert_eq!(header.0, "Windows");
        assert_eq!(header.1, "root row");
    }

    #[test]
    fn shell_status_row_component_should_render_settings_pills() {
        let row = shell_status_row_component("tiling", "3 tiled 1 floating", "Workspace");
        assert_eq!(
            row,
            vec![
                "tiling".to_string(),
                "3 tiled 1 floating".to_string(),
                "Workspace".to_string()
            ]
        );
    }

    #[test]
    fn installer_copy_should_write_source_to_target() {
        let dir = std::env::temp_dir();
        let suffix = std::process::id();
        let source = dir.join(format!("alpenglowed-installer-source-{suffix}"));
        let target = dir.join(format!("alpenglowed-installer-target-{suffix}"));
        let source_bytes = b"ALPENGLOWED-INSTALL\n";
        std::fs::write(&source, source_bytes).unwrap();
        std::fs::write(&target, b"....................").unwrap();

        let bytes = copy_install_image(&source, &target).unwrap();

        assert_eq!(bytes, source_bytes.len() as u64);
        assert_eq!(std::fs::read(&target).unwrap(), source_bytes);
        let _ = std::fs::remove_file(source);
        let _ = std::fs::remove_file(target);
    }
}
