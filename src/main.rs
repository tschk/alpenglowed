extern crate crepuscularity_gpui as gpui;

mod de;
mod layout;
mod plugin;
mod runner;
mod session;

use crepuscularity_gpui::prelude::*;
use crepuscularity_gpui::{
    actions, size, AnyWindowHandle, Div, EventEmitter, KeyBinding, KeyDownEvent, Modifiers, Pixels,
    WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowKind, WindowOptions,
};
use layout::{Axis, LayoutChildView, LayoutState, LayoutView, LayoutWindowView};
use plugin::PluginAction;
use runner::{Runner, WindowMode};
use std::process::Command;

actions!(
    alpenglowed,
    [
        Quit,
        FocusBar,
        DefocusBar,
        Confirm,
        SplitRow,
        SplitColumn,
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
        ClosePane,
        ToggleFloatPane
    ]
);

#[derive(Clone)]
struct UiOptions {
    status_bar: bool,
    open_settings: bool,
    initial_query: String,
    mode: WindowMode,
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
    last_action: String,
    runner: Runner,
    session_control: bool,
    launcher: Option<AnyWindowHandle>,
    settings: Option<AnyWindowHandle>,
}

impl EventEmitter<DesktopEvent> for DesktopModel {}

impl DesktopModel {
    fn new(options: UiOptions) -> Self {
        let mut runner = Runner::new();
        runner.query = options.initial_query.clone();
        runner.update();

        Self {
            query: options.initial_query,
            mode: options.mode.clone(),
            layout: {
                let mut layout = LayoutState::new();
                layout.set_window_mode(&options.mode);
                layout
            },
            status_bar: options.status_bar,
            last_action: "Ready: desktop active".to_string(),
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

    fn set_last_action(&mut self, title: impl Into<String>, detail: impl Into<String>) {
        let title = title.into();
        let detail = detail.into();
        self.last_action = format!("{title}: {detail}");
        self.layout.set_focused_window_content(title, detail);
    }

    fn apply(&mut self, action: PluginAction, cx: &mut Context<Self>) {
        match action {
            PluginAction::SetWindowMode { mode } => {
                self.mode = mode.clone();
                self.layout.set_window_mode(&mode);
                self.set_last_action("Window mode", mode.label());
                let _ = session::dispatch(&session::SessionRequest::SetWindowMode { mode });
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
                self.set_last_action("Shell", command.clone());
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

struct DesktopWindow {
    desktop: Entity<DesktopModel>,
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
            LayoutView::Window(window) => Self::render_window(desktop, window),
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
        let mut slot = div()
            .min_w(px(0.))
            .min_h(px(0.))
            .child(Self::render_layout(desktop, &child.node));
        slot.style().flex_grow = Some(child.grow.max(0.1));
        slot.style().flex_shrink = Some(1.);
        slot
    }

    fn render_window(desktop: &Entity<DesktopModel>, window: &LayoutWindowView) -> Div {
        let border = if window.focused { 0xf0f0f0 } else { 0x2a2a2a };
        let label = if window.floating { "floating" } else { "tiled" };
        let focus = if window.focused { "focused" } else { "ready" };
        let lines = Self::pane_lines(window);
        let window_id = window.id;
        let desktop = desktop.clone();
        let panel = div()
            .id(SharedString::from(format!("pane-{window_id}")))
            .size_full()
            .rounded(px(6.))
            .bg(rgb(0x050505))
            .border_1()
            .border_color(rgb(border))
            .p(px(16.))
            .flex()
            .flex_col()
            .gap(px(14.))
            .cursor_pointer()
            .on_click(move |_, _, cx| {
                desktop.update(cx, |desktop, cx| {
                    desktop.layout.focus_window(window_id);
                    desktop.changed(cx);
                });
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(
                                div()
                                    .w(px(8.))
                                    .h(px(8.))
                                    .rounded_full()
                                    .bg(rgb(if window.focused { 0xf0f0f0 } else { 0x5a5a5a })),
                            )
                            .child(
                                div()
                                    .text_size(px(16.))
                                    .text_color(rgb(0xf5f5f5))
                                    .child(window.title.clone()),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(8.))
                            .child(Self::window_pill(label, false))
                            .child(Self::window_pill(focus, window.focused)),
                    ),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(rgb(0xb8b8b8))
                    .child(window.detail.clone()),
            )
            .child(
                div()
                    .flex_1()
                    .rounded(px(4.))
                    .bg(rgb(0x090909))
                    .border_1()
                    .border_color(rgb(0x151515))
                    .p(px(12.))
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .children(lines.into_iter().map(Self::window_line)),
            )
            .child(
                div()
                    .flex()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(0x7e7e7e))
                            .child(format!("window {}", window.id)),
                    )
                    .child(div().text_size(px(11.)).text_color(rgb(0x7e7e7e)).child(
                        if window.floating {
                            format!(
                                "{:.0}x{:.0} @ {:.0},{:.0}",
                                window.width, window.height, window.x, window.y
                            )
                        } else {
                            "flex layout".to_string()
                        },
                    )),
            );

        div().size_full().child(panel)
    }

    fn window_pill(text: &str, active: bool) -> Div {
        div()
            .px(px(8.))
            .py(px(4.))
            .rounded(px(999.))
            .bg(rgb(if active { 0x111111 } else { 0x090909 }))
            .border_1()
            .border_color(rgb(if active { 0xf0f0f0 } else { 0x232323 }))
            .text_size(px(10.))
            .text_color(rgb(if active { 0xffffff } else { 0x8d8d8d }))
            .child(text.to_string())
    }

    fn window_line(text: String) -> Div {
        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(0x6e6e6e))
                    .child("~"),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(0xd8d8d8))
                    .child(text),
            )
    }

