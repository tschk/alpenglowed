use gpui::{
    div, px, rgb, Div, ElementId, FontWeight, InteractiveElement, ParentElement, SharedString,
    Stateful, Styled,
};

use crate::icons::{sf_icon, SfSymbol};

pub struct Theme;

impl Theme {
    pub const PANEL_WIDTH: f32 = 750.0;
    pub const PANEL_HEIGHT: f32 = 475.0;
    pub const PANEL_RADIUS: f32 = 26.0;
    pub const HEADER_PADDING_TOP: f32 = 10.0;
    pub const HEADER_HEIGHT: f32 = 44.0;
    pub const ROW_HEIGHT: f32 = 38.0;
    pub const ROW_RADIUS: f32 = 10.0;
    pub const BOTTOM_BAR_HEIGHT: f32 = 52.0;

    pub const BG: u32 = 0x161619;
    pub const PANEL_SCRIM: u32 = 0x161618;
    pub const PANEL_BORDER: u32 = 0x2c2c31;
    pub const PANEL_STROKE: u32 = 0x26262b;
    pub const CARD_BORDER: u32 = 0x3e3e46;
    pub const CARD_STROKE: u32 = 0x2b2b31;
    pub const ROW_SELECTED: u32 = 0x2a2a30;
    pub const ICON_SURFACE: u32 = 0x28282f;
    pub const BAR_SURFACE: u32 = 0x242429;
    pub const BAR_ACTIVE: u32 = 0x323239;
    pub const POPOVER_BG: u32 = 0x1f1f24;
    pub const POPOVER_BORDER: u32 = 0x34343c;
    pub const DIVIDER: u32 = 0x26262b;
    pub const KEYCAP_BORDER: u32 = 0x3e3e46;

    pub const TEXT_PRIMARY: u32 = 0xffffff;
    pub const TEXT_SECONDARY: u32 = 0x9a9aa0;
    pub const TEXT_TERTIARY: u32 = 0x68686e;
}

/// Tinycast `24x24` icon slot containing a `22x22` `6px`-radius tile and centered SF Symbol.
pub fn icon_tile(symbol: SfSymbol, bg_hex: u32, fg_hex: u32, running: bool) -> Div {
    div()
        .w(px(24.0))
        .h(px(24.0))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .relative()
        .child(
            div()
                .w(px(22.0))
                .h(px(22.0))
                .rounded(px(6.0))
                .bg(rgb(bg_hex))
                .flex()
                .items_center()
                .justify_center()
                .child(sf_icon(symbol, 13.5, fg_hex)),
        )
        .children(running.then(|| {
            div()
                .w(px(3.5))
                .h(px(3.5))
                .rounded(px(2.0))
                .bg(rgb(0xd8d8dc))
                .mt(px(1.5))
        }))
}

/// Tinycast `KeyCapChip` (`18px` height, `6px` radius, `1px` outline).
/// Renders an SF Symbol icon when the keycap is a symbol key (`↵`, `⌫`, `⌘`, arrows).
pub fn keycap_chip(key: &str) -> Div {
    let is_word = key.chars().count() > 1;
    let chip = div()
        .h(px(18.0))
        .min_w(px(20.0))
        .px(px(if is_word { 5.0 } else { 4.0 }))
        .rounded(px(6.0))
        .border_1()
        .border_color(rgb(Theme::KEYCAP_BORDER))
        .flex()
        .items_center()
        .justify_center();

    if let Some(sym) = SfSymbol::from_keycap(key) {
        chip.child(sf_icon(sym, 10.5, Theme::TEXT_SECONDARY))
    } else {
        chip.text_size(px(11.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(Theme::TEXT_SECONDARY))
            .child(key.to_string())
    }
}

/// Horizontal group of `KeyCapChip`s with Tinycast's `2px` spacing.
pub fn keycap_group(keys: &[&str]) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(2.0))
        .children(keys.iter().map(|k| keycap_chip(k)))
}

