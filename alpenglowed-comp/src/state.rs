//! The whole compositor, in one struct.
//!
//! Split the way cosmic-comp splits it: everything protocol- and
//! desktop-related lives in `Alpenglowed`, and the backend is a separate
//! value the event loop owns. That keeps the state monomorphic instead of
//! infecting every `impl` with anvil's `<BackendData: Backend>`.

use std::{
    io::Read, os::unix::net::UnixListener, path::PathBuf, process::Command, sync::Arc,
    time::Instant,
};

use smithay::{
    desktop::{layer_map_for_output, PopupManager, Space, Window, WindowSurfaceType},
    input::{Seat, SeatState},
    reexports::{
        calloop::{
            generic::Generic, ping::Ping, EventLoop, Interest, LoopSignal, Mode, PostAction,
        },
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::wl_surface::WlSurface,
            Display, DisplayHandle,
        },
    },
    utils::{Logical, Point, SERIAL_COUNTER},
    wayland::{
        compositor::{CompositorClientState, CompositorState},
        output::OutputManagerState,
        selection::data_device::DataDeviceState,
        shell::{wlr_layer::WlrLayerShellState, xdg::XdgShellState},
        shm::ShmState,
        socket::ListeningSocketSource,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompWindowMode {
    NiriStrip,
    Tiling,
    Floating,
    Monocle,
    Stack,
    Center,
    Grid,
}

impl CompWindowMode {
    pub fn from_label(label: &str) -> Option<Self> {
        match label.trim().to_lowercase().replace('-', "_").as_str() {
            "niri" | "niri_strip" | "strip" | "scrollable" | "columns" => Some(Self::NiriStrip),
            "tiling" | "tile" | "bsp" => Some(Self::Tiling),
            "floating" | "float" | "windowing" => Some(Self::Floating),
            "monocle" | "max" => Some(Self::Monocle),
            "stack" => Some(Self::Stack),
            "center" | "centered" => Some(Self::Center),
            "grid" => Some(Self::Grid),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::NiriStrip => "niri",
            Self::Tiling => "tiling",
            Self::Floating => "floating",
            Self::Monocle => "monocle",
            Self::Stack => "stack",
            Self::Center => "center",
            Self::Grid => "grid",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::NiriStrip => Self::Tiling,
            Self::Tiling => Self::Floating,
            Self::Floating => Self::Monocle,
            Self::Monocle => Self::Stack,
            Self::Stack => Self::Center,
            Self::Center => Self::Grid,
            Self::Grid => Self::NiriStrip,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompColumnWidth {
    OneThird,
    OneHalf,
    TwoThirds,
    Full,
}

impl CompColumnWidth {
    pub fn fraction(self) -> f64 {
        match self {
            Self::OneThird => 1.0 / 3.0,
            Self::OneHalf => 0.5,
            Self::TwoThirds => 2.0 / 3.0,
            Self::Full => 1.0,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::OneThird => Self::OneHalf,
            Self::OneHalf => Self::TwoThirds,
            Self::TwoThirds => Self::Full,
            Self::Full => Self::OneThird,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompNiriColumn {
    pub width: CompColumnWidth,
    pub window_indices: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompLayoutAction {
    SplitRow,
    SplitColumn,
    FlipAxis,
    Reset,
    NudgeLeft,
    NudgeRight,
    NudgeUp,
    NudgeDown,
    ExpandWindow,
    ContractWindow,
    FocusNext,
    FocusPrevious,
    FocusFirst,
    FocusLast,
    CloseFocused,
    ToggleFloat,
    GrowFocused,
    ShrinkFocused,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    BalancePanes,
    CenterFocused,
    SnapLeftHalf,
    SnapRightHalf,
    SnapTopHalf,
    SnapBottomHalf,
    SnapTopLeft,
    SnapTopRight,
    SnapBottomLeft,
    SnapBottomRight,
    SnapFirstThird,
    SnapCenterThird,
    SnapLastThird,
    SnapFirstTwoThirds,
    SnapLastTwoThirds,
    Maximize,
    AlmostMaximize,
    Restore,
    ReasonableSize,
    ToggleFullscreen,
    NextDisplay,
    PreviousDisplay,
    FocusColumnLeft,
    FocusColumnRight,
    MoveColumnLeft,
    MoveColumnRight,
    CycleColumnWidth,
    MaximizeColumn,
    ConsumeWindowIntoColumn,
    ExpelWindowFromColumn,
    ScrollViewportLeft,
    ScrollViewportRight,
    CycleRoomLayout,
    EnterRoomFocus,
    EnterRoomStack,
    EnterRoomColumns,
    EnterRoomGrid,
}

impl CompLayoutAction {
    pub fn from_label(label: &str) -> Option<Self> {
        match label
            .trim()
            .to_lowercase()
            .replace(['-', ' '], "_")
            .as_str()
        {
            "split_row" => Some(Self::SplitRow),
            "split_column" => Some(Self::SplitColumn),
            "flip_axis" => Some(Self::FlipAxis),
            "reset" => Some(Self::Reset),
            "nudge_left" => Some(Self::NudgeLeft),
            "nudge_right" => Some(Self::NudgeRight),
            "nudge_up" => Some(Self::NudgeUp),
            "nudge_down" => Some(Self::NudgeDown),
            "expand_window" => Some(Self::ExpandWindow),
            "contract_window" => Some(Self::ContractWindow),
            "focus_next" => Some(Self::FocusNext),
            "focus_previous" => Some(Self::FocusPrevious),
            "focus_first" => Some(Self::FocusFirst),
            "focus_last" => Some(Self::FocusLast),
            "close_focused" => Some(Self::CloseFocused),
            "toggle_float" => Some(Self::ToggleFloat),
            "grow_focused" => Some(Self::GrowFocused),
            "shrink_focused" => Some(Self::ShrinkFocused),
            "move_left" => Some(Self::MoveLeft),
            "move_right" => Some(Self::MoveRight),
            "move_up" => Some(Self::MoveUp),
            "move_down" => Some(Self::MoveDown),
            "balance_panes" => Some(Self::BalancePanes),
            "center_focused" => Some(Self::CenterFocused),
            "snap_left_half" => Some(Self::SnapLeftHalf),
            "snap_right_half" => Some(Self::SnapRightHalf),
            "snap_top_half" => Some(Self::SnapTopHalf),
            "snap_bottom_half" => Some(Self::SnapBottomHalf),
            "snap_top_left" => Some(Self::SnapTopLeft),
            "snap_top_right" => Some(Self::SnapTopRight),
            "snap_bottom_left" => Some(Self::SnapBottomLeft),
            "snap_bottom_right" => Some(Self::SnapBottomRight),
            "snap_first_third" => Some(Self::SnapFirstThird),
            "snap_center_third" => Some(Self::SnapCenterThird),
            "snap_last_third" => Some(Self::SnapLastThird),
            "snap_first_two_thirds" => Some(Self::SnapFirstTwoThirds),
            "snap_last_two_thirds" => Some(Self::SnapLastTwoThirds),
            "maximize" => Some(Self::Maximize),
            "almost_maximize" => Some(Self::AlmostMaximize),
            "restore" => Some(Self::Restore),
            "reasonable_size" => Some(Self::ReasonableSize),
            "toggle_fullscreen" => Some(Self::ToggleFullscreen),
            "next_display" => Some(Self::NextDisplay),
            "previous_display" => Some(Self::PreviousDisplay),
            "focus_column_left" | "column_left" => Some(Self::FocusColumnLeft),
            "focus_column_right" | "column_right" => Some(Self::FocusColumnRight),
            "move_column_left" => Some(Self::MoveColumnLeft),
            "move_column_right" => Some(Self::MoveColumnRight),
            "cycle_column_width" | "cycle_width" | "column_width" => Some(Self::CycleColumnWidth),
            "maximize_column" | "max_column" => Some(Self::MaximizeColumn),
            "consume_window_into_column" | "consume_window" | "consume" => {
                Some(Self::ConsumeWindowIntoColumn)
            }
            "expel_window_from_column" | "expel_window" | "expel" => {
                Some(Self::ExpelWindowFromColumn)
            }
            "scroll_viewport_left" | "scroll_left" => Some(Self::ScrollViewportLeft),
            "scroll_viewport_right" | "scroll_right" => Some(Self::ScrollViewportRight),
            "cycle_room_layout" | "room_next" | "switch_room" => Some(Self::CycleRoomLayout),
            "enter_room_focus" | "room_focus" => Some(Self::EnterRoomFocus),
            "enter_room_stack" | "room_stack" => Some(Self::EnterRoomStack),
            "enter_room_columns" | "room_columns" => Some(Self::EnterRoomColumns),
            "enter_room_grid" | "room_grid" => Some(Self::EnterRoomGrid),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompIpcCommand {
    SetWindowMode(CompWindowMode),
    CycleWindowMode,
    FocusWindow(usize),
    Layout(CompLayoutAction),
    DesktopAction(String),
}

fn extract_json_str<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let pattern = format!("\"{key}\"");
    let key_pos = json.find(&pattern)?;
    let after_key = &json[key_pos + pattern.len()..];
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key[colon_pos + 1..].trim_start();
    let stripped = after_colon.strip_prefix('"')?;
    let end_quote = stripped.find('"')?;
    Some(&stripped[..end_quote])
}

fn extract_json_usize(json: &str, key: &str) -> Option<usize> {
    let pattern = format!("\"{key}\"");
    let key_pos = json.find(&pattern)?;
    let after_key = &json[key_pos + pattern.len()..];
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key[colon_pos + 1..].trim_start();
    let digits: String = after_colon
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

pub fn parse_ipc_json(payload: &str) -> Option<CompIpcCommand> {
    let kind = extract_json_str(payload, "type").or_else(|| extract_json_str(payload, "kind"))?;
    match kind {
        "set_window_mode" => {
            let mode_str = extract_json_str(payload, "mode")?;
            Some(CompIpcCommand::SetWindowMode(CompWindowMode::from_label(
                mode_str,
            )?))
        }
        "cycle_window_mode" => Some(CompIpcCommand::CycleWindowMode),
        "focus_window" => {
            let id = extract_json_usize(payload, "id")?;
            Some(CompIpcCommand::FocusWindow(id))
        }
        "layout" => {
            let action_str = extract_json_str(payload, "action")?;
            Some(CompIpcCommand::Layout(CompLayoutAction::from_label(
                action_str,
            )?))
        }
        "desktop_action" => {
            let action = extract_json_str(payload, "action")?.to_string();
            Some(CompIpcCommand::DesktopAction(action))
        }
        _ => None,
    }
}

pub type CompWindowRect = (usize, (i32, i32, i32, i32));

pub fn compute_niri_strip_rects(
    workarea: (i32, i32, i32, i32),
    columns: &[CompNiriColumn],
    focused_index: usize,
    gap: i32,
) -> (Vec<CompWindowRect>, i32) {
    let (wx, wy, ww, wh) = workarea;
    let inner_x = wx + gap;
    let inner_y = wy + gap;
    let inner_w = (ww - gap * 2).max(120);
    let inner_h = (wh - gap * 2).max(90);

    let mut cursor_x = 0i32;
    let mut focused_span = None;
    let mut col_spans = Vec::with_capacity(columns.len());
    for (idx, col) in columns.iter().enumerate() {
        let col_w = ((inner_w as f64) * col.width.fraction()).round() as i32;
        let col_w = col_w.max(100);
        if col.window_indices.contains(&focused_index) {
            focused_span = Some((cursor_x, col_w));
        }
        col_spans.push((cursor_x, col_w));
        cursor_x += col_w;
        if idx + 1 < columns.len() {
            cursor_x += gap;
        }
    }
    let strip_w = cursor_x;
    let mut viewport_x = 0i32;
    if let Some((col_x, col_w)) = focused_span {
        if col_x + col_w > inner_w {
            viewport_x = (col_x + col_w - inner_w).max(0);
        }
    }
    viewport_x = viewport_x.clamp(0, (strip_w - inner_w).max(0));

    let mut out = Vec::new();
    for (col, (col_x, col_w)) in columns.iter().zip(col_spans) {
        if col.window_indices.is_empty() {
            continue;
        }
        let screen_x = inner_x + col_x - viewport_x;
        let k = col.window_indices.len() as i32;
        let cell_h = ((inner_h - gap * (k - 1).max(0)) / k).max(60);
        for (row_idx, &win_idx) in col.window_indices.iter().enumerate() {
            let ry = inner_y + (row_idx as i32) * (cell_h + gap);
            let rh = if (row_idx as i32) + 1 == k {
                (inner_y + inner_h - ry).max(60)
            } else {
                cell_h
            };
            out.push((win_idx, (screen_x, ry, col_w, rh)));
        }
    }
    (out, viewport_x)
}

pub fn compute_window_rects(
    mode: CompWindowMode,
    workarea: (i32, i32, i32, i32),
    count: usize,
    master_axis_row: bool,
    master_ratio: f64,
    gap: i32,
) -> Vec<(i32, i32, i32, i32)> {
    if count == 0 {
        return Vec::new();
    }
    let (wx, wy, ww, wh) = workarea;
    let inner_x = wx + gap;
    let inner_y = wy + gap;
    let inner_w = (ww - gap * 2).max(120);
    let inner_h = (wh - gap * 2).max(90);

    match mode {
        CompWindowMode::NiriStrip => {
            let cols: Vec<CompNiriColumn> = (0..count)
                .map(|i| CompNiriColumn {
                    width: CompColumnWidth::OneHalf,
                    window_indices: vec![i],
                })
                .collect();
            let (rects, _) =
                compute_niri_strip_rects(workarea, &cols, count.saturating_sub(1), gap);
            rects.into_iter().map(|(_, r)| r).collect()
        }
        CompWindowMode::Monocle => (0..count)
            .map(|_| (inner_x, inner_y, inner_w, inner_h))
            .collect(),
        CompWindowMode::Center => {
            let cw = ((inner_w as f64) * 0.78).round() as i32;
            let ch = ((inner_h as f64) * 0.78).round() as i32;
            let cx = inner_x + (inner_w - cw) / 2;
            let cy = inner_y + (inner_h - ch) / 2;
            (0..count).map(|_| (cx, cy, cw, ch)).collect()
        }
        CompWindowMode::Floating | CompWindowMode::Stack => {
            let step = if matches!(mode, CompWindowMode::Stack) {
                24
            } else {
                32
            };
            let w = (inner_w * 3 / 4).max(320);
            let h = (inner_h * 3 / 4).max(240);
            (0..count)
                .map(|i| {
                    let offset = (i as i32) * step;
                    (inner_x + offset, inner_y + offset, w, h)
                })
                .collect()
        }
        CompWindowMode::Grid => {
            let cols = (count as f64).sqrt().ceil().max(1.0) as usize;
            let rows = count.div_ceil(cols).max(1);
            let cell_w = ((inner_w - gap * (cols as i32 - 1).max(0)) / cols as i32).max(80);
            let cell_h = ((inner_h - gap * (rows as i32 - 1).max(0)) / rows as i32).max(60);
            (0..count)
                .map(|i| {
                    let col = (i % cols) as i32;
                    let row = (i / cols) as i32;
                    (
                        inner_x + col * (cell_w + gap),
                        inner_y + row * (cell_h + gap),
                        cell_w,
                        cell_h,
                    )
                })
                .collect()
        }
        CompWindowMode::Tiling => {
            if count == 1 {
                return vec![(inner_x, inner_y, inner_w, inner_h)];
            }
            let ratio = master_ratio.clamp(0.2, 0.8);
            let stack_count = (count - 1) as i32;
            let mut out = Vec::with_capacity(count);
            if master_axis_row {
                let master_w = (((inner_w - gap) as f64) * ratio).round() as i32;
                let stack_w = (inner_w - gap - master_w).max(80);
                out.push((inner_x, inner_y, master_w.max(80), inner_h));
                let stack_h = ((inner_h - gap * (stack_count - 1).max(0)) / stack_count).max(60);
                for i in 0..stack_count {
                    out.push((
                        inner_x + master_w + gap,
                        inner_y + i * (stack_h + gap),
                        stack_w,
                        stack_h,
                    ));
                }
            } else {
                let master_h = (((inner_h - gap) as f64) * ratio).round() as i32;
                let stack_h = (inner_h - gap - master_h).max(60);
                out.push((inner_x, inner_y, inner_w, master_h.max(60)));
                let stack_w = ((inner_w - gap * (stack_count - 1).max(0)) / stack_count).max(80);
                for i in 0..stack_count {
                    out.push((
                        inner_x + i * (stack_w + gap),
                        inner_y + master_h + gap,
                        stack_w,
                        stack_h,
                    ));
                }
            }
            out
        }
    }
}

pub fn snap_rect(
    action: CompLayoutAction,
    workarea: (i32, i32, i32, i32),
) -> Option<(i32, i32, i32, i32)> {
    let (wx, wy, ww, wh) = workarea;
    let hw = (ww / 2).max(120);
    let hh = (wh / 2).max(90);
    let tw = (ww / 3).max(100);
    let two_tw = ((ww * 2) / 3).max(160);
    match action {
        CompLayoutAction::SnapLeftHalf => Some((wx, wy, hw, wh)),
        CompLayoutAction::SnapRightHalf => Some((wx + ww - hw, wy, hw, wh)),
        CompLayoutAction::SnapTopHalf => Some((wx, wy, ww, hh)),
        CompLayoutAction::SnapBottomHalf => Some((wx, wy + wh - hh, ww, hh)),
        CompLayoutAction::SnapTopLeft => Some((wx, wy, hw, hh)),
        CompLayoutAction::SnapTopRight => Some((wx + ww - hw, wy, hw, hh)),
        CompLayoutAction::SnapBottomLeft => Some((wx, wy + wh - hh, hw, hh)),
        CompLayoutAction::SnapBottomRight => Some((wx + ww - hw, wy + wh - hh, hw, hh)),
        CompLayoutAction::SnapFirstThird => Some((wx, wy, tw, wh)),
        CompLayoutAction::SnapCenterThird => Some((wx + (ww - tw) / 2, wy, tw, wh)),
        CompLayoutAction::SnapLastThird => Some((wx + ww - tw, wy, tw, wh)),
        CompLayoutAction::SnapFirstTwoThirds => Some((wx, wy, two_tw, wh)),
        CompLayoutAction::SnapLastTwoThirds => Some((wx + ww - two_tw, wy, two_tw, wh)),
        CompLayoutAction::Maximize | CompLayoutAction::ToggleFullscreen => Some((wx, wy, ww, wh)),
        CompLayoutAction::AlmostMaximize => {
            let aw = ((ww as f64) * 0.92).round() as i32;
            let ah = ((wh as f64) * 0.92).round() as i32;
            Some((wx + (ww - aw) / 2, wy + (wh - ah) / 2, aw, ah))
        }
        CompLayoutAction::ReasonableSize
        | CompLayoutAction::Restore
        | CompLayoutAction::CenterFocused => {
            let rw = ((ww as f64) * 0.70).round() as i32;
            let rh = ((wh as f64) * 0.72).round() as i32;
            Some((wx + (ww - rw) / 2, wy + (wh - rh) / 2, rw, rh))
        }
        _ => None,
    }
}

pub struct Alpenglowed {
    #[cfg_attr(not(feature = "winit"), allow(dead_code))]
    pub start_time: Instant,
    pub display_handle: DisplayHandle,
    pub loop_signal: LoopSignal,
    pub socket_name: String,

    pub space: Space<Window>,
    pub popups: PopupManager,

    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub layer_shell_state: WlrLayerShellState,
    pub shm_state: ShmState,
    #[allow(dead_code)]
    pub output_manager_state: OutputManagerState,
    pub seat_state: SeatState<Self>,
    pub data_device_state: DataDeviceState,
    pub seat: Seat<Self>,

    pub window_mode: CompWindowMode,
    pub master_axis_row: bool,
    pub master_ratio: f64,
    pub focused_index: usize,
    pub niri_columns: Vec<CompNiriColumn>,
    pub floating_surfaces: Vec<WlSurface>,
    pub ipc_socket_path: Option<PathBuf>,

    /// Wakes the nested winit window to redraw without busy-spinning.
    pub redraw_ping: Option<Ping>,
}

impl Alpenglowed {
    pub fn new(event_loop: &mut EventLoop<'static, Self>, display: Display<Self>) -> Self {
        let display_handle = display.handle();
        let loop_signal = event_loop.get_signal();

        let compositor_state = CompositorState::new::<Self>(&display_handle);
        let xdg_shell_state = XdgShellState::new::<Self>(&display_handle);
        let layer_shell_state = WlrLayerShellState::new::<Self>(&display_handle);
        let shm_state = ShmState::new::<Self>(&display_handle, vec![]);
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&display_handle);
        let mut seat_state = SeatState::new();
        let data_device_state = DataDeviceState::new::<Self>(&display_handle);

        let mut seat: Seat<Self> = seat_state.new_wl_seat(&display_handle, "alpenglowed");
        seat.add_keyboard(Default::default(), 200, 25)
            .expect("a seat with no keyboard is not a desktop");
        seat.add_pointer();

        let socket_name = Self::init_socket(event_loop, display);
        let ipc_socket_path = Self::init_ipc_socket(event_loop);

        let initial_mode = std::env::var("ALPENGLOWED_MODE")
            .ok()
            .and_then(|m| CompWindowMode::from_label(&m))
            .unwrap_or(CompWindowMode::NiriStrip);

        Self {
            start_time: Instant::now(),
            display_handle,
            loop_signal,
            socket_name,
            space: Space::default(),
            popups: PopupManager::default(),
            compositor_state,
            xdg_shell_state,
            layer_shell_state,
            shm_state,
            output_manager_state,
            seat_state,
            data_device_state,
            seat,
            window_mode: initial_mode,
            master_axis_row: true,
            master_ratio: 0.5,
            focused_index: 0,
            niri_columns: Vec::new(),
            floating_surfaces: Vec::new(),
            ipc_socket_path,
            redraw_ping: None,
        }
    }

    pub fn queue_redraw(&self) {
        if let Some(ping) = &self.redraw_ping {
            ping.ping();
        }
    }

    pub fn output_workarea(&self) -> (i32, i32, i32, i32) {
        if let Some(output) = self.space.outputs().next() {
            let map = layer_map_for_output(output);
            let zone = map.non_exclusive_zone();
            if zone.size.w > 0 && zone.size.h > 0 {
                return (zone.loc.x, zone.loc.y, zone.size.w, zone.size.h);
            }
            if let Some(geo) = self.space.output_geometry(output) {
                return (geo.loc.x, geo.loc.y, geo.size.w, geo.size.h);
            }
        }
        (0, 0, 1440, 900)
    }

    pub fn relayout(&mut self) {
        let workarea = self.output_workarea();
        let all_windows: Vec<Window> = self.space.elements().cloned().collect();
        if all_windows.is_empty() {
            self.focused_index = 0;
            return;
        }
        if self.focused_index >= all_windows.len() {
            self.focused_index = all_windows.len() - 1;
        }

        if matches!(
            self.window_mode,
            CompWindowMode::Floating | CompWindowMode::Stack
        ) {
            let rects = compute_window_rects(
                self.window_mode,
                workarea,
                all_windows.len(),
                self.master_axis_row,
                self.master_ratio,
                8,
            );
            for (win, (x, y, w, h)) in all_windows.iter().zip(rects) {
                if let Some(toplevel) = win.toplevel() {
                    toplevel.with_pending_state(|state| {
                        state.size = Some((w, h).into());
                    });
                    toplevel.send_configure();
                }
                self.space.map_element(win.clone(), (x, y), false);
            }
            self.focus_window_at(self.focused_index);
            self.queue_redraw();
            return;
        }

        let tiled_windows: Vec<Window> = all_windows
            .iter()
            .filter(|w| {
                w.toplevel()
                    .map(|t| !self.floating_surfaces.contains(t.wl_surface()))
                    .unwrap_or(true)
            })
            .cloned()
            .collect();

        self.sync_niri_columns(tiled_windows.len());

        if self.window_mode == CompWindowMode::NiriStrip {
            let (indexed_rects, _) =
                compute_niri_strip_rects(workarea, &self.niri_columns, self.focused_index, 8);
            for (win_idx, (x, y, w, h)) in indexed_rects {
                if let Some(win) = tiled_windows.get(win_idx) {
                    if let Some(toplevel) = win.toplevel() {
                        toplevel.with_pending_state(|state| {
                            state.size = Some((w, h).into());
                            state.states.set(xdg_toplevel::State::TiledLeft);
                            state.states.set(xdg_toplevel::State::TiledRight);
                            state.states.set(xdg_toplevel::State::TiledTop);
                            state.states.set(xdg_toplevel::State::TiledBottom);
                        });
                        toplevel.send_configure();
                    }
                    self.space.map_element(win.clone(), (x, y), false);
                }
            }
            self.focus_window_at(self.focused_index);
            self.queue_redraw();
            return;
        }

        let rects = compute_window_rects(
            self.window_mode,
            workarea,
            tiled_windows.len(),
            self.master_axis_row,
            self.master_ratio,
            8,
        );

        for (win, (x, y, w, h)) in tiled_windows.iter().zip(rects) {
            if let Some(toplevel) = win.toplevel() {
                toplevel.with_pending_state(|state| {
                    state.size = Some((w, h).into());
                    state.states.set(xdg_toplevel::State::TiledLeft);
                    state.states.set(xdg_toplevel::State::TiledRight);
                    state.states.set(xdg_toplevel::State::TiledTop);
                    state.states.set(xdg_toplevel::State::TiledBottom);
                });
                toplevel.send_configure();
            }
            self.space.map_element(win.clone(), (x, y), false);
        }

        self.focus_window_at(self.focused_index);
        self.queue_redraw();
    }

    fn sync_niri_columns(&mut self, tiled_count: usize) {
        for col in &mut self.niri_columns {
            col.window_indices.retain(|&idx| idx < tiled_count);
        }
        self.niri_columns
            .retain(|col| !col.window_indices.is_empty());
        for idx in 0..tiled_count {
            if !self
                .niri_columns
                .iter()
                .any(|c| c.window_indices.contains(&idx))
            {
                self.niri_columns.push(CompNiriColumn {
                    width: CompColumnWidth::OneHalf,
                    window_indices: vec![idx],
                });
            }
        }
    }

    pub fn focus_window_at(&mut self, index: usize) {
        let windows: Vec<Window> = self.space.elements().cloned().collect();
        if windows.is_empty() {
            return;
        }
        let clamped = index.min(windows.len() - 1);
        self.focused_index = clamped;
        let win = &windows[clamped];
        self.space.raise_element(win, true);
        if let Some(toplevel) = win.toplevel() {
            let surface = toplevel.wl_surface().clone();
            let serial = SERIAL_COUNTER.next_serial();
            if let Some(keyboard) = self.seat.get_keyboard() {
                keyboard.set_focus(self, Some(surface), serial);
            }
        }
    }

    pub fn apply_layout_action(&mut self, action: CompLayoutAction) {
        let windows: Vec<Window> = self.space.elements().cloned().collect();
        let count = windows.len();

        if let Some((sx, sy, sw, sh)) = snap_rect(action, self.output_workarea()) {
            if let Some(win) = windows.get(self.focused_index).cloned() {
                if let Some(toplevel) = win.toplevel() {
                    let wl_surf = toplevel.wl_surface().clone();
                    if !self.floating_surfaces.contains(&wl_surf) {
                        self.floating_surfaces.push(wl_surf);
                    }
                    toplevel.with_pending_state(|state| {
                        state.size = Some((sw, sh).into());
                        if matches!(action, CompLayoutAction::ToggleFullscreen) {
                            state.states.set(xdg_toplevel::State::Fullscreen);
                        }
                    });
                    toplevel.send_configure();
                }
                self.space.map_element(win.clone(), (sx, sy), true);
                self.queue_redraw();
            }
            return;
        }

        match action {
            CompLayoutAction::SplitRow => {
                self.master_axis_row = true;
                self.relayout();
            }
            CompLayoutAction::SplitColumn => {
                self.master_axis_row = false;
                self.relayout();
            }
            CompLayoutAction::FlipAxis => {
                self.master_axis_row = !self.master_axis_row;
                self.relayout();
            }
            CompLayoutAction::Reset | CompLayoutAction::BalancePanes => {
                self.master_ratio = 0.5;
                self.niri_columns.clear();
                self.floating_surfaces.clear();
                self.relayout();
            }
            CompLayoutAction::GrowFocused | CompLayoutAction::ExpandWindow => {
                self.master_ratio = (self.master_ratio + 0.05).clamp(0.2, 0.8);
                self.relayout();
            }
            CompLayoutAction::ShrinkFocused | CompLayoutAction::ContractWindow => {
                self.master_ratio = (self.master_ratio - 0.05).clamp(0.2, 0.8);
                self.relayout();
            }
            CompLayoutAction::FocusColumnLeft | CompLayoutAction::ScrollViewportLeft => {
                if self.focused_index > 0 {
                    self.focus_window_at(self.focused_index - 1);
                    self.relayout();
                }
            }
            CompLayoutAction::FocusColumnRight | CompLayoutAction::ScrollViewportRight => {
                if self.focused_index + 1 < count {
                    self.focus_window_at(self.focused_index + 1);
                    self.relayout();
                }
            }
            CompLayoutAction::MoveColumnLeft => {
                self.sync_niri_columns(count);
                if let Some(pos) = self
                    .niri_columns
                    .iter()
                    .position(|c| c.window_indices.contains(&self.focused_index))
                {
                    if pos > 0 {
                        self.niri_columns.swap(pos, pos - 1);
                    }
                }
                self.relayout();
            }
            CompLayoutAction::MoveColumnRight => {
                self.sync_niri_columns(count);
                if let Some(pos) = self
                    .niri_columns
                    .iter()
                    .position(|c| c.window_indices.contains(&self.focused_index))
                {
                    if pos + 1 < self.niri_columns.len() {
                        self.niri_columns.swap(pos, pos + 1);
                    }
                }
                self.relayout();
            }
            CompLayoutAction::CycleColumnWidth => {
                self.sync_niri_columns(count);
                if let Some(col) = self
                    .niri_columns
                    .iter_mut()
                    .find(|c| c.window_indices.contains(&self.focused_index))
                {
                    col.width = col.width.next();
                }
                self.relayout();
            }
            CompLayoutAction::MaximizeColumn => {
                self.sync_niri_columns(count);
                if let Some(col) = self
                    .niri_columns
                    .iter_mut()
                    .find(|c| c.window_indices.contains(&self.focused_index))
                {
                    col.width = if col.width == CompColumnWidth::Full {
                        CompColumnWidth::OneHalf
                    } else {
                        CompColumnWidth::Full
                    };
                }
                self.relayout();
            }
            CompLayoutAction::ConsumeWindowIntoColumn => {
                self.sync_niri_columns(count);
                if let Some(pos) = self
                    .niri_columns
                    .iter()
                    .position(|c| c.window_indices.contains(&self.focused_index))
                {
                    let donor = if pos + 1 < self.niri_columns.len() {
                        Some(pos + 1)
                    } else if pos > 0 {
                        Some(pos - 1)
                    } else {
                        None
                    };
                    if let Some(d) = donor {
                        if !self.niri_columns[d].window_indices.is_empty() {
                            let idx = self.niri_columns[d].window_indices.remove(0);
                            self.niri_columns[pos].window_indices.push(idx);
                            if self.niri_columns[d].window_indices.is_empty() {
                                self.niri_columns.remove(d);
                            }
                        }
                    }
                }
                self.relayout();
            }
            CompLayoutAction::ExpelWindowFromColumn => {
                self.sync_niri_columns(count);
                if let Some(pos) = self
                    .niri_columns
                    .iter()
                    .position(|c| c.window_indices.contains(&self.focused_index))
                {
                    if self.niri_columns[pos].window_indices.len() > 1 {
                        let w = self.niri_columns[pos].width;
                        let focused = self.focused_index;
                        self.niri_columns[pos]
                            .window_indices
                            .retain(|&i| i != focused);
                        self.niri_columns.insert(
                            pos + 1,
                            CompNiriColumn {
                                width: w,
                                window_indices: vec![focused],
                            },
                        );
                    }
                }
                self.relayout();
            }
            CompLayoutAction::FocusNext
            | CompLayoutAction::MoveRight
            | CompLayoutAction::MoveDown
            | CompLayoutAction::NextDisplay => {
                if count > 0 {
                    self.focus_window_at((self.focused_index + 1) % count);
                    self.relayout();
                }
            }
            CompLayoutAction::FocusPrevious
            | CompLayoutAction::MoveLeft
            | CompLayoutAction::MoveUp
            | CompLayoutAction::PreviousDisplay => {
                if count > 0 {
                    let prev = if self.focused_index == 0 {
                        count - 1
                    } else {
                        self.focused_index - 1
                    };
                    self.focus_window_at(prev);
                    self.relayout();
                }
            }
            CompLayoutAction::FocusFirst => {
                if count > 0 {
                    self.focus_window_at(0);
                    self.relayout();
                }
            }
            CompLayoutAction::FocusLast => {
                if count > 0 {
                    self.focus_window_at(count - 1);
                    self.relayout();
                }
            }
            CompLayoutAction::CloseFocused => {
                if let Some(win) = windows.get(self.focused_index) {
                    if let Some(toplevel) = win.toplevel() {
                        toplevel.send_close();
                    }
                }
            }
            CompLayoutAction::ToggleFloat => {
                if let Some(win) = windows.get(self.focused_index) {
                    if let Some(toplevel) = win.toplevel() {
                        let surf = toplevel.wl_surface().clone();
                        if let Some(pos) = self.floating_surfaces.iter().position(|s| s == &surf) {
                            self.floating_surfaces.remove(pos);
                        } else {
                            self.floating_surfaces.push(surf);
                        }
                    }
                }
                self.relayout();
            }
            CompLayoutAction::NudgeLeft
            | CompLayoutAction::NudgeRight
            | CompLayoutAction::NudgeUp
            | CompLayoutAction::NudgeDown => {
                let (dx, dy) = match action {
                    CompLayoutAction::NudgeLeft => (-24, 0),
                    CompLayoutAction::NudgeRight => (24, 0),
                    CompLayoutAction::NudgeUp => (0, -24),
                    CompLayoutAction::NudgeDown => (0, 24),
                    _ => (0, 0),
                };
                if let Some(win) = windows.get(self.focused_index).cloned() {
                    if let Some(loc) = self.space.element_location(&win) {
                        self.space.map_element(win, (loc.x + dx, loc.y + dy), true);
                        self.queue_redraw();
                    }
                }
            }
            CompLayoutAction::CycleRoomLayout => {
                self.window_mode = self.window_mode.next();
                self.relayout();
            }
            CompLayoutAction::EnterRoomFocus => {
                self.window_mode = CompWindowMode::Tiling;
                self.relayout();
            }
            CompLayoutAction::EnterRoomStack => {
                self.window_mode = CompWindowMode::Stack;
                self.relayout();
            }
            CompLayoutAction::EnterRoomColumns => {
                self.window_mode = CompWindowMode::NiriStrip;
                self.relayout();
            }
            CompLayoutAction::EnterRoomGrid => {
                self.window_mode = CompWindowMode::Grid;
                self.relayout();
            }
            _ => {}
        }
    }

    pub fn handle_ipc_command(&mut self, cmd: CompIpcCommand) {
        match cmd {
            CompIpcCommand::SetWindowMode(mode) => {
                tracing::info!(mode = mode.label(), "compositor window mode updated");
                self.window_mode = mode;
                self.relayout();
            }
            CompIpcCommand::CycleWindowMode => {
                self.window_mode = self.window_mode.next();
                tracing::info!(
                    mode = self.window_mode.label(),
                    "compositor window mode cycled"
                );
                self.relayout();
            }
            CompIpcCommand::FocusWindow(id) => {
                let idx = id.saturating_sub(1);
                self.focus_window_at(idx);
                self.relayout();
            }
            CompIpcCommand::Layout(action) => {
                self.apply_layout_action(action);
            }
            CompIpcCommand::DesktopAction(action) => match action.as_str() {
                "apps" => {
                    self.spawn_tinycast_bar();
                }
                "terminal" => {
                    let _ = Command::new("foot")
                        .spawn()
                        .or_else(|_| Command::new("alacritty").spawn())
                        .or_else(|_| Command::new("kitty").spawn());
                }
                "logout" => {
                    self.loop_signal.stop();
                }
                _ => {
                    tracing::debug!(action = %action, "received desktop action over IPC");
                }
            },
        }
    }

    pub fn spawn_tinycast_bar(&self) {
        let bin = std::env::var("ALPENGLOWED_BIN").unwrap_or_else(|_| "alpenglowed".to_string());
        let mut cmd = Command::new(bin);
        cmd.env("WAYLAND_DISPLAY", &self.socket_name);
        if let Some(ref ipc) = self.ipc_socket_path {
            cmd.env("ALPENGLOW_SESSION_CONTROL", ipc);
        }
        let _ = cmd.spawn();
    }

    fn init_ipc_socket(event_loop: &mut EventLoop<'static, Self>) -> Option<PathBuf> {
        let path = std::env::var_os("ALPENGLOW_SESSION_CONTROL")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let base = std::env::var_os("XDG_RUNTIME_DIR")
                    .map(PathBuf::from)
                    .unwrap_or_else(std::env::temp_dir);
                let dir = base.join("alpenglowed");
                let _ = std::fs::create_dir_all(&dir);
                dir.join(format!("session-{}.sock", std::process::id()))
            });
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).ok()?;
        let _ = listener.set_nonblocking(true);
        std::env::set_var("ALPENGLOW_SESSION_CONTROL", &path);

        let _ = event_loop.handle().insert_source(
            Generic::new(listener, Interest::READ, Mode::Level),
            |_, listener, state| {
                while let Ok((mut stream, _)) = listener.accept() {
                    let mut buf = String::new();
                    if stream.read_to_string(&mut buf).is_ok() {
                        if let Some(cmd) = parse_ipc_json(&buf) {
                            state.handle_ipc_command(cmd);
                        }
                    }
                }
                Ok(PostAction::Continue)
            },
        );
        Some(path)
    }

    /// Listen, and hand every readable client fd back to wayland-server.
    fn init_socket(event_loop: &mut EventLoop<'static, Self>, display: Display<Self>) -> String {
        let source = ListeningSocketSource::new_auto().expect("no wayland socket available");
        let socket_name = source.socket_name().to_string_lossy().into_owned();
        let handle = event_loop.handle();

        handle
            .insert_source(source, move |stream, _, state| {
                state
                    .display_handle
                    .insert_client(stream, Arc::new(ClientState::default()))
                    .expect("client insert");
            })
            .expect("listening socket source");

        handle
            .insert_source(
                Generic::new(display, Interest::READ, Mode::Level),
                |_, display, state| {
                    // Safety: the display is only ever dispatched from here.
                    unsafe { display.get_mut().dispatch_clients(state).unwrap() };
                    Ok(PostAction::Continue)
                },
            )
            .expect("wayland display source");

        socket_name
    }

    /// What is under the pointer, in surface-local coordinates.
    pub fn surface_under(
        &self,
        pos: Point<f64, Logical>,
    ) -> Option<(WlSurface, Point<f64, Logical>)> {
        if let Some(output) = self.space.outputs().next() {
            let map = layer_map_for_output(output);
            if let Some(layer) = map
                .layer_under(smithay::wayland::shell::wlr_layer::Layer::Overlay, pos)
                .or_else(|| map.layer_under(smithay::wayland::shell::wlr_layer::Layer::Top, pos))
            {
                if let Some(geo) = map.layer_geometry(layer) {
                    if let Some((surf, offset)) =
                        layer.surface_under(pos - geo.loc.to_f64(), WindowSurfaceType::ALL)
                    {
                        return Some((surf, (geo.loc + offset).to_f64()));
                    }
                }
            }
        }
        self.space
            .element_under(pos)
            .and_then(|(window, location)| {
                window
                    .surface_under(pos - location.to_f64(), WindowSurfaceType::ALL)
                    .map(|(surface, offset)| (surface, (location + offset).to_f64()))
            })
    }
}

#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ipc_json_should_decode_session_requests() {
        assert_eq!(
            parse_ipc_json(r#"{"kind":"set_window_mode","mode":"tiling"}"#),
            Some(CompIpcCommand::SetWindowMode(CompWindowMode::Tiling))
        );
        assert_eq!(
            parse_ipc_json(r#"{"type":"set_window_mode","mode":"niri"}"#),
            Some(CompIpcCommand::SetWindowMode(CompWindowMode::NiriStrip))
        );
        assert_eq!(
            parse_ipc_json(r#"{"kind":"cycle_window_mode"}"#),
            Some(CompIpcCommand::CycleWindowMode)
        );
        assert_eq!(
            parse_ipc_json(r#"{"kind":"focus_window","id":3}"#),
            Some(CompIpcCommand::FocusWindow(3))
        );
        assert_eq!(
            parse_ipc_json(r#"{"kind":"layout","action":"snap_left_half"}"#),
            Some(CompIpcCommand::Layout(CompLayoutAction::SnapLeftHalf))
        );
        assert_eq!(
            parse_ipc_json(r#"{"type":"layout","action":"cycle_column_width"}"#),
            Some(CompIpcCommand::Layout(CompLayoutAction::CycleColumnWidth))
        );
        assert_eq!(
            parse_ipc_json(r#"{"kind":"desktop_action","action":"apps"}"#),
            Some(CompIpcCommand::DesktopAction("apps".to_string()))
        );
    }

    #[test]
    fn compute_window_rects_should_tile_monocle_and_float() {
        let workarea = (0, 0, 1200, 800);
        let tiled = compute_window_rects(CompWindowMode::Tiling, workarea, 2, true, 0.5, 0);
        assert_eq!(tiled.len(), 2);
        assert_eq!(tiled[0], (0, 0, 600, 800));
        assert_eq!(tiled[1], (600, 0, 600, 800));

        let monocle = compute_window_rects(CompWindowMode::Monocle, workarea, 2, true, 0.5, 0);
        assert_eq!(monocle[0], (0, 0, 1200, 800));
        assert_eq!(monocle[1], (0, 0, 1200, 800));
    }

    #[test]
    fn compute_niri_strip_rects_should_pan_and_stack_columns() {
        let workarea = (0, 0, 1200, 800);
        let cols = vec![
            CompNiriColumn {
                width: CompColumnWidth::OneHalf,
                window_indices: vec![0, 1],
            },
            CompNiriColumn {
                width: CompColumnWidth::TwoThirds,
                window_indices: vec![2],
            },
        ];
        let (rects, viewport_x) = compute_niri_strip_rects(workarea, &cols, 2, 0);
        assert_eq!(rects.len(), 3);
        assert!(viewport_x > 0);
        // First two windows are stacked vertically in column 0
        assert_eq!(rects[0].1 .3, 400);
        assert_eq!(rects[1].1 .1, 400);
    }

    #[test]
    fn snap_rect_should_position_halves_quarters_and_thirds() {
        let workarea = (0, 0, 1200, 800);
        assert_eq!(
            snap_rect(CompLayoutAction::SnapLeftHalf, workarea),
            Some((0, 0, 600, 800))
        );
        assert_eq!(
            snap_rect(CompLayoutAction::SnapRightHalf, workarea),
            Some((600, 0, 600, 800))
        );
        assert_eq!(
            snap_rect(CompLayoutAction::SnapBottomRight, workarea),
            Some((600, 400, 600, 400))
        );
        assert_eq!(
            snap_rect(CompLayoutAction::SnapFirstThird, workarea),
            Some((0, 0, 400, 800))
        );
    }
}
