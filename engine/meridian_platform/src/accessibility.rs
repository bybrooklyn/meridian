//! Private AccessKit projection behind Meridian-owned platform events.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use accesskit::{
    Action, ActionData, ActionRequest, Invalid, Live, Node, NodeId, Rect, Role, Tree, TreeId,
    TreeUpdate,
};
use meridian_ui_core::{SemanticRole, UiNodeId, MAX_RETAINED_NODES, MAX_TEXT_BYTES};
use meridian_ui_semantics::{
    SemanticAction, SemanticLive, SemanticRelationships, SemanticTree, SemanticTreeError,
};

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

#[derive(Clone, Debug, Default)]
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
        let semantic_root = semantic
            .root
            .ok_or(PlatformAccessibilityError::MissingRoot)?;
        // Project against a staged bridge so an unavailable or malformed
        // platform update cannot discard the last complete assistive tree.
        // Re-activation must continue to return that accepted tree until a
        // replacement snapshot has projected successfully.
        let mut staged = self.clone();
        staged.synchronize_identities(semantic)?;
        let root = staged.platform_id(semantic_root)?;
        let focus = staged.platform_id(semantic.focus.unwrap_or(semantic_root))?;
        let children = staged.collect_children(semantic)?;
        let nodes = semantic
            .nodes
            .iter()
            .map(|node| staged.project_node(node, &children))
            .collect::<Result<Vec<_>, PlatformAccessibilityError>>()?;
        let mut tree = Tree::new(root);
        tree.toolkit_name = Some("Meridian UI".to_owned());
        tree.toolkit_version = Some(env!("CARGO_PKG_VERSION").to_owned());
        let update = TreeUpdate {
            nodes,
            tree: Some(tree),
            tree_id: TreeId::ROOT,
            focus,
        };
        *self = staged;
        Ok(update)
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
        self.project_relationships(&mut node, &semantic_node.relationships)?;
        project_state(&mut node, semantic_node)?;
        let bounds = semantic_node.bounds;
        let x0 = f64::from(bounds.origin.x);
        let y0 = f64::from(bounds.origin.y);
        node.set_bounds(Rect::new(
            x0,
            y0,
            x0 + f64::from(bounds.size.width),
            y0 + f64::from(bounds.size.height),
        ));
        for action in &semantic_node.actions {
            node.add_action(map_action(*action));
        }
        Ok((self.platform_id(semantic_node.id)?, node))
    }

    fn project_relationships(
        &self,
        node: &mut Node,
        relationships: &SemanticRelationships,
    ) -> Result<(), PlatformAccessibilityError> {
        if !relationships.labelled_by.is_empty() {
            node.set_labelled_by(self.platform_ids(&relationships.labelled_by)?);
        }
        if !relationships.described_by.is_empty() {
            node.set_described_by(self.platform_ids(&relationships.described_by)?);
        }
        if !relationships.controls.is_empty() {
            node.set_controls(self.platform_ids(&relationships.controls)?);
        }
        if !relationships.details.is_empty() {
            node.set_details(self.platform_ids(&relationships.details)?);
        }
        if !relationships.flow_to.is_empty() {
            node.set_flow_to(self.platform_ids(&relationships.flow_to)?);
        }
        if let Some(error_message) = relationships.error_message {
            node.set_error_message(self.platform_id(error_message)?);
        }
        Ok(())
    }

    fn platform_ids(&self, nodes: &[UiNodeId]) -> Result<Vec<NodeId>, PlatformAccessibilityError> {
        nodes.iter().map(|node| self.platform_id(*node)).collect()
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
    match semantic_node.role {
        // Meridian represents the logical on/off state as `selected`; the
        // private adapter must express it as a native switch toggle rather
        // than a list/tab selection state.
        SemanticRole::ToggleButton => node.set_toggled(semantic_node.state.selected.into()),
        SemanticRole::Tab | SemanticRole::Option | SemanticRole::TreeItem
            if semantic_node.state.selected =>
        {
            node.set_selected(true);
        }
        _ => {}
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

    use accesskit::{ActionData, ActionRequest, ActivationHandler, Uuid};
    use meridian_ui_core::{UiControlState, UiPoint, UiRect, UiSize};
    use meridian_ui_semantics::{
        SemanticCollectionItem, SemanticNode, SemanticRelationships, SemanticTree,
    };

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
                    relationships: SemanticRelationships::default(),
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
                    relationships: SemanticRelationships::default(),
                    live: SemanticLive::Off,
                    collection_item: None,
                    bounds: UiRect::new(UiPoint::default(), UiSize::new(80.0, 44.0)),
                    focused: true,
                },
            ],
        }
    }

    fn semantic_node(
        id: UiNodeId,
        parent: Option<UiNodeId>,
        role: SemanticRole,
        name: &str,
        bounds: UiRect,
    ) -> SemanticNode {
        SemanticNode {
            id,
            parent,
            role,
            name: name.to_owned(),
            description: None,
            command: None,
            actions: Vec::new(),
            value: None,
            state: UiControlState::default(),
            relationships: SemanticRelationships::default(),
            live: SemanticLive::Off,
            collection_item: None,
            bounds,
            focused: false,
        }
    }

    fn rich_tree() -> SemanticTree {
        let root = UiNodeId::new(10);
        let text = UiNodeId::new(11);
        let alert = UiNodeId::new(12);
        let option = UiNodeId::new(13);
        let mut root_node = semantic_node(
            root,
            None,
            SemanticRole::Dialog,
            "Recovery",
            UiRect::new(UiPoint::default(), UiSize::new(640.0, 480.0)),
        );
        root_node.description = Some("Restore latest Meridian UI snapshot".to_owned());

        let mut text_node = semantic_node(
            text,
            Some(root),
            SemanticRole::TextInput,
            "Project name",
            UiRect::new(UiPoint { x: 16.0, y: 20.0 }, UiSize::new(240.0, 32.0)),
        );
        text_node.description = Some("Shown in the window title".to_owned());
        text_node.command = Some("project.rename".to_owned());
        text_node.actions = vec![
            SemanticAction::Focus,
            SemanticAction::SetValue,
            SemanticAction::ReplaceSelectedText,
        ];
        text_node.value = Some("Creator Alpha".to_owned());
        text_node.state = UiControlState {
            invalid: true,
            ..UiControlState::default()
        };
        text_node.relationships = SemanticRelationships {
            labelled_by: vec![root],
            described_by: vec![alert],
            controls: vec![option],
            details: vec![alert],
            flow_to: vec![option],
            error_message: Some(alert),
        };
        text_node.focused = true;

        let mut alert_node = semantic_node(
            alert,
            Some(root),
            SemanticRole::LiveRegion,
            "Build failed",
            UiRect::new(UiPoint { x: 16.0, y: 64.0 }, UiSize::new(280.0, 28.0)),
        );
        alert_node.description = Some("One recoverable diagnostic".to_owned());
        alert_node.value = Some("1 diagnostic".to_owned());
        alert_node.live = SemanticLive::Assertive;

        let mut option_node = semantic_node(
            option,
            Some(root),
            SemanticRole::TreeItem,
            "World",
            UiRect::new(UiPoint { x: 16.0, y: 100.0 }, UiSize::new(120.0, 24.0)),
        );
        option_node.actions = vec![SemanticAction::Focus, SemanticAction::Collapse];
        option_node.state = UiControlState {
            expanded: true,
            selected: true,
            ..UiControlState::default()
        };
        option_node.collection_item = Some(SemanticCollectionItem {
            position: 2,
            set_size: 5,
        });

        SemanticTree {
            root: Some(root),
            focus: Some(text),
            nodes: vec![root_node, text_node, alert_node, option_node],
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
    fn context_menu_projects_and_routes_without_a_primary_command() {
        let target = UiNodeId::new(0x51);
        let mut context_region = semantic_node(
            target,
            None,
            SemanticRole::Canvas,
            "World viewport",
            UiRect::new(UiPoint::default(), UiSize::new(800.0, 600.0)),
        );
        context_region.focused = true;
        context_region.actions = vec![SemanticAction::Focus, SemanticAction::ShowContextMenu];
        let semantic = SemanticTree {
            root: Some(target),
            focus: Some(target),
            nodes: vec![context_region],
        };

        let mut bridge = AccessKitBridge::default();
        let update = bridge
            .project(&semantic)
            .expect("context-only tree projects");
        assert!(update.nodes[0].1.supports_action(Action::ShowContextMenu));
        assert!(!update.nodes[0].1.supports_action(Action::Click));
        assert_eq!(
            bridge.translate_action(ActionRequest {
                action: Action::ShowContextMenu,
                target_tree: TreeId::ROOT,
                target_node: update.nodes[0].0,
                data: None,
            }),
            Ok(PlatformAccessibilityActionRequest {
                target,
                action: SemanticAction::ShowContextMenu,
                data: None,
            })
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

    #[test]
    fn failed_empty_projection_keeps_the_last_accepted_identity_and_action() {
        let mut bridge = AccessKitBridge::default();
        let first = bridge
            .project(&rich_tree())
            .expect("initial semantic tree projects");
        let focused_platform = first.focus;

        assert_eq!(
            bridge.project(&SemanticTree::default()),
            Err(PlatformAccessibilityError::MissingRoot)
        );
        assert_eq!(
            bridge.translate_action(ActionRequest {
                action: Action::SetValue,
                target_tree: TreeId::ROOT,
                target_node: focused_platform,
                data: Some(ActionData::Value("Retained after failure".into())),
            }),
            Ok(PlatformAccessibilityActionRequest {
                target: UiNodeId::new(11),
                action: SemanticAction::SetValue,
                data: Some(PlatformAccessibilityActionData::Text(
                    "Retained after failure".to_owned(),
                )),
            })
        );

        let recovered = bridge
            .project(&rich_tree())
            .expect("recovery reuses the accepted semantic identity");
        assert_eq!(recovered.focus, focused_platform);
        assert_eq!(recovered.nodes[1].0, focused_platform);
    }

    #[test]
    fn projection_uses_wide_arithmetic_for_large_finite_bounds() {
        let mut semantic = tree();
        semantic.nodes[0].bounds = UiRect::new(
            UiPoint {
                x: f32::MAX,
                y: -f32::MAX,
            },
            UiSize::new(f32::MAX, f32::MAX),
        );

        let mut bridge = AccessKitBridge::default();
        let update = bridge
            .project(&semantic)
            .expect("finite semantic bounds project without an f32 overflow");
        let bounds = update.nodes[0].1.bounds().expect("projected bounds exist");
        assert_eq!(bounds.x0.to_bits(), f64::from(f32::MAX).to_bits());
        assert_eq!(bounds.x1.to_bits(), (f64::from(f32::MAX) * 2.0).to_bits());
        assert!(bounds.x1.is_finite());
        assert!(bounds.y1.abs() <= f64::EPSILON);
    }

    #[test]
    fn projection_maps_names_actions_values_live_regions_focus_order_and_state() {
        let mut bridge = AccessKitBridge::default();
        let update = bridge
            .project(&rich_tree())
            .expect("rich semantic tree projects");
        let root_node = &update.nodes[0].1;
        let text_node = &update.nodes[1].1;
        let alert_node = &update.nodes[2].1;
        let option_node = &update.nodes[3].1;

        assert_eq!(update.focus, update.nodes[1].0);
        assert_eq!(
            root_node.children(),
            &[update.nodes[1].0, update.nodes[2].0, update.nodes[3].0]
        );
        assert_eq!(root_node.role(), Role::Dialog);
        assert_eq!(root_node.label(), Some("Recovery"));
        assert_eq!(
            root_node.description(),
            Some("Restore latest Meridian UI snapshot")
        );
        assert_eq!(text_node.role(), Role::TextInput);
        assert_eq!(text_node.label(), Some("Project name"));
        assert_eq!(text_node.description(), Some("Shown in the window title"));
        assert_eq!(text_node.value(), Some("Creator Alpha"));
        assert_eq!(text_node.invalid(), Some(Invalid::True));
        assert_eq!(text_node.labelled_by(), &[update.nodes[0].0]);
        assert_eq!(text_node.described_by(), &[update.nodes[2].0]);
        assert_eq!(text_node.controls(), &[update.nodes[3].0]);
        assert_eq!(text_node.details(), &[update.nodes[2].0]);
        assert_eq!(text_node.flow_to(), &[update.nodes[3].0]);
        assert_eq!(text_node.error_message(), Some(update.nodes[2].0));
        assert!(text_node.supports_action(Action::SetValue));
        assert!(text_node.supports_action(Action::ReplaceSelectedText));
        assert_eq!(alert_node.role(), Role::Status);
        assert_eq!(alert_node.live(), Some(Live::Assertive));
        assert_eq!(alert_node.value(), Some("1 diagnostic"));
        assert_eq!(option_node.role(), Role::TreeItem);
        assert_eq!(option_node.is_selected(), Some(true));
        assert_eq!(option_node.is_expanded(), Some(true));
        assert!(option_node.supports_action(Action::Collapse));
        assert!(!option_node.supports_action(Action::Expand));
        assert_eq!(option_node.position_in_set(), Some(2));
        assert_eq!(option_node.size_of_set(), Some(5));
    }

    #[test]
    fn switch_projection_uses_toggled_while_selection_roles_remain_selected() {
        let root = UiNodeId::new(0x60);
        let toggle = UiNodeId::new(0x61);
        let tab = UiNodeId::new(0x62);
        let mut toggle_node = semantic_node(
            toggle,
            Some(root),
            SemanticRole::ToggleButton,
            "Snap to grid",
            UiRect::new(UiPoint::default(), UiSize::new(120.0, 32.0)),
        );
        toggle_node.state.selected = true;
        toggle_node.focused = true;
        toggle_node.actions.push(SemanticAction::Focus);
        let mut tab_node = semantic_node(
            tab,
            Some(root),
            SemanticRole::Tab,
            "World",
            UiRect::new(UiPoint { x: 128.0, y: 0.0 }, UiSize::new(80.0, 32.0)),
        );
        tab_node.state.selected = true;
        let semantic = SemanticTree {
            root: Some(root),
            focus: Some(toggle),
            nodes: vec![
                semantic_node(
                    root,
                    None,
                    SemanticRole::Group,
                    "Meridian",
                    UiRect::new(UiPoint::default(), UiSize::new(640.0, 480.0)),
                ),
                toggle_node,
                tab_node,
            ],
        };

        let mut bridge = AccessKitBridge::default();
        let update = bridge.project(&semantic).expect("switch tree projects");
        let toggle = &update.nodes[1].1;
        let tab = &update.nodes[2].1;
        assert_eq!(toggle.role(), Role::Switch);
        assert_eq!(toggle.toggled(), Some(accesskit::Toggled::True));
        assert_eq!(toggle.is_selected(), None);
        assert_eq!(tab.role(), Role::Tab);
        assert_eq!(tab.is_selected(), Some(true));
    }

    #[test]
    fn rejected_stale_unknown_and_malformed_actions_preserve_projected_focus() {
        let mut bridge = AccessKitBridge::default();
        let projected = bridge.project(&tree()).expect("semantic tree projects");
        let focused_platform = projected.focus;
        let focused_meridian = UiNodeId::new(2);

        assert_eq!(
            bridge.translate_action(ActionRequest {
                action: Action::Click,
                target_tree: TreeId(Uuid::from_u128(1)),
                target_node: focused_platform,
                data: None,
            }),
            Err(PlatformAccessibilityError::UnknownPlatformNode)
        );
        assert_eq!(
            bridge.translate_action(ActionRequest {
                action: Action::Click,
                target_tree: TreeId::ROOT,
                target_node: NodeId(999_999),
                data: None,
            }),
            Err(PlatformAccessibilityError::UnknownPlatformNode)
        );
        assert_eq!(
            bridge.translate_action(ActionRequest {
                action: Action::SetValue,
                target_tree: TreeId::ROOT,
                target_node: focused_platform,
                data: Some(ActionData::CustomAction(7)),
            }),
            Err(PlatformAccessibilityError::ActionNotSupported(
                focused_meridian
            ))
        );

        let after_rejection = bridge
            .project(&tree())
            .expect("tree reprojects after rejection");
        assert_eq!(after_rejection.focus, focused_platform);
        assert_eq!(after_rejection.nodes[1].0, focused_platform);
        assert_eq!(
            bridge.translate_action(ActionRequest {
                action: Action::Click,
                target_tree: TreeId::ROOT,
                target_node: focused_platform,
                data: None,
            }),
            Ok(PlatformAccessibilityActionRequest {
                target: focused_meridian,
                action: SemanticAction::Activate,
                data: None,
            })
        );
    }

    #[test]
    fn recovery_reprojection_removes_stale_adapter_actions_and_keeps_current_focus() {
        let mut bridge = AccessKitBridge::default();
        let first = bridge.project(&tree()).expect("first tree projects");
        let stale_button = first.nodes[1].0;
        let root = UiNodeId::new(1);
        let retry = UiNodeId::new(3);
        let recovered_tree = SemanticTree {
            root: Some(root),
            focus: Some(retry),
            nodes: vec![
                SemanticNode {
                    id: root,
                    parent: None,
                    role: SemanticRole::Group,
                    name: "Meridian".to_owned(),
                    description: None,
                    command: None,
                    actions: Vec::new(),
                    value: None,
                    state: UiControlState::default(),
                    relationships: SemanticRelationships::default(),
                    live: SemanticLive::Off,
                    collection_item: None,
                    bounds: UiRect::new(UiPoint::default(), UiSize::new(800.0, 600.0)),
                    focused: false,
                },
                SemanticNode {
                    id: retry,
                    parent: Some(root),
                    role: SemanticRole::Button,
                    name: "Retry recovery".to_owned(),
                    description: Some("Rebuild the UI snapshot".to_owned()),
                    command: Some("recovery.retry".to_owned()),
                    actions: vec![SemanticAction::Focus, SemanticAction::Activate],
                    value: None,
                    state: UiControlState::default(),
                    relationships: SemanticRelationships::default(),
                    live: SemanticLive::Polite,
                    collection_item: None,
                    bounds: UiRect::new(UiPoint { x: 24.0, y: 24.0 }, UiSize::new(160.0, 44.0)),
                    focused: true,
                },
            ],
        };

        let recovered = bridge
            .project(&recovered_tree)
            .expect("recovery tree projects");
        assert_eq!(recovered.focus, recovered.nodes[1].0);
        assert_ne!(recovered.focus, stale_button);
        assert_eq!(
            bridge.translate_action(ActionRequest {
                action: Action::Click,
                target_tree: TreeId::ROOT,
                target_node: stale_button,
                data: None,
            }),
            Err(PlatformAccessibilityError::UnknownPlatformNode)
        );
        assert_eq!(
            bridge.translate_action(ActionRequest {
                action: Action::Click,
                target_tree: TreeId::ROOT,
                target_node: recovered.focus,
                data: None,
            }),
            Ok(PlatformAccessibilityActionRequest {
                target: retry,
                action: SemanticAction::Activate,
                data: None,
            })
        );
    }

    #[test]
    fn native_smoke_contract_routes_action_and_reactivates_with_latest_tree() {
        let mut bridge = AccessKitBridge::default();
        let initial = bridge
            .project(&rich_tree())
            .expect("native smoke tree projects");
        let focused_platform = initial.focus;
        let cache = Arc::new(Mutex::new(initial));
        let mut activation = crate::InitialAccessibilityTree(Arc::clone(&cache));

        let action = bridge
            .translate_action(ActionRequest {
                action: Action::SetValue,
                target_tree: TreeId::ROOT,
                target_node: focused_platform,
                data: Some(ActionData::Value("Native Accessibility Fixture".into())),
            })
            .expect("assistive action routes through the Meridian bridge");
        assert_eq!(
            action,
            PlatformAccessibilityActionRequest {
                target: UiNodeId::new(11),
                action: SemanticAction::SetValue,
                data: Some(PlatformAccessibilityActionData::Text(
                    "Native Accessibility Fixture".to_owned(),
                )),
            }
        );

        let mut refreshed_tree = rich_tree();
        refreshed_tree.nodes[1].value = Some("Native Accessibility Fixture".to_owned());
        refreshed_tree.nodes[2].value = Some("Adapter cache refreshed".to_owned());
        let refreshed = bridge
            .project(&refreshed_tree)
            .expect("refreshed native smoke tree projects");
        *cache.lock().expect("native smoke cache available") = refreshed;

        let reactivated = activation
            .request_initial_tree()
            .expect("adapter reactivation receives the latest complete tree");
        assert_eq!(reactivated.focus, focused_platform);
        assert_eq!(
            reactivated.nodes[1].1.value(),
            Some("Native Accessibility Fixture")
        );
        assert_eq!(
            reactivated.nodes[2].1.value(),
            Some("Adapter cache refreshed")
        );
    }
}
