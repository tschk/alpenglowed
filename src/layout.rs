use serde::{Deserialize, Serialize};

use crate::runner::WindowMode;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Axis {
    Row,
    Column,
}

impl Axis {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Row => "row",
            Self::Column => "column",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutAction {
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
    FocusFirst,
    FocusLast,
    // Tinycast / Rectangle-style window snapping & sizing actions
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
    // Niri-style scrollable strip actions
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
    // Tinycast Window Rooms actions
    CycleRoomLayout,
    EnterRoomFocus,
    EnterRoomStack,
    EnterRoomColumns,
    EnterRoomGrid,
}

impl LayoutAction {
    pub fn title(&self) -> &'static str {
        match self {
            Self::SplitRow => "Split row",
            Self::SplitColumn => "Split column",
            Self::FlipAxis => "Flip layout axis",
            Self::Reset => "Reset layout",
            Self::NudgeLeft => "Nudge left",
            Self::NudgeRight => "Nudge right",
            Self::NudgeUp => "Nudge up",
            Self::NudgeDown => "Nudge down",
            Self::ExpandWindow => "Expand window",
            Self::ContractWindow => "Contract window",
            Self::FocusNext => "Focus next",
            Self::FocusPrevious => "Focus previous",
            Self::CloseFocused => "Close focused",
            Self::ToggleFloat => "Toggle float",
            Self::GrowFocused => "Grow focused",
            Self::ShrinkFocused => "Shrink focused",
            Self::MoveLeft => "Move window left",
            Self::MoveRight => "Move window right",
            Self::MoveUp => "Move window up",
            Self::MoveDown => "Move window down",
            Self::BalancePanes => "Balance panes",
            Self::CenterFocused => "Center focused window",
            Self::FocusFirst => "Focus first window",
            Self::FocusLast => "Focus last window",
            Self::SnapLeftHalf => "Left Half",
            Self::SnapRightHalf => "Right Half",
            Self::SnapTopHalf => "Top Half",
            Self::SnapBottomHalf => "Bottom Half",
            Self::SnapTopLeft => "Top Left Quarter",
            Self::SnapTopRight => "Top Right Quarter",
            Self::SnapBottomLeft => "Bottom Left Quarter",
            Self::SnapBottomRight => "Bottom Right Quarter",
            Self::SnapFirstThird => "First Third",
            Self::SnapCenterThird => "Center Third",
            Self::SnapLastThird => "Last Third",
            Self::SnapFirstTwoThirds => "First Two Thirds",
            Self::SnapLastTwoThirds => "Last Two Thirds",
            Self::Maximize => "Maximize",
            Self::AlmostMaximize => "Almost Maximize",
            Self::Restore => "Restore",
            Self::ReasonableSize => "Reasonable Size",
            Self::ToggleFullscreen => "Toggle Fullscreen",
            Self::NextDisplay => "Next Display",
            Self::PreviousDisplay => "Previous Display",
            Self::FocusColumnLeft => "Niri: Focus Column Left",
            Self::FocusColumnRight => "Niri: Focus Column Right",
            Self::MoveColumnLeft => "Niri: Move Column Left",
            Self::MoveColumnRight => "Niri: Move Column Right",
            Self::CycleColumnWidth => "Niri: Cycle Column Width",
            Self::MaximizeColumn => "Niri: Maximize Column",
            Self::ConsumeWindowIntoColumn => "Niri: Consume Window into Column",
            Self::ExpelWindowFromColumn => "Niri: Expel Window from Column",
            Self::ScrollViewportLeft => "Niri: Scroll Strip Left",
            Self::ScrollViewportRight => "Niri: Scroll Strip Right",
            Self::CycleRoomLayout => "Room: Next Layout (Auto/Focus/Stack/Columns/Grid)",
            Self::EnterRoomFocus => "Room: Focus Layout (60% Main + Side Stack)",
            Self::EnterRoomStack => "Room: Stack Layout (Main + Cascading Peek)",
            Self::EnterRoomColumns => "Room: Columns Layout (Equal Columns)",
            Self::EnterRoomGrid => "Room: Grid Layout (Balanced Grid)",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::SplitRow => "split_row",
            Self::SplitColumn => "split_column",
            Self::FlipAxis => "flip_axis",
            Self::Reset => "reset",
            Self::NudgeLeft => "nudge_left",
            Self::NudgeRight => "nudge_right",
            Self::NudgeUp => "nudge_up",
            Self::NudgeDown => "nudge_down",
            Self::ExpandWindow => "expand_window",
            Self::ContractWindow => "contract_window",
            Self::FocusNext => "focus_next",
            Self::FocusPrevious => "focus_previous",
            Self::CloseFocused => "close_focused",
            Self::ToggleFloat => "toggle_float",
            Self::GrowFocused => "grow_focused",
            Self::ShrinkFocused => "shrink_focused",
            Self::MoveLeft => "move_left",
            Self::MoveRight => "move_right",
            Self::MoveUp => "move_up",
            Self::MoveDown => "move_down",
            Self::BalancePanes => "balance_panes",
            Self::CenterFocused => "center_focused",
            Self::FocusFirst => "focus_first",
            Self::FocusLast => "focus_last",
            Self::SnapLeftHalf => "snap_left_half",
            Self::SnapRightHalf => "snap_right_half",
            Self::SnapTopHalf => "snap_top_half",
            Self::SnapBottomHalf => "snap_bottom_half",
            Self::SnapTopLeft => "snap_top_left",
            Self::SnapTopRight => "snap_top_right",
            Self::SnapBottomLeft => "snap_bottom_left",
            Self::SnapBottomRight => "snap_bottom_right",
            Self::SnapFirstThird => "snap_first_third",
            Self::SnapCenterThird => "snap_center_third",
            Self::SnapLastThird => "snap_last_third",
            Self::SnapFirstTwoThirds => "snap_first_two_thirds",
            Self::SnapLastTwoThirds => "snap_last_two_thirds",
            Self::Maximize => "maximize",
            Self::AlmostMaximize => "almost_maximize",
            Self::Restore => "restore",
            Self::ReasonableSize => "reasonable_size",
            Self::ToggleFullscreen => "toggle_fullscreen",
            Self::NextDisplay => "next_display",
            Self::PreviousDisplay => "previous_display",
            Self::FocusColumnLeft => "focus_column_left",
            Self::FocusColumnRight => "focus_column_right",
            Self::MoveColumnLeft => "move_column_left",
            Self::MoveColumnRight => "move_column_right",
            Self::CycleColumnWidth => "cycle_column_width",
            Self::MaximizeColumn => "maximize_column",
            Self::ConsumeWindowIntoColumn => "consume_window_into_column",
            Self::ExpelWindowFromColumn => "expel_window_from_column",
            Self::ScrollViewportLeft => "scroll_viewport_left",
            Self::ScrollViewportRight => "scroll_viewport_right",
            Self::CycleRoomLayout => "cycle_room_layout",
            Self::EnterRoomFocus => "enter_room_focus",
            Self::EnterRoomStack => "enter_room_stack",
            Self::EnterRoomColumns => "enter_room_columns",
            Self::EnterRoomGrid => "enter_room_grid",
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        match label
            .trim()
            .to_lowercase()
            .replace(['-', ' '], "_")
            .as_str()
        {
            "split_row" | "split_horizontal" => Some(Self::SplitRow),
            "split_column" | "split_vertical" => Some(Self::SplitColumn),
            "flip_axis" | "flip" => Some(Self::FlipAxis),
            "reset" | "reset_layout" => Some(Self::Reset),
            "nudge_left" => Some(Self::NudgeLeft),
            "nudge_right" => Some(Self::NudgeRight),
            "nudge_up" => Some(Self::NudgeUp),
            "nudge_down" => Some(Self::NudgeDown),
            "expand_window" | "expand" => Some(Self::ExpandWindow),
            "contract_window" | "contract" => Some(Self::ContractWindow),
            "focus_next" => Some(Self::FocusNext),
            "focus_previous" | "focus_prev" => Some(Self::FocusPrevious),
            "close_focused" | "close" => Some(Self::CloseFocused),
            "toggle_float" | "float" => Some(Self::ToggleFloat),
            "grow_focused" | "grow" => Some(Self::GrowFocused),
            "shrink_focused" | "shrink" => Some(Self::ShrinkFocused),
            "move_left" => Some(Self::MoveLeft),
            "move_right" => Some(Self::MoveRight),
            "move_up" => Some(Self::MoveUp),
            "move_down" => Some(Self::MoveDown),
            "balance_panes" | "balance" => Some(Self::BalancePanes),
            "center_focused" | "center" => Some(Self::CenterFocused),
            "focus_first" => Some(Self::FocusFirst),
            "focus_last" => Some(Self::FocusLast),
            "snap_left_half" | "left_half" => Some(Self::SnapLeftHalf),
            "snap_right_half" | "right_half" => Some(Self::SnapRightHalf),
            "snap_top_half" | "top_half" => Some(Self::SnapTopHalf),
            "snap_bottom_half" | "bottom_half" => Some(Self::SnapBottomHalf),
            "snap_top_left" | "top_left" => Some(Self::SnapTopLeft),
            "snap_top_right" | "top_right" => Some(Self::SnapTopRight),
            "snap_bottom_left" | "bottom_left" => Some(Self::SnapBottomLeft),
            "snap_bottom_right" | "bottom_right" => Some(Self::SnapBottomRight),
            "snap_first_third" | "first_third" => Some(Self::SnapFirstThird),
            "snap_center_third" | "center_third" => Some(Self::SnapCenterThird),
            "snap_last_third" | "last_third" => Some(Self::SnapLastThird),
            "snap_first_two_thirds" | "first_two_thirds" => Some(Self::SnapFirstTwoThirds),
            "snap_last_two_thirds" | "last_two_thirds" => Some(Self::SnapLastTwoThirds),
            "maximize" => Some(Self::Maximize),
            "almost_maximize" => Some(Self::AlmostMaximize),
            "restore" => Some(Self::Restore),
            "reasonable_size" | "reasonable" => Some(Self::ReasonableSize),
            "toggle_fullscreen" | "fullscreen" => Some(Self::ToggleFullscreen),
            "next_display" => Some(Self::NextDisplay),
            "previous_display" | "prev_display" => Some(Self::PreviousDisplay),
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

    #[cfg(test)]
    pub fn all() -> &'static [Self] {
        &[
            Self::SplitRow,
            Self::SplitColumn,
            Self::FlipAxis,
            Self::Reset,
            Self::NudgeLeft,
            Self::NudgeRight,
            Self::NudgeUp,
            Self::NudgeDown,
            Self::ExpandWindow,
            Self::ContractWindow,
            Self::FocusNext,
            Self::FocusPrevious,
            Self::CloseFocused,
            Self::ToggleFloat,
            Self::GrowFocused,
            Self::ShrinkFocused,
            Self::MoveLeft,
            Self::MoveRight,
            Self::MoveUp,
            Self::MoveDown,
            Self::BalancePanes,
            Self::CenterFocused,
            Self::FocusFirst,
            Self::FocusLast,
            Self::SnapLeftHalf,
            Self::SnapRightHalf,
            Self::SnapTopHalf,
            Self::SnapBottomHalf,
            Self::SnapTopLeft,
            Self::SnapTopRight,
            Self::SnapBottomLeft,
            Self::SnapBottomRight,
            Self::SnapFirstThird,
            Self::SnapCenterThird,
            Self::SnapLastThird,
            Self::SnapFirstTwoThirds,
            Self::SnapLastTwoThirds,
            Self::Maximize,
            Self::AlmostMaximize,
            Self::Restore,
            Self::ReasonableSize,
            Self::ToggleFullscreen,
            Self::NextDisplay,
            Self::PreviousDisplay,
            Self::FocusColumnLeft,
            Self::FocusColumnRight,
            Self::MoveColumnLeft,
            Self::MoveColumnRight,
            Self::CycleColumnWidth,
            Self::MaximizeColumn,
            Self::ConsumeWindowIntoColumn,
            Self::ExpelWindowFromColumn,
            Self::ScrollViewportLeft,
            Self::ScrollViewportRight,
            Self::CycleRoomLayout,
            Self::EnterRoomFocus,
            Self::EnterRoomStack,
            Self::EnterRoomColumns,
            Self::EnterRoomGrid,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnWidth {
    OneThird,
    OneHalf,
    TwoThirds,
    Full,
}

impl ColumnWidth {
    pub fn fraction(self) -> f32 {
        match self {
            Self::OneThird => 1.0 / 3.0,
            Self::OneHalf => 0.5,
            Self::TwoThirds => 2.0 / 3.0,
            Self::Full => 1.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::OneThird => "1/3",
            Self::OneHalf => "1/2",
            Self::TwoThirds => "2/3",
            Self::Full => "Full",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomLayoutKind {
    Auto,
    Focus,
    Stack,
    Columns,
    Grid,
}

impl RoomLayoutKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Focus => "Focus",
            Self::Stack => "Stack",
            Self::Columns => "Columns",
            Self::Grid => "Grid",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Auto => Self::Focus,
            Self::Focus => Self::Stack,
            Self::Stack => Self::Columns,
            Self::Columns => Self::Grid,
            Self::Grid => Self::Auto,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowCycleRecord {
    pub window_id: usize,
    pub last_action: LayoutAction,
    pub step: usize,
    pub restore_floating: bool,
    pub restore_rect: (f32, f32, f32, f32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NiriColumn {
    pub width: ColumnWidth,
    pub window_ids: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowGeometry {
    pub id: usize,
    pub title: String,
    pub focused: bool,
    pub floating: bool,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct LayoutState {
    root: Node,
    focused: usize,
    next_id: usize,
    workarea_width: f32,
    workarea_height: f32,
    columns: Vec<NiriColumn>,
    viewport_x: f32,
    room_layout: RoomLayoutKind,
    cycle_memory: Option<WindowCycleRecord>,
}

#[derive(Debug, Clone)]
pub enum LayoutView {
    Window(LayoutWindowView),
    Container(LayoutContainerView),
}

#[derive(Debug, Clone)]
pub struct LayoutWindowView {
    pub id: usize,
    pub title: String,
    pub detail: String,
    pub floating: bool,
    pub focused: bool,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct LayoutContainerView {
    pub axis: Axis,
    pub children: Vec<LayoutChildView>,
}

#[derive(Debug, Clone)]
pub struct LayoutChildView {
    pub grow: f32,
    pub node: LayoutView,
}

#[derive(Debug, Clone)]
enum Node {
    Window(WindowNode),
    Container(ContainerNode),
}

#[derive(Debug, Clone)]
struct WindowNode {
    id: usize,
    title: String,
    detail: String,
    floating: bool,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Debug, Clone)]
struct ContainerNode {
    axis: Axis,
    children: Vec<ChildNode>,
}

#[derive(Debug, Clone)]
struct ChildNode {
    grow: f32,
    node: Node,
}

impl LayoutState {
    pub fn new() -> Self {
        Self::seed()
    }

    pub fn demo() -> Self {
        Self {
            root: Node::Container(ContainerNode {
                axis: Axis::Row,
                children: vec![
                    ChildNode {
                        grow: 1.2,
                        node: Node::Container(ContainerNode {
                            axis: Axis::Column,
                            children: vec![
                                ChildNode {
                                    grow: 1.0,
                                    node: Node::Window(WindowNode {
                                        id: 1,
                                        title: "Workspace".to_string(),
                                        detail: "Ready".to_string(),
                                        floating: false,
                                        x: 72.,
                                        y: 72.,
                                        width: 420.,
                                        height: 280.,
                                    }),
                                },
                                ChildNode {
                                    grow: 0.8,
                                    node: Node::Window(WindowNode {
                                        id: 3,
                                        title: "Shell".to_string(),
                                        detail: "Ready".to_string(),
                                        floating: false,
                                        x: 112.,
                                        y: 112.,
                                        width: 420.,
                                        height: 260.,
                                    }),
                                },
                            ],
                        }),
                    },
                    ChildNode {
                        grow: 0.9,
                        node: Node::Window(WindowNode {
                            id: 2,
                            title: "Scratch".to_string(),
                            detail: "Ready".to_string(),
                            floating: false,
                            x: 100.,
                            y: 100.,
                            width: 420.,
                            height: 280.,
                        }),
                    },
                    ChildNode {
                        grow: 0.7,
                        node: Node::Window(WindowNode {
                            id: 4,
                            title: "Inspector".to_string(),
                            detail: "Ready".to_string(),
                            floating: true,
                            x: 920.,
                            y: 120.,
                            width: 380.,
                            height: 320.,
                        }),
                    },
                ],
            }),
            focused: 1,
            next_id: 5,
            workarea_width: 1440.0,
            workarea_height: 900.0,
            columns: vec![
                NiriColumn {
                    width: ColumnWidth::OneHalf,
                    window_ids: vec![1, 3],
                },
                NiriColumn {
                    width: ColumnWidth::OneHalf,
                    window_ids: vec![2],
                },
            ],
            viewport_x: 0.0,
            room_layout: RoomLayoutKind::Auto,
            cycle_memory: None,
        }
    }

    fn seed() -> Self {
        Self {
            root: Node::Container(ContainerNode {
                axis: Axis::Row,
                children: vec![
                    ChildNode {
                        grow: 1.4,
                        node: Node::Window(WindowNode {
                            id: 1,
                            title: "Workspace".to_string(),
                            detail: "Ready".to_string(),
                            floating: false,
                            x: 72.,
                            y: 72.,
                            width: 420.,
                            height: 280.,
                        }),
                    },
                    ChildNode {
                        grow: 0.9,
                        node: Node::Window(WindowNode {
                            id: 2,
                            title: "Scratch".to_string(),
                            detail: "Ready".to_string(),
                            floating: false,
                            x: 100.,
                            y: 100.,
                            width: 420.,
                            height: 280.,
                        }),
                    },
                ],
            }),
            focused: 1,
            next_id: 3,
            workarea_width: 1440.0,
            workarea_height: 900.0,
            columns: vec![
                NiriColumn {
                    width: ColumnWidth::OneHalf,
                    window_ids: vec![1],
                },
                NiriColumn {
                    width: ColumnWidth::OneHalf,
                    window_ids: vec![2],
                },
            ],
            viewport_x: 0.0,
            room_layout: RoomLayoutKind::Auto,
            cycle_memory: None,
        }
    }

    pub fn set_workarea(&mut self, width: f32, height: f32) {
        self.workarea_width = width.max(320.0);
        self.workarea_height = height.max(240.0);
        self.ensure_focused_column_visible(8.0);
    }

    pub fn workarea(&self) -> (f32, f32) {
        (self.workarea_width, self.workarea_height)
    }

    pub fn niri_columns(&self) -> &[NiriColumn] {
        &self.columns
    }

    pub fn viewport_x(&self) -> f32 {
        self.viewport_x
    }

    pub fn room_layout(&self) -> RoomLayoutKind {
        self.room_layout
    }

    pub fn focused_column_width(&self) -> ColumnWidth {
        self.columns
            .iter()
            .find(|col| col.window_ids.contains(&self.focused))
            .map(|col| col.width)
            .unwrap_or(ColumnWidth::OneHalf)
    }

    pub fn niri_summary(&self) -> String {
        let col_count = self.columns.len();
        let active_col = self
            .columns
            .iter()
            .position(|c| c.window_ids.contains(&self.focused))
            .map(|i| i + 1)
            .unwrap_or(0);
        let width_label = self.focused_column_width().label();
        let room_label = self.room_layout().label();
        format!(
            "{col_count} cols • col {active_col}/{col_count} ({width_label}) • room {room_label} • pan {:.0}px",
            self.viewport_x()
        )
    }

    pub fn apply(&mut self, action: &LayoutAction) {
        match action {
            LayoutAction::SplitRow => self.split(Axis::Row),
            LayoutAction::SplitColumn => self.split(Axis::Column),
            LayoutAction::FlipAxis => flip_axes(&mut self.root),
            LayoutAction::Reset => *self = Self::seed(),
            LayoutAction::NudgeLeft => self.nudge_focused(-24., 0.),
            LayoutAction::NudgeRight => self.nudge_focused(24., 0.),
            LayoutAction::NudgeUp => self.nudge_focused(0., -24.),
            LayoutAction::NudgeDown => self.nudge_focused(0., 24.),
            LayoutAction::ExpandWindow => self.resize_floating_focused(40., 28.),
            LayoutAction::ContractWindow => self.resize_floating_focused(-40., -28.),
            LayoutAction::FocusNext => self.focus_next(),
            LayoutAction::FocusPrevious => self.focus_previous(),
            LayoutAction::CloseFocused => self.close_focused(),
            LayoutAction::ToggleFloat => self.toggle_float(),
            LayoutAction::GrowFocused => self.resize_focused(0.2),
            LayoutAction::ShrinkFocused => self.resize_focused(-0.2),
            LayoutAction::MoveLeft => self.move_focused(-1, 0),
            LayoutAction::MoveRight => self.move_focused(1, 0),
            LayoutAction::MoveUp => self.move_focused(0, -1),
            LayoutAction::MoveDown => self.move_focused(0, 1),
            LayoutAction::BalancePanes => balance_panes(&mut self.root),
            LayoutAction::CenterFocused => self.center_focused(),
            LayoutAction::FocusFirst => self.focus_first(),
            LayoutAction::FocusLast => self.focus_last(),
            LayoutAction::SnapLeftHalf => self.snap_half_with_cycle(action.clone(), "Left"),
            LayoutAction::SnapRightHalf => self.snap_half_with_cycle(action.clone(), "Right"),
            LayoutAction::SnapTopHalf => self.snap_half_with_cycle(action.clone(), "Top"),
            LayoutAction::SnapBottomHalf => self.snap_half_with_cycle(action.clone(), "Bottom"),
            LayoutAction::SnapTopLeft => self.snap_focused_fraction(0.0, 0.0, 0.5, 0.5, "Top Left"),
            LayoutAction::SnapTopRight => {
                self.snap_focused_fraction(0.5, 0.0, 0.5, 0.5, "Top Right")
            }
            LayoutAction::SnapBottomLeft => {
                self.snap_focused_fraction(0.0, 0.5, 0.5, 0.5, "Bottom Left")
            }
            LayoutAction::SnapBottomRight => {
                self.snap_focused_fraction(0.5, 0.5, 0.5, 0.5, "Bottom Right")
            }
            LayoutAction::SnapFirstThird => {
                self.snap_focused_fraction(0.0, 0.0, 1.0 / 3.0, 1.0, "First Third")
            }
            LayoutAction::SnapCenterThird => {
                self.snap_focused_fraction(1.0 / 3.0, 0.0, 1.0 / 3.0, 1.0, "Center Third")
            }
            LayoutAction::SnapLastThird => {
                self.snap_focused_fraction(2.0 / 3.0, 0.0, 1.0 / 3.0, 1.0, "Last Third")
            }
            LayoutAction::SnapFirstTwoThirds => {
                self.snap_focused_fraction(0.0, 0.0, 2.0 / 3.0, 1.0, "First Two Thirds")
            }
            LayoutAction::SnapLastTwoThirds => {
                self.snap_focused_fraction(1.0 / 3.0, 0.0, 2.0 / 3.0, 1.0, "Last Two Thirds")
            }
            LayoutAction::Maximize => self.snap_focused_fraction(0.0, 0.0, 1.0, 1.0, "Maximized"),
            LayoutAction::AlmostMaximize => {
                self.snap_focused_fraction(0.05, 0.05, 0.90, 0.90, "Almost Maximized")
            }
            LayoutAction::Restore => self.restore_focused(),
            LayoutAction::ReasonableSize => self.reasonable_size_focused(),
            LayoutAction::ToggleFullscreen => self.toggle_fullscreen_focused(),
            LayoutAction::NextDisplay => self.move_display_focused("Next Display"),
            LayoutAction::PreviousDisplay => self.move_display_focused("Previous Display"),
            LayoutAction::FocusColumnLeft => self.focus_column_left(),
            LayoutAction::FocusColumnRight => self.focus_column_right(),
            LayoutAction::MoveColumnLeft => self.move_column_left(),
            LayoutAction::MoveColumnRight => self.move_column_right(),
            LayoutAction::CycleColumnWidth => self.cycle_column_width(),
            LayoutAction::MaximizeColumn => self.maximize_column(),
            LayoutAction::ConsumeWindowIntoColumn => self.consume_window_into_column(),
            LayoutAction::ExpelWindowFromColumn => self.expel_window_from_column(),
            LayoutAction::ScrollViewportLeft => self.scroll_viewport(-1.0),
            LayoutAction::ScrollViewportRight => self.scroll_viewport(1.0),
            LayoutAction::CycleRoomLayout => {
                let next_kind = self.room_layout.next();
                self.apply_room_layout(next_kind);
            }
            LayoutAction::EnterRoomFocus => self.apply_room_layout(RoomLayoutKind::Focus),
            LayoutAction::EnterRoomStack => self.apply_room_layout(RoomLayoutKind::Stack),
            LayoutAction::EnterRoomColumns => self.apply_room_layout(RoomLayoutKind::Columns),
            LayoutAction::EnterRoomGrid => self.apply_room_layout(RoomLayoutKind::Grid),
        }
        self.sync_niri_columns();
        self.ensure_focused_column_visible(8.0);
    }

    pub fn focused_id(&self) -> usize {
        self.focused
    }

    pub fn focused_title(&self) -> &str {
        self.find(self.focused)
            .map(|window| window.title.as_str())
            .unwrap_or("none")
    }

    pub fn summary(&self) -> String {
        let mut windows = Vec::new();
        self.collect(&self.root, &mut windows);
        let floating = windows.iter().filter(|window| window.floating).count();
        let tiled = windows.len().saturating_sub(floating);
        format!("{tiled} tiled {floating} floating")
    }

    pub fn axis(&self) -> &'static str {
        match &self.root {
            Node::Window(_) => "single",
            Node::Container(container) => container.axis.label(),
        }
    }

    pub fn view(&self) -> LayoutView {
        self.view_node(&self.root)
    }

    pub fn set_window_mode(&mut self, mode: &WindowMode) {
        let floating = matches!(
            mode,
            WindowMode::Floating | WindowMode::Stack | WindowMode::Center
        );
        set_floating(&mut self.root, floating);
        if matches!(mode, WindowMode::Stack) {
            stack_offset(&mut self.root);
        }
        if matches!(mode, WindowMode::Center) {
            center_all(&mut self.root);
        }
        if matches!(mode, WindowMode::Grid) {
            balance_panes(&mut self.root);
        }
        self.sync_niri_columns();
        self.ensure_focused_column_visible(8.0);
    }

    pub fn split_next(&mut self, title: String) -> usize {
        if self.find(self.focused).is_none() {
            self.focus_first();
        }
        let new_id = self.next_id;
        let focused = self.focused;
        if split_window(&mut self.root, focused, Axis::Row, new_id, title) {
            self.next_id += 1;
            self.focused = new_id;
            self.sync_niri_columns();
            self.ensure_focused_column_visible(8.0);
        }
        self.focused
    }

    pub fn remove_window_by_id(&mut self, id: usize) -> bool {
        let mut windows = Vec::new();
        self.collect(&self.root, &mut windows);
        if windows.len() < 2 || !windows.iter().any(|w| w.id == id) {
            return false;
        }
        let fallback = windows
            .iter()
            .find(|window| window.id != id)
            .map(|window| window.id)
            .unwrap_or(self.focused);
        let _ = remove_window(&mut self.root, id);
        collapse(&mut self.root);
        if self.focused == id || self.find(self.focused).is_none() {
            self.focused = fallback;
        }
        self.sync_niri_columns();
        self.ensure_focused_column_visible(8.0);
        true
    }

    pub fn compute_geometries(&self, mode: &WindowMode, gap: f32) -> Vec<WindowGeometry> {
        let w = self.workarea_width.max(320.0);
        let h = self.workarea_height.max(240.0);
        let pad = gap.max(0.0);
        let inner_x = pad;
        let inner_y = pad;
        let inner_w = (w - pad * 2.0).max(100.0);
        let inner_h = (h - pad * 2.0).max(100.0);

        match mode {
            WindowMode::Monocle => {
                let mut out = Vec::new();
                for win in self.windows() {
                    out.push(WindowGeometry {
                        id: win.id,
                        title: win.title,
                        focused: win.focused,
                        floating: false,
                        x: inner_x,
                        y: inner_y,
                        width: inner_w,
                        height: inner_h,
                    });
                }
                out
            }
            WindowMode::Floating | WindowMode::Stack | WindowMode::Center => self
                .windows()
                .into_iter()
                .map(|win| WindowGeometry {
                    id: win.id,
                    title: win.title,
                    focused: win.focused,
                    floating: true,
                    x: win.x,
                    y: win.y,
                    width: win.width,
                    height: win.height,
                })
                .collect(),
            WindowMode::NiriStrip => {
                let mut out = Vec::new();
                let mut cursor_x = inner_x;
                for col in &self.columns {
                    if col.window_ids.is_empty() {
                        continue;
                    }
                    let col_w = (inner_w * col.width.fraction()).round().max(100.0);
                    let screen_x = (cursor_x - self.viewport_x).round();
                    let k = col.window_ids.len();
                    let total_v_gap = pad * (k.saturating_sub(1) as f32);
                    let cell_h = ((inner_h - total_v_gap) / (k as f32)).round().max(60.0);
                    for (row_idx, &win_id) in col.window_ids.iter().enumerate() {
                        if let Some(win) = self.find(win_id) {
                            let win_y = (inner_y + (row_idx as f32) * (cell_h + pad)).round();
                            let win_h = if row_idx + 1 == k {
                                (inner_y + inner_h - win_y).round().max(60.0)
                            } else {
                                cell_h
                            };
                            out.push(WindowGeometry {
                                id: win.id,
                                title: win.title.clone(),
                                focused: win.id == self.focused,
                                floating: false,
                                x: screen_x,
                                y: win_y,
                                width: col_w,
                                height: win_h,
                            });
                        }
                    }
                    cursor_x += col_w + pad;
                }
                for win in self.view().floating_windows() {
                    out.push(WindowGeometry {
                        id: win.id,
                        title: win.title,
                        focused: win.focused,
                        floating: true,
                        x: win.x,
                        y: win.y,
                        width: win.width,
                        height: win.height,
                    });
                }
                out
            }
            WindowMode::Tiling | WindowMode::Grid => {
                let mut out = Vec::new();
                let view = self.view();
                if let Some(tiled) = view.tiled() {
                    compute_tiled_geometries(
                        &tiled, inner_x, inner_y, inner_w, inner_h, pad, &mut out,
                    );
                }
                for win in view.floating_windows() {
                    out.push(WindowGeometry {
                        id: win.id,
                        title: win.title,
                        focused: win.focused,
                        floating: true,
                        x: win.x,
                        y: win.y,
                        width: win.width,
                        height: win.height,
                    });
                }
                out
            }
        }
    }

    pub fn set_window_floating(&mut self, id: usize, floating: bool) {
        if let Some(window) = find_mut(&mut self.root, id) {
            window.floating = floating;
        }
        self.sync_niri_columns();
        self.ensure_focused_column_visible(8.0);
    }

    pub fn set_focused_window_content(
        &mut self,
        title: impl Into<String>,
        detail: impl Into<String>,
    ) {
        if let Some(window) = find_mut(&mut self.root, self.focused) {
            window.title = title.into();
            window.detail = detail.into();
        }
    }

    pub fn focus_window(&mut self, id: usize) {
        if self.find(id).is_some() {
            self.focused = id;
            self.ensure_focused_column_visible(8.0);
        }
    }

    pub fn windows(&self) -> Vec<LayoutWindowView> {
        let mut windows = Vec::new();
        self.collect_views(&self.root, &mut windows);
        windows
    }

    pub fn focused_window(&self) -> Option<LayoutWindowView> {
        self.windows().into_iter().find(|window| window.focused)
    }

    fn sync_niri_columns(&mut self) {
        let mut windows = Vec::new();
        self.collect(&self.root, &mut windows);
        let tiled_ids: Vec<usize> = windows
            .into_iter()
            .filter(|w| !w.floating)
            .map(|w| w.id)
            .collect();

        for col in &mut self.columns {
            col.window_ids.retain(|id| tiled_ids.contains(id));
        }
        self.columns.retain(|col| !col.window_ids.is_empty());

        for id in tiled_ids {
            if !self.columns.iter().any(|c| c.window_ids.contains(&id)) {
                self.columns.push(NiriColumn {
                    width: ColumnWidth::OneHalf,
                    window_ids: vec![id],
                });
            }
        }
    }

    fn ensure_focused_column_visible(&mut self, gap: f32) {
        let w = self.workarea_width.max(320.0);
        let pad = gap.max(0.0);
        let inner_w = (w - pad * 2.0).max(100.0);

        let mut cursor_x = 0.0f32;
        let mut focused_span = None;
        for (idx, col) in self.columns.iter().enumerate() {
            let col_w = (inner_w * col.width.fraction()).round().max(100.0);
            if col.window_ids.contains(&self.focused) {
                focused_span = Some((cursor_x, col_w));
            }
            cursor_x += col_w;
            if idx + 1 < self.columns.len() {
                cursor_x += pad;
            }
        }
        let strip_width = cursor_x;
        if let Some((col_x, col_w)) = focused_span {
            if col_x < self.viewport_x {
                self.viewport_x = col_x;
            } else if col_x + col_w > self.viewport_x + inner_w {
                self.viewport_x = (col_x + col_w - inner_w).max(0.0);
            }
        }
        let max_scroll = (strip_width - inner_w).max(0.0);
        self.viewport_x = self.viewport_x.clamp(0.0, max_scroll);
    }

    fn focus_column_left(&mut self) {
        self.sync_niri_columns();
        if let Some(col_idx) = self
            .columns
            .iter()
            .position(|c| c.window_ids.contains(&self.focused))
        {
            if col_idx > 0 {
                if let Some(&first_id) = self.columns[col_idx - 1].window_ids.first() {
                    self.focused = first_id;
                }
            }
        }
    }

    fn focus_column_right(&mut self) {
        self.sync_niri_columns();
        if let Some(col_idx) = self
            .columns
            .iter()
            .position(|c| c.window_ids.contains(&self.focused))
        {
            if col_idx + 1 < self.columns.len() {
                if let Some(&first_id) = self.columns[col_idx + 1].window_ids.first() {
                    self.focused = first_id;
                }
            }
        }
    }

    fn move_column_left(&mut self) {
        self.sync_niri_columns();
        if let Some(col_idx) = self
            .columns
            .iter()
            .position(|c| c.window_ids.contains(&self.focused))
        {
            if col_idx > 0 {
                self.columns.swap(col_idx, col_idx - 1);
            }
        }
    }

    fn move_column_right(&mut self) {
        self.sync_niri_columns();
        if let Some(col_idx) = self
            .columns
            .iter()
            .position(|c| c.window_ids.contains(&self.focused))
        {
            if col_idx + 1 < self.columns.len() {
                self.columns.swap(col_idx, col_idx + 1);
            }
        }
    }

    fn cycle_column_width(&mut self) {
        self.sync_niri_columns();
        if let Some(col) = self
            .columns
            .iter_mut()
            .find(|c| c.window_ids.contains(&self.focused))
        {
            col.width = col.width.next();
            let width_label = col.width.label();
            if let Some(win) = find_mut(&mut self.root, self.focused) {
                win.detail = format!("Column width: {width_label}");
            }
        }
    }

    fn maximize_column(&mut self) {
        self.sync_niri_columns();
        if let Some(col) = self
            .columns
            .iter_mut()
            .find(|c| c.window_ids.contains(&self.focused))
        {
            col.width = if col.width == ColumnWidth::Full {
                ColumnWidth::OneHalf
            } else {
                ColumnWidth::Full
            };
            let width_label = col.width.label();
            if let Some(win) = find_mut(&mut self.root, self.focused) {
                win.detail = format!("Column width: {width_label}");
            }
        }
    }

    fn consume_window_into_column(&mut self) {
        self.sync_niri_columns();
        let Some(col_idx) = self
            .columns
            .iter()
            .position(|c| c.window_ids.contains(&self.focused))
        else {
            return;
        };
        let donor_idx = if col_idx + 1 < self.columns.len() {
            Some(col_idx + 1)
        } else if col_idx > 0 {
            Some(col_idx - 1)
        } else {
            None
        };
        if let Some(donor) = donor_idx {
            if !self.columns[donor].window_ids.is_empty() {
                let win_id = self.columns[donor].window_ids.remove(0);
                self.columns[col_idx].window_ids.push(win_id);
                if self.columns[donor].window_ids.is_empty() {
                    self.columns.remove(donor);
                }
            }
        }
    }

    fn expel_window_from_column(&mut self) {
        self.sync_niri_columns();
        let Some(col_idx) = self
            .columns
            .iter()
            .position(|c| c.window_ids.contains(&self.focused))
        else {
            return;
        };
        if self.columns[col_idx].window_ids.len() < 2 {
            return;
        }
        let col_width = self.columns[col_idx].width;
        self.columns[col_idx]
            .window_ids
            .retain(|&id| id != self.focused);
        self.columns.insert(
            col_idx + 1,
            NiriColumn {
                width: col_width,
                window_ids: vec![self.focused],
            },
        );
    }

    fn scroll_viewport(&mut self, dir: f32) {
        let step = (self.workarea_width.max(320.0) * 0.35).round();
        let w = self.workarea_width.max(320.0);
        let pad = 8.0f32;
        let inner_w = (w - pad * 2.0).max(100.0);
        let mut strip_w = 0.0f32;
        for (idx, col) in self.columns.iter().enumerate() {
            strip_w += (inner_w * col.width.fraction()).round().max(100.0);
            if idx + 1 < self.columns.len() {
                strip_w += pad;
            }
        }
        let max_scroll = (strip_w - inner_w).max(0.0);
        self.viewport_x = (self.viewport_x + dir * step).clamp(0.0, max_scroll);
    }

    fn split(&mut self, axis: Axis) {
        if self.find(self.focused).is_none() {
            self.focus_first();
        }
        let new_id = self.next_id;
        let title = format!("Window {new_id}");
        let focused = self.focused;
        if split_window(&mut self.root, focused, axis, new_id, title) {
            self.next_id += 1;
            self.focused = new_id;
        }
    }

    fn focus_next(&mut self) {
        let mut windows = Vec::new();
        self.collect(&self.root, &mut windows);
        if windows.len() < 2 {
            return;
        }
        let index = windows
            .iter()
            .position(|window| window.id == self.focused)
            .unwrap_or(0);
        self.focused = windows[(index + 1) % windows.len()].id;
    }

    fn focus_previous(&mut self) {
        let mut windows = Vec::new();
        self.collect(&self.root, &mut windows);
        if windows.len() < 2 {
            return;
        }
        let index = windows
            .iter()
            .position(|window| window.id == self.focused)
            .unwrap_or(0);
        let prev = if index == 0 {
            windows.len() - 1
        } else {
            index - 1
        };
        self.focused = windows[prev].id;
    }

    fn focus_first(&mut self) {
        let mut windows = Vec::new();
        self.collect(&self.root, &mut windows);
        if let Some(first) = windows.first() {
            self.focused = first.id;
        }
    }

    fn focus_last(&mut self) {
        let mut windows = Vec::new();
        self.collect(&self.root, &mut windows);
        if let Some(last) = windows.last() {
            self.focused = last.id;
        }
    }

    fn center_focused(&mut self) {
        if let Some(window) = find_mut(&mut self.root, self.focused) {
            window.floating = true;
            window.x = 480.;
            window.y = 240.;
        }
    }

    fn capture_restore_point(&mut self, action: LayoutAction) -> usize {
        let focused = self.focused;
        if let Some(mem) = &mut self.cycle_memory {
            if mem.window_id == focused {
                if mem.last_action == action {
                    mem.step = (mem.step + 1) % 3;
                } else {
                    mem.last_action = action;
                    mem.step = 0;
                }
                return mem.step;
            }
        }
        if let Some(window) = find_mut(&mut self.root, focused) {
            self.cycle_memory = Some(WindowCycleRecord {
                window_id: focused,
                last_action: action,
                step: 0,
                restore_floating: window.floating,
                restore_rect: (window.x, window.y, window.width, window.height),
            });
        }
        0
    }

    fn snap_half_with_cycle(&mut self, action: LayoutAction, edge: &str) {
        let step = self.capture_restore_point(action);
        let frac = match step {
            0 => 0.5,
            1 => 1.0 / 3.0,
            _ => 2.0 / 3.0,
        };
        let frac_label = match step {
            0 => "1/2",
            1 => "1/3",
            _ => "2/3",
        };
        let (fx, fy, fw, fh) = match edge {
            "Left" => (0.0, 0.0, frac, 1.0),
            "Right" => (1.0 - frac, 0.0, frac, 1.0),
            "Top" => (0.0, 0.0, 1.0, frac),
            _ => (0.0, 1.0 - frac, 1.0, frac),
        };
        let label = format!("{edge} {frac_label}");
        self.snap_focused_raw(fx, fy, fw, fh, &label);
    }

    fn snap_focused_fraction(&mut self, fx: f32, fy: f32, fw: f32, fh: f32, label: &str) {
        let _ = self.capture_restore_point(LayoutAction::Maximize);
        self.snap_focused_raw(fx, fy, fw, fh, label);
    }

    fn snap_focused_raw(&mut self, fx: f32, fy: f32, fw: f32, fh: f32, label: &str) {
        let w = self.workarea_width.max(320.0);
        let h = self.workarea_height.max(240.0);
        if let Some(window) = find_mut(&mut self.root, self.focused) {
            window.floating = true;
            window.x = (w * fx).round();
            window.y = (h * fy).round();
            window.width = (w * fw).round().max(180.0);
            window.height = (h * fh).round().max(140.0);
            window.detail = format!("Snapped: {label}");
        }
    }

    fn restore_focused(&mut self) {
        let focused = self.focused;
        let saved = self
            .cycle_memory
            .take()
            .filter(|mem| mem.window_id == focused);
        if let Some(window) = find_mut(&mut self.root, focused) {
            if let Some(rec) = saved {
                window.floating = rec.restore_floating;
                window.x = rec.restore_rect.0;
                window.y = rec.restore_rect.1;
                window.width = rec.restore_rect.2;
                window.height = rec.restore_rect.3;
            } else {
                window.floating = false;
                window.x = 72.;
                window.y = 72.;
                window.width = 420.;
                window.height = 280.;
            }
            window.detail = "Restored".to_string();
        }
    }

    fn apply_room_layout(&mut self, kind: RoomLayoutKind) {
        self.room_layout = kind;
        let w = self.workarea_width.max(320.0);
        let h = self.workarea_height.max(240.0);
        let mut ids = Vec::new();
        let mut refs = Vec::new();
        self.collect(&self.root, &mut refs);
        for win in refs {
            ids.push(win.id);
        }
        let n = ids.len();
        if n == 0 {
            return;
        }
        let effective = match kind {
            RoomLayoutKind::Auto => {
                if n <= 4 {
                    RoomLayoutKind::Focus
                } else {
                    RoomLayoutKind::Grid
                }
            }
            other => other,
        };
        for (idx, win_id) in ids.iter().copied().enumerate() {
            let (rx, ry, rw, rh) = match effective {
                RoomLayoutKind::Focus | RoomLayoutKind::Auto => {
                    if n == 1 {
                        (0.0, 0.0, w, h)
                    } else if idx == 0 {
                        let main_w = (w * 0.60).round();
                        (0.0, 0.0, main_w, h)
                    } else {
                        let main_w = (w * 0.60).round();
                        let side_w = (w - main_w).max(200.0);
                        let side_count = (n - 1).max(1) as f32;
                        let slot_h = (h / side_count).round();
                        let side_idx = (idx - 1) as f32;
                        (main_w, (side_idx * slot_h).round(), side_w, slot_h)
                    }
                }
                RoomLayoutKind::Stack => {
                    if n == 1 {
                        (0.0, 0.0, w, h)
                    } else if idx == 0 {
                        let main_w = (w * 0.60).round();
                        (0.0, 0.0, main_w, h)
                    } else {
                        let main_w = (w * 0.60).round();
                        let side_w = (w - main_w).max(200.0);
                        let peek = ((idx - 1) as f32) * 32.0;
                        let sh = (h - peek).max(180.0);
                        (main_w, peek, side_w, sh)
                    }
                }
                RoomLayoutKind::Columns => {
                    let cols = n.clamp(1, 4) as f32;
                    let col_w = (w / cols).round();
                    let col_idx = (idx.min(3)) as f32;
                    (col_idx * col_w, 0.0, col_w, h)
                }
                RoomLayoutKind::Grid => {
                    let cols = ((n as f32).sqrt().ceil() as usize).max(1);
                    let rows = n.div_ceil(cols).max(1);
                    let col_w = (w / cols as f32).round();
                    let row_h = (h / rows as f32).round();
                    let c = (idx % cols) as f32;
                    let r = (idx / cols) as f32;
                    (c * col_w, r * row_h, col_w, row_h)
                }
            };
            if let Some(win) = find_mut(&mut self.root, win_id) {
                win.floating = true;
                win.x = rx;
                win.y = ry;
                win.width = rw;
                win.height = rh;
                win.detail = format!("Room: {}", kind.label());
            }
        }
    }

    fn reasonable_size_focused(&mut self) {
        let w = self.workarea_width.max(320.0);
        let h = self.workarea_height.max(240.0);
        let target_w = (w * 0.65).round().clamp(420.0, 1120.0);
        let target_h = (h * 0.70).round().clamp(280.0, 760.0);
        if let Some(window) = find_mut(&mut self.root, self.focused) {
            window.floating = true;
            window.width = target_w;
            window.height = target_h;
            window.x = ((w - target_w) * 0.5).round().max(16.0);
            window.y = ((h - target_h) * 0.5).round().max(16.0);
            window.detail = "Reasonable Size".to_string();
        }
    }

    fn toggle_fullscreen_focused(&mut self) {
        let w = self.workarea_width.max(320.0);
        let h = self.workarea_height.max(240.0);
        if let Some(window) = find_mut(&mut self.root, self.focused) {
            let is_full = window.floating
                && (window.width - w).abs() < 2.0
                && (window.height - h).abs() < 2.0;
            if is_full {
                window.floating = false;
                window.x = 72.;
                window.y = 72.;
                window.width = 420.;
                window.height = 280.;
                window.detail = "Exited Fullscreen".to_string();
            } else {
                window.floating = true;
                window.x = 0.;
                window.y = 0.;
                window.width = w;
                window.height = h;
                window.detail = "Fullscreen".to_string();
            }
        }
    }

    fn move_display_focused(&mut self, display_label: &str) {
        let w = self.workarea_width.max(320.0);
        let h = self.workarea_height.max(240.0);
        if let Some(window) = find_mut(&mut self.root, self.focused) {
            if window.floating {
                window.x = ((w - window.width) * 0.5).round().max(16.0);
                window.y = ((h - window.height) * 0.5).round().max(16.0);
            }
            window.detail = format!("Moved to {display_label}");
        }
    }

    fn move_focused(&mut self, dx: i32, dy: i32) {
        if let Some(window) = find_mut(&mut self.root, self.focused) {
            if window.floating {
                window.x = (window.x + dx as f32 * 48.).max(16.);
                window.y = (window.y + dy as f32 * 48.).max(16.);
            } else {
                swap_focused(&mut self.root, self.focused, dx, dy);
            }
        }
    }

    fn close_focused(&mut self) {
        let focused = self.focused;
        let _ = self.remove_window_by_id(focused);
    }

    fn toggle_float(&mut self) {
        if let Some(window) = find_mut(&mut self.root, self.focused) {
            window.floating = !window.floating;
        }
    }

    fn nudge_focused(&mut self, dx: f32, dy: f32) {
        if let Some(window) = find_mut(&mut self.root, self.focused) {
            if window.floating {
                window.x = (window.x + dx).max(16.);
                window.y = (window.y + dy).max(16.);
            }
        }
    }

    fn resize_floating_focused(&mut self, dw: f32, dh: f32) {
        if let Some(window) = find_mut(&mut self.root, self.focused) {
            if window.floating {
                window.width = (window.width + dw).max(240.);
                window.height = (window.height + dh).max(180.);
            }
        }
    }

    fn resize_focused(&mut self, delta: f32) {
        resize_focused(&mut self.root, self.focused, delta);
    }

    fn collect<'a>(&'a self, node: &'a Node, windows: &mut Vec<&'a WindowNode>) {
        let _ = self;
        match node {
            Node::Window(window) => windows.push(window),
            Node::Container(container) => {
                for child in &container.children {
                    self.collect(&child.node, windows);
                }
            }
        }
    }

    fn collect_views(&self, node: &Node, windows: &mut Vec<LayoutWindowView>) {
        let _ = self;
        match node {
            Node::Window(window) => windows.push(LayoutWindowView {
                id: window.id,
                title: window.title.clone(),
                detail: window.detail.clone(),
                floating: window.floating,
                focused: window.id == self.focused,
                x: window.x,
                y: window.y,
                width: window.width,
                height: window.height,
            }),
            Node::Container(container) => {
                for child in &container.children {
                    self.collect_views(&child.node, windows);
                }
            }
        }
    }

    fn find(&self, id: usize) -> Option<&WindowNode> {
        find(&self.root, id)
    }

    fn view_node(&self, node: &Node) -> LayoutView {
        match node {
            Node::Window(window) => LayoutView::Window(LayoutWindowView {
                id: window.id,
                title: window.title.clone(),
                detail: window.detail.clone(),
                floating: window.floating,
                focused: window.id == self.focused,
                x: window.x,
                y: window.y,
                width: window.width,
                height: window.height,
            }),
            Node::Container(container) => LayoutView::Container(LayoutContainerView {
                axis: container.axis.clone(),
                children: container
                    .children
                    .iter()
                    .map(|child| LayoutChildView {
                        grow: child.grow,
                        node: self.view_node(&child.node),
                    })
                    .collect(),
            }),
        }
    }
}

impl LayoutView {
    pub fn focused_detail(&self) -> Option<String> {
        match self {
            LayoutView::Window(window) if window.focused => Some(window.detail.clone()),
            LayoutView::Window(_) => None,
            LayoutView::Container(container) => container
                .children
                .iter()
                .find_map(|child| child.node.focused_detail()),
        }
    }

    pub fn tiled(&self) -> Option<LayoutView> {
        match self {
            LayoutView::Window(window) if window.floating => None,
            LayoutView::Window(window) => Some(LayoutView::Window(window.clone())),
            LayoutView::Container(container) => {
                let children = container
                    .children
                    .iter()
                    .filter_map(|child| {
                        child.node.tiled().map(|node| LayoutChildView {
                            grow: child.grow,
                            node,
                        })
                    })
                    .collect::<Vec<_>>();
                match children.len() {
                    0 => None,
                    1 => Some(children.into_iter().next().unwrap().node),
                    _ => Some(LayoutView::Container(LayoutContainerView {
                        axis: container.axis.clone(),
                        children,
                    })),
                }
            }
        }
    }

    pub fn floating_windows(&self) -> Vec<LayoutWindowView> {
        let mut windows = Vec::new();
        self.collect_floating(&mut windows);
        windows.sort_by_key(|window| window.focused);
        windows
    }

    fn collect_floating(&self, windows: &mut Vec<LayoutWindowView>) {
        match self {
            LayoutView::Window(window) if window.floating => windows.push(window.clone()),
            LayoutView::Window(_) => {}
            LayoutView::Container(container) => {
                for child in &container.children {
                    child.node.collect_floating(windows);
                }
            }
        }
    }
}

fn compute_tiled_geometries(
    view: &LayoutView,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    gap: f32,
    out: &mut Vec<WindowGeometry>,
) {
    match view {
        LayoutView::Window(win) => {
            out.push(WindowGeometry {
                id: win.id,
                title: win.title.clone(),
                focused: win.focused,
                floating: false,
                x: x.round(),
                y: y.round(),
                width: width.round().max(60.0),
                height: height.round().max(60.0),
            });
        }
        LayoutView::Container(container) => {
            if container.children.is_empty() {
                return;
            }
            let total_grow: f32 = container
                .children
                .iter()
                .map(|c| c.grow.max(0.1))
                .sum::<f32>()
                .max(0.1);
            let count = container.children.len();
            let total_gap = gap * (count.saturating_sub(1) as f32);
            match container.axis {
                Axis::Row => {
                    let avail_w = (width - total_gap).max(count as f32 * 60.0);
                    let mut cursor_x = x;
                    for (idx, child) in container.children.iter().enumerate() {
                        let ratio = child.grow.max(0.1) / total_grow;
                        let child_w = if idx + 1 == count {
                            (x + width - cursor_x).max(60.0)
                        } else {
                            (avail_w * ratio).max(60.0)
                        };
                        compute_tiled_geometries(
                            &child.node,
                            cursor_x,
                            y,
                            child_w,
                            height,
                            gap,
                            out,
                        );
                        cursor_x += child_w + gap;
                    }
                }
                Axis::Column => {
                    let avail_h = (height - total_gap).max(count as f32 * 60.0);
                    let mut cursor_y = y;
                    for (idx, child) in container.children.iter().enumerate() {
                        let ratio = child.grow.max(0.1) / total_grow;
                        let child_h = if idx + 1 == count {
                            (y + height - cursor_y).max(60.0)
                        } else {
                            (avail_h * ratio).max(60.0)
                        };
                        compute_tiled_geometries(
                            &child.node,
                            x,
                            cursor_y,
                            width,
                            child_h,
                            gap,
                            out,
                        );
                        cursor_y += child_h + gap;
                    }
                }
            }
        }
    }
}

fn find(node: &Node, id: usize) -> Option<&WindowNode> {
    match node {
        Node::Window(window) if window.id == id => Some(window),
        Node::Window(_) => None,
        Node::Container(container) => container
            .children
            .iter()
            .find_map(|child| find(&child.node, id)),
    }
}

fn find_mut(node: &mut Node, id: usize) -> Option<&mut WindowNode> {
    match node {
        Node::Window(window) if window.id == id => Some(window),
        Node::Window(_) => None,
        Node::Container(container) => container
            .children
            .iter_mut()
            .find_map(|child| find_mut(&mut child.node, id)),
    }
}

fn split_window(node: &mut Node, id: usize, axis: Axis, new_id: usize, title: String) -> bool {
    match node {
        Node::Window(window) if window.id == id => {
            let existing = window.clone();
            *node = Node::Container(ContainerNode {
                axis,
                children: vec![
                    ChildNode {
                        grow: 1.0,
                        node: Node::Window(existing),
                    },
                    ChildNode {
                        grow: 1.0,
                        node: Node::Window(WindowNode {
                            id: new_id,
                            title,
                            detail: "Ready".to_string(),
                            floating: false,
                            x: 72. + new_id as f32 * 20.,
                            y: 72. + new_id as f32 * 20.,
                            width: 420.,
                            height: 280.,
                        }),
                    },
                ],
            });
            true
        }
        Node::Window(_) => false,
        Node::Container(container) => container
            .children
            .iter_mut()
            .any(|child| split_window(&mut child.node, id, axis.clone(), new_id, title.clone())),
    }
}

fn remove_window(node: &mut Node, id: usize) -> bool {
    match node {
        Node::Window(window) => window.id == id,
        Node::Container(container) => {
            container
                .children
                .retain_mut(|child| !remove_window(&mut child.node, id));
            container
                .children
                .iter_mut()
                .for_each(|child| collapse(&mut child.node));
            container.children.is_empty()
        }
    }
}

fn collapse(node: &mut Node) {
    if let Node::Container(container) = node {
        container
            .children
            .iter_mut()
            .for_each(|child| collapse(&mut child.node));
        container.children.retain(
            |child| !matches!(&child.node, Node::Container(inner) if inner.children.is_empty()),
        );
        if container.children.len() == 1 {
            let child = container.children.remove(0);
            *node = child.node;
        }
    }
}

fn set_floating(node: &mut Node, floating: bool) {
    match node {
        Node::Window(window) => window.floating = floating,
        Node::Container(container) => {
            for child in &mut container.children {
                set_floating(&mut child.node, floating);
            }
        }
    }
}

fn stack_offset(node: &mut Node) {
    let mut index = 0u32;
    stack_offset_recursive(node, &mut index);
}

fn stack_offset_recursive(node: &mut Node, index: &mut u32) {
    match node {
        Node::Window(window) => {
            window.x = 120. + *index as f32 * 32.;
            window.y = 80. + *index as f32 * 32.;
            *index += 1;
        }
        Node::Container(container) => {
            for child in &mut container.children {
                stack_offset_recursive(&mut child.node, index);
            }
        }
    }
}

fn center_all(node: &mut Node) {
    match node {
        Node::Window(window) => {
            window.x = 480.;
            window.y = 240.;
        }
        Node::Container(container) => {
            for child in &mut container.children {
                center_all(&mut child.node);
            }
        }
    }
}

fn balance_panes(node: &mut Node) {
    match node {
        Node::Window(_) => {}
        Node::Container(container) => {
            for child in &mut container.children {
                child.grow = 1.0;
                balance_panes(&mut child.node);
            }
        }
    }
}

fn swap_focused(node: &mut Node, focused: usize, dx: i32, dy: i32) {
    swap_focused_recursive(node, focused, dx, dy);
}

fn swap_focused_recursive(node: &mut Node, focused: usize, dx: i32, dy: i32) -> bool {
    match node {
        Node::Window(_) => false,
        Node::Container(container) => {
            if let Some(index) = container
                .children
                .iter()
                .position(|child| contains_window(&child.node, focused))
            {
                let horizontal = matches!(container.axis, Axis::Row);
                let want = if horizontal { dx != 0 } else { dy != 0 };
                if want {
                    let step = if horizontal { dx } else { dy };
                    let target = index as i32 + step;
                    if target >= 0 && target < container.children.len() as i32 {
                        container.children.swap(index, target as usize);
                        return true;
                    }
                }
                return swap_focused_recursive(
                    &mut container.children[index].node,
                    focused,
                    dx,
                    dy,
                );
            }
            container
                .children
                .iter_mut()
                .any(|child| swap_focused_recursive(&mut child.node, focused, dx, dy))
        }
    }
}

fn flip_axes(node: &mut Node) {
    match node {
        Node::Window(_) => {}
        Node::Container(container) => {
            container.axis = match container.axis {
                Axis::Row => Axis::Column,
                Axis::Column => Axis::Row,
            };
            for child in &mut container.children {
                flip_axes(&mut child.node);
            }
        }
    }
}

fn resize_focused(node: &mut Node, focused: usize, delta: f32) -> bool {
    match node {
        Node::Window(_) => false,
        Node::Container(container) => {
            if let Some(index) = container
                .children
                .iter()
                .position(|child| contains_window(&child.node, focused))
            {
                if container.children.len() > 1 {
                    let target = container.children[index].grow + delta;
                    let sibling = neighbor_index(container.children.len(), index);
                    let sibling_target = container.children[sibling].grow - delta;
                    if target >= 0.3 && sibling_target >= 0.3 {
                        container.children[index].grow = target;
                        container.children[sibling].grow = sibling_target;
                        return true;
                    }
                }
                return resize_focused(&mut container.children[index].node, focused, delta);
            }
            false
        }
    }
}

fn contains_window(node: &Node, id: usize) -> bool {
    match node {
        Node::Window(window) => window.id == id,
        Node::Container(container) => container
            .children
            .iter()
            .any(|child| contains_window(&child.node, id)),
    }
}

fn neighbor_index(len: usize, index: usize) -> usize {
    if index + 1 < len {
        index + 1
    } else {
        index - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_should_add_window_and_change_axis() {
        let mut layout = LayoutState::new();
        layout.apply(&LayoutAction::SplitRow);

        assert_eq!(layout.axis(), "row");
        assert_eq!(layout.focused_title(), "Window 3");
        assert_eq!(layout.summary(), "3 tiled 0 floating");
    }

    #[test]
    fn reset_should_restore_seed_layout() {
        let mut layout = LayoutState::new();
        layout.apply(&LayoutAction::SplitColumn);
        layout.apply(&LayoutAction::ToggleFloat);
        layout.apply(&LayoutAction::Reset);
        assert_eq!(layout.summary(), "2 tiled 0 floating");
        assert_eq!(layout.axis(), "row");
        assert_eq!(layout.focused_title(), "Workspace");
    }

    #[test]
    fn nudge_should_move_floating_window_only() {
        let mut layout = LayoutState::new();
        layout.apply(&LayoutAction::ToggleFloat);
        let before = match layout.view() {
            LayoutView::Container(container) => match &container.children[0].node {
                LayoutView::Window(window) => (window.x, window.y),
                _ => panic!("expected window"),
            },
            _ => panic!("expected container"),
        };
        layout.apply(&LayoutAction::NudgeRight);
        layout.apply(&LayoutAction::NudgeDown);
        let after = match layout.view() {
            LayoutView::Container(container) => match &container.children[0].node {
                LayoutView::Window(window) => (window.x, window.y),
                _ => panic!("expected window"),
            },
            _ => panic!("expected container"),
        };
        assert!(after.0 > before.0);
        assert!(after.1 > before.1);
    }

    #[test]
    fn resize_should_change_floating_window_size_only() {
        let mut layout = LayoutState::new();
        layout.apply(&LayoutAction::ToggleFloat);
        let before = match layout.view() {
            LayoutView::Container(container) => match &container.children[0].node {
                LayoutView::Window(window) => (window.width, window.height),
                _ => panic!("expected window"),
            },
            _ => panic!("expected container"),
        };
        layout.apply(&LayoutAction::ExpandWindow);
        layout.apply(&LayoutAction::ContractWindow);
        layout.apply(&LayoutAction::ContractWindow);
        let after = match layout.view() {
            LayoutView::Container(container) => match &container.children[0].node {
                LayoutView::Window(window) => (window.width, window.height),
                _ => panic!("expected window"),
            },
            _ => panic!("expected container"),
        };
        assert!(after.0 < before.0);
        assert!(after.1 < before.1);
        assert!(after.0 >= 240.);
        assert!(after.1 >= 180.);
    }

    #[test]
    fn focus_next_should_cycle() {
        let mut layout = LayoutState::new();
        layout.apply(&LayoutAction::FocusNext);

        assert_eq!(layout.focused_title(), "Scratch");
    }

    #[test]
    fn toggle_float_should_flip_focused_window() {
        let mut layout = LayoutState::new();
        layout.apply(&LayoutAction::ToggleFloat);

        assert_eq!(layout.summary(), "1 tiled 1 floating");
    }

    #[test]
    fn close_focused_should_keep_at_least_one_window() {
        let mut layout = LayoutState::new();
        layout.apply(&LayoutAction::CloseFocused);

        assert_eq!(layout.summary(), "1 tiled 0 floating");
        assert_eq!(layout.focused_title(), "Scratch");
    }

    #[test]
    fn set_window_mode_should_flip_all_windows() {
        let mut layout = LayoutState::new();
        layout.set_window_mode(&WindowMode::Floating);
        assert_eq!(layout.summary(), "0 tiled 2 floating");
        layout.set_window_mode(&WindowMode::Tiling);
        assert_eq!(layout.summary(), "2 tiled 0 floating");
    }

    #[test]
    fn demo_layout_should_seed_multiple_windows_and_floating_utility() {
        let layout = LayoutState::demo();
        assert_eq!(layout.summary(), "3 tiled 1 floating");
        assert_eq!(layout.focused_title(), "Workspace");
        assert_eq!(layout.view().floating_windows().len(), 1);
    }

    #[test]
    fn set_window_floating_should_override_single_window() {
        let mut layout = LayoutState::demo();
        layout.set_window_mode(&WindowMode::Tiling);
        assert_eq!(layout.summary(), "4 tiled 0 floating");
        layout.set_window_floating(4, true);
        assert_eq!(layout.summary(), "3 tiled 1 floating");
    }

    #[test]
    fn set_focused_window_content_should_update_focused_pane() {
        let mut layout = LayoutState::new();
        layout.set_focused_window_content("Terminal", "echo hi");
        match layout.view() {
            LayoutView::Container(container) => match &container.children[0].node {
                LayoutView::Window(window) => {
                    assert_eq!(window.title, "Terminal");
                    assert_eq!(window.detail, "echo hi");
                }
                _ => panic!("expected window"),
            },
            _ => panic!("expected container"),
        }
    }

    #[test]
    fn focus_window_should_ignore_unknown_ids() {
        let mut layout = LayoutState::new();
        layout.focus_window(2);
        assert_eq!(layout.focused_title(), "Scratch");
        layout.focus_window(999);
        assert_eq!(layout.focused_title(), "Scratch");
    }

    #[test]
    fn view_should_expose_child_grow_ratios() {
        let layout = LayoutState::new();
        match layout.view() {
            LayoutView::Container(container) => {
                assert_eq!(container.children.len(), 2);
                assert!(container.children[0].grow > container.children[1].grow);
            }
            _ => panic!("expected container"),
        }
    }

    #[test]
    fn tiled_view_should_drop_floating_windows() {
        let mut layout = LayoutState::new();
        layout.apply(&LayoutAction::ToggleFloat);
        let view = layout.view();
        let tiled = view.tiled().expect("expected tiled view");
        match tiled {
            LayoutView::Window(window) => assert_eq!(window.title, "Scratch"),
            _ => panic!("expected remaining tiled window"),
        }
        assert_eq!(view.floating_windows().len(), 1);
    }

    #[test]
    fn floating_windows_should_put_focused_last() {
        let mut layout = LayoutState::new();
        layout.apply(&LayoutAction::ToggleFloat);
        layout.apply(&LayoutAction::SplitRow);
        layout.apply(&LayoutAction::ToggleFloat);
        let windows = layout.view().floating_windows();
        assert_eq!(windows.len(), 2);
        assert!(windows[1].focused);
    }

    #[test]
    fn grow_focused_should_change_flex_ratios() {
        let mut layout = LayoutState::new();
        let before = match layout.view() {
            LayoutView::Container(container) => container.children[0].grow,
            _ => panic!("expected container"),
        };
        layout.apply(&LayoutAction::GrowFocused);
        let after = match layout.view() {
            LayoutView::Container(container) => container.children[0].grow,
            _ => panic!("expected container"),
        };
        assert!(after > before);
    }

    #[test]
    fn shrink_focused_should_respect_minimum_ratio() {
        let mut layout = LayoutState::new();
        for _ in 0..20 {
            layout.apply(&LayoutAction::ShrinkFocused);
        }
        match layout.view() {
            LayoutView::Container(container) => {
                assert!(container.children[0].grow >= 0.3);
                assert!(container.children[1].grow >= 0.3);
            }
            _ => panic!("expected container"),
        }
    }

    #[test]
    fn flip_axis_should_toggle_container_directions() {
        let mut layout = LayoutState::demo();
        assert_eq!(layout.axis(), "row");
        layout.apply(&LayoutAction::FlipAxis);
        assert_eq!(layout.axis(), "column");
        match layout.view() {
            LayoutView::Container(container) => match &container.children[0].node {
                LayoutView::Container(nested) => assert_eq!(nested.axis, Axis::Row),
                _ => panic!("expected nested container"),
            },
            _ => panic!("expected container"),
        }
    }

    #[test]
    fn focus_previous_should_cycle_backwards() {
        let mut layout = LayoutState::new();
        assert_eq!(layout.focused_title(), "Workspace");
        layout.apply(&LayoutAction::FocusPrevious);
        assert_eq!(layout.focused_title(), "Scratch");
        layout.apply(&LayoutAction::FocusPrevious);
        assert_eq!(layout.focused_title(), "Workspace");
    }

    #[test]
    fn focus_first_and_last_should_jump_to_ends() {
        let mut layout = LayoutState::demo();
        layout.apply(&LayoutAction::FocusLast);
        let last = layout.focused_title().to_string();
        layout.apply(&LayoutAction::FocusFirst);
        let first = layout.focused_title().to_string();
        assert_eq!(first, "Workspace");
        assert_eq!(last, "Inspector");
    }

    #[test]
    fn balance_panes_should_equalize_grow_ratios() {
        let mut layout = LayoutState::new();
        layout.apply(&LayoutAction::GrowFocused);
        layout.apply(&LayoutAction::BalancePanes);
        match layout.view() {
            LayoutView::Container(container) => {
                assert_eq!(container.children[0].grow, 1.0);
                assert_eq!(container.children[1].grow, 1.0);
            }
            _ => panic!("expected container"),
        }
    }

    #[test]
    fn center_focused_should_float_and_reposition() {
        let mut layout = LayoutState::new();
        layout.apply(&LayoutAction::CenterFocused);
        let window = layout.focused_window().expect("focused window");
        assert!(window.floating);
        assert_eq!(window.x, 480.);
        assert_eq!(window.y, 240.);
    }

    #[test]
    fn move_right_should_swap_tiled_window_order() {
        let mut layout = LayoutState::new();
        assert_eq!(layout.focused_title(), "Workspace");
        layout.apply(&LayoutAction::MoveRight);
        match layout.view() {
            LayoutView::Container(container) => match &container.children[0].node {
                LayoutView::Window(window) => {
                    assert_eq!(window.title, "Scratch");
                }
                _ => panic!("expected window"),
            },
            _ => panic!("expected container"),
        }
    }

    #[test]
    fn set_window_mode_stack_should_float_and_offset() {
        let mut layout = LayoutState::new();
        layout.set_window_mode(&WindowMode::Stack);
        assert_eq!(layout.summary(), "0 tiled 2 floating");
        let windows = layout.windows();
        assert!(windows[1].x > windows[0].x);
        assert!(windows[1].y > windows[0].y);
    }

    #[test]
    fn set_window_mode_center_should_float_and_center() {
        let mut layout = LayoutState::new();
        layout.set_window_mode(&WindowMode::Center);
        assert_eq!(layout.summary(), "0 tiled 2 floating");
        for window in layout.windows() {
            assert_eq!(window.x, 480.);
            assert_eq!(window.y, 240.);
        }
    }

    #[test]
    fn set_window_mode_grid_should_balance_panes() {
        let mut layout = LayoutState::new();
        layout.apply(&LayoutAction::GrowFocused);
        layout.set_window_mode(&WindowMode::Grid);
        match layout.view() {
            LayoutView::Container(container) => {
                assert_eq!(container.children[0].grow, 1.0);
            }
            _ => panic!("expected container"),
        }
    }

    #[test]
    fn window_mode_next_should_cycle_through_all_modes() {
        let modes = WindowMode::all();
        let mut mode = WindowMode::Tiling;
        for _ in 0..modes.len() {
            mode = mode.next();
        }
        assert_eq!(mode, WindowMode::Tiling);
    }

    #[test]
    fn window_mode_from_label_should_roundtrip() {
        for mode in WindowMode::all() {
            assert_eq!(WindowMode::from_label(mode.label()), Some(*mode));
        }
        assert_eq!(WindowMode::from_label("unknown"), None);
    }

    #[test]
    fn layout_action_from_label_should_roundtrip() {
        for action in LayoutAction::all() {
            assert_eq!(
                LayoutAction::from_label(action.label()),
                Some(action.clone())
            );
        }
        assert_eq!(LayoutAction::from_label("not-a-layout-action"), None);
    }

    #[test]
    fn snap_halves_quarters_and_thirds_should_position_window() {
        let mut layout = LayoutState::new();
        layout.set_workarea(1200.0, 800.0);

        layout.apply(&LayoutAction::SnapLeftHalf);
        let win = layout.focused_window().unwrap();
        assert!(win.floating);
        assert_eq!(
            (win.x, win.y, win.width, win.height),
            (0.0, 0.0, 600.0, 800.0)
        );

        layout.apply(&LayoutAction::SnapRightHalf);
        let win = layout.focused_window().unwrap();
        assert_eq!(
            (win.x, win.y, win.width, win.height),
            (600.0, 0.0, 600.0, 800.0)
        );

        layout.apply(&LayoutAction::SnapTopHalf);
        let win = layout.focused_window().unwrap();
        assert_eq!(
            (win.x, win.y, win.width, win.height),
            (0.0, 0.0, 1200.0, 400.0)
        );

        layout.apply(&LayoutAction::SnapBottomHalf);
        let win = layout.focused_window().unwrap();
        assert_eq!(
            (win.x, win.y, win.width, win.height),
            (0.0, 400.0, 1200.0, 400.0)
        );

        layout.apply(&LayoutAction::SnapBottomRight);
        let win = layout.focused_window().unwrap();
        assert_eq!(
            (win.x, win.y, win.width, win.height),
            (600.0, 400.0, 600.0, 400.0)
        );

        layout.apply(&LayoutAction::SnapFirstThird);
        let win = layout.focused_window().unwrap();
        assert_eq!(
            (win.x, win.y, win.width, win.height),
            (0.0, 0.0, 400.0, 800.0)
        );

        layout.apply(&LayoutAction::SnapCenterThird);
        let win = layout.focused_window().unwrap();
        assert_eq!(
            (win.x, win.y, win.width, win.height),
            (400.0, 0.0, 400.0, 800.0)
        );

        layout.apply(&LayoutAction::SnapLastTwoThirds);
        let win = layout.focused_window().unwrap();
        assert_eq!(
            (win.x, win.y, win.width, win.height),
            (400.0, 0.0, 800.0, 800.0)
        );

        layout.apply(&LayoutAction::Maximize);
        let win = layout.focused_window().unwrap();
        assert_eq!(
            (win.x, win.y, win.width, win.height),
            (0.0, 0.0, 1200.0, 800.0)
        );

        layout.apply(&LayoutAction::Restore);
        let win = layout.focused_window().unwrap();
        assert!(!win.floating);
    }

    #[test]
    fn compute_geometries_should_tile_float_and_monocle_by_mode() {
        let mut layout = LayoutState::new();
        layout.set_workarea(1000.0, 600.0);
        layout.apply(&LayoutAction::BalancePanes);

        let tiled = layout.compute_geometries(&WindowMode::Tiling, 0.0);
        assert_eq!(tiled.len(), 2);
        assert_eq!(tiled[0].x, 0.0);
        assert_eq!(tiled[0].width, 500.0);
        assert_eq!(tiled[1].x, 500.0);
        assert_eq!(tiled[1].width, 500.0);

        let monocle = layout.compute_geometries(&WindowMode::Monocle, 10.0);
        assert_eq!(monocle.len(), 2);
        for geo in &monocle {
            assert_eq!(
                (geo.x, geo.y, geo.width, geo.height),
                (10.0, 10.0, 980.0, 580.0)
            );
        }

        let floating = layout.compute_geometries(&WindowMode::Floating, 0.0);
        assert_eq!(floating.len(), 2);
        assert!(floating.iter().all(|g| g.floating));
    }

    #[test]
    fn niri_strip_should_cycle_column_widths_and_pan_viewport() {
        let mut layout = LayoutState::new();
        layout.set_workarea(1200.0, 800.0);
        layout.focus_window(1);

        assert_eq!(layout.focused_column_width(), ColumnWidth::OneHalf);
        layout.apply(&LayoutAction::CycleColumnWidth);
        assert_eq!(layout.focused_column_width(), ColumnWidth::TwoThirds);
        layout.apply(&LayoutAction::CycleColumnWidth);
        assert_eq!(layout.focused_column_width(), ColumnWidth::Full);

        // Add a third window so total strip width exceeds workarea and focusing right pans viewport
        let _ = layout.split_next("Browser".to_string());
        assert_eq!(layout.niri_columns().len(), 3);
        assert!(layout.viewport_x() > 0.0);

        let geoms = layout.compute_geometries(&WindowMode::NiriStrip, 0.0);
        assert_eq!(geoms.len(), 3);
        let focused_geom = geoms.iter().find(|g| g.focused).unwrap();
        assert!(focused_geom.x >= 0.0 && focused_geom.x + focused_geom.width <= 1200.0);

        // Focus back to first column and verify viewport pans back to 0
        layout.apply(&LayoutAction::FocusColumnLeft);
        layout.apply(&LayoutAction::FocusColumnLeft);
        assert_eq!(layout.focused_id(), 1);
        assert_eq!(layout.viewport_x(), 0.0);
    }

    #[test]
    fn niri_strip_should_consume_and_expel_windows_in_column() {
        let mut layout = LayoutState::new();
        layout.set_workarea(1200.0, 800.0);
        assert_eq!(layout.niri_columns().len(), 2);

        layout.apply(&LayoutAction::ConsumeWindowIntoColumn);
        assert_eq!(layout.niri_columns().len(), 1);
        assert_eq!(layout.niri_columns()[0].window_ids, vec![1, 2]);

        let geoms = layout.compute_geometries(&WindowMode::NiriStrip, 0.0);
        assert_eq!(geoms.len(), 2);
        assert_eq!(geoms[0].x, geoms[1].x);
        assert_eq!(geoms[0].height, 400.0);
        assert_eq!(geoms[1].y, 400.0);

        layout.apply(&LayoutAction::ExpelWindowFromColumn);
        assert_eq!(layout.niri_columns().len(), 2);
    }

    #[test]
    fn niri_strip_should_move_columns_left_and_right() {
        let mut layout = LayoutState::new();
        assert_eq!(layout.niri_columns()[0].window_ids, vec![1]);
        assert_eq!(layout.niri_columns()[1].window_ids, vec![2]);

        layout.apply(&LayoutAction::MoveColumnRight);
        assert_eq!(layout.niri_columns()[0].window_ids, vec![2]);
        assert_eq!(layout.niri_columns()[1].window_ids, vec![1]);

        layout.apply(&LayoutAction::MoveColumnLeft);
        assert_eq!(layout.niri_columns()[0].window_ids, vec![1]);
        assert_eq!(layout.niri_columns()[1].window_ids, vec![2]);
    }

    #[test]
    fn half_snap_should_cycle_one_half_one_third_two_thirds_and_restore() {
        let mut layout = LayoutState::new();
        layout.set_workarea(1200.0, 800.0);

        layout.apply(&LayoutAction::SnapLeftHalf);
        assert_eq!(layout.focused_window().unwrap().width, 600.0);

        layout.apply(&LayoutAction::SnapLeftHalf);
        assert_eq!(layout.focused_window().unwrap().width, 400.0);

        layout.apply(&LayoutAction::SnapLeftHalf);
        assert_eq!(layout.focused_window().unwrap().width, 800.0);

        layout.apply(&LayoutAction::Restore);
        let restored = layout.focused_window().unwrap();
        assert!(!restored.floating);
        assert_eq!(restored.width, 420.0);
    }

    #[test]
    fn room_layouts_should_arrange_focus_stack_columns_and_grid() {
        let mut layout = LayoutState::demo();
        layout.set_workarea(1000.0, 600.0);

        layout.apply(&LayoutAction::EnterRoomFocus);
        assert_eq!(layout.room_layout(), RoomLayoutKind::Focus);
        let wins = layout.windows();
        assert_eq!(wins[0].width, 600.0);
        assert_eq!(wins[1].x, 600.0);

        layout.apply(&LayoutAction::EnterRoomStack);
        assert_eq!(layout.room_layout(), RoomLayoutKind::Stack);
        let wins = layout.windows();
        assert_eq!(wins[2].y, 32.0);

        layout.apply(&LayoutAction::EnterRoomColumns);
        assert_eq!(layout.room_layout(), RoomLayoutKind::Columns);
        let wins = layout.windows();
        assert_eq!(wins[0].width, 250.0);

        layout.apply(&LayoutAction::EnterRoomGrid);
        assert_eq!(layout.room_layout(), RoomLayoutKind::Grid);
        let wins = layout.windows();
        assert_eq!(wins[0].width, 500.0);
        assert_eq!(wins[0].height, 300.0);
    }
}