/// Tinycast `SectionHeader` (`12px` Medium `#9a9aa0`, `4px` first top pad, `12px` subsequent).
pub fn section_header(title: impl Into<SharedString>, is_first: bool) -> Div {
    div()
        .w_full()
        .px(px(8.0))
        .pt(px(if is_first { 4.0 } else { 12.0 }))
        .pb(px(4.0))
        .text_size(px(12.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(Theme::TEXT_SECONDARY))
        .child(title.into())
}

/// Tinycast `38px` borderless `10px`-radius list row component.
#[allow(clippy::too_many_arguments)]
pub fn list_row(
    id: impl Into<ElementId>,
    selected: bool,
    symbol: SfSymbol,
    icon_bg: u32,
    running: bool,
    title: impl Into<SharedString>,
    inline_preview: Option<SharedString>,
    keycaps: &[&str],
    kind_label: impl Into<SharedString>,
) -> Stateful<Div> {
    let mut left = div()
        .flex()
        .items_center()
        .gap(px(10.0))
        .overflow_hidden()
        .child(icon_tile(symbol, icon_bg, Theme::TEXT_PRIMARY, running))
        .child(
            div()
                .text_size(px(13.5))
                .font_weight(FontWeight::NORMAL)
                .text_color(rgb(Theme::TEXT_PRIMARY))
                .child(title.into()),
        );

    if let Some(preview) = inline_preview {
        left = left.child(
            div()
                .text_size(px(12.5))
                .text_color(rgb(Theme::TEXT_SECONDARY))
                .child(preview),
        );
    }

    if !keycaps.is_empty() {
        left = left.child(keycap_group(keycaps));
    }

    let row = div()
        .id(id)
        .w_full()
        .h(px(Theme::ROW_HEIGHT))
        .px(px(8.0))
        .py(px(6.0))
        .rounded(px(Theme::ROW_RADIUS))
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.0));

    let row = if selected {
        row.bg(rgb(Theme::ROW_SELECTED))
    } else {
        row
    };

    row.child(left).child(
        div()
            .text_size(px(12.5))
            .text_color(rgb(Theme::TEXT_TERTIARY))
            .child(kind_label.into()),
    )
}

/// Tinycast bottom-left `36x36` circular icon button.
pub fn circle_icon_button(
    id: impl Into<ElementId>,
    symbol: SfSymbol,
    active: bool,
) -> Stateful<Div> {
    div()
        .id(id)
        .w(px(36.0))
        .h(px(36.0))
        .rounded(px(18.0))
        .bg(rgb(if active {
            Theme::BAR_ACTIVE
        } else {
            Theme::BAR_SURFACE
        }))
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_center()
        .child(sf_icon(symbol, 14.0, Theme::TEXT_SECONDARY))
}

/// Tinycast `28px` height `BarButton` inside the `36px` bottom-right action capsule.
pub fn bar_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    active: bool,
    primary: bool,
    keys: &[&str],
) -> Stateful<Div> {
    let btn = div()
        .id(id)
        .h(px(28.0))
        .pl(px(10.0))
        .pr(px(6.0))
        .rounded(px(14.0))
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(6.0));

    let btn = if active {
        btn.bg(rgb(Theme::BAR_ACTIVE))
    } else {
        btn
    };

    btn.child(
        div()
            .text_size(px(12.5))
            .font_weight(if primary {
                FontWeight::MEDIUM
            } else {
                FontWeight::NORMAL
            })
            .text_color(rgb(if primary || active {
                Theme::TEXT_PRIMARY
            } else {
                Theme::TEXT_SECONDARY
            }))
            .child(label.into()),
    )
    .child(keycap_group(keys))
}

/// Tinycast `32px` popover menu row with optional trailing keycaps.
pub fn popover_row(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    selected: bool,
    keys: &[&str],
) -> Stateful<Div> {
    let row = div()
        .id(id)
        .w_full()
        .h(px(32.0))
        .px(px(10.0))
        .rounded(px(8.0))
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_between();

    let row = if selected { row.bg(rgb(0x313139)) } else { row };

    let row = row.child(
        div()
            .text_size(px(13.0))
            .text_color(rgb(Theme::TEXT_PRIMARY))
            .child(label.into()),
    );

    if keys.is_empty() {
        row
    } else {
        row.child(keycap_group(keys))
    }
}
