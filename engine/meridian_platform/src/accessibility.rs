//! Private AccessKit projection behind Meridian-owned platform events.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use accesskit::{
    Action, ActionData, ActionRequest, Invalid, Live, Node, NodeId, Rect, Role, Tree, TreeId,
    TreeUpdate,
};
use meridian_ui_core::{SemanticRole, UiNodeId, MAX_RETAINED_NODES, MAX_TEXT_BYTES};
use meridian_ui_semantics::{SemanticAction, SemanticLive, SemanticTree, SemanticTreeError};

/// Meridian-owned assistive action payload.
#[derive(Clone, Debug, PartialEq)]
pub enum PlatformAccessibilityActionData {
    Text(String),
    Numeric(f64),
    Custom(i32),
}

/// Meridian-owned assistive action request delivered through [`crate::PlatformEvent`].
#[derive(Clone, Debug, PartialEq)]
pub struct PlatformAccessibilityActionRequest {
    pub target: UiNodeId,
    pub action: SemanticAction,
    pub data: Option<PlatformAccessibilityActionData>,
}

/// Typed private-adapter rejection without leaking AccessKit values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlatformAccessibilityError {
    InvalidSemanticTree(SemanticTreeError),
    TooManyAdapterIdentities { count: usize, maximum: usize },
    MissingRoot,
    MissingIdentity(UiNodeId),
    IdentityExhausted,
    CollectionIndexOverflow,
    UnknownPlatformNode,
    UnsupportedAction,
    ActionNotSupported(UiNodeId),
    UnexpectedActionData,
    ActionDataTooLarge { bytes: usize, maximum: usize },
    NonFiniteActionValue,
}

impl Display for PlatformAccessibilityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "platform accessibility adapter rejected input: {self:?}"
        )
    }
}

impl Error for PlatformAccessibilityError {}

#[derive(Debug, Default)]
pub(super) struct AccessKitBridge {
    forward: BTreeMap<UiNodeId, NodeId>,
    reverse: BTreeMap<NodeId, UiNodeId>,
    actions: BTreeMap<UiNodeId, BTreeSet<SemanticAction>>,
    next_node: u64,
}

impl AccessKitBridge {
    pub(super) fn project(
        &mut self,
        semantic: &SemanticTree,
    ) -> Result<TreeUpdate, PlatformAccessibilityError> {
        semantic
            .validate()
            .map_err(PlatformAccessibilityError::InvalidSemanticTree)?;
        self.synchronize_identities(semantic)?;
        let semantic_root = semantic
            .root
            .ok_or(PlatformAccessibilityError::MissingRoot)?;
        let root = self.platform_id(semantic_root)?;
        let focus = self.platform_id(semantic.focus.unwrap_or(semantic_root))?;
        let children = self.collect_children(semantic)?;
        let nodes = semantic
            .nodes
            .iter()
            .map(|node| self.project_node(node, &children))
            .collect::<Result<Vec<_>, PlatformAccessibilityError>>()?;
        let mut tree = Tree::new(root);
        tree.toolkit_name = Some("Meridian UI".to_owned());
        tree.toolkit_version = Some(env!("CARGO_PKG_VERSION").to_owned());
        Ok(TreeUpdate {
            nodes,
            tree: Some(tree),
            tree_id: TreeId::ROOT,
            focus,
        })
    }

    fn synchronize_identities(
        &mut self,
        semantic: &SemanticTree,
    ) -> Result<(), PlatformAccessibilityError> {
        let current = semantic
            .nodes
            .iter()
            .map(|node| node.id)
            .collect::<BTreeSet<_>>();
        self.forward.retain(|id, _| current.contains(id));
        self.reverse.retain(|_, id| current.contains(id));
        self.actions.retain(|id, _| current.contains(id));
        for node in &semantic.nodes {
            if !self.forward.contains_key(&node.id) {
                if self.forward.len() >= MAX_RETAINED_NODES {
                    return Err(PlatformAccessibilityError::TooManyAdapterIdentities {
                        count: self.forward.len().saturating_add(1),
                        maximum: MAX_RETAINED_NODES,
                    });
                }
                self.next_node = self
                    .next_node
                    .checked_add(1)
                    .ok_or(PlatformAccessibilityError::IdentityExhausted)?;
                let platform = NodeId(self.next_node);
                self.forward.insert(node.id, platform);
                self.reverse.insert(platform, node.id);
            }
            self.actions
                .insert(node.id, node.actions.iter().copied().collect());
        }
        Ok(())
    }

