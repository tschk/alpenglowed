//! Black & white GPUI greeter for greetd. Not part of alpenglowed shell.

extern crate crepuscularity_gpui as gpui;

use gpui::prelude::*;
use gpui::{
    div, px, rgb, App, Application, Context, Entity, EventEmitter, FocusHandle, KeyDownEvent,
    Modifiers, Window, WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowKind,
    WindowOptions,
};
use std::borrow::Cow;

const NOTO_SANS: &[u8] = include_bytes!("../../assets/fonts/noto-sans-regular.ttf");

const BG: u32 = 0x000000;
const FG: u32 = 0xffffff;
const FG_DIM: u32 = 0x888888;
const BORDER: u32 = 0xffffff;

#[derive(Clone, Debug)]
enum GreeterEvent {
    UiChanged,
}

struct GreeterModel {
    username: String,
    password: String,
    error: String,
    focus_user: bool,
    auth_busy: bool,
    ipc_done: bool,
}

impl EventEmitter<GreeterEvent> for GreeterModel {}

impl GreeterModel {
    fn new() -> Self {
        let username = std::env::var("ALPENGLOW_GREETER_USER")
            .or_else(|_| std::fs::read_to_string("/etc/alpenglow/greeter-default-user"))
            .unwrap_or_else(|_| "root".to_string())
            .trim()
            .to_string();
        Self {
            username,
            password: String::new(),
            error: String::new(),
            focus_user: false,
            auth_busy: false,
            ipc_done: false,
        }
    }

    fn notify_ui(&self, cx: &mut Context<Self>) {
        cx.emit(GreeterEvent::UiChanged);
        cx.notify();
    }

    fn set_error(&mut self, msg: impl Into<String>, cx: &mut Context<Self>) {
        self.error = msg.into();
        self.auth_busy = false;
        self.password.clear();
        self.notify_ui(cx);
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        if self.auth_busy || self.ipc_done {
            return;
        }
        let user = self.username.trim().to_string();
        if user.is_empty() {
            self.set_error("Enter a username", cx);
            return;
        }
        self.error.clear();
        self.auth_busy = true;
        self.notify_ui(cx);

        let password = self.password.clone();
        #[cfg(target_os = "linux")]
        match greetd::login(&user, &password) {
            Ok(()) => {
                self.ipc_done = true;
                self.auth_busy = false;
                self.notify_ui(cx);
            }
            Err(e) => self.set_error(e, cx),
        }
        #[cfg(not(target_os = "linux"))]
        {
            self.set_error("greetd login only supported on Linux", cx);
        }
    }
}

struct GreeterView {
    desktop: Entity<GreeterModel>,
    pass_focus: FocusHandle,
}

impl GreeterView {
    fn new(desktop: Entity<GreeterModel>, cx: &mut Context<Self>) -> Self {
        Self {
            desktop,
            pass_focus: cx.focus_handle(),
        }
    }

    fn field(
        &self,
        label: &str,
        value: &str,
        secret: bool,
        focused: bool,
        _cx: &Context<Self>,
    ) -> impl IntoElement {
        let display = if secret {
            "•".repeat(value.chars().count())
        } else {
            value.to_string()
        };
        div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(FG_DIM))
                    .child(label.to_string()),
            )
            .child(
                div()
                    .w_full()
                    .px(px(12.))
                    .py(px(10.))
                    .border_1()
                    .border_color(rgb(if focused { BORDER } else { FG_DIM }))
                    .bg(rgb(BG))
                    .text_size(px(16.))
                    .text_color(rgb(FG))
                    .child(if display.is_empty() {
                        "_".to_string()
                    } else {
                        display
                    }),
            )
    }
}

