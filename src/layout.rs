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
    Reset,
    FocusNext,
    CloseFocused,
    ToggleFloat,
    GrowFocused,
    ShrinkFocused,
}

impl LayoutAction {
    pub fn title(&self) -> &'static str {
        match self {
            Self::SplitRow => "Split row",
            Self::SplitColumn => "Split column",
            Self::Reset => "Reset layout",
            Self::FocusNext => "Focus next",
            Self::CloseFocused => "Close focused",
            Self::ToggleFloat => "Toggle float",
            Self::GrowFocused => "Grow focused",
            Self::ShrinkFocused => "Shrink focused",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LayoutState {
    root: Node,
    focused: usize,
    next_id: usize,
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
                        }),
                    },
                    ChildNode {
                        grow: 0.9,
                        node: Node::Window(WindowNode {
                            id: 2,
                            title: "Scratch".to_string(),
                            detail: "Ready".to_string(),
                            floating: false,
                        }),
                    },
                ],
            }),
            focused: 1,
            next_id: 3,
        }
    }

    pub fn apply(&mut self, action: &LayoutAction) {
        match action {
            LayoutAction::SplitRow => self.split(Axis::Row),
            LayoutAction::SplitColumn => self.split(Axis::Column),
            LayoutAction::Reset => *self = Self::seed(),
            LayoutAction::FocusNext => self.focus_next(),
            LayoutAction::CloseFocused => self.close_focused(),
            LayoutAction::ToggleFloat => self.toggle_float(),
            LayoutAction::GrowFocused => self.resize_focused(0.2),
            LayoutAction::ShrinkFocused => self.resize_focused(-0.2),
        }
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
        let floating = matches!(mode, WindowMode::Floating);
        set_floating(&mut self.root, floating);
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
        }
    }

    fn split(&mut self, axis: Axis) {
        let new_id = self.next_id;
        self.next_id += 1;
        let title = format!("Window {new_id}");
        let focused = self.focused;
        let _ = split_window(&mut self.root, focused, axis, new_id, title);
        self.focused = new_id;
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

    fn close_focused(&mut self) {
        let mut windows = Vec::new();
        self.collect(&self.root, &mut windows);
        if windows.len() < 2 {
            return;
        }
        let fallback = windows
            .iter()
            .find(|window| window.id != self.focused)
            .map(|window| window.id)
            .unwrap_or(self.focused);
        let _ = remove_window(&mut self.root, self.focused);
        collapse(&mut self.root);
        self.focused = fallback;
    }

    fn toggle_float(&mut self) {
        if let Some(window) = find_mut(&mut self.root, self.focused) {
            window.floating = !window.floating;
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
    pub fn into_focused_detail(&self) -> Option<String> {
        match self {
            LayoutView::Window(window) if window.focused => Some(window.detail.clone()),
            LayoutView::Window(_) => None,
            LayoutView::Container(container) => container
                .children
                .iter()
                .find_map(|child| child.node.into_focused_detail()),
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
            false
        }
    }
}

fn collapse(node: &mut Node) {
    if let Node::Container(container) = node {
        if container.children.len() == 1 {
            let child = container.children.remove(0);
            *node = child.node;
            return;
        }
        container
            .children
            .iter_mut()
            .for_each(|child| collapse(&mut child.node));
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
}
