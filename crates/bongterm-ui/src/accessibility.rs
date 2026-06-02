//! Safe UI Automation model over `BongTerm` shell state.

use crate::BongTermShell;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AxRole {
    Window,
    TabList,
    Tab,
    Pane,
    TerminalText,
    Scrollback,
    CommandBlock,
    Control,
}

impl AxRole {
    #[must_use]
    pub const fn uia_control_type_id(self) -> i32 {
        match self {
            Self::Window => 50032,
            Self::TabList => 50018,
            Self::Tab => 50019,
            Self::Pane => 50033,
            Self::TerminalText | Self::Scrollback => 50020,
            Self::CommandBlock => 50026,
            Self::Control => 50000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxNode {
    pub role: AxRole,
    pub name: String,
    pub value: String,
    pub children: Vec<AxNode>,
}

impl AxNode {
    #[must_use]
    pub fn new(role: AxRole, name: impl Into<String>) -> Self {
        Self {
            role,
            name: name.into(),
            value: String::new(),
            children: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }

    #[must_use]
    pub fn child(mut self, node: AxNode) -> Self {
        self.children.push(node);
        self
    }

    #[must_use]
    pub fn find_role(&self, role: AxRole) -> Option<&Self> {
        if self.role == role {
            return Some(self);
        }
        self.children.iter().find_map(|child| child.find_role(role))
    }
}

pub struct AccessibilityTree;

impl AccessibilityTree {
    #[must_use]
    pub fn from_shell(shell: &BongTermShell) -> AxNode {
        let mut root = AxNode::new(AxRole::Window, shell.title());
        for region in shell.region_names() {
            let role = match region {
                "tab-strip" => AxRole::TabList,
                "terminal-surface" => AxRole::TerminalText,
                "command-palette" => AxRole::Control,
                _ => AxRole::Pane,
            };
            root = root.child(AxNode::new(role, region));
        }
        root
    }

    #[must_use]
    pub fn from_shell_with_surface(shell: &BongTermShell, surface_text: &str) -> AxNode {
        let mut root = Self::from_shell(shell);
        for child in &mut root.children {
            if child.role == AxRole::TerminalText {
                child.value = surface_text.to_string();
            }
        }
        root
    }
}

pub trait UiaProvider {
    fn root(&self) -> AxNode;
    fn name_of(&self, index: usize) -> Option<String>;
    fn value_of(&self, index: usize) -> Option<String>;
    fn control_type_of(&self, index: usize) -> Option<i32>;
}

pub struct TreeUiaProvider {
    flat: Vec<AxNode>,
}

impl TreeUiaProvider {
    #[must_use]
    pub fn new(root: AxNode) -> Self {
        let mut flat = Vec::new();
        Self::flatten(root, &mut flat);
        Self { flat }
    }

    fn flatten(node: AxNode, out: &mut Vec<AxNode>) {
        out.push(AxNode {
            role: node.role,
            name: node.name,
            value: node.value,
            children: Vec::new(),
        });
        for child in node.children {
            Self::flatten(child, out);
        }
    }
}

impl UiaProvider for TreeUiaProvider {
    fn root(&self) -> AxNode {
        self.flat
            .first()
            .cloned()
            .unwrap_or_else(|| AxNode::new(AxRole::Window, "BongTerm"))
    }

    fn name_of(&self, index: usize) -> Option<String> {
        self.flat.get(index).map(|node| node.name.clone())
    }

    fn value_of(&self, index: usize) -> Option<String> {
        self.flat.get(index).map(|node| node.value.clone())
    }

    fn control_type_of(&self, index: usize) -> Option<i32> {
        self.flat
            .get(index)
            .map(|node| node.role.uia_control_type_id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_type_ids_are_documented_uia_constants() {
        assert_eq!(AxRole::Window.uia_control_type_id(), 50032);
        assert_eq!(AxRole::TabList.uia_control_type_id(), 50018);
        assert_eq!(AxRole::TerminalText.uia_control_type_id(), 50020);
        assert_eq!(AxRole::CommandBlock.uia_control_type_id(), 50026);
    }

    #[test]
    fn tree_from_shell_exposes_terminal_text() {
        let shell = BongTermShell::default();
        let tree = AccessibilityTree::from_shell_with_surface(&shell, "PS> cargo test");
        assert_eq!(
            tree.find_role(AxRole::TerminalText).unwrap().value,
            "PS> cargo test"
        );
    }

    #[test]
    fn provider_flattens_depth_first() {
        let tree = AxNode::new(AxRole::Window, "root")
            .child(AxNode::new(AxRole::TabList, "tabs"))
            .child(AxNode::new(AxRole::TerminalText, "surface").with_value("prompt"));
        let provider = TreeUiaProvider::new(tree);
        assert_eq!(provider.name_of(0).as_deref(), Some("root"));
        assert_eq!(provider.name_of(2).as_deref(), Some("surface"));
        assert_eq!(provider.value_of(2).as_deref(), Some("prompt"));
        assert_eq!(provider.control_type_of(2), Some(50020));
    }
}