    fn pane_lines(window: &LayoutWindowView) -> Vec<String> {
        let title = window.title.to_lowercase();
        if title.contains("workspace") {
            return vec![
                "launcher ready for app, shell, and os actions".to_string(),
                "tiling shortcuts stay on desktop surface".to_string(),
                window.detail.clone(),
                "terminal and browser stay in main flow".to_string(),
            ];
        }
        if title.contains("scratch") {
            return vec![
                "clipboard history and quick notes".to_string(),
                "transient actions and settings access".to_string(),
                window.detail.clone(),
                "good target for floating utility panes".to_string(),
            ];
        }
        if title.contains("shell") {
            return vec![
                "command output stays attached to focused pane".to_string(),
                "enter runs selected launcher action".to_string(),
                "escape keeps desktop visible".to_string(),
            ];
        }
        vec![
            window.detail.clone(),
            "split, focus, float, resize".to_string(),
            "bar drives window actions".to_string(),
        ]
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
            .child(Self::render_window(desktop, window))
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

    fn render_workspace(desktop: &Entity<DesktopModel>, layout: &LayoutView) -> Div {
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
        let state = de::DesktopState::detect(desktop.mode.label());
        let display = state.display.unwrap_or_else(|| "no-display".to_string());
        let backend = if state.wayland { "wayland" } else { "offline" };
        let focused = desktop.layout.focused_title().to_string();
        let detail = desktop
            .layout
            .view()
            .focused_detail()
            .unwrap_or_else(|| "Ready".to_string());

        div()
            .absolute()
            .top(px(20.))
            .left(px(24.))
            .flex()
            .gap(px(8.))
            .children(
                [
                    desktop.mode.label().to_string(),
                    desktop.layout.summary(),
                    focused,
                    detail,
                    backend.to_string(),
                    display,
                ]
                .into_iter()
                .map(Self::status_pill),
            )
    }

    fn status_pill(text: String) -> Div {
        div()
            .h(px(34.))
            .px(px(12.))
            .rounded(px(17.))
            .bg(rgb(0x050505))
            .border_1()
            .border_color(rgb(0x2a2a2a))
            .flex()
            .items_center()
            .text_size(px(12.))
            .text_color(rgb(0xcfcfcf))
            .child(text)
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
            if !matches!(action, PluginAction::None) {
                self.desktop.update(cx, |desktop, cx| {
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
        let selection = desktop.runner.selection_label();
        let subtitle = if desktop.query.trim().is_empty() {
            "type to search desktop actions".to_string()
        } else {
            desktop
                .runner
                .selected_result()
                .map(|result| result.subtitle.clone())
                .unwrap_or_else(|| "no match".to_string())
        };
        let query = if desktop.query.is_empty() {
            " ".to_string()
        } else {
            desktop.query.clone()
        };

        div()
            .w(px(720.))
            .h(px(52.))
            .rounded(px(6.))
            .bg(rgb(0x050505))
            .border_1()
            .border_color(rgb(0x2a2a2a))
            .px(px(16.))
            .flex()
            .items_center()
            .gap(px(12.))
            .child(
                div()
                    .flex_1()
                    .text_size(px(18.))
                    .text_color(rgb(0xffffff))
                    .child(query),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(0xb8b8b8))
                    .child(format!("{selection} {subtitle}")),
            )
    }

    fn render_results(&self, cx: &App) -> impl IntoElement {
        let desktop = self.desktop.read(cx);

        if desktop.query.trim().is_empty() {
            return div();
        }

        if desktop.runner.results.is_empty() {
            return div()
                .w(px(720.))
                .rounded_b(px(6.))
                .bg(rgb(0x0f0f0f))
                .border_1()
                .border_color(rgb(0x232323))
                .border_t_0()
                .px(px(14.))
                .py(px(10.))
                .text_size(px(12.))
                .text_color(rgb(0x8d8d8d))
                .child("No results");
        }

        div()
            .w(px(720.))
            .rounded_b(px(6.))
            .bg(rgb(0x0f0f0f))
            .border_1()
            .border_color(rgb(0x232323))
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
                        div()
                            .id(SharedString::from(format!("launcher-result-{index}")))
                            .rounded(px(4.))
                            .bg(if selected {
                                rgb(0x141414)
                            } else {
                                rgb(0x0f0f0f)
                            })
                            .px(px(10.))
                            .py(px(7.))
                            .flex()
                            .items_center()
                            .gap(px(10.))
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
                                        rgb(0xffffff)
                                    } else {
                                        rgb(0xb8b8b8)
                                    })
                                    .child(if selected { ">" } else { "$" }),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(px(13.))
                                    .text_color(if selected {
                                        rgb(0xffffff)
                                    } else {
                                        rgb(0xf0f0f0)
                                    })
                                    .child(result.title.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(0x8d8d8d))
                                    .child(result.subtitle.clone()),
                            )
                    }),
            )
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

    fn mode_button(&self, label: &'static str, mode: WindowMode, active: bool) -> impl IntoElement {
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
                    desktop.apply(PluginAction::SetWindowMode { mode: mode.clone() }, cx);
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
        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .rounded(px(6.))
            .bg(rgb(0x090909))
            .border_1()
            .border_color(rgb(0x171717))
            .p(px(14.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(rgb(0xf0f0f0))
                            .child(title),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(0x8d8d8d))
                            .child(detail.into()),
                    ),
            )
            .child(content)
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
        div().size_full().bg(rgb(0x080808)).child(
            div().size_full().bg(rgb(0x050505)).p(px(20.)).child(
                div()
                    .size_full()
                    .rounded(px(6.))
                    .bg(rgb(0x080808))
                    .border_1()
                    .border_color(rgb(0x252525))
                    .p(px(20.))
                    .flex()
                    .flex_col()
                    .gap(px(18.))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .items_start()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(4.))
                                    .child(
                                        div()
                                            .text_size(px(20.))
                                            .text_color(rgb(0xf0f0f0))
                                            .child("Settings"),
                                    )
                                    .child(
                                        div().text_size(px(12.)).text_color(rgb(0x8d8d8d)).child(
                                            "Desktop, launcher, session, and system controls",
                                        ),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap(px(8.))
                                    .child(DesktopWindow::status_pill(
                                        desktop.mode.label().to_string(),
                                    ))
                                    .child(DesktopWindow::status_pill(layout_summary.clone()))
                                    .child(DesktopWindow::status_pill(focused.clone())),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(14.))
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap(px(14.))
                                    .child(
                                        self.section_card(
                                            "Windows",
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
                                                .child(self.layout_action_button(
                                                    "Split row",
                                                    layout::LayoutAction::SplitRow,
                                                ))
                                                .child(self.layout_action_button(
                                                    "Split column",
                                                    layout::LayoutAction::SplitColumn,
                                                ))
                                                .child(self.layout_action_button(
                                                    "Focus next",
                                                    layout::LayoutAction::FocusNext,
                                                ))
                                                .child(self.layout_action_button(
                                                    "Toggle float",
                                                    layout::LayoutAction::ToggleFloat,
                                                ))
                                                .child(self.layout_action_button(
                                                    "Close focused",
                                                    layout::LayoutAction::CloseFocused,
                                                )),
                                        ),
                                    )
                                    .child(
                                        self.section_card(
                                            "Layout",
                                            "nudge, resize, reset",
                                            div()
                                                .flex()
                                                .flex_wrap()
                                                .gap(px(8.))
                                                .child(self.layout_action_button(
                                                    "Reset layout",
                                                    layout::LayoutAction::Reset,
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
                                                )),
                                        ),
                                    )
                                    .child(
                                        self.section_card(
                                            "System",
                                            "fast desktop actions from launcher and control center",
                                            div()
                                                .flex()
                                                .flex_wrap()
                                                .gap(px(8.))
                                                .child(self.desktop_action_button(
                                                    "Lock",
                                                    de::DesktopAction::Lock,
                                                ))
                                                .child(self.desktop_action_button(
                                                    "Terminal",
                                                    de::DesktopAction::Terminal,
                                                ))
                                                .child(self.desktop_action_button(
                                                    "Files",
                                                    de::DesktopAction::Files,
                                                ))
                                                .child(self.desktop_action_button(
                                                    "Screenshot",
                                                    de::DesktopAction::Screenshot,
                                                ))
                                                .child(self.desktop_action_button(
                                                    "Clipboard",
                                                    de::DesktopAction::Clipboard,
                                                ))
                                                .child(self.desktop_action_button(
                                                    "Wi-Fi",
                                                    de::DesktopAction::Wifi,
                                                ))
                                                .child(self.desktop_action_button(
                                                    "Notifications",
                                                    de::DesktopAction::Notifications,
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
                                                    "Reboot",
                                                    de::DesktopAction::Reboot,
                                                ))
                                                .child(self.desktop_action_button(
                                                    "Shutdown",
                                                    de::DesktopAction::Shutdown,
                                                )),
                                        ),
                                    ),
                            )
                            .child(
                                div()
                                    .w(px(280.))
                                    .flex()
                                    .flex_col()
                                    .gap(px(14.))
                                    .child(
                                        self.section_card(
                                            "Interface",
                                            format!("status bar {status_bar}"),
                                            div()
                                                .flex()
                                                .flex_col()
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
                                                    "Reset query",
                                                    |desktop, cx| {
                                                        desktop.update(cx, |desktop, cx| {
                                                            desktop.set_query(
                                                                "window".to_string(),
                                                                cx,
                                                            );
                                                        });
                                                    },
                                                )),
                                        ),
                                    )
                                    .child(
                                        self.section_card(
                                            "Session",
                                            session_status,
                                            div()
                                                .flex()
                                                .flex_col()
                                                .gap(px(6.))
                                                .child(
                                                    div()
                                                        .text_size(px(11.))
                                                        .text_color(rgb(0x8d8d8d))
                                                        .child(format!("Focused pane: {focused}")),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(11.))
                                                        .text_color(rgb(0x8d8d8d))
                                                        .child(format!(
                                                            "Mode: {}",
                                                            desktop.mode.label()
                                                        )),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(11.))
                                                        .text_color(rgb(0x8d8d8d))
                                                        .child(format!(
                                                            "Layout axis: {layout_axis}"
                                                        )),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(11.))
                                                        .text_color(rgb(0x8d8d8d))
                                                        .child(format!(
                                                            "Last action: {last_action}"
                                                        )),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(11.))
                                                        .text_color(rgb(0x8d8d8d))
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
                                                .child(
                                                    div()
                                                        .text_size(px(11.))
                                                        .text_color(rgb(0x8d8d8d))
                                                        .child("Cmd-Space launcher"),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(11.))
                                                        .text_color(rgb(0x8d8d8d))
                                                        .child("Cmd-Alt-H split row"),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(11.))
                                                        .text_color(rgb(0x8d8d8d))
                                                        .child("Cmd-Alt-V split column"),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(11.))
                                                        .text_color(rgb(0x8d8d8d))
                                                        .child("Cmd-Alt-R reset"),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(11.))
                                                        .text_color(rgb(0x8d8d8d))
                                                        .child("Cmd-Shift-] focus next"),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(11.))
                                                        .text_color(rgb(0x8d8d8d))
                                                        .child("Cmd-Alt-F toggle float"),
                                                ),
                                        ),
                                    ),
                            ),
                    ),
            ),
        )
    }
}

