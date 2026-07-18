//! Meridian-owned semantic output projected by private platform adapters.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use meridian_ui_core::{
    SemanticRole, UiControlState, UiNodeId, UiRect, MAX_RETAINED_NODES, MAX_TEXT_BYTES,
};

/// Platform-neutral assistive action vocabulary.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SemanticAction {
    Activate,
    Focus,
    Expand,
    Collapse,
    Increment,
    Decrement,
    ReplaceSelectedText,
    SetValue,
    ScrollIntoView,
    ShowContextMenu,
}

/// Bounded live-region announcement policy.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SemanticLive {
    #[default]
    Off,
    Polite,
    Assertive,
}

/// Position metadata for a virtualized item without realizing its collection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticCollectionItem {
    pub position: u32,
    pub set_size: u32,
}

/// One platform-neutral semantic node in declared reading order.
#[derive(Clone, Debug, PartialEq)]
pub struct SemanticNode {
    pub id: UiNodeId,
    pub parent: Option<UiNodeId>,
    pub role: SemanticRole,
    pub name: String,
    pub description: Option<String>,
    /// Product command identifier; adapters never execute it directly.
    pub command: Option<String>,
    pub actions: Vec<SemanticAction>,
    pub value: Option<String>,
    pub state: UiControlState,
    pub live: SemanticLive,
    pub collection_item: Option<SemanticCollectionItem>,
    pub bounds: UiRect,
    pub focused: bool,
}

/// Accepted flat semantic tree. Node order is the assistive reading order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SemanticTree {
    pub root: Option<UiNodeId>,
    pub focus: Option<UiNodeId>,
    pub nodes: Vec<SemanticNode>,
}

impl SemanticTree {
    /// Validates stable identity, reading order, focus, bounds, actions, and limits.
    ///
    /// # Errors
    ///
    /// Returns the first typed malformed-tree or aggregate-limit error.
    pub fn validate(&self) -> Result<(), SemanticTreeError> {
        if self.nodes.len() > MAX_RETAINED_NODES {
            return Err(SemanticTreeError::TooManyNodes {
                count: self.nodes.len(),
                maximum: MAX_RETAINED_NODES,
            });
        }
        if self.nodes.is_empty() {
            if self.root.is_none() && self.focus.is_none() {
                return Ok(());
            }
            return Err(SemanticTreeError::MissingRoot);
        }
        let root = self.root.ok_or(SemanticTreeError::MissingRoot)?;
        if self.nodes[0].id != root || self.nodes[0].parent.is_some() {
            return Err(SemanticTreeError::InvalidRoot(root));
        }
        let mut seen = BTreeSet::new();
        let mut focused = None;
        let mut text_bytes = 0_usize;
        for node in &self.nodes {
            if !seen.insert(node.id) {
                return Err(SemanticTreeError::DuplicateNode(node.id));
            }
            if node.id != root {
                let parent = node
                    .parent
                    .ok_or(SemanticTreeError::MissingParent(node.id))?;
                if !seen.contains(&parent) {
                    return Err(SemanticTreeError::ParentAfterChild {
                        node: node.id,
                        parent,
                    });
                }
            }
            validate_bounds(node.id, node.bounds)?;
            if node.actions.len() > SemanticAction::ALL.len() {
                return Err(SemanticTreeError::TooManyActions(node.id));
            }
            let action_set = node.actions.iter().copied().collect::<BTreeSet<_>>();
            if action_set.len() != node.actions.len() {
                return Err(SemanticTreeError::DuplicateAction(node.id));
            }
            if node.state.disabled && !node.actions.is_empty() {
                return Err(SemanticTreeError::DisabledNodeHasActions(node.id));
            }
            for action in &node.actions {
                if !action_is_valid(node, *action) {
                    return Err(SemanticTreeError::InvalidAction {
                        node: node.id,
                        action: *action,
                    });
                }
            }
            if node.focused && !action_set.contains(&SemanticAction::Focus) {
                return Err(SemanticTreeError::FocusedNodeCannotFocus(node.id));
            }
            if (!node.actions.is_empty() || node.command.is_some() || node.focused)
                && node.name.trim().is_empty()
            {
                return Err(SemanticTreeError::UnnamedInteractiveNode(node.id));
            }
            for text in [
                Some(node.name.as_str()),
                node.description.as_deref(),
                node.command.as_deref(),
                node.value.as_deref(),
            ]
            .into_iter()
            .flatten()
            {
                text_bytes = text_bytes.saturating_add(text.len());
                if text.len() > MAX_TEXT_BYTES || text_bytes > MAX_TEXT_BYTES {
                    return Err(SemanticTreeError::TextTooLarge {
                        bytes: text_bytes,
                        maximum: MAX_TEXT_BYTES,
                    });
                }
            }
            if let Some(item) = node.collection_item {
                if item.position == 0 || item.set_size == 0 || item.position > item.set_size {
                    return Err(SemanticTreeError::InvalidCollectionItem(node.id));
                }
            }
            if node.focused && focused.replace(node.id).is_some() {
                return Err(SemanticTreeError::MultipleFocusedNodes);
            }
        }
        if self.focus != focused {
            return Err(SemanticTreeError::FocusMismatch {
                declared: self.focus,
                node: focused,
            });
        }
        Ok(())
    }

