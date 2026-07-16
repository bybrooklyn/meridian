//! Meridian-native Creator Editor panel declarations and accessible UI document.
//!
//! Panel declarations describe presentation and commands only. Project source,
//! selections, history, recipes, and model documents remain owned by their
//! respective Meridian domain crates.

use meridian_alluvium::ProceduralRecipe;
use meridian_editor_core::EditorSession;
use meridian_modeler::{ModelDocument, ModelSelection, PenumbraPreview};
use meridian_ui::{UiDocument, UiDocumentError, UiLayout, UiNode, UiNodeId};

/// Stable Creator Editor panel identifiers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EditorPanelId {
    /// Start, project open, and recovery entry point.
    ProjectRecovery,
    /// World viewport host.
    Viewport,
    /// Hierarchical world object browser.
    Hierarchy,
    /// Generation-checked property editor.
    Inspector,
    /// Typed command history and checkpoints.
    History,
    /// Source asset listing and import entry point.
    Assets,
    /// Bounded build service output and action entry point.
    Build,
    /// Text-first Alluvium recipe surface.
    Recipe,
    /// Native editable-model source surface.
    Modeler,
    /// Typed diagnostics and recovery information.
    Diagnostics,
}

impl EditorPanelId {
    /// Returns the stable serialized panel identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProjectRecovery => "project-recovery",
            Self::Viewport => "viewport",
            Self::Hierarchy => "hierarchy",
            Self::Inspector => "inspector",
            Self::History => "history",
            Self::Assets => "assets",
            Self::Build => "build",
            Self::Recipe => "recipe",
            Self::Modeler => "modeler",
            Self::Diagnostics => "diagnostics",
        }
    }
}

/// Declarative UI metadata for one Creator Editor panel.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditorPanel {
    /// Stable serialized panel ID.
    pub id: EditorPanelId,
    /// Accessible panel name.
    pub title: &'static str,
    /// Semantic command identifiers offered by the panel.
    pub commands: &'static [&'static str],
    /// Read-only source/query domains rendered by the panel.
    pub query_dependencies: &'static [&'static str],
    /// Workspace layout hint.
    pub layout_hint: &'static str,
    /// Required capability label, empty for local editor source operations.
    pub permission: &'static str,
    /// Panel state serialization version.
    pub serialization_version: u16,
    /// Plain-language view shown before a future domain becomes available.
    pub unavailable_state: &'static str,
}

const CREATOR_ALPHA_PANELS: &[EditorPanel] = &[
    EditorPanel {
        id: EditorPanelId::ProjectRecovery,
        title: "Project and recovery",
        commands: &["editor.recover", "editor.open-project"],
        query_dependencies: &["editor.session"],
        layout_hint: "left-top",
        permission: "",
        serialization_version: 1,
        unavailable_state: "No recovery snapshot is available.",
    },
    EditorPanel {
        id: EditorPanelId::Viewport,
        title: "World viewport",
        commands: &["editor.focus-selection"],
        query_dependencies: &["editor.project.placements"],
        layout_hint: "center",
        permission: "",
        serialization_version: 1,
        unavailable_state: "No source placement is selected.",
    },
    EditorPanel {
        id: EditorPanelId::Hierarchy,
        title: "Hierarchy",
        commands: &["editor.select-placement"],
        query_dependencies: &["editor.project.placements"],
        layout_hint: "left",
        permission: "",
        serialization_version: 1,
        unavailable_state: "The world has no editable placements.",
    },
    EditorPanel {
        id: EditorPanelId::Inspector,
        title: "Inspector",
        commands: &["editor.preview-command", "editor.commit-command"],
        query_dependencies: &["editor.selection", "editor.project"],
        layout_hint: "right",
        permission: "",
        serialization_version: 1,
        unavailable_state: "Select an object to edit its source properties.",
    },
    EditorPanel {
        id: EditorPanelId::History,
        title: "History",
        commands: &["editor.undo", "editor.redo"],
        query_dependencies: &["editor.history", "editor.checkpoints"],
        layout_hint: "bottom",
        permission: "",
        serialization_version: 1,
        unavailable_state: "No source command has been committed.",
    },
    EditorPanel {
        id: EditorPanelId::Assets,
        title: "Assets and import",
        commands: &["asset.import", "asset.inspect-source"],
        query_dependencies: &["editor.project.sources"],
        layout_hint: "left-bottom",
        permission: "import-source",
        serialization_version: 1,
        unavailable_state: "No imported source is registered.",
    },
    EditorPanel {
        id: EditorPanelId::Build,
        title: "Build",
        commands: &["build.submit", "build.inspect"],
        query_dependencies: &["build.events"],
        layout_hint: "bottom",
        permission: "run-local-build",
        serialization_version: 1,
        unavailable_state: "No bounded local build has been submitted.",
    },
    EditorPanel {
        id: EditorPanelId::Recipe,
        title: "Recipe",
        commands: &[
            "procedural.validate",
            "procedural.preview",
            "procedural.bake",
            "procedural.license-audit",
        ],
        query_dependencies: &[
            "procedural.recipe",
            "procedural.cache",
            "procedural.provenance",
        ],
        layout_hint: "right-bottom",
        permission: "",
        serialization_version: 1,
        unavailable_state:
            "Select a text recipe to inspect its typed parameters and derived output.",
    },
    EditorPanel {
        id: EditorPanelId::Modeler,
        title: "Modeler",
        commands: &[
            "model.inspect-source",
            "model.create-primitive",
            "model.transform",
            "model.split-edge",
            "model.undo",
            "model.redo",
            "model.recover",
        ],
        query_dependencies: &["model.document", "model.selection", "model.preview"],
        layout_hint: "right-bottom",
        permission: "",
        serialization_version: 1,
        unavailable_state:
            "Select editable model source to inspect stable elements and semantic history.",
    },
    EditorPanel {
        id: EditorPanelId::Diagnostics,
        title: "Diagnostics",
        commands: &["editor.show-diagnostic", "editor.recover"],
        query_dependencies: &["editor.diagnostics", "editor.recovery"],
        layout_hint: "bottom",
        permission: "",
        serialization_version: 1,
        unavailable_state: "No diagnostics are active.",
    },
];