impl Render for LauncherWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status_bar = self.desktop.read(cx).status_bar;

        let mut root = div()
            .size_full()
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
                            .w(px(720.))
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(self.render_bar(cx))
                            .child(self.render_results(cx)),
                    ),
            );

        if status_bar {
            root = root.child(DesktopWindow::render_status_bar(self.desktop.read(cx)));
        }

        root
    }
}

impl Render for DesktopWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let desktop = self.desktop.read(cx);
        let layout = desktop.layout.view();
        let status = desktop.status_bar;

        let mut root = div()
            .size_full()
            .bg(rgb(0x000000))
            .key_context("alpenglowed")
            .on_action(cx.listener(|this, _: &FocusBar, _, cx| {
                focus_or_open_launcher(&this.desktop, cx);
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
            .child(
                div()
                    .size_full()
                    .p(px(24.))
                    .pt(px(if status { 72. } else { 24. }))
                    .child(Self::render_workspace(&self.desktop, &layout)),
            );

        if status {
            root = root.child(Self::render_status_bar(desktop));
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

fn desktop_window_options(cx: &App) -> WindowOptions {
    WindowOptions {
        app_id: Some("alpenglowed-desktop".into()),
        titlebar: None,
        window_bounds: Some(WindowBounds::Fullscreen(bounds(px(1440.), px(900.), cx))),
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
    if std::env::args().any(|arg| arg == "--probe-actions") {
        for line in de::probe_actions() {
            println!("{line}");
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

    ensure_wayland_display();

    Application::new().run(move |cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("cmd-space", FocusBar, None),
            KeyBinding::new("escape", DefocusBar, None),
            KeyBinding::new("enter", Confirm, None),
            KeyBinding::new("cmd-q", Quit, None),
            KeyBinding::new("cmd-shift-]", FocusNextPane, None),
            KeyBinding::new("cmd-shift--", ClosePane, None),
            KeyBinding::new("cmd-alt-f", ToggleFloatPane, None),
            KeyBinding::new("cmd-alt-h", SplitRow, None),
            KeyBinding::new("cmd-alt-v", SplitColumn, None),
            KeyBinding::new("cmd-alt-r", ResetLayout, None),
            KeyBinding::new("cmd-alt-left", NudgeLeft, None),
            KeyBinding::new("cmd-alt-right", NudgeRight, None),
            KeyBinding::new("cmd-alt-up", NudgeUp, None),
            KeyBinding::new("cmd-alt-down", NudgeDown, None),
            KeyBinding::new("cmd-alt-=", ExpandWindow, None),
            KeyBinding::new("cmd-alt--", ContractWindow, None),
            KeyBinding::new("cmd-alt-l", GrowPane, None),
            KeyBinding::new("cmd-alt-j", ShrinkPane, None),
        ]);

        let desktop_options = options.clone();
        let desktop = cx.new(|_| DesktopModel::new(desktop_options));
        open_desktop_window(&desktop, cx);

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
        let initial_query = std::env::args()
            .find_map(|arg| arg.strip_prefix("--query=").map(ToString::to_string))
            .or_else(|| std::env::var("ALPENGLOWED_QUERY").ok())
            .unwrap_or_default();
        let mode = if std::env::args().any(|arg| arg == "--floating")
            || matches!(std::env::var("ALPENGLOWED_MODE").as_deref(), Ok("floating"))
        {
            WindowMode::Floating
        } else {
            WindowMode::Tiling
        };

        Self {
            status_bar,
            open_settings,
            initial_query,
            mode,
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