    /// Computes a bounded incremental change from a previously accepted tree.
    ///
    /// # Errors
    ///
    /// Rejects either malformed tree before constructing a delta.
    pub fn delta_from(&self, previous: Option<&Self>) -> Result<SemanticDelta, SemanticTreeError> {
        self.validate()?;
        let Some(previous) = previous else {
            return Ok(SemanticDelta::Replace(self.clone()));
        };
        previous.validate()?;
        if self == previous {
            return Ok(SemanticDelta::Unchanged);
        }
        let current_ids = self
            .nodes
            .iter()
            .map(|node| node.id)
            .collect::<BTreeSet<_>>();
        if self.nodes.iter().map(|node| node.id).ne(previous
            .nodes
            .iter()
            .filter(|node| current_ids.contains(&node.id))
            .map(|node| node.id))
        {
            return Ok(SemanticDelta::Replace(self.clone()));
        }
        let previous_nodes = previous
            .nodes
            .iter()
            .map(|node| (node.id, node))
            .collect::<BTreeMap<_, _>>();
        let updated = self
            .nodes
            .iter()
            .filter(|node| previous_nodes.get(&node.id).copied() != Some(*node))
            .cloned()
            .collect();
        let removed = previous
            .nodes
            .iter()
            .filter_map(|node| (!current_ids.contains(&node.id)).then_some(node.id))
            .collect();
        Ok(SemanticDelta::Update(SemanticTreeDelta {
            root: self.root,
            focus: self.focus,
            updated,
            removed,
        }))
    }
}

impl SemanticAction {
    const ALL: [Self; 10] = [
        Self::Activate,
        Self::Focus,
        Self::Expand,
        Self::Collapse,
        Self::Increment,
        Self::Decrement,
        Self::ReplaceSelectedText,
        Self::SetValue,
        Self::ScrollIntoView,
        Self::ShowContextMenu,
    ];
}

/// Atomic incremental semantic update.
#[derive(Clone, Debug, PartialEq)]
pub struct SemanticTreeDelta {
    pub root: Option<UiNodeId>,
    pub focus: Option<UiNodeId>,
    pub updated: Vec<SemanticNode>,
    pub removed: Vec<UiNodeId>,
}

/// Semantic output emitted at one immutable UI frame boundary.
#[derive(Clone, Debug, PartialEq)]
pub enum SemanticDelta {
    Unchanged,
    Replace(SemanticTree),
    Update(SemanticTreeDelta),
}

/// Typed semantic-tree rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticTreeError {
    TooManyNodes {
        count: usize,
        maximum: usize,
    },
    MissingRoot,
    InvalidRoot(UiNodeId),
    DuplicateNode(UiNodeId),
    MissingParent(UiNodeId),
    ParentAfterChild {
        node: UiNodeId,
        parent: UiNodeId,
    },
    InvalidBounds(UiNodeId),
    TooManyActions(UiNodeId),
    DuplicateAction(UiNodeId),
    DisabledNodeHasActions(UiNodeId),
    InvalidAction {
        node: UiNodeId,
        action: SemanticAction,
    },
    FocusedNodeCannotFocus(UiNodeId),
    UnnamedInteractiveNode(UiNodeId),
    TextTooLarge {
        bytes: usize,
        maximum: usize,
    },
    InvalidCollectionItem(UiNodeId),
    MultipleFocusedNodes,
    FocusMismatch {
        declared: Option<UiNodeId>,
        node: Option<UiNodeId>,
    },
}

impl Display for SemanticTreeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid semantic tree: {self:?}")
    }
}

impl Error for SemanticTreeError {}

fn action_is_valid(node: &SemanticNode, action: SemanticAction) -> bool {
    match action {
        SemanticAction::Focus | SemanticAction::ScrollIntoView => true,
        SemanticAction::Activate | SemanticAction::ShowContextMenu => node.command.is_some(),
        SemanticAction::Expand | SemanticAction::Collapse => {
            matches!(node.role, SemanticRole::TreeItem | SemanticRole::ComboBox)
        }
        SemanticAction::Increment | SemanticAction::Decrement => {
            matches!(node.role, SemanticRole::Splitter | SemanticRole::Timeline)
        }
        SemanticAction::ReplaceSelectedText | SemanticAction::SetValue => {
            matches!(node.role, SemanticRole::TextInput | SemanticRole::SearchBox)
        }
    }
}