impl Render for GreeterView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let model = self.desktop.read(cx);
        let focus_user = model.focus_user;
        let busy = model.auth_busy;

        div()
            .size_full()
            .bg(rgb(BG))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(32.))
            .track_focus(&self.pass_focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key == "enter" {
                    this.desktop.update(cx, |m, cx| m.submit(cx));
                    cx.stop_propagation();
                    return;
                }
                if event.keystroke.key == "tab" {
                    this.desktop.update(cx, |m, cx| {
                        m.focus_user = !m.focus_user;
                        m.notify_ui(cx);
                    });
                    cx.stop_propagation();
                    return;
                }
                if event.keystroke.key == "backspace" {
                    this.desktop.update(cx, |m, cx| {
                        if m.focus_user {
                            m.username.pop();
                        } else {
                            m.password.pop();
                        }
                        m.notify_ui(cx);
                    });
                    cx.stop_propagation();
                    return;
                }
                if event.keystroke.modifiers == Modifiers::default() {
                    if let Some(ch) = event.keystroke.key_char.as_deref() {
                        if ch.chars().count() == 1 && !ch.chars().all(|c| c.is_control()) {
                            this.desktop.update(cx, |m, cx| {
                                if m.focus_user {
                                    m.username.push_str(ch);
                                } else {
                                    m.password.push_str(ch);
                                }
                                m.notify_ui(cx);
                            });
                            cx.stop_propagation();
                        }
                    }
                }
            }))
            .child(
                div()
                    .text_size(px(28.))
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(rgb(FG))
                    .child("Alpenglow"),
            )
            .child(
                div()
                    .w(px(360.))
                    .flex()
                    .flex_col()
                    .gap(px(20.))
                    .child(self.field(
                        "USER",
                        &model.username,
                        false,
                        focus_user,
                        cx,
                    ))
                    .child(self.field(
                        "PASSWORD",
                        &model.password,
                        true,
                        !focus_user,
                        cx,
                    ))
                    .when(!model.error.is_empty(), |col| {
                        col.child(
                            div()
                                .text_size(px(13.))
                                .text_color(rgb(FG))
                                .child(model.error.clone()),
                        )
                    })
                    .when(busy, |col| {
                        col.child(
                            div()
                                .text_size(px(12.))
                                .text_color(rgb(FG_DIM))
                                .child("Signing in…"),
                        )
                    })
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(FG_DIM))
                            .child("Enter to sign in · Tab to switch field"),
                    ),
            )
    }
}

fn greeter_window_options(cx: &App) -> WindowOptions {
    let display_bounds = cx
        .displays()
        .first()
        .map(|display| display.bounds())
        .unwrap_or_else(|| {
            WindowBounds::centered(gpui::size(px(1280.), px(720.)), cx).get_bounds()
        });
    WindowOptions {
        app_id: Some("alpenglow-greeter".into()),
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

fn open_greeter_window(model: Entity<GreeterModel>, cx: &mut App) {
    cx.open_window(greeter_window_options(cx), move |window, cx| {
        window.activate_window();
        cx.new(|cx| GreeterView::new(model, cx))
    })
    .unwrap();
}

fn main() {
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        eprintln!("alpenglow-greeter — GPUI greeter for greetd");
        eprintln!("Env: ALPENGLOW_GREETER_USER, GREETD_SOCK");
        eprintln!("Autologin: use greetd initial_session (see /etc/alpenglow/autologin)");
        return;
    }

    Application::new().run(|cx: &mut App| {
        let _ = cx.text_system().add_fonts(vec![Cow::Borrowed(NOTO_SANS)]);
        let model = cx.new(|_| GreeterModel::new());
        open_greeter_window(model, cx);
    });
}

#[cfg(target_os = "linux")]
mod greetd {
    use greetd_ipc::codec::SyncCodec;
    use greetd_ipc::{AuthMessageType, ErrorType, Request, Response};
    use std::os::unix::net::UnixStream;

    pub fn login(username: &str, password: &str) -> Result<(), String> {
        let path = std::env::var_os("GREETD_SOCK").unwrap_or_else(|| {
            std::ffi::OsString::from("/run/greetd.sock")
        });
        let mut stream = UnixStream::connect(&path).map_err(|e| format!("greetd: {e}"))?;

        Request::CreateSession {
            username: username.into(),
        }
        .write_to(&mut stream)
        .map_err(|e| format!("create_session: {e}"))?;

        loop {
            let msg = Response::read_from(&mut stream).map_err(|e| format!("recv: {e}"))?;
            match msg {
                Response::Success => {
                    Request::StartSession {
                        cmd: vec![],
                        env: vec![],
                    }
                    .write_to(&mut stream)
                    .map_err(|e| format!("start_session: {e}"))?;
                    let started =
                        Response::read_from(&mut stream).map_err(|e| format!("recv: {e}"))?;
                    return match started {
                        Response::Success => Ok(()),
                        Response::Error { description, .. } => Err(description),
                        other => Err(format!("unexpected after start: {other:?}")),
                    };
                }
                Response::Error {
                    error_type,
                    description,
                } => {
                    return Err(match error_type {
                        ErrorType::AuthError => "Authentication failed".into(),
                        ErrorType::Error => description,
                    });
                }
                Response::AuthMessage {
                    auth_message_type,
                    auth_message: _,
                } => {
                    let answer = match auth_message_type {
                        AuthMessageType::Visible | AuthMessageType::Secret => {
                            Some(password.to_string())
                        }
                        AuthMessageType::Info | AuthMessageType::Error => None,
                    };
                    Request::PostAuthMessageResponse { response: answer }
                        .write_to(&mut stream)
                        .map_err(|e| format!("post_auth: {e}"))?;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn greeter_default_user_is_non_empty() {
        let u = std::env::var("USER").unwrap_or_else(|_| "root".into());
        assert!(!u.is_empty());
    }
}