    fn collect_children(
        &self,
        semantic: &SemanticTree,
    ) -> Result<BTreeMap<UiNodeId, Vec<NodeId>>, PlatformAccessibilityError> {
        let mut children = BTreeMap::<UiNodeId, Vec<NodeId>>::new();
        for node in &semantic.nodes {
            if let Some(parent) = node.parent {
                children
                    .entry(parent)
                    .or_default()
                    .push(self.platform_id(node.id)?);
            }
        }
        Ok(children)
    }

    fn project_node(
        &self,
        semantic_node: &meridian_ui_semantics::SemanticNode,
        children: &BTreeMap<UiNodeId, Vec<NodeId>>,
    ) -> Result<(NodeId, Node), PlatformAccessibilityError> {
        let mut node = Node::new(map_role(semantic_node.role));
        node.set_label(semantic_node.name.clone());
        if let Some(description) = &semantic_node.description {
            node.set_description(description.clone());
        }
        if let Some(value) = &semantic_node.value {
            node.set_value(value.clone());
        }
        if let Some(children) = children.get(&semantic_node.id) {
            node.set_children(children.clone());
        }
        project_state(&mut node, semantic_node)?;
        let bounds = semantic_node.bounds;
        node.set_bounds(Rect::new(
            f64::from(bounds.origin.x),
            f64::from(bounds.origin.y),
            f64::from(bounds.origin.x + bounds.size.width),
            f64::from(bounds.origin.y + bounds.size.height),
        ));
        for action in &semantic_node.actions {
            node.add_action(map_action(*action));
        }
        Ok((self.platform_id(semantic_node.id)?, node))
    }

    pub(super) fn translate_action(
        &self,
        request: ActionRequest,
    ) -> Result<PlatformAccessibilityActionRequest, PlatformAccessibilityError> {
        let target = self
            .reverse
            .get(&request.target_node)
            .copied()
            .ok_or(PlatformAccessibilityError::UnknownPlatformNode)?;
        if request.target_tree != TreeId::ROOT {
            return Err(PlatformAccessibilityError::UnknownPlatformNode);
        }
        let action = unmap_action(request.action)?;
        if !self
            .actions
            .get(&target)
            .is_some_and(|actions| actions.contains(&action))
        {
            return Err(PlatformAccessibilityError::ActionNotSupported(target));
        }
        let data = match request.data {
            Some(ActionData::Value(value)) => {
                if value.len() > MAX_TEXT_BYTES {
                    return Err(PlatformAccessibilityError::ActionDataTooLarge {
                        bytes: value.len(),
                        maximum: MAX_TEXT_BYTES,
                    });
                }
                Some(PlatformAccessibilityActionData::Text(value.into()))
            }
            Some(ActionData::NumericValue(value)) => {
                if !value.is_finite() {
                    return Err(PlatformAccessibilityError::NonFiniteActionValue);
                }
                Some(PlatformAccessibilityActionData::Numeric(value))
            }
            Some(ActionData::CustomAction(value)) => {
                Some(PlatformAccessibilityActionData::Custom(value))
            }
            None => None,
            Some(_) => return Err(PlatformAccessibilityError::UnsupportedAction),
        };
        let data_is_valid = matches!(
            (action, &data),
            (
                SemanticAction::SetValue,
                None | Some(
                    PlatformAccessibilityActionData::Text(_)
                        | PlatformAccessibilityActionData::Numeric(_)
                )
            ) | (
                SemanticAction::ReplaceSelectedText,
                Some(PlatformAccessibilityActionData::Text(_))
            ) | (
                SemanticAction::Activate
                    | SemanticAction::Focus
                    | SemanticAction::Expand
                    | SemanticAction::Collapse
                    | SemanticAction::Increment
                    | SemanticAction::Decrement
                    | SemanticAction::ScrollIntoView
                    | SemanticAction::ShowContextMenu,
                None
            )
        );
        if !data_is_valid {
            return Err(PlatformAccessibilityError::UnexpectedActionData);
        }
        Ok(PlatformAccessibilityActionRequest {
            target,
            action,
            data,
        })
    }