/// Returns the complete MS-03 Creator Alpha panel contract.
#[must_use]
pub fn creator_alpha_panels() -> &'static [EditorPanel] {
    CREATOR_ALPHA_PANELS
}

/// Builds the retained, semantic Creator Alpha workspace for a session.
///
/// Every interactive action is a Meridian-native focusable button with an
/// explicit semantic action. No project mutation happens while this document is
/// built; callers submit typed commands to `meridian-editor-core` at a barrier.
///
/// # Errors
///
/// Returns an error if the generated retained document violates UI semantics.
pub fn creator_alpha_document(session: &EditorSession) -> Result<UiDocument, UiDocumentError> {
    let root = UiNodeId::new(1);
    let mut nodes = Vec::new();
    let mut root_children = Vec::new();
    let selected = session.selection().ids.len();
    let project_status = format!(
        "Project generation {} with {} source(s), {} placement(s), and {selected} selection(s).",
        session.document().generation,
        session.document().sources.len(),
        session.document().placements.len(),
    );
    nodes.push(UiNode::label(
        UiNodeId::new(2),
        "Creator Alpha status",
        project_status,
    ));
    root_children.push(UiNodeId::new(2));

    for (index, panel) in creator_alpha_panels().iter().enumerate() {
        let base = 100_u128 + (index as u128 * 10);
        let panel_id = UiNodeId::new(base);
        let status_id = UiNodeId::new(base + 1);
        let mut children = vec![status_id];
        nodes.push(UiNode::label(
            status_id,
            format!("{} status", panel.title),
            panel.unavailable_state,
        ));
        for (command_index, command) in panel.commands.iter().enumerate() {
            let action_id = UiNodeId::new(base + 2 + command_index as u128);
            children.push(action_id);
            nodes.push(UiNode::button(
                action_id,
                format!("{} action", panel.title),
                *command,
                command.replace('.', " "),
            ));
        }
        nodes.push(UiNode::container(
            panel_id,
            panel.title,
            UiLayout::VerticalStack { gap: 6.0 },
            children,
        ));
        root_children.push(panel_id);
    }

    nodes.push(UiNode::container(
        root,
        "Creator Editor Alpha workspace",
        UiLayout::VerticalStack { gap: 8.0 },
        root_children,
    ));
    UiDocument::new(root, nodes)
}

/// Builds the keyboard-accessible, text-first Alluvium inspector. It exposes
/// recipe source and typed scalar parameters but never mutates recipe source;
/// callers route actions through the shared Alluvium command adapter.
///
/// # Errors
///
/// Returns an error when the generated retained document violates UI semantics.
pub fn recipe_inspector_document(recipe: &ProceduralRecipe) -> Result<UiDocument, UiDocumentError> {
    let root = UiNodeId::new(10_000);
    let details = UiNodeId::new(10_001);
    let validate = UiNodeId::new(10_002);
    let preview = UiNodeId::new(10_003);
    let bake = UiNodeId::new(10_004);
    let source = format!(
        "{}: {} placements every {} mm, strict source ID {}.",
        recipe.label, recipe.operation.count, recipe.operation.spacing_mm, recipe.id
    );
    UiDocument::new(
        root,
        vec![
            UiNode::label(details, "Alluvium recipe details", source),
            UiNode::button(
                validate,
                "Validate recipe",
                "procedural.validate",
                "validate recipe",
            ),
            UiNode::button(
                preview,
                "Preview recipe",
                "procedural.preview",
                "preview recipe",
            ),
            UiNode::button(bake, "Bake recipe", "procedural.bake", "bake recipe"),
            UiNode::container(
                root,
                "Alluvium recipe inspector",
                UiLayout::VerticalStack { gap: 6.0 },
                vec![details, validate, preview, bake],
            ),
        ],
    )
}

