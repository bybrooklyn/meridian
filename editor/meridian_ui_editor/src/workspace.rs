//! Transactional editor docking and versioned workspace-state persistence.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};

use meridian_save::{SaveConfig, SaveError, SaveStore};
use meridian_ui::{MAX_RETAINED_NODES, MAX_TEXT_BYTES};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::EditorPanelId;

pub const WORKSPACE_STATE_SCHEMA: &str = "meridian.ui-workspace-state/v1";
const LEGACY_WORKSPACE_STATE_SCHEMA: &str = "meridian.ui-workspace-state/v0";
pub const WORKSPACE_STATE_VERSION: u16 = 1;
pub const DOCK_RATIO_PER_MILLE: u16 = 1_000;
pub const DOCK_GUTTER: u32 = 8;
pub const TIGHT_DOCK_GUTTER: u32 = DOCK_GUTTER / 2;
pub const MIN_ACCESSIBLE_PANEL_EXTENT: u32 = 44;

const DOCUMENT_FIELDS: &[&str] = &[
    "schema",
    "version",
    "revision",
    "session",
    "active_workspace",
    "active_layout_name",
    "layouts",
];
const LAYOUT_FIELDS: &[&str] = &[
    "name",
    "workspace",
    "dock",
    "selected",
    "active_document",
    "camera",
    "browser_query",
    "expanded",
    "scroll",
    "focused_panel",
    "focus_layout",
    "companions",
];

/// Opaque, bounded forward-compatible fields retained from workspace JSON.
///
/// Values deliberately remain private so `serde_json::Value` never becomes a
/// stable Meridian API. Callers can observe retained keys but cannot use this
/// compatibility sidecar as an untyped authority channel.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct WorkspaceExtensions(BTreeMap<String, serde_json::Value>);

impl WorkspaceExtensions {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn contains_key(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    fn validate(&self, reserved: &[&str]) -> Result<(), WorkspaceStateError> {
        if self.len() > MAX_RETAINED_NODES {
            return Err(WorkspaceStateError::TooManyExtensions {
                count: self.len(),
                maximum: MAX_RETAINED_NODES,
            });
        }
        for key in self.0.keys() {
            validate_bounded_text(key, WorkspaceTextField::ExtensionKey)?;
            if key.is_empty() {
                return Err(WorkspaceStateError::EmptyExtensionKey);
            }
            if reserved.contains(&key.as_str()) {
                return Err(WorkspaceStateError::ReservedExtensionKey(key.clone()));
            }
        }
        Ok(())
    }

    fn merge_from(&mut self, newer: Self) {
        self.0.extend(newer.0);
    }
}

macro_rules! workspace_id {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(u128);

        impl $name {
            #[must_use]
            pub const fn new(value: u128) -> Self {
                Self(value)
            }

            #[must_use]
            pub const fn value(self) -> u128 {
                self.0
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(&format!("{:032x}", self.0))
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                if value.len() != 32 {
                    return Err(serde::de::Error::custom(
                        "stable workspace identity must contain 32 hexadecimal digits",
                    ));
                }
                u128::from_str_radix(&value, 16)
                    .map(Self)
                    .map_err(serde::de::Error::custom)
            }
        }
    };
}

workspace_id!(PanelId, "Stable serialized identity for one editor panel.");
workspace_id!(DockNodeId, "Stable serialized identity for one dock node.");
workspace_id!(
    CompanionWindowId,
    "Stable identity for a companion window descriptor."
);
workspace_id!(
    WorkspaceSessionId,
    "Shared editor-session identity used by companion windows."
);
workspace_id!(MonitorId, "Stable host-provided identity for one monitor.");
workspace_id!(
    WorkspaceObjectId,
    "Stable identity for remembered editor context."
);

