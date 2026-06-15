// Alpenglowed Pills — status indicators in the bar top

use gpui::prelude::*;
use gpui::*;
use std::time::SystemTime;

pub struct Pills;

impl Pills {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self
    }

    fn render_pill(&self, text: &str, bg_color: Rgba) -> impl IntoElement {
        div()
            .px(px(10.))
            .py(px(4.))
            .bg(bg_color)
            .rounded(px(6.))
            .text_size(px(12.))
            .text_color(rgb(0xffffff))
            .child(text.to_string())
    }

    fn now_str() -> String {
        let epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = epoch.as_secs();
        let h = (secs / 3600) % 12;
        let m = (secs / 60) % 60;
        let ampm = if h < 12 { "AM" } else { "PM" };
        format!("{}:{:02} {}", if h == 0 { 12 } else { h }, m, ampm)
    }
}

impl Render for Pills {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let time = Self::now_str();

        div()
            .w_full()
            .h(px(32.))
            .bg(rgb(0x111111))
            .flex()
            .items_center()
            .px(px(12.))
            .gap(px(4.))
            .child(self.render_pill(&time, rgb(0x2d5a27)))
            .child(self.render_pill("87%", rgb(0x1a3a1a)))
            .child(div().flex_1())
            .child(self.render_pill("CPU 12%", rgb(0x3a3a1a)))
            .child(self.render_pill("GPU 5%", rgb(0x3a1a3a)))
    }
}
