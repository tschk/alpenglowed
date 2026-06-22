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
    FocusNext,
    CloseFocused,
    ToggleFloat,
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
    pub floating: bool,
    pub focused: bool,
}

#[derive(Debug, Clone)]
pub struct LayoutContainerView {
    pub axis: Axis,
    pub children: Vec<LayoutView>,
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
    floating: bool,
}

#[derive(Debug, Clone)]
struct ContainerNode {
    axis: Axis,
    children: Vec<Node>,
}

impl LayoutState {
    pub fn new() -> Self {
        Self {
            root: Node::Container(ContainerNode {
                axis: Axis::Row,
                children: vec![
                    Node::Window(WindowNode {
                        id: 1,
                        title: "Workspace".to_string(),
                        floating: false,
                    }),
                    Node::Window(WindowNode {
                        id: 2,
                        title: "Scratch".to_string(),
                        floating: false,
                    }),
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
            LayoutAction::FocusNext => self.focus_next(),
            LayoutAction::CloseFocused => self.close_focused(),
            LayoutAction::ToggleFloat => self.toggle_float(),
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

    fn collect<'a>(&'a self, node: &'a Node, windows: &mut Vec<&'a WindowNode>) {
        let _ = self;
        match node {
            Node::Window(window) => windows.push(window),
            Node::Container(container) => {
                for child in &container.children {
                    self.collect(child, windows);
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
                floating: window.floating,
                focused: window.id == self.focused,
            }),
            Node::Container(container) => LayoutView::Container(LayoutContainerView {
                axis: container.axis.clone(),
                children: container
                    .children
                    .iter()
                    .map(|child| self.view_node(child))
                    .collect(),
            }),
        }
    }
}

fn find(node: &Node, id: usize) -> Option<&WindowNode> {
    match node {
        Node::Window(window) if window.id == id => Some(window),
        Node::Window(_) => None,
        Node::Container(container) => container.children.iter().find_map(|child| find(child, id)),
    }
}

fn find_mut(node: &mut Node, id: usize) -> Option<&mut WindowNode> {
    match node {
        Node::Window(window) if window.id == id => Some(window),
        Node::Window(_) => None,
        Node::Container(container) => container
            .children
            .iter_mut()
            .find_map(|child| find_mut(child, id)),
    }
}

fn split_window(node: &mut Node, id: usize, axis: Axis, new_id: usize, title: String) -> bool {
    match node {
        Node::Window(window) if window.id == id => {
            let existing = window.clone();
            *node = Node::Container(ContainerNode {
                axis,
                children: vec![
                    Node::Window(existing),
                    Node::Window(WindowNode {
                        id: new_id,
                        title,
                        floating: false,
                    }),
                ],
            });
            true
        }
        Node::Window(_) => false,
        Node::Container(container) => container
            .children
            .iter_mut()
            .any(|child| split_window(child, id, axis.clone(), new_id, title.clone())),
    }
}

fn remove_window(node: &mut Node, id: usize) -> bool {
    match node {
        Node::Window(window) => window.id == id,
        Node::Container(container) => {
            container
                .children
                .retain_mut(|child| !remove_window(child, id));
            container.children.iter_mut().for_each(collapse);
            false
        }
    }
}

fn collapse(node: &mut Node) {
    if let Node::Container(container) = node {
        if container.children.len() == 1 {
            let child = container.children.remove(0);
            *node = child;
            return;
        }
        container.children.iter_mut().for_each(collapse);
    }
}

fn set_floating(node: &mut Node, floating: bool) {
    match node {
        Node::Window(window) => window.floating = floating,
        Node::Container(container) => {
            for child in &mut container.children {
                set_floating(child, floating);
            }
        }
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
}