    fn platform_id(&self, node: UiNodeId) -> Result<NodeId, PlatformAccessibilityError> {
        self.forward
            .get(&node)
            .copied()
            .ok_or(PlatformAccessibilityError::MissingIdentity(node))
    }
}

fn project_state(
    node: &mut Node,
    semantic_node: &meridian_ui_semantics::SemanticNode,
) -> Result<(), PlatformAccessibilityError> {
    if semantic_node.state.disabled {
        node.set_disabled();
    }
    if semantic_node.state.selected {
        node.set_selected(true);
    }
    if matches!(
        semantic_node.role,
        SemanticRole::TreeItem | SemanticRole::ComboBox
    ) {
        node.set_expanded(semantic_node.state.expanded);
    }
    if semantic_node.state.invalid {
        node.set_invalid(Invalid::True);
    }
    match semantic_node.live {
        SemanticLive::Off => {}
        SemanticLive::Polite => node.set_live(Live::Polite),
        SemanticLive::Assertive => node.set_live(Live::Assertive),
    }
    if let Some(item) = semantic_node.collection_item {
        node.set_position_in_set(
            usize::try_from(item.position)
                .map_err(|_| PlatformAccessibilityError::CollectionIndexOverflow)?,
        );
        node.set_size_of_set(
            usize::try_from(item.set_size)
                .map_err(|_| PlatformAccessibilityError::CollectionIndexOverflow)?,
        );
    }
    Ok(())
}

fn map_role(role: SemanticRole) -> Role {
    match role {
        SemanticRole::Group => Role::Group,
        SemanticRole::Status | SemanticRole::LiveRegion => Role::Status,
        SemanticRole::Button => Role::Button,
        SemanticRole::ToggleButton => Role::Switch,
        SemanticRole::ProgressIndicator => Role::ProgressIndicator,
        SemanticRole::TextInput => Role::TextInput,
        SemanticRole::SearchBox => Role::SearchInput,
        SemanticRole::ComboBox => Role::ComboBox,
        SemanticRole::Option => Role::ListBoxOption,
        SemanticRole::MenuBar => Role::MenuBar,
        SemanticRole::Menu => Role::Menu,
        SemanticRole::MenuItem => Role::MenuItem,
        SemanticRole::Tooltip => Role::Tooltip,
        SemanticRole::TabList => Role::TabList,
        SemanticRole::Tab => Role::Tab,
        SemanticRole::Tree => Role::Tree,
        SemanticRole::TreeItem => Role::TreeItem,
        SemanticRole::Table => Role::Table,
        SemanticRole::Row => Role::Row,
        SemanticRole::Cell => Role::Cell,
        SemanticRole::PropertyGrid => Role::Grid,
        SemanticRole::List => Role::List,
        SemanticRole::ListItem => Role::ListItem,
        SemanticRole::Timeline => Role::Slider,
        SemanticRole::Splitter => Role::Splitter,
        SemanticRole::Dialog => Role::Dialog,
        SemanticRole::Graph => Role::GraphicsDocument,
        SemanticRole::Canvas => Role::Canvas,
    }
}

fn map_action(action: SemanticAction) -> Action {
    match action {
        SemanticAction::Activate => Action::Click,
        SemanticAction::Focus => Action::Focus,
        SemanticAction::Expand => Action::Expand,
        SemanticAction::Collapse => Action::Collapse,
        SemanticAction::Increment => Action::Increment,
        SemanticAction::Decrement => Action::Decrement,
        SemanticAction::ReplaceSelectedText => Action::ReplaceSelectedText,
        SemanticAction::SetValue => Action::SetValue,
        SemanticAction::ScrollIntoView => Action::ScrollIntoView,
        SemanticAction::ShowContextMenu => Action::ShowContextMenu,
    }
}

