//! Meridian-owned semantic output projected by private platform adapters.

use meridian_ui_core::{SemanticRole, UiNodeId, UiRect};

/// Flat semantic tree; platform adapters turn this into their native tree/delta.
#[derive(Clone, Debug, PartialEq)]
pub struct SemanticNode {
    pub id: UiNodeId,
    pub parent: Option<UiNodeId>,
    pub role: SemanticRole,
    pub name: String,
    pub action: Option<String>,
    pub value: Option<String>,
    pub bounds: UiRect,
    pub focused: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SemanticTree {
    pub nodes: Vec<SemanticNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SemanticDelta {
    Unchanged,
    Replace(SemanticTree),
}