/// Builds the keyboard-accessible native-model source inspector. Actions are
/// semantic commands only; callers submit their typed transactions to the
/// modeler source boundary and never mutate source while constructing UI.
///
/// # Errors
///
/// Returns an error when the retained semantic tree is invalid.
pub fn model_inspector_document(
    document: &ModelDocument,
    selection: &ModelSelection,
    preview: &PenumbraPreview,
) -> Result<UiDocument, UiDocumentError> {
    let root = UiNodeId::new(20_000);
    let source = UiNodeId::new(20_001);
    let selection_status = UiNodeId::new(20_002);
    let create = UiNodeId::new(20_003);
    let transform = UiNodeId::new(20_004);
    let split = UiNodeId::new(20_005);
    let undo = UiNodeId::new(20_006);
    let redo = UiNodeId::new(20_007);
    let recover = UiNodeId::new(20_008);
    let source_details = format!(
        "{} revision {}: {} object(s), preview object {} with {} derived triangle index value(s).",
        document.label,
        document.document_generation,
        document.objects.len(),
        preview.object_id,
        preview.triangle_indices.len(),
    );
    let selection_details = format!(
        "{} selected {:?} element(s) at generation {}.",
        selection.ids.len(),
        selection.kind,
        selection.document_generation,
    );
    UiDocument::new(
        root,
        vec![
            UiNode::label(source, "Editable model source", source_details),
            UiNode::label(selection_status, "Model selection", selection_details),
            UiNode::button(
                create,
                "Create primitive",
                "model.create-primitive",
                "create primitive",
            ),
            UiNode::button(
                transform,
                "Transform selected object",
                "model.transform",
                "transform selected object",
            ),
            UiNode::button(
                split,
                "Split selected edge",
                "model.split-edge",
                "split selected edge",
            ),
            UiNode::button(undo, "Undo model edit", "model.undo", "undo model edit"),
            UiNode::button(redo, "Redo model edit", "model.redo", "redo model edit"),
            UiNode::button(
                recover,
                "Recover model source",
                "model.recover",
                "recover model source",
            ),
            UiNode::container(
                root,
                "Native model inspector",
                UiLayout::VerticalStack { gap: 6.0 },
                vec![
                    source,
                    selection_status,
                    create,
                    transform,
                    split,
                    undo,
                    redo,
                    recover,
                ],
            ),
        ],
    )
}

#[cfg(test)]
mod tests {
    use meridian_core::StableId;
    use meridian_editor_core::ProjectDocument;
    use meridian_ui::UiWidgetKind;

    use super::*;

    #[test]
    fn complete_panel_contract_has_unique_ids_and_accessible_commands() {
        let panels = creator_alpha_panels();
        assert_eq!(panels.len(), 10);
        for panel in panels {
            assert!(!panel.id.as_str().is_empty());
            assert!(!panel.title.is_empty());
            assert!(!panel.commands.is_empty());
            assert_eq!(panel.serialization_version, 1);
        }
    }

    #[test]
    fn inspector_and_history_actions_are_focusable_semantic_buttons() {
        let session = EditorSession::open(ProjectDocument::new(StableId::new(1))).expect("session");
        let document = creator_alpha_document(&session).expect("valid UI document");
        for id in [UiNodeId::new(132), UiNodeId::new(142), UiNodeId::new(143)] {
            let node = document.node(id).expect("declared action");
            assert_eq!(node.kind, UiWidgetKind::Button);
            assert!(node.focusable);
            assert!(node.semantics.action.is_some());
        }
    }

    #[test]
    fn recipe_inspector_is_keyboard_accessible_without_recipe_mutation() {
        let recipe = meridian_alluvium::ProceduralRecipe::from_json(include_str!(
            "../../../examples/creator-alpha/recipes/public-placement.mproc"
        ))
        .expect("public recipe");
        let document = recipe_inspector_document(&recipe).expect("valid inspector");
        for id in [
            UiNodeId::new(10_002),
            UiNodeId::new(10_003),
            UiNodeId::new(10_004),
        ] {
            let node = document.node(id).expect("action");
            assert_eq!(node.kind, UiWidgetKind::Button);
            assert!(node.focusable);
            assert!(node.semantics.action.is_some());
        }
    }

    #[test]
    fn model_inspector_exposes_keyboard_semantic_source_actions() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("repository root");
        let document = meridian_modeler::ModelDocument::read_source(
            root.join("examples/creator-alpha/models/public-box.model.json"),
        )
        .expect("public editable model");
        let object = document.objects.first().expect("public object");
        let selection = meridian_modeler::ModelSelection::new(
            &document,
            meridian_modeler::ModelElementKind::Object,
            [object.id],
        )
        .expect("object selection");
        let preview = document
            .penumbra_preview(object.id)
            .expect("derived preview");
        let inspector = model_inspector_document(&document, &selection, &preview)
            .expect("valid model inspector");
        for id in [20_003_u128, 20_004, 20_005, 20_006, 20_007, 20_008] {
            let node = inspector.node(UiNodeId::new(id)).expect("model action");
            assert_eq!(node.kind, UiWidgetKind::Button);
            assert!(node.focusable);
            assert!(node.semantics.action.is_some());
        }
    }
}