fn validate_bounds(node: UiNodeId, bounds: UiRect) -> Result<(), SemanticTreeError> {
    let values = [
        bounds.origin.x,
        bounds.origin.y,
        bounds.size.width,
        bounds.size.height,
    ];
    if values.iter().all(|value| value.is_finite())
        && bounds.size.width >= 0.0
        && bounds.size.height >= 0.0
    {
        Ok(())
    } else {
        Err(SemanticTreeError::InvalidBounds(node))
    }
}

#[cfg(test)]
mod tests {
    use meridian_ui_core::{UiPoint, UiSize};

    use super::*;

    fn node(id: u128, parent: Option<u128>, focused: bool) -> SemanticNode {
        SemanticNode {
            id: UiNodeId::new(id),
            parent: parent.map(UiNodeId::new),
            role: if parent.is_some() {
                SemanticRole::Button
            } else {
                SemanticRole::Group
            },
            name: format!("node {id}"),
            description: None,
            command: parent.is_some().then(|| "fixture.activate".to_owned()),
            actions: parent
                .is_some()
                .then_some(vec![SemanticAction::Activate, SemanticAction::Focus])
                .unwrap_or_default(),
            value: None,
            state: UiControlState::default(),
            live: SemanticLive::Off,
            collection_item: None,
            bounds: UiRect::new(UiPoint::default(), UiSize::new(100.0, 44.0)),
            focused,
        }
    }

    #[test]
    fn semantic_tree_validates_reading_order_focus_and_actions() {
        let tree = SemanticTree {
            root: Some(UiNodeId::new(1)),
            focus: Some(UiNodeId::new(2)),
            nodes: vec![node(1, None, false), node(2, Some(1), true)],
        };
        tree.validate().expect("semantic fixture validates");

        let mut invalid = tree.clone();
        invalid.nodes.swap(0, 1);
        assert!(matches!(
            invalid.validate(),
            Err(SemanticTreeError::InvalidRoot(_))
        ));
        let mut duplicate = tree;
        duplicate.nodes[1].actions.push(SemanticAction::Activate);
        assert!(matches!(
            duplicate.validate(),
            Err(SemanticTreeError::DuplicateAction(_))
        ));
    }

    #[test]
    fn semantic_delta_updates_only_changed_identity_and_reports_removal() {
        let previous = SemanticTree {
            root: Some(UiNodeId::new(1)),
            focus: Some(UiNodeId::new(2)),
            nodes: vec![
                node(1, None, false),
                node(2, Some(1), true),
                node(3, Some(1), false),
            ],
        };
        let mut current = previous.clone();
        current.nodes[1].name = "renamed".to_owned();
        current.nodes.remove(2);
        let SemanticDelta::Update(delta) = current
            .delta_from(Some(&previous))
            .expect("valid trees produce a delta")
        else {
            panic!("changed tree must emit an incremental update");
        };
        assert_eq!(delta.updated.len(), 1);
        assert_eq!(delta.updated[0].id, UiNodeId::new(2));
        assert_eq!(delta.removed, vec![UiNodeId::new(3)]);
    }

    #[test]
    fn semantic_delta_replaces_the_tree_when_reading_order_changes() {
        let previous = SemanticTree {
            root: Some(UiNodeId::new(1)),
            focus: None,
            nodes: vec![
                node(1, None, false),
                node(2, Some(1), false),
                node(3, Some(1), false),
            ],
        };
        let mut current = previous.clone();
        current.nodes.swap(1, 2);
        assert_eq!(
            current
                .delta_from(Some(&previous))
                .expect("valid reordered tree produces a complete replacement"),
            SemanticDelta::Replace(current)
        );
    }

    #[test]
    fn semantic_tree_rejects_untrusted_role_actions_and_disabled_authority() {
        let mut invalid_role = SemanticTree {
            root: Some(UiNodeId::new(1)),
            focus: None,
            nodes: vec![node(1, None, false), node(2, Some(1), false)],
        };
        invalid_role.nodes[1].actions = vec![SemanticAction::SetValue];
        assert_eq!(
            invalid_role.validate(),
            Err(SemanticTreeError::InvalidAction {
                node: UiNodeId::new(2),
                action: SemanticAction::SetValue,
            })
        );

        invalid_role.nodes[1].actions = vec![SemanticAction::Activate];
        invalid_role.nodes[1].state.disabled = true;
        assert_eq!(
            invalid_role.validate(),
            Err(SemanticTreeError::DisabledNodeHasActions(UiNodeId::new(2)))
        );
    }
}