impl From<EditorPanelId> for PanelId {
    fn from(panel: EditorPanelId) -> Self {
        Self::new(match panel {
            EditorPanelId::ProjectRecovery => 1,
            EditorPanelId::Viewport => 2,
            EditorPanelId::Hierarchy => 3,
            EditorPanelId::Inspector => 4,
            EditorPanelId::History => 5,
            EditorPanelId::Assets => 6,
            EditorPanelId::Build => 7,
            EditorPanelId::Recipe => 8,
            EditorPanelId::Modeler => 9,
            EditorPanelId::Diagnostics => 10,
        })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DockAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DockEdge {
    Left,
    Right,
    Bottom,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DockTabMode {
    Preview,
    Pinned,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DockTab {
    pub panel: PanelId,
    pub mode: DockTabMode,
}

impl DockTab {
    #[must_use]
    pub const fn pinned(panel: PanelId) -> Self {
        Self {
            panel,
            mode: DockTabMode::Pinned,
        }
    }

    #[must_use]
    pub const fn preview(panel: PanelId) -> Self {
        Self {
            panel,
            mode: DockTabMode::Preview,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum DockNode {
    Split {
        axis: DockAxis,
        ratio_per_mille: u16,
        first: DockNodeId,
        second: DockNodeId,
    },
    Tabs {
        tabs: Vec<DockTab>,
        active: PanelId,
    },
    Collapsed {
        edge: DockEdge,
        tabs: Vec<DockTab>,
        active: PanelId,
    },
}

impl DockNode {
    fn tabs(&self) -> Option<&[DockTab]> {
        match self {
            Self::Tabs { tabs, .. } | Self::Collapsed { tabs, .. } => Some(tabs),
            Self::Split { .. } => None,
        }
    }

    fn tabs_mut(&mut self) -> Option<(&mut Vec<DockTab>, &mut PanelId)> {
        match self {
            Self::Tabs { tabs, active } | Self::Collapsed { tabs, active, .. } => {
                Some((tabs, active))
            }
            Self::Split { .. } => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DockRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl DockRect {
    #[must_use]
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    fn valid_panel_extent(self) -> bool {
        self.width >= MIN_ACCESSIBLE_PANEL_EXTENT && self.height >= MIN_ACCESSIBLE_PANEL_EXTENT
    }

    fn clamp_to(self, monitor: Self) -> Self {
        let width = self
            .width
            .min(monitor.width)
            .max(MIN_ACCESSIBLE_PANEL_EXTENT);
        let height = self
            .height
            .min(monitor.height)
            .max(MIN_ACCESSIBLE_PANEL_EXTENT);
        let maximum_x = monitor
            .x
            .saturating_add(i32::try_from(monitor.width.saturating_sub(width)).unwrap_or(i32::MAX));
        let maximum_y = monitor.y.saturating_add(
            i32::try_from(monitor.height.saturating_sub(height)).unwrap_or(i32::MAX),
        );
        Self {
            x: self.x.clamp(monitor.x, maximum_x),
            y: self.y.clamp(monitor.y, maximum_y),
            width,
            height,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FloatingDock {
    pub root: DockNodeId,
    pub monitor: MonitorId,
    pub bounds: DockRect,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DockTree {
    pub root: DockNodeId,
    pub nodes: BTreeMap<DockNodeId, DockNode>,
    pub floating: Vec<FloatingDock>,
    pub maximized: Option<PanelId>,
}

impl DockTree {
    /// Creates and validates one dock tree from stable nodes.
    ///
    /// # Errors
    ///
    /// Returns a typed structural, accessibility, identity, or bounds error.
    pub fn new(root: DockNodeId, nodes: BTreeMap<DockNodeId, DockNode>) -> Result<Self, DockError> {
        let tree = Self {
            root,
            nodes,
            floating: Vec::new(),
            maximized: None,
        };
        tree.validate()?;
        Ok(tree)
    }

    /// Validates the complete primary and floating dock forest.
    ///
    /// # Errors
    ///
    /// Returns the first typed structural, accessibility, identity, or bounds error.
    pub fn validate(&self) -> Result<(), DockError> {
        if self.nodes.len() > MAX_RETAINED_NODES {
            return Err(DockError::TooManyNodes {
                count: self.nodes.len(),
                maximum: MAX_RETAINED_NODES,
            });
        }
        if !self.nodes.contains_key(&self.root) {
            return Err(DockError::MissingNode(self.root));
        }
        if self.floating.len() > MAX_RETAINED_NODES {
            return Err(DockError::TooManyFloating {
                count: self.floating.len(),
                maximum: MAX_RETAINED_NODES,
            });
        }

        let mut roots = vec![self.root];
        let mut floating_roots = BTreeSet::new();
        for floating in &self.floating {
            if !floating.bounds.valid_panel_extent() {
                return Err(DockError::PanelBelowAccessibleExtent);
            }
            if !floating_roots.insert(floating.root) {
                return Err(DockError::DuplicateFloatingRoot(floating.root));
            }
            roots.push(floating.root);
        }

        let mut visited = BTreeSet::new();
        let mut panels = BTreeSet::new();
        for root in roots {
            let mut visiting = BTreeSet::new();
            self.validate_node(root, &mut visiting, &mut visited, &mut panels)?;
        }
        if let Some(node) = self.nodes.keys().find(|node| !visited.contains(node)) {
            return Err(DockError::UnreachableNode(*node));
        }
        if let Some(panel) = self.maximized {
            if !panels.contains(&panel) {
                return Err(DockError::UnknownPanel(panel));
            }
        }
        Ok(())
    }

    fn validate_node(
        &self,
        id: DockNodeId,
        visiting: &mut BTreeSet<DockNodeId>,
        visited: &mut BTreeSet<DockNodeId>,
        panels: &mut BTreeSet<PanelId>,
    ) -> Result<(), DockError> {
        if visited.contains(&id) {
            return Err(DockError::MultipleParents(id));
        }
        if !visiting.insert(id) {
            return Err(DockError::Cycle(id));
        }
        let node = self.nodes.get(&id).ok_or(DockError::MissingNode(id))?;
        match node {
            DockNode::Split {
                ratio_per_mille,
                first,
                second,
                ..
            } => {
                if *ratio_per_mille == 0 || *ratio_per_mille >= DOCK_RATIO_PER_MILLE {
                    return Err(DockError::InvalidSplitRatio(*ratio_per_mille));
                }
                if first == second {
                    return Err(DockError::DuplicateChild(*first));
                }
                self.validate_node(*first, visiting, visited, panels)?;
                self.validate_node(*second, visiting, visited, panels)?;
            }
            DockNode::Tabs { tabs, active } | DockNode::Collapsed { tabs, active, .. } => {
                Self::validate_tabs(id, tabs, *active, panels)?;
            }
        }
        visiting.remove(&id);
        visited.insert(id);
        Ok(())
    }

    fn validate_tabs(
        node: DockNodeId,
        tabs: &[DockTab],
        active: PanelId,
        panels: &mut BTreeSet<PanelId>,
    ) -> Result<(), DockError> {
        if tabs.is_empty() {
            return Err(DockError::EmptyTabGroup(node));
        }
        if tabs.len() > MAX_RETAINED_NODES {
            return Err(DockError::TooManyTabs {
                node,
                count: tabs.len(),
                maximum: MAX_RETAINED_NODES,
            });
        }
        if !tabs.iter().any(|tab| tab.panel == active) {
            return Err(DockError::InactivePanel {
                node,
                panel: active,
            });
        }
        if tabs
            .iter()
            .filter(|tab| tab.mode == DockTabMode::Preview)
            .count()
            > 1
        {
            return Err(DockError::MultiplePreviewTabs(node));
        }
        for tab in tabs {
            if !panels.insert(tab.panel) {
                return Err(DockError::DuplicatePanel(tab.panel));
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn panels(&self) -> Vec<PanelId> {
        self.nodes
            .values()
            .filter_map(DockNode::tabs)
            .flatten()
            .map(|tab| tab.panel)
            .collect()
    }

    fn visible_panel_order(&self) -> Result<Vec<PanelId>, DockError> {
        self.validate()?;
        if let Some(maximized) = self.maximized {
            return Ok(vec![maximized]);
        }
        let mut panels = Vec::new();
        self.collect_visible_panels(self.root, &mut panels)?;
        for floating in &self.floating {
            self.collect_visible_panels(floating.root, &mut panels)?;
        }
        Ok(panels)
    }

    fn collect_visible_panels(
        &self,
        node: DockNodeId,
        panels: &mut Vec<PanelId>,
    ) -> Result<(), DockError> {
        match self.nodes.get(&node).ok_or(DockError::MissingNode(node))? {
            DockNode::Split { first, second, .. } => {
                self.collect_visible_panels(*first, panels)?;
                self.collect_visible_panels(*second, panels)
            }
            DockNode::Tabs { active, .. } | DockNode::Collapsed { active, .. } => {
                panels.push(*active);
                Ok(())
            }
        }
    }

    /// Applies one dock mutation atomically and restores the prior tree on rejection.
    ///
    /// # Errors
    ///
    /// Returns a typed mutation or post-mutation validation error.
    pub fn transact(&mut self, mutation: DockMutation) -> Result<DockChange, DockError> {
        self.validate()?;
        let before = self.clone();
        let result = self.apply_mutation(mutation).and_then(|change| {
            self.validate()?;
            Ok(change)
        });
        if result.is_err() {
            *self = before;
        }
        result
    }

    /// Restores a validated baseline only when it owns the same panel set.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid baseline or a mismatched panel set.
    pub fn reset(&mut self, baseline: &Self) -> Result<DockChange, DockError> {
        self.validate()?;
        baseline.validate()?;
        let previous = self.panels().into_iter().collect::<BTreeSet<_>>();
        let replacement = baseline.panels().into_iter().collect::<BTreeSet<_>>();
        if previous != replacement {
            return Err(DockError::ResetPanelSetMismatch);
        }
        *self = baseline.clone();
        Ok(DockChange::Reset)
    }

    fn apply_mutation(&mut self, mutation: DockMutation) -> Result<DockChange, DockError> {
        match mutation {
            DockMutation::Activate(panel) => {
                let (node, _) = self.locate_panel(panel)?;
                let (_, active) = self
                    .nodes
                    .get_mut(&node)
                    .and_then(DockNode::tabs_mut)
                    .ok_or(DockError::NotTabGroup(node))?;
                *active = panel;
                Ok(DockChange::PanelActivated(panel))
            }
            DockMutation::SetTabMode { panel, mode } => self.set_tab_mode(panel, mode),
            DockMutation::Reorder {
                group,
                panel,
                index,
            } => self.reorder(group, panel, index),
            DockMutation::MoveToGroup {
                panel,
                target,
                index,
            } => self.move_to_group(panel, target, index),
            DockMutation::Split {
                panel,
                target,
                new_tabs,
                new_split,
                axis,
                side,
            } => self.split_panel(panel, target, new_tabs, new_split, axis, side),
            DockMutation::TearOff {
                panel,
                floating_root,
                monitor,
                bounds,
            } => self.tear_off(panel, floating_root, monitor, bounds),
            DockMutation::Redock {
                floating_root,
                target,
                index,
            } => self.redock(floating_root, target, index),
            DockMutation::Collapse { group, edge } => self.collapse(group, edge),
            DockMutation::Expand(group) => self.expand(group),
            DockMutation::Maximize(panel) => {
                self.locate_panel(panel)?;
                self.maximized = Some(panel);
                Ok(DockChange::Maximized(Some(panel)))
            }
            DockMutation::RestoreMaximized => {
                self.maximized = None;
                Ok(DockChange::Maximized(None))
            }
        }
    }

    fn set_tab_mode(&mut self, panel: PanelId, mode: DockTabMode) -> Result<DockChange, DockError> {
        let (node, index) = self.locate_panel(panel)?;
        let (tabs, _) = self
            .nodes
            .get_mut(&node)
            .and_then(DockNode::tabs_mut)
            .ok_or(DockError::NotTabGroup(node))?;
        if mode == DockTabMode::Preview {
            for tab in tabs.iter_mut() {
                if tab.mode == DockTabMode::Preview {
                    tab.mode = DockTabMode::Pinned;
                }
            }
        }
        tabs[index].mode = mode;
        Ok(DockChange::TabModeChanged { panel, mode })
    }

    fn reorder(
        &mut self,
        group: DockNodeId,
        panel: PanelId,
        index: usize,
    ) -> Result<DockChange, DockError> {
        let (tabs, _) = self
            .nodes
            .get_mut(&group)
            .and_then(DockNode::tabs_mut)
            .ok_or(DockError::NotTabGroup(group))?;
        let current = tabs
            .iter()
            .position(|tab| tab.panel == panel)
            .ok_or(DockError::UnknownPanel(panel))?;
        if index >= tabs.len() {
            return Err(DockError::InvalidTabIndex {
                index,
                count: tabs.len(),
            });
        }
        let tab = tabs.remove(current);
        tabs.insert(index, tab);
        Ok(DockChange::PanelMoved(panel))
    }

    fn move_to_group(
        &mut self,
        panel: PanelId,
        target: DockNodeId,
        index: usize,
    ) -> Result<DockChange, DockError> {
        let (source, _) = self.locate_panel(panel)?;
        if source == target {
            return self.reorder(target, panel, index);
        }
        let target_count = self
            .nodes
            .get(&target)
            .and_then(DockNode::tabs)
            .ok_or(DockError::NotTabGroup(target))?
            .len();
        if index > target_count {
            return Err(DockError::InvalidTabIndex {
                index,
                count: target_count,
            });
        }
        let tab = self.detach_panel(panel)?;
        self.insert_tab(target, index, tab)?;
        Ok(DockChange::PanelMoved(panel))
    }

    fn split_panel(
        &mut self,
        panel: PanelId,
        target: DockNodeId,
        new_tabs: DockNodeId,
        new_split: DockNodeId,
        axis: DockAxis,
        side: DockSide,
    ) -> Result<DockChange, DockError> {
        if new_tabs == new_split || self.nodes.contains_key(&new_tabs) {
            return Err(DockError::DuplicateNode(new_tabs));
        }
        if self.nodes.contains_key(&new_split) {
            return Err(DockError::DuplicateNode(new_split));
        }
        self.nodes
            .get(&target)
            .and_then(DockNode::tabs)
            .ok_or(DockError::NotTabGroup(target))?;
        let tab = self.detach_panel(panel)?;
        self.replace_root_reference(target, new_split)?;
        self.nodes.insert(
            new_tabs,
            DockNode::Tabs {
                tabs: vec![tab],
                active: panel,
            },
        );
        let (first, second) = match side {
            DockSide::Before => (new_tabs, target),
            DockSide::After => (target, new_tabs),
        };
        self.nodes.insert(
            new_split,
            DockNode::Split {
                axis,
                ratio_per_mille: DOCK_RATIO_PER_MILLE / 2,
                first,
                second,
            },
        );
        Ok(DockChange::PanelMoved(panel))
    }

    fn tear_off(
        &mut self,
        panel: PanelId,
        floating_root: DockNodeId,
        monitor: MonitorId,
        bounds: DockRect,
    ) -> Result<DockChange, DockError> {
        if self.nodes.contains_key(&floating_root) {
            return Err(DockError::DuplicateNode(floating_root));
        }
        if !bounds.valid_panel_extent() {
            return Err(DockError::PanelBelowAccessibleExtent);
        }
        let tab = self.detach_panel(panel)?;
        self.nodes.insert(
            floating_root,
            DockNode::Tabs {
                tabs: vec![tab],
                active: panel,
            },
        );
        self.floating.push(FloatingDock {
            root: floating_root,
            monitor,
            bounds,
        });
        Ok(DockChange::PanelTornOff(panel))
    }

    fn redock(
        &mut self,
        floating_root: DockNodeId,
        target: DockNodeId,
        index: usize,
    ) -> Result<DockChange, DockError> {
        let floating_index = self
            .floating
            .iter()
            .position(|floating| floating.root == floating_root)
            .ok_or(DockError::UnknownFloatingRoot(floating_root))?;
        let (source_nodes, tabs) = self.subtree_nodes_and_tabs(floating_root)?;
        if source_nodes.contains(&target) {
            return Err(DockError::FloatingTargetWithinSource {
                floating_root,
                target,
            });
        }
        let target_count = self
            .nodes
            .get(&target)
            .and_then(DockNode::tabs)
            .ok_or(DockError::NotTabGroup(target))?
            .len();
        if index > target_count {
            return Err(DockError::InvalidTabIndex {
                index,
                count: target_count,
            });
        }
        for (offset, tab) in tabs.iter().copied().enumerate() {
            self.insert_tab(target, index + offset, tab)?;
        }
        self.floating.remove(floating_index);
        for node in source_nodes {
            self.nodes.remove(&node);
        }
        Ok(DockChange::FloatingRedocked(floating_root))
    }

    fn subtree_nodes_and_tabs(
        &self,
        root: DockNodeId,
    ) -> Result<(BTreeSet<DockNodeId>, Vec<DockTab>), DockError> {
        let mut pending = vec![root];
        let mut nodes = BTreeSet::new();
        let mut tabs = Vec::new();
        while let Some(node_id) = pending.pop() {
            if !nodes.insert(node_id) {
                return Err(DockError::Cycle(node_id));
            }
            match self
                .nodes
                .get(&node_id)
                .ok_or(DockError::MissingNode(node_id))?
            {
                DockNode::Split { first, second, .. } => {
                    pending.push(*second);
                    pending.push(*first);
                }
                DockNode::Tabs {
                    tabs: node_tabs, ..
                }
                | DockNode::Collapsed {
                    tabs: node_tabs, ..
                } => tabs.extend(node_tabs.iter().copied()),
            }
        }
        Ok((nodes, tabs))
    }

    fn collapse(&mut self, group: DockNodeId, edge: DockEdge) -> Result<DockChange, DockError> {
        let node = self
            .nodes
            .get_mut(&group)
            .ok_or(DockError::MissingNode(group))?;
        let replacement = match node {
            DockNode::Tabs { tabs, active } => DockNode::Collapsed {
                edge,
                tabs: std::mem::take(tabs),
                active: *active,
            },
            DockNode::Collapsed { .. } => return Ok(DockChange::GroupCollapsed(group)),
            DockNode::Split { .. } => return Err(DockError::NotTabGroup(group)),
        };
        *node = replacement;
        Ok(DockChange::GroupCollapsed(group))
    }

    fn expand(&mut self, group: DockNodeId) -> Result<DockChange, DockError> {
        let node = self
            .nodes
            .get_mut(&group)
            .ok_or(DockError::MissingNode(group))?;
        let replacement = match node {
            DockNode::Collapsed { tabs, active, .. } => DockNode::Tabs {
                tabs: std::mem::take(tabs),
                active: *active,
            },
            DockNode::Tabs { .. } => return Ok(DockChange::GroupExpanded(group)),
            DockNode::Split { .. } => return Err(DockError::NotTabGroup(group)),
        };
        *node = replacement;
        Ok(DockChange::GroupExpanded(group))
    }

    fn locate_panel(&self, panel: PanelId) -> Result<(DockNodeId, usize), DockError> {
        self.nodes
            .iter()
            .find_map(|(node, value)| {
                value.tabs().and_then(|tabs| {
                    tabs.iter()
                        .position(|tab| tab.panel == panel)
                        .map(|index| (*node, index))
                })
            })
            .ok_or(DockError::UnknownPanel(panel))
    }

    fn detach_panel(&mut self, panel: PanelId) -> Result<DockTab, DockError> {
        let (node, index) = self.locate_panel(panel)?;
        let tab_count = self
            .nodes
            .get(&node)
            .and_then(DockNode::tabs)
            .ok_or(DockError::NotTabGroup(node))?
            .len();
        if tab_count == 1 {
            if node == self.root {
                return Err(DockError::WouldEmptyGroup(node));
            }
            let tab = self
                .nodes
                .get(&node)
                .and_then(DockNode::tabs)
                .and_then(|tabs| tabs.first())
                .copied()
                .ok_or(DockError::WouldEmptyGroup(node))?;
            if let Some(floating_index) = self
                .floating
                .iter()
                .position(|floating| floating.root == node)
            {
                self.floating.remove(floating_index);
                self.nodes.remove(&node);
                return Ok(tab);
            }
            let (parent, sibling) = self.parent_and_sibling(node)?;
            self.replace_root_reference(parent, sibling)?;
            self.nodes.remove(&node);
            self.nodes.remove(&parent);
            return Ok(tab);
        }
        let (tabs, active) = self
            .nodes
            .get_mut(&node)
            .and_then(DockNode::tabs_mut)
            .ok_or(DockError::NotTabGroup(node))?;
        let tab = tabs.remove(index);
        if *active == panel {
            *active = tabs[index.min(tabs.len() - 1)].panel;
        }
        Ok(tab)
    }

    fn parent_and_sibling(&self, child: DockNodeId) -> Result<(DockNodeId, DockNodeId), DockError> {
        self.nodes
            .iter()
            .find_map(|(parent, node)| match node {
                DockNode::Split { first, second, .. } if *first == child => {
                    Some((*parent, *second))
                }
                DockNode::Split { first, second, .. } if *second == child => {
                    Some((*parent, *first))
                }
                _ => None,
            })
            .ok_or(DockError::MissingParent(child))
    }

    fn insert_tab(
        &mut self,
        target: DockNodeId,
        index: usize,
        tab: DockTab,
    ) -> Result<(), DockError> {
        let (tabs, active) = self
            .nodes
            .get_mut(&target)
            .and_then(DockNode::tabs_mut)
            .ok_or(DockError::NotTabGroup(target))?;
        if index > tabs.len() {
            return Err(DockError::InvalidTabIndex {
                index,
                count: tabs.len(),
            });
        }
        if tab.mode == DockTabMode::Preview {
            for existing in tabs.iter_mut() {
                if existing.mode == DockTabMode::Preview {
                    existing.mode = DockTabMode::Pinned;
                }
            }
        }
        tabs.insert(index, tab);
        *active = tab.panel;
        Ok(())
    }

    fn replace_root_reference(
        &mut self,
        target: DockNodeId,
        replacement: DockNodeId,
    ) -> Result<(), DockError> {
        if self.root == target {
            self.root = replacement;
            return Ok(());
        }
        if let Some(floating) = self.floating.iter_mut().find(|dock| dock.root == target) {
            floating.root = replacement;
            return Ok(());
        }
        for node in self.nodes.values_mut() {
            if let DockNode::Split { first, second, .. } = node {
                if *first == target {
                    *first = replacement;
                    return Ok(());
                }
                if *second == target {
                    *second = replacement;
                    return Ok(());
                }
            }
        }
        Err(DockError::MissingParent(target))
    }

    fn recover_monitors(
        &mut self,
        monitors: &[MonitorArea],
        report: &mut WorkspaceRecoveryReport,
    ) -> Result<(), WorkspaceStateError> {
        let primary = monitors
            .iter()
            .find(|monitor| monitor.primary)
            .or_else(|| monitors.first())
            .ok_or(WorkspaceStateError::NoMonitor)?;
        for floating in &mut self.floating {
            let monitor = monitors
                .iter()
                .find(|monitor| monitor.id == floating.monitor)
                .unwrap_or(primary);
            if floating.monitor != monitor.id {
                if !report.moved_floating.contains(&floating.root) {
                    report.moved_floating.push(floating.root);
                }
                floating.monitor = monitor.id;
            }
            let clamped = floating.bounds.clamp_to(monitor.bounds);
            if clamped != floating.bounds {
                if !report.moved_floating.contains(&floating.root) {
                    report.moved_floating.push(floating.root);
                }
                floating.bounds = clamped;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DockSide {
    Before,
    After,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DockMutation {
    Activate(PanelId),
    SetTabMode {
        panel: PanelId,
        mode: DockTabMode,
    },
    Reorder {
        group: DockNodeId,
        panel: PanelId,
        index: usize,
    },
    MoveToGroup {
        panel: PanelId,
        target: DockNodeId,
        index: usize,
    },
    Split {
        panel: PanelId,
        target: DockNodeId,
        new_tabs: DockNodeId,
        new_split: DockNodeId,
        axis: DockAxis,
        side: DockSide,
    },
    TearOff {
        panel: PanelId,
        floating_root: DockNodeId,
        monitor: MonitorId,
        bounds: DockRect,
    },
    Redock {
        floating_root: DockNodeId,
        target: DockNodeId,
        index: usize,
    },
    Collapse {
        group: DockNodeId,
        edge: DockEdge,
    },
    Expand(DockNodeId),
    Maximize(PanelId),
    RestoreMaximized,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DockChange {
    PanelActivated(PanelId),
    TabModeChanged { panel: PanelId, mode: DockTabMode },
    PanelMoved(PanelId),
    PanelTornOff(PanelId),
    FloatingRedocked(DockNodeId),
    GroupCollapsed(DockNodeId),
    GroupExpanded(DockNodeId),
    Maximized(Option<PanelId>),
    Reset,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DockError {
    TooManyNodes {
        count: usize,
        maximum: usize,
    },
    TooManyFloating {
        count: usize,
        maximum: usize,
    },
    TooManyTabs {
        node: DockNodeId,
        count: usize,
        maximum: usize,
    },
    MissingNode(DockNodeId),
    DuplicateNode(DockNodeId),
    MissingParent(DockNodeId),
    UnreachableNode(DockNodeId),
    Cycle(DockNodeId),
    MultipleParents(DockNodeId),
    DuplicateChild(DockNodeId),
    EmptyTabGroup(DockNodeId),
    InactivePanel {
        node: DockNodeId,
        panel: PanelId,
    },
    MultiplePreviewTabs(DockNodeId),
    DuplicatePanel(PanelId),
    UnknownPanel(PanelId),
    NotTabGroup(DockNodeId),
    InvalidSplitRatio(u16),
    InsufficientSplitExtent {
        available: u32,
        minimum: u32,
    },
    InvalidTabIndex {
        index: usize,
        count: usize,
    },
    WouldEmptyGroup(DockNodeId),
    PanelBelowAccessibleExtent,
    DuplicateFloatingRoot(DockNodeId),
    UnknownFloatingRoot(DockNodeId),
    FloatingTargetWithinSource {
        floating_root: DockNodeId,
        target: DockNodeId,
    },
    ResetPanelSetMismatch,
}

impl Display for DockError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid dock tree: {self:?}")
    }
}

impl std::error::Error for DockError {}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkspaceKind {
    Hub,
    World,
    Code,
    Modeler,
    UiAuthoring,
    Materials,
    Alluvium,
    Build,
    Profile,
    Settings,
    Recovery,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CameraMemory {
    pub x_mm: i64,
    pub y_mm: i64,
    pub z_mm: i64,
    pub yaw_millidegrees: i32,
    pub pitch_millidegrees: i32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PanelScrollMemory {
    pub panel: PanelId,
    pub logical_offset: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MonitorArea {
    pub id: MonitorId,
    pub bounds: DockRect,
    pub primary: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompanionWindow {
    pub id: CompanionWindowId,
    pub session: WorkspaceSessionId,
    pub panels: Vec<DockTab>,
    pub active: PanelId,
    pub monitor: MonitorId,
    pub bounds: DockRect,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceLayout {
    pub name: String,
    pub workspace: WorkspaceKind,
    pub dock: DockTree,
    pub selected: Option<WorkspaceObjectId>,
    pub active_document: Option<WorkspaceObjectId>,
    pub camera: Option<CameraMemory>,
    pub browser_query: String,
    pub expanded: Vec<WorkspaceObjectId>,
    pub scroll: Vec<PanelScrollMemory>,
    pub focused_panel: Option<PanelId>,
    pub focus_layout: bool,
    pub companions: Vec<CompanionWindow>,
    #[serde(default, flatten)]
    pub extensions: WorkspaceExtensions,
}

impl WorkspaceLayout {
    /// Validates remembered context, dock ownership, and companion session sharing.
    ///
    /// # Errors
    ///
    /// Returns a typed bounds, identity, dock, or companion contract error.
    pub fn validate(&self, session: WorkspaceSessionId) -> Result<(), WorkspaceStateError> {
        validate_bounded_text(&self.name, WorkspaceTextField::LayoutName)?;
        validate_bounded_text(&self.browser_query, WorkspaceTextField::BrowserQuery)?;
        self.extensions.validate(LAYOUT_FIELDS)?;
        if self.name.trim().is_empty() {
            return Err(WorkspaceStateError::EmptyLayoutName);
        }
        self.dock.validate().map_err(WorkspaceStateError::Dock)?;
        if self.expanded.len() > MAX_RETAINED_NODES || self.scroll.len() > MAX_RETAINED_NODES {
            return Err(WorkspaceStateError::TooManyContextEntries);
        }
        let expanded = self.expanded.iter().copied().collect::<BTreeSet<_>>();
        if expanded.len() != self.expanded.len() {
            return Err(WorkspaceStateError::DuplicateExpandedIdentity);
        }
        let mut panels = self.dock.panels().into_iter().collect::<BTreeSet<_>>();
        if self.companions.len() > MAX_RETAINED_NODES {
            return Err(WorkspaceStateError::TooManyCompanions);
        }
        let mut window_ids = BTreeSet::new();
        for companion in &self.companions {
            if companion.session != session {
                return Err(WorkspaceStateError::CompanionSessionMismatch(companion.id));
            }
            if !window_ids.insert(companion.id) {
                return Err(WorkspaceStateError::DuplicateCompanion(companion.id));
            }
            if companion.panels.is_empty() || companion.panels.len() > MAX_RETAINED_NODES {
                return Err(WorkspaceStateError::InvalidCompanionPanels(companion.id));
            }
            if !companion.bounds.valid_panel_extent() {
                return Err(WorkspaceStateError::CompanionBelowAccessibleExtent(
                    companion.id,
                ));
            }
            if !companion
                .panels
                .iter()
                .any(|tab| tab.panel == companion.active)
            {
                return Err(WorkspaceStateError::InvalidCompanionActive(companion.id));
            }
            if companion
                .panels
                .iter()
                .filter(|tab| tab.mode == DockTabMode::Preview)
                .count()
                > 1
            {
                return Err(WorkspaceStateError::MultipleCompanionPreviews(companion.id));
            }
            for tab in &companion.panels {
                if !panels.insert(tab.panel) {
                    return Err(WorkspaceStateError::DuplicateWorkspacePanel(tab.panel));
                }
            }
        }
        let mut scroll_panels = BTreeSet::new();
        for memory in &self.scroll {
            if !panels.contains(&memory.panel) {
                return Err(WorkspaceStateError::UnknownContextPanel(memory.panel));
            }
            if !scroll_panels.insert(memory.panel) {
                return Err(WorkspaceStateError::DuplicateScrollPanel(memory.panel));
            }
        }
        if let Some(panel) = self.focused_panel {
            if !panels.contains(&panel) {
                return Err(WorkspaceStateError::UnknownContextPanel(panel));
            }
        }
        Ok(())
    }

    fn cycle_panel_focus(&mut self, forward: bool) -> Result<PanelId, WorkspaceStateError> {
        let mut order = self
            .dock
            .visible_panel_order()
            .map_err(WorkspaceStateError::Dock)?;
        order.extend(self.companions.iter().map(|window| window.active));
        if order.is_empty() {
            return Err(WorkspaceStateError::NoFocusablePanel);
        }
        let current = self
            .focused_panel
            .and_then(|panel| order.iter().position(|candidate| *candidate == panel));
        let index = match (current, forward) {
            (Some(index), true) => (index + 1) % order.len(),
            (Some(0) | None, false) => order.len() - 1,
            (Some(index), false) => index - 1,
            (None, true) => 0,
        };
        let focused = order
            .get(index)
            .copied()
            .ok_or(WorkspaceStateError::NoFocusablePanel)?;
        self.focused_panel = Some(focused);
        Ok(focused)
    }

    fn tear_off_companion(
        &mut self,
        session: WorkspaceSessionId,
        panel: PanelId,
        id: CompanionWindowId,
        monitor: MonitorId,
        bounds: DockRect,
    ) -> Result<(), WorkspaceStateError> {
        let before = self.clone();
        let result = (|| {
            let tab = self
                .dock
                .detach_panel(panel)
                .map_err(WorkspaceStateError::Dock)?;
            if self.dock.maximized == Some(panel) {
                self.dock.maximized = None;
            }
            self.companions.push(CompanionWindow {
                id,
                session,
                panels: vec![tab],
                active: panel,
                monitor,
                bounds,
            });
            self.validate(session)
        })();
        if result.is_err() {
            *self = before;
        }
        result
    }

    fn redock_companion(
        &mut self,
        session: WorkspaceSessionId,
        id: CompanionWindowId,
        target: DockNodeId,
        index: usize,
    ) -> Result<(), WorkspaceStateError> {
        let before = self.clone();
        let result = (|| {
            let companion_index = self
                .companions
                .iter()
                .position(|window| window.id == id)
                .ok_or(WorkspaceStateError::UnknownCompanion(id))?;
            let companion = self.companions.remove(companion_index);
            for (offset, tab) in companion.panels.iter().copied().enumerate() {
                self.dock
                    .insert_tab(target, index + offset, tab)
                    .map_err(WorkspaceStateError::Dock)?;
            }
            self.validate(session)
        })();
        if result.is_err() {
            *self = before;
        }
        result
    }

    fn recover_environment(
        &mut self,
        known_panels: &BTreeSet<PanelId>,
        monitors: &[MonitorArea],
        session: WorkspaceSessionId,
        report: &mut WorkspaceRecoveryReport,
    ) -> Result<(), WorkspaceStateError> {
        for panel in self.dock.panels().into_iter().chain(
            self.companions
                .iter()
                .flat_map(|window| window.panels.iter().map(|tab| tab.panel)),
        ) {
            if !known_panels.contains(&panel) {
                report.missing_panels.insert(panel);
            }
        }
        self.dock.recover_monitors(monitors, report)?;
        let primary = monitors
            .iter()
            .find(|monitor| monitor.primary)
            .or_else(|| monitors.first())
            .ok_or(WorkspaceStateError::NoMonitor)?;
        for companion in &mut self.companions {
            if companion.session != session {
                return Err(WorkspaceStateError::CompanionSessionMismatch(companion.id));
            }
            let monitor = monitors
                .iter()
                .find(|monitor| monitor.id == companion.monitor)
                .unwrap_or(primary);
            if companion.monitor != monitor.id {
                companion.monitor = monitor.id;
                report.moved_companions.insert(companion.id);
            }
            let clamped = companion.bounds.clamp_to(monitor.bounds);
            if clamped != companion.bounds {
                companion.bounds = clamped;
                report.moved_companions.insert(companion.id);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceStateDocument {
    pub schema: String,
    pub version: u16,
    pub revision: u64,
    pub session: WorkspaceSessionId,
    pub active_workspace: WorkspaceKind,
    pub active_layout_name: String,
    pub layouts: Vec<WorkspaceLayout>,
    #[serde(default, flatten)]
    pub extensions: WorkspaceExtensions,
}

#[derive(Deserialize)]
struct WorkspaceStateHeader {
    schema: String,
    version: u16,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LegacyWorkspaceStateV0 {
    schema: String,
    version: u16,
    session: WorkspaceSessionId,
    active_workspace: WorkspaceKind,
    layouts: Vec<WorkspaceLayout>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceMigrationOutcome {
    pub document: WorkspaceStateDocument,
    pub migrated_from: Option<u16>,
}

impl WorkspaceStateDocument {
    #[must_use]
    pub fn new(session: WorkspaceSessionId, active_workspace: WorkspaceKind) -> Self {
        Self {
            schema: WORKSPACE_STATE_SCHEMA.to_owned(),
            version: WORKSPACE_STATE_VERSION,
            revision: 0,
            session,
            active_workspace,
            active_layout_name: String::new(),
            layouts: Vec::new(),
            extensions: WorkspaceExtensions::default(),
        }
    }

    /// Validates schema, named layouts, active workspace, and shared session state.
    ///
    /// # Errors
    ///
    /// Returns the first typed schema, bound, identity, layout, or session error.
    pub fn validate(&self) -> Result<(), WorkspaceStateError> {
        if self.schema != WORKSPACE_STATE_SCHEMA || self.version != WORKSPACE_STATE_VERSION {
            return Err(WorkspaceStateError::UnsupportedSchema {
                schema: self.schema.clone(),
                version: self.version,
            });
        }
        if self.layouts.len() > MAX_RETAINED_NODES {
            return Err(WorkspaceStateError::TooManyLayouts {
                count: self.layouts.len(),
                maximum: MAX_RETAINED_NODES,
            });
        }
        self.extensions.validate(DOCUMENT_FIELDS)?;
        validate_bounded_text(
            &self.active_layout_name,
            WorkspaceTextField::ActiveLayoutName,
        )?;
        let mut names = BTreeSet::new();
        for layout in &self.layouts {
            layout.validate(self.session)?;
            if !names.insert((layout.workspace, layout.name.clone())) {
                return Err(WorkspaceStateError::DuplicateLayoutName {
                    workspace: layout.workspace,
                    name: layout.name.clone(),
                });
            }
        }
        if !self.layouts.is_empty()
            && !self.layouts.iter().any(|layout| {
                layout.workspace == self.active_workspace && layout.name == self.active_layout_name
            })
        {
            return Err(WorkspaceStateError::MissingActiveLayout {
                workspace: self.active_workspace,
                name: self.active_layout_name.clone(),
            });
        }
        Ok(())
    }

    /// Encodes validated state as stable pretty JSON with a trailing newline.
    ///
    /// # Errors
    ///
    /// Returns a validation, encoding, or shared byte-bound error.
    pub fn canonical_json(&self) -> Result<String, WorkspaceStateError> {
        self.validate()?;
        let mut json = serde_json::to_string_pretty(self)
            .map_err(|error| WorkspaceStateError::Encoding(error.to_string()))?;
        json.push('\n');
        if json.len() > MAX_TEXT_BYTES {
            return Err(WorkspaceStateError::StateTooLarge {
                bytes: json.len(),
                maximum: MAX_TEXT_BYTES,
            });
        }
        Ok(json)
    }

    /// Parses strict versioned JSON within the shared UI text bound.
    ///
    /// # Errors
    ///
    /// Returns a decoding, schema, validation, or byte-bound error.
    pub fn from_json(bytes: &[u8]) -> Result<Self, WorkspaceStateError> {
        if bytes.len() > MAX_TEXT_BYTES {
            return Err(WorkspaceStateError::StateTooLarge {
                bytes: bytes.len(),
                maximum: MAX_TEXT_BYTES,
            });
        }
        let document: Self = serde_json::from_slice(bytes)
            .map_err(|error| WorkspaceStateError::Decoding(error.to_string()))?;
        document.validate()?;
        Ok(document)
    }

    /// Parses the current format or explicitly migrates the supported v0 shape.
    ///
    /// Migration never rewrites the caller's file implicitly.
    ///
    /// # Errors
    ///
    /// Returns a decoding, unsupported-version, validation, or byte-bound error.
    pub fn migrate_json(bytes: &[u8]) -> Result<WorkspaceMigrationOutcome, WorkspaceStateError> {
        if bytes.len() > MAX_TEXT_BYTES {
            return Err(WorkspaceStateError::StateTooLarge {
                bytes: bytes.len(),
                maximum: MAX_TEXT_BYTES,
            });
        }
        let header: WorkspaceStateHeader = serde_json::from_slice(bytes)
            .map_err(|error| WorkspaceStateError::Decoding(error.to_string()))?;
        match (header.schema.as_str(), header.version) {
            (WORKSPACE_STATE_SCHEMA, WORKSPACE_STATE_VERSION) => Ok(WorkspaceMigrationOutcome {
                document: Self::from_json(bytes)?,
                migrated_from: None,
            }),
            (LEGACY_WORKSPACE_STATE_SCHEMA, 0) => {
                let legacy: LegacyWorkspaceStateV0 = serde_json::from_slice(bytes)
                    .map_err(|error| WorkspaceStateError::Decoding(error.to_string()))?;
                if legacy.schema != LEGACY_WORKSPACE_STATE_SCHEMA || legacy.version != 0 {
                    return Err(WorkspaceStateError::UnsupportedSchema {
                        schema: legacy.schema,
                        version: legacy.version,
                    });
                }
                let document = Self {
                    schema: WORKSPACE_STATE_SCHEMA.to_owned(),
                    version: WORKSPACE_STATE_VERSION,
                    revision: 0,
                    session: legacy.session,
                    active_workspace: legacy.active_workspace,
                    active_layout_name: legacy
                        .layouts
                        .iter()
                        .find(|layout| layout.workspace == legacy.active_workspace)
                        .map_or_else(String::new, |layout| layout.name.clone()),
                    layouts: legacy.layouts,
                    extensions: WorkspaceExtensions::default(),
                };
                document.validate()?;
                Ok(WorkspaceMigrationOutcome {
                    document,
                    migrated_from: Some(0),
                })
            }
            _ => Err(WorkspaceStateError::UnsupportedSchema {
                schema: header.schema,
                version: header.version,
            }),
        }
    }

    /// Atomically inserts or replaces one validated named layout in memory.
    ///
    /// # Errors
    ///
    /// Returns a layout validation, document validation, or revision error.
    pub fn save_named_layout(
        &mut self,
        mut layout: WorkspaceLayout,
    ) -> Result<(), WorkspaceStateError> {
        layout.validate(self.session)?;
        let layout_workspace = layout.workspace;
        let layout_name = layout.name.clone();
        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or(WorkspaceStateError::RevisionExhausted)?;
        let before = self.clone();
        if let Some(existing) = self
            .layouts
            .iter_mut()
            .find(|existing| existing.workspace == layout.workspace && existing.name == layout.name)
        {
            let mut extensions = existing.extensions.clone();
            extensions.merge_from(std::mem::take(&mut layout.extensions));
            layout.extensions = extensions;
            *existing = layout;
        } else {
            self.layouts.push(layout);
        }
        if self.active_layout_name.is_empty() && self.active_workspace == layout_workspace {
            self.active_layout_name = layout_name;
        }
        self.revision = next_revision;
        if let Err(error) = self.validate() {
            *self = before;
            return Err(error);
        }
        Ok(())
    }

    /// Selects an existing named layout; a second activation toggles its focus layout.
    ///
    /// # Errors
    ///
    /// Returns an error when the named layout is absent or revision is exhausted.
    pub fn activate_workspace(
        &mut self,
        workspace: WorkspaceKind,
        layout_name: &str,
    ) -> Result<WorkspaceActivation, WorkspaceStateError> {
        let layout_index = self
            .layouts
            .iter()
            .position(|layout| layout.workspace == workspace && layout.name == layout_name)
            .ok_or_else(|| WorkspaceStateError::UnknownLayout {
                workspace,
                name: layout_name.to_owned(),
            })?;
        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or(WorkspaceStateError::RevisionExhausted)?;
        let activation =
            if self.active_workspace == workspace && self.active_layout_name == layout_name {
                let layout = &mut self.layouts[layout_index];
                layout.focus_layout = !layout.focus_layout;
                if layout.focus_layout {
                    WorkspaceActivation::FocusEntered
                } else {
                    WorkspaceActivation::FocusExited
                }
            } else {
                self.active_workspace = workspace;
                layout_name.clone_into(&mut self.active_layout_name);
                WorkspaceActivation::Switched
            };
        self.revision = next_revision;
        Ok(activation)
    }

    /// Cycles keyboard focus through visible panes in visual dock order and
    /// then through session-sharing companion windows.
    ///
    /// # Errors
    ///
    /// Returns a layout, dock, validation, or revision error without changing
    /// the previously accepted workspace state.
    pub fn cycle_panel_focus(
        &mut self,
        workspace: WorkspaceKind,
        layout_name: &str,
        forward: bool,
    ) -> Result<PanelId, WorkspaceStateError> {
        let before = self.clone();
        let result = (|| {
            let focused = self
                .layout_mut(workspace, layout_name)?
                .cycle_panel_focus(forward)?;
            self.revision = self
                .revision
                .checked_add(1)
                .ok_or(WorkspaceStateError::RevisionExhausted)?;
            self.validate()?;
            Ok(focused)
        })();
        if result.is_err() {
            *self = before;
        }
        result
    }

    /// Moves one tab to a session-sharing companion descriptor transactionally.
    ///
    /// # Errors
    ///
    /// Returns a layout lookup, dock mutation, companion, or revision error.
    pub fn tear_off_companion(
        &mut self,
        workspace: WorkspaceKind,
        layout_name: &str,
        panel: PanelId,
        id: CompanionWindowId,
        monitor: MonitorId,
        bounds: DockRect,
    ) -> Result<(), WorkspaceStateError> {
        let before = self.clone();
        let result = (|| {
            let session = self.session;
            let layout = self.layout_mut(workspace, layout_name)?;
            layout.tear_off_companion(session, panel, id, monitor, bounds)?;
            self.revision = self
                .revision
                .checked_add(1)
                .ok_or(WorkspaceStateError::RevisionExhausted)?;
            self.validate()
        })();
        if result.is_err() {
            *self = before;
        }
        result
    }

    /// Returns all tabs from one companion descriptor to a dock group atomically.
    ///
    /// # Errors
    ///
    /// Returns a layout lookup, dock mutation, companion, or revision error.
    pub fn redock_companion(
        &mut self,
        workspace: WorkspaceKind,
        layout_name: &str,
        id: CompanionWindowId,
        target: DockNodeId,
        index: usize,
    ) -> Result<(), WorkspaceStateError> {
        let before = self.clone();
        let result = (|| {
            let session = self.session;
            let layout = self.layout_mut(workspace, layout_name)?;
            layout.redock_companion(session, id, target, index)?;
            self.revision = self
                .revision
                .checked_add(1)
                .ok_or(WorkspaceStateError::RevisionExhausted)?;
            self.validate()
        })();
        if result.is_err() {
            *self = before;
        }
        result
    }

    fn layout_mut(
        &mut self,
        workspace: WorkspaceKind,
        name: &str,
    ) -> Result<&mut WorkspaceLayout, WorkspaceStateError> {
        self.layouts
            .iter_mut()
            .find(|layout| layout.workspace == workspace && layout.name == name)
            .ok_or_else(|| WorkspaceStateError::UnknownLayout {
                workspace,
                name: name.to_owned(),
            })
    }

    /// Recovers parse failure, missing panels, and monitor loss without source mutation.
    ///
    /// # Errors
    ///
    /// Returns an error when the fallback is invalid or no visible monitor exists.
    pub fn recover_or_default(
        bytes: &[u8],
        fallback: Self,
        known_panels: &BTreeSet<PanelId>,
        monitors: &[MonitorArea],
    ) -> Result<WorkspaceRecoveryOutcome, WorkspaceStateError> {
        fallback.validate()?;
        validate_monitors(monitors)?;
        let expected_session = fallback.session;
        let (mut document, corrupt_state, migrated_from) = match Self::migrate_json(bytes) {
            Ok(outcome) if outcome.document.session == expected_session => {
                (outcome.document, None, outcome.migrated_from)
            }
            Ok(outcome) => (
                fallback,
                Some(
                    WorkspaceStateError::SessionMismatch {
                        expected: expected_session,
                        actual: outcome.document.session,
                    }
                    .to_string(),
                ),
                None,
            ),
            Err(error) => (fallback, Some(error.to_string()), None),
        };
        let mut report = WorkspaceRecoveryReport {
            corrupt_state,
            migrated_from,
            ..WorkspaceRecoveryReport::default()
        };
        for layout in &mut document.layouts {
            layout.recover_environment(known_panels, monitors, document.session, &mut report)?;
        }
        document.validate()?;
        Ok(WorkspaceRecoveryOutcome { document, report })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WorkspaceHistoryEntry {
    document: WorkspaceStateDocument,
    encoded_bytes: usize,
}

/// Bounded transactional history for one validated workspace-state document.
///
/// Past and future checkpoints share the existing canonical workspace byte
/// bound. Undo, redo, and reset allocate a fresh monotonic revision instead of
/// reviving a stale persisted revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceStateHistory {
    current: WorkspaceStateDocument,
    baseline: WorkspaceStateDocument,
    undo: VecDeque<WorkspaceHistoryEntry>,
    redo: VecDeque<WorkspaceHistoryEntry>,
    retained_bytes: usize,
}

impl WorkspaceStateHistory {
    /// Starts a bounded history from one validated persistence baseline.
    ///
    /// # Errors
    ///
    /// Returns the document's validation, encoding, or byte-bound error.
    pub fn new(document: WorkspaceStateDocument) -> Result<Self, WorkspaceStateError> {
        document.canonical_json()?;
        Ok(Self {
            baseline: document.clone(),
            current: document,
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            retained_bytes: 0,
        })
    }

    #[must_use]
    pub const fn current(&self) -> &WorkspaceStateDocument {
        &self.current
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    #[must_use]
    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    #[must_use]
    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    #[must_use]
    pub const fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }

    /// Commits one validated, newer workspace document and clears redo state.
    ///
    /// Callers construct `next` through the typed workspace operations before
    /// crossing this checkpoint boundary.
    ///
    /// # Errors
    ///
    /// Returns a validation, size, session, or monotonic-revision error without
    /// modifying the accepted history.
    pub fn commit(&mut self, next: WorkspaceStateDocument) -> Result<(), WorkspaceStateError> {
        let before = self.clone();
        let result = (|| {
            next.canonical_json()?;
            if next.session != self.current.session {
                return Err(WorkspaceStateError::SessionMismatch {
                    expected: self.current.session,
                    actual: next.session,
                });
            }
            if next.revision <= self.current.revision {
                return Err(WorkspaceStateError::HistoryRevisionNotAdvanced {
                    current: self.current.revision,
                    candidate: next.revision,
                });
            }
            let current = Self::entry(self.current.clone())?;
            self.clear_redo();
            self.retained_bytes = self.retained_bytes.saturating_add(current.encoded_bytes);
            self.undo.push_back(current);
            self.current = next;
            self.trim_to_bound();
            Ok(())
        })();
        if result.is_err() {
            *self = before;
        }
        result
    }

    /// Restores the newest past checkpoint under a fresh revision.
    ///
    /// # Errors
    ///
    /// Returns a typed unavailable, revision, validation, or size error without
    /// changing history.
    pub fn undo(&mut self) -> Result<&WorkspaceStateDocument, WorkspaceStateError> {
        self.move_history(WorkspaceHistoryDirection::Undo)
    }

    /// Restores the newest future checkpoint under a fresh revision.
    ///
    /// # Errors
    ///
    /// Returns a typed unavailable, revision, validation, or size error without
    /// changing history.
    pub fn redo(&mut self) -> Result<&WorkspaceStateDocument, WorkspaceStateError> {
        self.move_history(WorkspaceHistoryDirection::Redo)
    }

    /// Restores the captured baseline as a new, undoable revision.
    ///
    /// # Errors
    ///
    /// Returns a revision, validation, or size error without changing history.
    pub fn reset(&mut self) -> Result<&WorkspaceStateDocument, WorkspaceStateError> {
        let mut baseline = self.baseline.clone();
        baseline.revision = self
            .current
            .revision
            .checked_add(1)
            .ok_or(WorkspaceStateError::RevisionExhausted)?;
        self.commit(baseline)?;
        Ok(&self.current)
    }

    fn move_history(
        &mut self,
        direction: WorkspaceHistoryDirection,
    ) -> Result<&WorkspaceStateDocument, WorkspaceStateError> {
        let before = self.clone();
        let result = (|| {
            let next_revision = self
                .current
                .revision
                .checked_add(1)
                .ok_or(WorkspaceStateError::RevisionExhausted)?;
            let mut target = match direction {
                WorkspaceHistoryDirection::Undo => self.undo.pop_back(),
                WorkspaceHistoryDirection::Redo => self.redo.pop_back(),
            }
            .ok_or(WorkspaceStateError::HistoryUnavailable(direction))?;
            self.retained_bytes = self.retained_bytes.saturating_sub(target.encoded_bytes);
            let current = Self::entry(self.current.clone())?;
            self.retained_bytes = self.retained_bytes.saturating_add(current.encoded_bytes);
            match direction {
                WorkspaceHistoryDirection::Undo => self.redo.push_back(current),
                WorkspaceHistoryDirection::Redo => self.undo.push_back(current),
            }
            target.document.revision = next_revision;
            target.document.canonical_json()?;
            self.current = target.document;
            self.trim_to_bound();
            Ok(())
        })();
        if let Err(error) = result {
            *self = before;
            return Err(error);
        }
        Ok(&self.current)
    }

    fn entry(
        document: WorkspaceStateDocument,
    ) -> Result<WorkspaceHistoryEntry, WorkspaceStateError> {
        let encoded_bytes = document.canonical_json()?.len();
        Ok(WorkspaceHistoryEntry {
            document,
            encoded_bytes,
        })
    }

    fn clear_redo(&mut self) {
        self.retained_bytes = self.redo.iter().fold(self.retained_bytes, |bytes, entry| {
            bytes.saturating_sub(entry.encoded_bytes)
        });
        self.redo.clear();
    }

    fn trim_to_bound(&mut self) {
        while self.retained_bytes > MAX_TEXT_BYTES {
            let removed = self.undo.pop_front().or_else(|| self.redo.pop_front());
            let Some(removed) = removed else {
                self.retained_bytes = 0;
                break;
            };
            self.retained_bytes = self.retained_bytes.saturating_sub(removed.encoded_bytes);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceHistoryDirection {
    Undo,
    Redo,
}

pub struct WorkspaceStateStore {
    store: SaveStore,
}

impl WorkspaceStateStore {
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            store: SaveStore::new(
                path,
                SaveConfig {
                    schema_version: u32::from(WORKSPACE_STATE_VERSION),
                    max_payload_bytes: MAX_TEXT_BYTES,
                },
            ),
        }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        self.store.path()
    }

    /// Persists one validated state through the engine's atomic save boundary.
    ///
    /// # Errors
    ///
    /// Returns a validation, encoding, byte-bound, or storage error.
    pub fn save(&self, document: &WorkspaceStateDocument) -> Result<(), WorkspaceStateError> {
        let json = document.canonical_json()?;
        self.store
            .save(json.as_bytes())
            .map_err(WorkspaceStateError::Storage)
    }

    /// Loads the primary state or its verified previous-state backup.
    ///
    /// # Errors
    ///
    /// Returns a storage, decoding, schema, or validation error.
    pub fn load(&self) -> Result<WorkspaceStateDocument, WorkspaceStateError> {
        let bytes = self.store.load().map_err(WorkspaceStateError::Storage)?;
        WorkspaceStateDocument::from_json(&bytes)
    }

    /// Loads current state or returns an explicit in-memory legacy migration.
    ///
    /// The store is not rewritten until the caller explicitly saves the result.
    ///
    /// # Errors
    ///
    /// Returns a storage, decoding, migration, schema, or validation error.
    pub fn load_migrated(&self) -> Result<WorkspaceMigrationOutcome, WorkspaceStateError> {
        let bytes = self.store.load().map_err(WorkspaceStateError::Storage)?;
        WorkspaceStateDocument::migrate_json(&bytes)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WorkspaceRecoveryReport {
    pub corrupt_state: Option<String>,
    pub migrated_from: Option<u16>,
    pub missing_panels: BTreeSet<PanelId>,
    pub moved_floating: Vec<DockNodeId>,
    pub moved_companions: BTreeSet<CompanionWindowId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceRecoveryOutcome {
    pub document: WorkspaceStateDocument,
    pub report: WorkspaceRecoveryReport,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceActivation {
    Switched,
    FocusEntered,
    FocusExited,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceTextField {
    LayoutName,
    BrowserQuery,
    ActiveLayoutName,
    ExtensionKey,
}

#[derive(Debug)]
pub enum WorkspaceStateError {
    Dock(DockError),
    UnsupportedSchema {
        schema: String,
        version: u16,
    },
    TooManyLayouts {
        count: usize,
        maximum: usize,
    },
    TooManyExtensions {
        count: usize,
        maximum: usize,
    },
    EmptyExtensionKey,
    ReservedExtensionKey(String),
    DuplicateLayoutName {
        workspace: WorkspaceKind,
        name: String,
    },
    UnknownLayout {
        workspace: WorkspaceKind,
        name: String,
    },
    MissingActiveLayout {
        workspace: WorkspaceKind,
        name: String,
    },
    EmptyLayoutName,
    TextTooLong {
        field: WorkspaceTextField,
        bytes: usize,
        maximum: usize,
    },
    TooManyContextEntries,
    DuplicateExpandedIdentity,
    UnknownContextPanel(PanelId),
    DuplicateScrollPanel(PanelId),
    TooManyCompanions,
    DuplicateCompanion(CompanionWindowId),
    CompanionSessionMismatch(CompanionWindowId),
    InvalidCompanionPanels(CompanionWindowId),
    CompanionBelowAccessibleExtent(CompanionWindowId),
    InvalidCompanionActive(CompanionWindowId),
    MultipleCompanionPreviews(CompanionWindowId),
    DuplicateWorkspacePanel(PanelId),
    UnknownCompanion(CompanionWindowId),
    NoFocusablePanel,
    NoMonitor,
    DuplicateMonitor(MonitorId),
    MultiplePrimaryMonitors,
    MonitorBelowAccessibleExtent(MonitorId),
    SessionMismatch {
        expected: WorkspaceSessionId,
        actual: WorkspaceSessionId,
    },
    StateTooLarge {
        bytes: usize,
        maximum: usize,
    },
    Encoding(String),
    Decoding(String),
    Storage(SaveError),
    RevisionExhausted,
    HistoryRevisionNotAdvanced {
        current: u64,
        candidate: u64,
    },
    HistoryUnavailable(WorkspaceHistoryDirection),
}

impl Display for WorkspaceStateError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dock(error) => Display::fmt(error, formatter),
            Self::Storage(error) => write!(formatter, "workspace storage failed: {error}"),
            Self::Encoding(error) => write!(formatter, "workspace encoding failed: {error}"),
            Self::Decoding(error) => write!(formatter, "workspace decoding failed: {error}"),
            other => write!(formatter, "invalid workspace state: {other:?}"),
        }
    }
}

impl std::error::Error for WorkspaceStateError {}

fn validate_bounded_text(
    value: &str,
    field: WorkspaceTextField,
) -> Result<(), WorkspaceStateError> {
    if value.len() > MAX_TEXT_BYTES {
        return Err(WorkspaceStateError::TextTooLong {
            field,
            bytes: value.len(),
            maximum: MAX_TEXT_BYTES,
        });
    }
    Ok(())
}

fn validate_monitors(monitors: &[MonitorArea]) -> Result<(), WorkspaceStateError> {
    if monitors.is_empty() {
        return Err(WorkspaceStateError::NoMonitor);
    }
    if monitors.len() > MAX_RETAINED_NODES {
        return Err(WorkspaceStateError::TooManyContextEntries);
    }
    let mut identities = BTreeSet::new();
    let mut primary_count = 0_usize;
    for monitor in monitors {
        if !identities.insert(monitor.id) {
            return Err(WorkspaceStateError::DuplicateMonitor(monitor.id));
        }
        if monitor.primary {
            primary_count += 1;
        }
        if !monitor.bounds.valid_panel_extent() {
            return Err(WorkspaceStateError::MonitorBelowAccessibleExtent(
                monitor.id,
            ));
        }
    }
    if primary_count > 1 {
        return Err(WorkspaceStateError::MultiplePrimaryMonitors);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResponsiveRegion {
    pub panel: PanelId,
    pub priority: u8,
    pub preferred_extent: u32,
    /// Extent after nonessential labels and chrome use their compact form.
    pub compact_extent: u32,
    pub minimum_extent: u32,
    pub pinned: bool,
    pub working_canvas: bool,
    /// Prevents collapse while the panel owns an active error or recovery path.
    pub must_remain_visible: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResponsiveStage {
    Preferred,
    Tightened,
    LabelsShortened,
    Collapsed,
    ControlledOverflow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResponsiveLayout {
    pub stage: ResponsiveStage,
    pub gutter: u32,
    pub visible: Vec<PanelId>,
    pub shortened: Vec<PanelId>,
    pub collapsed: Vec<PanelId>,
    pub minimum_required: u32,
}

fn validate_responsive_regions(regions: &[ResponsiveRegion]) -> Result<(), ResponsiveError> {
    if regions.len() > MAX_RETAINED_NODES {
        return Err(ResponsiveError::TooManyRegions {
            count: regions.len(),
            maximum: MAX_RETAINED_NODES,
        });
    }
    let mut identities = BTreeSet::new();
    for region in regions {
        if !identities.insert(region.panel) {
            return Err(ResponsiveError::DuplicatePanel(region.panel));
        }
        if !(1..=5).contains(&region.priority)
            || region.minimum_extent < MIN_ACCESSIBLE_PANEL_EXTENT
            || region.compact_extent < region.minimum_extent
            || region.compact_extent > region.preferred_extent
            || region.preferred_extent < region.minimum_extent
        {
            return Err(ResponsiveError::InvalidRegion(region.panel));
        }
    }
    Ok(())
}

/// Adapts unpinned regions by priority while preserving canvas and accessible minima.
///
/// # Errors
///
/// Returns a typed count, identity, priority, or minimum-size error.
pub fn adapt_responsive_regions(
    available_extent: u32,
    regions: &[ResponsiveRegion],
) -> Result<ResponsiveLayout, ResponsiveError> {
    validate_responsive_regions(regions)?;
    let preferred_gutters = DOCK_GUTTER
        .saturating_mul(u32::try_from(regions.len().saturating_sub(1)).unwrap_or(u32::MAX));
    let tight_gutters = TIGHT_DOCK_GUTTER
        .saturating_mul(u32::try_from(regions.len().saturating_sub(1)).unwrap_or(u32::MAX));
    let preferred = regions
        .iter()
        .map(|region| region.preferred_extent)
        .fold(preferred_gutters, u32::saturating_add);
    let tightened = regions
        .iter()
        .map(|region| region.preferred_extent)
        .fold(tight_gutters, u32::saturating_add);
    let compact = regions
        .iter()
        .map(|region| region.compact_extent)
        .fold(tight_gutters, u32::saturating_add);
    let minimum = regions
        .iter()
        .map(|region| region.minimum_extent)
        .fold(tight_gutters, u32::saturating_add);
    let mut visible = regions
        .iter()
        .map(|region| region.panel)
        .collect::<Vec<_>>();
    let mut collapsed = Vec::new();
    let mut required = minimum;
    let (stage, gutter) = if preferred <= available_extent {
        (ResponsiveStage::Preferred, DOCK_GUTTER)
    } else if tightened <= available_extent {
        (ResponsiveStage::Tightened, TIGHT_DOCK_GUTTER)
    } else if compact <= available_extent {
        (ResponsiveStage::LabelsShortened, TIGHT_DOCK_GUTTER)
    } else {
        let mut candidates = regions
            .iter()
            .filter(|region| {
                !region.pinned && !region.working_canvas && !region.must_remain_visible
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|region| (region.priority, region.panel));
        for region in candidates {
            required = required.saturating_sub(region.minimum_extent);
            if visible.len() > 1 {
                required = required.saturating_sub(TIGHT_DOCK_GUTTER);
            }
            visible.retain(|panel| *panel != region.panel);
            collapsed.push(region.panel);
            if required <= available_extent {
                break;
            }
        }
        if !collapsed.is_empty() && required <= available_extent {
            (ResponsiveStage::Collapsed, TIGHT_DOCK_GUTTER)
        } else {
            (ResponsiveStage::ControlledOverflow, TIGHT_DOCK_GUTTER)
        }
    };
    let shortened = if matches!(
        stage,
        ResponsiveStage::LabelsShortened
            | ResponsiveStage::Collapsed
            | ResponsiveStage::ControlledOverflow
    ) {
        regions
            .iter()
            .filter(|region| {
                region.compact_extent < region.preferred_extent && visible.contains(&region.panel)
            })
            .map(|region| region.panel)
            .collect()
    } else {
        Vec::new()
    };
    Ok(ResponsiveLayout {
        stage,
        gutter,
        visible,
        shortened,
        collapsed,
        minimum_required: required,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResponsiveError {
    TooManyRegions { count: usize, maximum: usize },
    DuplicatePanel(PanelId),
    InvalidRegion(PanelId),
}

impl Display for ResponsiveError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid responsive layout: {self:?}")
    }
}

impl std::error::Error for ResponsiveError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DockSplitExtents {
    pub first: u32,
    pub second: u32,
}

/// Resolves an integer split without shrinking either pane below its minimum.
///
/// # Errors
///
/// Returns a typed ratio, accessible-minimum, or available-extent error.
pub fn resolve_split_extents(
    total_extent: u32,
    ratio_per_mille: u16,
    first_minimum: u32,
    second_minimum: u32,
) -> Result<DockSplitExtents, DockError> {
    if ratio_per_mille == 0 || ratio_per_mille >= DOCK_RATIO_PER_MILLE {
        return Err(DockError::InvalidSplitRatio(ratio_per_mille));
    }
    if first_minimum < MIN_ACCESSIBLE_PANEL_EXTENT || second_minimum < MIN_ACCESSIBLE_PANEL_EXTENT {
        return Err(DockError::PanelBelowAccessibleExtent);
    }
    let available = total_extent.saturating_sub(DOCK_GUTTER);
    let minimum = first_minimum.saturating_add(second_minimum);
    if available < minimum {
        return Err(DockError::InsufficientSplitExtent { available, minimum });
    }
    let desired = u32::try_from(
        u64::from(available) * u64::from(ratio_per_mille) / u64::from(DOCK_RATIO_PER_MILLE),
    )
    .unwrap_or(u32::MAX);
    let first = desired.clamp(first_minimum, available - second_minimum);
    Ok(DockSplitExtents {
        first,
        second: available - first,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn tab_group(id: u128, panels: &[u128]) -> (DockNodeId, DockNode) {
        let id = DockNodeId::new(id);
        let tabs = panels
            .iter()
            .map(|panel| DockTab::pinned(PanelId::new(*panel)))
            .collect::<Vec<_>>();
        (
            id,
            DockNode::Tabs {
                active: tabs[0].panel,
                tabs,
            },
        )
    }

    fn dock_fixture() -> DockTree {
        let root = DockNodeId::new(1);
        let left = DockNodeId::new(2);
        let right = DockNodeId::new(3);
        let mut nodes = BTreeMap::new();
        nodes.insert(
            root,
            DockNode::Split {
                axis: DockAxis::Horizontal,
                ratio_per_mille: 300,
                first: left,
                second: right,
            },
        );
        nodes.insert(left, tab_group(2, &[1, 2]).1);
        nodes.insert(right, tab_group(3, &[3, 4]).1);
        DockTree::new(root, nodes).expect("dock fixture")
    }

    fn workspace_fixture() -> WorkspaceStateDocument {
        let session = WorkspaceSessionId::new(90);
        let mut document = WorkspaceStateDocument::new(session, WorkspaceKind::World);
        document
            .save_named_layout(WorkspaceLayout {
                name: "Default".to_owned(),
                workspace: WorkspaceKind::World,
                dock: dock_fixture(),
                selected: Some(WorkspaceObjectId::new(7)),
                active_document: Some(WorkspaceObjectId::new(8)),
                camera: Some(CameraMemory {
                    x_mm: 1,
                    y_mm: 2,
                    z_mm: 3,
                    yaw_millidegrees: 4,
                    pitch_millidegrees: 5,
                }),
                browser_query: "camera".to_owned(),
                expanded: vec![WorkspaceObjectId::new(7)],
                scroll: vec![PanelScrollMemory {
                    panel: PanelId::new(1),
                    logical_offset: 12,
                }],
                focused_panel: Some(PanelId::new(3)),
                focus_layout: false,
                companions: Vec::new(),
                extensions: WorkspaceExtensions::default(),
            })
            .expect("default layout saves");
        document
    }

    fn monitor(id: u128, primary: bool) -> MonitorArea {
        MonitorArea {
            id: MonitorId::new(id),
            bounds: DockRect::new(0, 0, 1_440, 900),
            primary,
        }
    }

    #[test]
    fn invalid_tree_and_failed_mutation_roll_back_without_losing_panels() {
        let mut tree = dock_fixture();
        let before = tree.clone();
        let error = tree
            .transact(DockMutation::MoveToGroup {
                panel: PanelId::new(1),
                target: DockNodeId::new(3),
                index: 99,
            })
            .expect_err("invalid target index rejects");
        assert!(matches!(error, DockError::InvalidTabIndex { .. }));
        assert_eq!(tree, before);

        let error = tree
            .transact(DockMutation::Split {
                panel: PanelId::new(1),
                target: DockNodeId::new(3),
                new_tabs: DockNodeId::new(10),
                new_split: DockNodeId::new(10),
                axis: DockAxis::Horizontal,
                side: DockSide::After,
            })
            .expect_err("split identities must differ");
        assert!(matches!(error, DockError::DuplicateNode(_)));
        assert_eq!(tree, before);

        let root = DockNodeId::new(50);
        let mut cyclic = BTreeMap::new();
        cyclic.insert(
            root,
            DockNode::Split {
                axis: DockAxis::Horizontal,
                ratio_per_mille: 500,
                first: root,
                second: DockNodeId::new(51),
            },
        );
        cyclic.insert(DockNodeId::new(51), tab_group(51, &[9]).1);
        assert!(matches!(
            DockTree::new(root, cyclic),
            Err(DockError::Cycle(_))
        ));
    }

    #[test]
    fn singleton_branch_prunes_and_split_floating_redocks_without_panel_loss() {
        let root = DockNodeId::new(1);
        let left = DockNodeId::new(2);
        let right = DockNodeId::new(3);
        let mut nodes = BTreeMap::new();
        nodes.insert(
            root,
            DockNode::Split {
                axis: DockAxis::Horizontal,
                ratio_per_mille: 500,
                first: left,
                second: right,
            },
        );
        nodes.insert(left, tab_group(2, &[1]).1);
        nodes.insert(right, tab_group(3, &[2, 3]).1);
        let mut singleton = DockTree::new(root, nodes).expect("singleton branch fixture");
        singleton
            .transact(DockMutation::MoveToGroup {
                panel: PanelId::new(1),
                target: right,
                index: 1,
            })
            .expect("last branch tab moves and prunes its empty split");
        assert_eq!(singleton.root, right);
        assert_eq!(singleton.nodes.len(), 1);
        assert_eq!(singleton.panels().len(), 3);

        let mut tree = dock_fixture();
        tree.transact(DockMutation::TearOff {
            panel: PanelId::new(4),
            floating_root: DockNodeId::new(6),
            monitor: MonitorId::new(1),
            bounds: DockRect::new(40, 40, 480, 320),
        })
        .expect("floating tab");
        tree.transact(DockMutation::Split {
            panel: PanelId::new(1),
            target: DockNodeId::new(6),
            new_tabs: DockNodeId::new(7),
            new_split: DockNodeId::new(8),
            axis: DockAxis::Vertical,
            side: DockSide::Before,
        })
        .expect("floating window accepts a split dock tree");
        let before = tree.clone();
        assert!(matches!(
            tree.transact(DockMutation::Redock {
                floating_root: DockNodeId::new(8),
                target: DockNodeId::new(6),
                index: 0,
            }),
            Err(DockError::FloatingTargetWithinSource { .. })
        ));
        assert_eq!(tree, before);
        tree.transact(DockMutation::Redock {
            floating_root: DockNodeId::new(8),
            target: DockNodeId::new(3),
            index: 0,
        })
        .expect("complete floating subtree redocks");
        assert!(tree.floating.is_empty());
        assert_eq!(tree.panels().len(), 4);
    }

    #[test]
    fn tabs_split_float_collapse_maximize_and_reset_are_transactional() {
        let baseline = dock_fixture();
        let mut tree = baseline.clone();
        tree.transact(DockMutation::SetTabMode {
            panel: PanelId::new(1),
            mode: DockTabMode::Preview,
        })
        .expect("preview tab");
        tree.transact(DockMutation::SetTabMode {
            panel: PanelId::new(2),
            mode: DockTabMode::Preview,
        })
        .expect("new preview pins prior preview");
        tree.transact(DockMutation::Split {
            panel: PanelId::new(1),
            target: DockNodeId::new(3),
            new_tabs: DockNodeId::new(4),
            new_split: DockNodeId::new(5),
            axis: DockAxis::Vertical,
            side: DockSide::Before,
        })
        .expect("split panel");
        tree.transact(DockMutation::TearOff {
            panel: PanelId::new(4),
            floating_root: DockNodeId::new(6),
            monitor: MonitorId::new(1),
            bounds: DockRect::new(40, 40, 480, 320),
        })
        .expect("tear off panel");
        tree.transact(DockMutation::Redock {
            floating_root: DockNodeId::new(6),
            target: DockNodeId::new(3),
            index: 0,
        })
        .expect("redock floating panel");
        tree.transact(DockMutation::Collapse {
            group: DockNodeId::new(3),
            edge: DockEdge::Right,
        })
        .expect("collapse group");
        tree.transact(DockMutation::Expand(DockNodeId::new(3)))
            .expect("expand group");
        tree.transact(DockMutation::Maximize(PanelId::new(3)))
            .expect("maximize panel");
        tree.transact(DockMutation::RestoreMaximized)
            .expect("restore maximized");
        tree.reset(&baseline).expect("same panel set resets");
        assert_eq!(tree, baseline);
    }

    #[test]
    fn workspace_json_preserves_compatible_extensions_and_rejects_structural_drift() {
        let document = workspace_fixture();
        let json = document.canonical_json().expect("workspace encodes");
        assert!(json.ends_with('\n'));
        assert!(json.contains("0000000000000000000000000000005a"));
        assert_eq!(
            WorkspaceStateDocument::from_json(json.as_bytes()).expect("workspace parses"),
            document
        );
        let extended = json
            .replacen(
                "\"version\": 1,",
                "\"version\": 1,\n  \"future-document-field\": true,",
                1,
            )
            .replacen(
                "\"workspace\": \"world\",",
                "\"workspace\": \"world\",\n      \"future-layout-field\": {\"mode\": \"kept\"},",
                1,
            );
        let mut parsed = WorkspaceStateDocument::from_json(extended.as_bytes())
            .expect("compatible fields are retained");
        assert!(parsed.extensions.contains_key("future-document-field"));
        assert!(parsed.layouts[0]
            .extensions
            .contains_key("future-layout-field"));

        let mut replacement = parsed.layouts[0].clone();
        replacement.extensions = WorkspaceExtensions::default();
        replacement.browser_query = "updated".to_owned();
        parsed
            .save_named_layout(replacement)
            .expect("typed mutation preserves compatible fields");
        let reencoded = parsed.canonical_json().expect("extended state encodes");
        let value: serde_json::Value =
            serde_json::from_str(&reencoded).expect("canonical extension JSON");
        assert_eq!(value["future-document-field"], true);
        assert_eq!(value["layouts"][0]["future-layout-field"]["mode"], "kept");
        assert_eq!(
            WorkspaceStateDocument::from_json(reencoded.as_bytes())
                .expect("extended state round trips"),
            parsed
        );

        let mut structural_drift: serde_json::Value =
            serde_json::from_str(&json).expect("strict fixture JSON");
        structural_drift["layouts"][0]["dock"]["unknown-structural-field"] = true.into();
        let structural_drift =
            serde_json::to_vec(&structural_drift).expect("structural drift encodes");
        assert!(matches!(
            WorkspaceStateDocument::from_json(&structural_drift),
            Err(WorkspaceStateError::Decoding(_))
        ));
        assert!(matches!(
            WorkspaceStateDocument::from_json(&vec![b' '; MAX_TEXT_BYTES + 1]),
            Err(WorkspaceStateError::StateTooLarge { .. })
        ));

        let mut duplicate_scroll = document;
        let scroll = duplicate_scroll.layouts[0].scroll[0];
        duplicate_scroll.layouts[0].scroll.push(scroll);
        assert!(matches!(
            duplicate_scroll.validate(),
            Err(WorkspaceStateError::DuplicateScrollPanel(_))
        ));
    }

    #[test]
    fn legacy_state_migrates_explicitly_without_rewrite() {
        let current = workspace_fixture();
        let legacy = LegacyWorkspaceStateV0 {
            schema: LEGACY_WORKSPACE_STATE_SCHEMA.to_owned(),
            version: 0,
            session: current.session,
            active_workspace: current.active_workspace,
            layouts: current.layouts.clone(),
        };
        let bytes = serde_json::to_vec_pretty(&legacy).expect("legacy fixture encodes");
        let migrated = WorkspaceStateDocument::migrate_json(&bytes).expect("v0 migrates");
        assert_eq!(migrated.migrated_from, Some(0));
        assert_eq!(migrated.document.revision, 0);
        assert_eq!(migrated.document.active_layout_name, "Default");
        assert_eq!(migrated.document.schema, WORKSPACE_STATE_SCHEMA);
    }

    #[test]
    fn workspace_activation_toggles_focus_and_revision_overflow_rolls_back() {
        let mut document = workspace_fixture();
        assert_eq!(
            document
                .activate_workspace(WorkspaceKind::World, "Default")
                .expect("second activation enters focus"),
            WorkspaceActivation::FocusEntered
        );
        assert!(document.layouts[0].focus_layout);
        assert_eq!(
            document
                .activate_workspace(WorkspaceKind::World, "Default")
                .expect("same activation exits focus"),
            WorkspaceActivation::FocusExited
        );

        document.revision = u64::MAX;
        let before = document.clone();
        assert!(matches!(
            document.activate_workspace(WorkspaceKind::World, "Default"),
            Err(WorkspaceStateError::RevisionExhausted)
        ));
        assert_eq!(document, before);
        assert!(matches!(
            document.save_named_layout(before.layouts[0].clone()),
            Err(WorkspaceStateError::RevisionExhausted)
        ));
        assert_eq!(document, before);
    }

    #[test]
    fn pane_cycle_preserves_focus_across_primary_and_companion_windows() {
        let mut document = workspace_fixture();
        document
            .tear_off_companion(
                WorkspaceKind::World,
                "Default",
                PanelId::new(4),
                CompanionWindowId::new(1),
                MonitorId::new(1),
                DockRect::new(20, 20, 480, 320),
            )
            .expect("companion tear off");
        assert_eq!(
            document
                .cycle_panel_focus(WorkspaceKind::World, "Default", true)
                .expect("forward cycle reaches companion"),
            PanelId::new(4)
        );
        assert_eq!(
            document
                .cycle_panel_focus(WorkspaceKind::World, "Default", true)
                .expect("forward cycle wraps to primary"),
            PanelId::new(1)
        );
        assert_eq!(
            document
                .cycle_panel_focus(WorkspaceKind::World, "Default", false)
                .expect("reverse cycle wraps to companion"),
            PanelId::new(4)
        );
        let persisted = document
            .canonical_json()
            .expect("focused companion persists");
        assert_eq!(
            WorkspaceStateDocument::from_json(persisted.as_bytes())
                .expect("focused companion restores"),
            document
        );
    }

    #[test]
    fn corrupt_state_missing_panels_and_monitor_loss_recover_visibly() {
        let mut fallback = workspace_fixture();
        fallback.layouts[0]
            .dock
            .transact(DockMutation::TearOff {
                panel: PanelId::new(2),
                floating_root: DockNodeId::new(8),
                monitor: MonitorId::new(99),
                bounds: DockRect::new(5_000, 5_000, 600, 500),
            })
            .expect("offscreen float fixture");
        let known = [PanelId::new(1), PanelId::new(2), PanelId::new(3)]
            .into_iter()
            .collect::<BTreeSet<_>>();
        let outcome = WorkspaceStateDocument::recover_or_default(
            b"not-json",
            fallback,
            &known,
            &[monitor(1, true)],
        )
        .expect("fallback recovers");
        assert!(outcome.report.corrupt_state.is_some());
        assert_eq!(outcome.report.missing_panels, [PanelId::new(4)].into());
        assert_eq!(outcome.report.moved_floating, vec![DockNodeId::new(8)]);
        let floating = outcome.document.layouts[0].dock.floating[0];
        assert_eq!(floating.monitor, MonitorId::new(1));
        assert!(floating.bounds.x < 1_440);

        let mut foreign = workspace_fixture();
        foreign.session = WorkspaceSessionId::new(999);
        let foreign_json = foreign.canonical_json().expect("foreign state encodes");
        let fallback = workspace_fixture();
        let recovered = WorkspaceStateDocument::recover_or_default(
            foreign_json.as_bytes(),
            fallback.clone(),
            &known,
            &[monitor(1, true)],
        )
        .expect("session mismatch falls back");
        assert_eq!(recovered.document.session, fallback.session);
        assert!(recovered.report.corrupt_state.is_some());
        assert!(matches!(
            WorkspaceStateDocument::recover_or_default(
                b"not-json",
                fallback,
                &known,
                &[MonitorArea {
                    bounds: DockRect::new(0, 0, 20, 20),
                    ..monitor(1, true)
                }],
            ),
            Err(WorkspaceStateError::MonitorBelowAccessibleExtent(_))
        ));
        assert!(matches!(
            WorkspaceStateDocument::recover_or_default(
                b"not-json",
                workspace_fixture(),
                &known,
                &[monitor(1, true), monitor(2, true)],
            ),
            Err(WorkspaceStateError::MultiplePrimaryMonitors)
        ));
    }

    #[test]
    fn companion_windows_share_session_and_redock_without_panel_loss() {
        let mut document = workspace_fixture();
        let session = document.session;
        document
            .tear_off_companion(
                WorkspaceKind::World,
                "Default",
                PanelId::new(2),
                CompanionWindowId::new(1),
                MonitorId::new(1),
                DockRect::new(20, 20, 480, 320),
            )
            .expect("companion tear off");
        assert_eq!(document.layouts[0].companions.len(), 1);
        document
            .redock_companion(
                WorkspaceKind::World,
                "Default",
                CompanionWindowId::new(1),
                DockNodeId::new(3),
                1,
            )
            .expect("companion redocks");
        assert!(document.layouts[0].companions.is_empty());
        assert_eq!(document.layouts[0].dock.panels().len(), 4);

        let before = document.clone();
        assert!(document
            .tear_off_companion(
                WorkspaceKind::Code,
                "Missing",
                PanelId::new(2),
                CompanionWindowId::new(2),
                MonitorId::new(1),
                DockRect::new(20, 20, 480, 320),
            )
            .is_err());
        assert_eq!(document, before);
        assert_eq!(session, document.session);
    }

    #[test]
    fn state_store_round_trips_and_recovers_previous_payload() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("meridian-ui-workspace-{nonce}"));
        fs::create_dir_all(&root).expect("temporary root");
        let store = WorkspaceStateStore::new(root.join("workspace.state"));
        let mut document = workspace_fixture();
        store.save(&document).expect("initial state saves");
        document.revision += 1;
        store.save(&document).expect("replacement state saves");
        assert_eq!(store.load().expect("state loads"), document);
        fs::write(store.path(), b"corrupt").expect("primary corrupts");
        let recovered = store.load().expect("backup recovers");
        assert_eq!(recovered.revision, document.revision - 1);
        fs::remove_dir_all(root).expect("temporary root removes");
    }

    #[test]
    fn workspace_history_undo_redo_branch_and_reset_use_fresh_revisions() {
        let baseline = workspace_fixture();
        let baseline_query = baseline.layouts[0].browser_query.clone();
        let mut history = WorkspaceStateHistory::new(baseline.clone()).expect("history starts");
        let update_query = |document: &WorkspaceStateDocument, query: &str| {
            let mut next = document.clone();
            let mut layout = next.layouts[0].clone();
            query.clone_into(&mut layout.browser_query);
            next.save_named_layout(layout).expect("typed layout update");
            next
        };

        history
            .commit(update_query(history.current(), "first"))
            .expect("first checkpoint commits");
        history
            .commit(update_query(history.current(), "second"))
            .expect("second checkpoint commits");
        let second_revision = history.current().revision;
        assert_eq!(history.current().layouts[0].browser_query, "second");

        let undone = history.undo().expect("history undoes");
        assert_eq!(undone.layouts[0].browser_query, "first");
        assert!(undone.revision > second_revision);
        let undo_revision = undone.revision;
        let redone = history.redo().expect("history redoes");
        assert_eq!(redone.layouts[0].browser_query, "second");
        assert!(redone.revision > undo_revision);

        history.undo().expect("branch point restores");
        history
            .commit(update_query(history.current(), "branch"))
            .expect("branch checkpoint commits");
        assert!(!history.can_redo());
        assert!(matches!(
            history.redo(),
            Err(WorkspaceStateError::HistoryUnavailable(
                WorkspaceHistoryDirection::Redo
            ))
        ));

        let before_rejection = history.clone();
        let stale = history.current().clone();
        assert!(matches!(
            history.commit(stale),
            Err(WorkspaceStateError::HistoryRevisionNotAdvanced { .. })
        ));
        assert_eq!(history, before_rejection);

        let before_reset_revision = history.current().revision;
        let reset = history.reset().expect("baseline resets");
        assert_eq!(reset.layouts[0].browser_query, baseline_query);
        assert!(reset.revision > before_reset_revision);
        assert!(history.can_undo());
    }

    #[test]
    fn workspace_history_uses_the_existing_aggregate_state_byte_bound() {
        let mut history =
            WorkspaceStateHistory::new(workspace_fixture()).expect("bounded history starts");
        for index in 0..64 {
            let mut next = history.current().clone();
            let mut layout = next.layouts[0].clone();
            layout.browser_query = format!("{index:02}-{}", "query".repeat(300));
            next.save_named_layout(layout).expect("typed layout update");
            history.commit(next).expect("bounded checkpoint commits");
            assert!(history.retained_bytes() <= MAX_TEXT_BYTES);
        }
        assert!(history.can_undo());
        assert!(history.undo_len() < 64);
        history
            .undo()
            .expect("newest retained checkpoint remains usable");
        assert!(history.retained_bytes() <= MAX_TEXT_BYTES);
    }

    #[test]
    fn responsive_priority_preserves_pins_canvas_and_accessible_minima() {
        let regions = [
            ResponsiveRegion {
                panel: PanelId::new(1),
                priority: 2,
                preferred_extent: 240,
                compact_extent: 120,
                minimum_extent: 44,
                pinned: false,
                working_canvas: false,
                must_remain_visible: false,
            },
            ResponsiveRegion {
                panel: PanelId::new(2),
                priority: 5,
                preferred_extent: 800,
                compact_extent: 600,
                minimum_extent: 300,
                pinned: false,
                working_canvas: true,
                must_remain_visible: false,
            },
            ResponsiveRegion {
                panel: PanelId::new(3),
                priority: 1,
                preferred_extent: 344,
                compact_extent: 240,
                minimum_extent: 100,
                pinned: true,
                working_canvas: false,
                must_remain_visible: false,
            },
        ];
        let layout = adapt_responsive_regions(420, &regions).expect("responsive layout");
        assert_eq!(layout.stage, ResponsiveStage::Collapsed);
        assert_eq!(layout.gutter, TIGHT_DOCK_GUTTER);
        assert_eq!(layout.collapsed, vec![PanelId::new(1)]);
        assert_eq!(layout.minimum_required, 404);
        assert!(layout.visible.contains(&PanelId::new(2)));
        assert!(layout.visible.contains(&PanelId::new(3)));
        assert!(matches!(
            adapt_responsive_regions(
                1,
                &[ResponsiveRegion {
                    minimum_extent: 20,
                    ..regions[0]
                }]
            ),
            Err(ResponsiveError::InvalidRegion(_))
        ));
        assert_eq!(
            resolve_split_extents(1_000, 500, 300, 344).expect("split resolves"),
            DockSplitExtents {
                first: 496,
                second: 496,
            }
        );
        assert!(matches!(
            resolve_split_extents(200, 500, 100, 100),
            Err(DockError::InsufficientSplitExtent { .. })
        ));
    }

    #[test]
    fn active_error_region_forces_controlled_overflow_instead_of_hiding_recovery() {
        let regions = [
            ResponsiveRegion {
                panel: PanelId::new(1),
                priority: 1,
                preferred_extent: 240,
                compact_extent: 160,
                minimum_extent: 100,
                pinned: false,
                working_canvas: false,
                must_remain_visible: true,
            },
            ResponsiveRegion {
                panel: PanelId::new(2),
                priority: 5,
                preferred_extent: 800,
                compact_extent: 600,
                minimum_extent: 300,
                pinned: false,
                working_canvas: true,
                must_remain_visible: false,
            },
        ];

        let layout = adapt_responsive_regions(300, &regions).expect("responsive layout");
        assert_eq!(layout.stage, ResponsiveStage::ControlledOverflow);
        assert_eq!(layout.gutter, TIGHT_DOCK_GUTTER);
        assert_eq!(layout.minimum_required, 404);
        assert!(layout.collapsed.is_empty());
        assert_eq!(layout.visible, vec![PanelId::new(1), PanelId::new(2)]);
    }

    #[test]
    fn responsive_adaptation_tightens_then_shortens_before_collapsing() {
        let regions = [
            ResponsiveRegion {
                panel: PanelId::new(1),
                priority: 1,
                preferred_extent: 100,
                compact_extent: 80,
                minimum_extent: 44,
                pinned: false,
                working_canvas: false,
                must_remain_visible: false,
            },
            ResponsiveRegion {
                panel: PanelId::new(2),
                priority: 5,
                preferred_extent: 100,
                compact_extent: 80,
                minimum_extent: 44,
                pinned: false,
                working_canvas: true,
                must_remain_visible: false,
            },
        ];

        let preferred = adapt_responsive_regions(208, &regions).expect("preferred layout");
        assert_eq!(preferred.stage, ResponsiveStage::Preferred);
        assert_eq!(preferred.gutter, DOCK_GUTTER);
        assert!(preferred.shortened.is_empty());

        let tightened = adapt_responsive_regions(204, &regions).expect("tight layout");
        assert_eq!(tightened.stage, ResponsiveStage::Tightened);
        assert_eq!(tightened.gutter, TIGHT_DOCK_GUTTER);
        assert!(tightened.shortened.is_empty());

        let shortened = adapt_responsive_regions(164, &regions).expect("short layout");
        assert_eq!(shortened.stage, ResponsiveStage::LabelsShortened);
        assert_eq!(shortened.shortened, vec![PanelId::new(1), PanelId::new(2)]);
        assert!(shortened.collapsed.is_empty());

        let collapsed_after_shortening =
            adapt_responsive_regions(150, &regions).expect("collapse follows shortening");
        assert_eq!(collapsed_after_shortening.stage, ResponsiveStage::Collapsed);
        assert_eq!(collapsed_after_shortening.collapsed, vec![PanelId::new(1)]);

        let collapsed = adapt_responsive_regions(90, &regions).expect("collapsed layout");
        assert_eq!(collapsed.stage, ResponsiveStage::Collapsed);
        assert_eq!(collapsed.collapsed, vec![PanelId::new(1)]);
        assert_eq!(collapsed.visible, vec![PanelId::new(2)]);
        assert_eq!(collapsed.minimum_required, 44);
    }
}