fn unmap_action(action: Action) -> Result<SemanticAction, PlatformAccessibilityError> {
    match action {
        Action::Click => Ok(SemanticAction::Activate),
        Action::Focus => Ok(SemanticAction::Focus),
        Action::Expand => Ok(SemanticAction::Expand),
        Action::Collapse => Ok(SemanticAction::Collapse),
        Action::Increment => Ok(SemanticAction::Increment),
        Action::Decrement => Ok(SemanticAction::Decrement),
        Action::ReplaceSelectedText => Ok(SemanticAction::ReplaceSelectedText),
        Action::SetValue => Ok(SemanticAction::SetValue),
        Action::ScrollIntoView => Ok(SemanticAction::ScrollIntoView),
        Action::ShowContextMenu => Ok(SemanticAction::ShowContextMenu),
        _ => Err(PlatformAccessibilityError::UnsupportedAction),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use accesskit::{ActionRequest, ActivationHandler};
    use meridian_ui_core::{UiControlState, UiPoint, UiRect, UiSize};
    use meridian_ui_semantics::{SemanticNode, SemanticTree};

    use super::*;

    fn tree() -> SemanticTree {
        SemanticTree {
            root: Some(UiNodeId::new(1)),
            focus: Some(UiNodeId::new(2)),
            nodes: vec![
                SemanticNode {
                    id: UiNodeId::new(1),
                    parent: None,
                    role: SemanticRole::Group,
                    name: "Meridian".to_owned(),
                    description: None,
                    command: None,
                    actions: Vec::new(),
                    value: None,
                    state: UiControlState::default(),
                    live: SemanticLive::Off,
                    collection_item: None,
                    bounds: UiRect::new(UiPoint::default(), UiSize::new(800.0, 600.0)),
                    focused: false,
                },
                SemanticNode {
                    id: UiNodeId::new(2),
                    parent: Some(UiNodeId::new(1)),
                    role: SemanticRole::Button,
                    name: "Build".to_owned(),
                    description: Some("Build the active project".to_owned()),
                    command: Some("build.start".to_owned()),
                    actions: vec![SemanticAction::Focus, SemanticAction::Activate],
                    value: None,
                    state: UiControlState::default(),
                    live: SemanticLive::Off,
                    collection_item: None,
                    bounds: UiRect::new(UiPoint::default(), UiSize::new(80.0, 44.0)),
                    focused: true,
                },
            ],
        }
    }

    #[test]
    fn projection_preserves_stable_identity_focus_children_and_actions() {
        let mut bridge = AccessKitBridge::default();
        let first = bridge.project(&tree()).expect("semantic tree projects");
        let second = bridge.project(&tree()).expect("same tree reprojects");
        assert_eq!(first.focus, second.focus);
        assert_eq!(first.nodes[1].0, second.nodes[1].0);
        assert_eq!(first.nodes[0].1.children(), &[first.nodes[1].0]);
        assert!(first.nodes[1].1.supports_action(Action::Click));

        let request = ActionRequest {
            action: Action::Click,
            target_tree: TreeId::ROOT,
            target_node: first.nodes[1].0,
            data: None,
        };
        assert_eq!(
            bridge.translate_action(request).expect("action maps"),
            PlatformAccessibilityActionRequest {
                target: UiNodeId::new(2),
                action: SemanticAction::Activate,
                data: None,
            }
        );
    }

    #[test]
    fn reactivation_handler_returns_the_latest_complete_tree() {
        let mut bridge = AccessKitBridge::default();
        let first = bridge.project(&tree()).expect("first tree projects");
        let cache = Arc::new(Mutex::new(first));
        let mut activation = crate::InitialAccessibilityTree(Arc::clone(&cache));

        let mut updated_tree = tree();
        updated_tree.nodes[1].name = "Build project".to_owned();
        let update = bridge
            .project(&updated_tree)
            .expect("updated tree projects");
        *cache.lock().expect("test cache available") = update;

        let recovered = activation
            .request_initial_tree()
            .expect("reactivation receives full tree");
        assert_eq!(recovered.nodes[1].1.label(), Some("Build project"));
    }
}
