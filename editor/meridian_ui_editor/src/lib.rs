//! Meridian-native Creator Editor panel declarations and accessible UI document.
//!
//! Panel declarations describe presentation and commands only. Project source,
//! selections, history, recipes, and model documents remain owned by their
//! respective Meridian domain crates.

mod workspace;

pub use workspace::*;

use std::sync::Arc;

use meridian_alluvium::ProceduralRecipe;
use meridian_editor_core::{EditorSession, WorldPlacement};
use meridian_modeler::{ModelDocument, ModelSelection, PenumbraPreview};
use meridian_ui::{
    DisplayList, DisplayListError, DisplayPrimitive, IconId, UiAlignment, UiAuthoredStyle,
    UiBorder, UiColor, UiComponentDefinition, UiComponentId, UiConstraints, UiCornerRadii,
    UiDocument, UiDocumentCompiler, UiDocumentError, UiElevation, UiFontRole, UiFontWeight,
    UiFrameInput, UiFrameOutput, UiGlyphBitmap, UiLayout, UiLayoutHints, UiNode, UiNodeId,
    UiNodeSource, UiPathCommand, UiPoint, UiRect, UiSize, UiStroke, UiStyle, UiStyleId,
    UiStyleVariant, UiTextInputOptions, UiTextLayout, UiTextRaster,
};

/// Stable node for the Creator hub's project-name field.
pub const CREATOR_HUB_PROJECT_NAME: UiNodeId = UiNodeId::new(90_002);

/// Stable editable X-coordinate field for the selected Creator placement.
pub const CREATOR_INSPECTOR_X_MM: UiNodeId = UiNodeId::new(91_001);
/// Stable editable Y-coordinate field for the selected Creator placement.
pub const CREATOR_INSPECTOR_Y_MM: UiNodeId = UiNodeId::new(91_002);
/// Stable editable Z-coordinate field for the selected Creator placement.
pub const CREATOR_INSPECTOR_Z_MM: UiNodeId = UiNodeId::new(91_003);
/// Stable source-derived canvas hosted by the World viewport panel.
pub const CREATOR_WORLD_VIEWPORT_CANVAS: UiNodeId = UiNodeId::new(162);
/// Stable derived-frame preview hosted by the UI authoring workspace.
///
/// This is a compiled display-list inspection surface, not the deferred
/// direct-manipulation canvas editor.
pub const CREATOR_UI_AUTHORING_PREVIEW_CANVAS: UiNodeId = UiNodeId::new(96_238);
/// Stable source-derived mesh preview hosted by the Modeler workspace.
pub const CREATOR_MODELER_PREVIEW_CANVAS: UiNodeId = UiNodeId::new(96_180);
/// Stable browser search field for the World workspace.
pub const CREATOR_WORLD_SEARCH: UiNodeId = UiNodeId::new(92_050);
/// Stable search field for Meridian-owned application preferences.
pub const CREATOR_SETTINGS_SEARCH: UiNodeId = UiNodeId::new(90_130);
/// Stable search field shared by non-World Creator workspaces.
pub const CREATOR_DOMAIN_SEARCH: UiNodeId = UiNodeId::new(92_059);

const MAX_CREATOR_HUB_RECENT_ROWS: usize = 5;

/// One bounded recent-project entry rendered by the Creator hub.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecentProjectView {
    /// Human-readable project label.
    pub label: String,
    /// Non-authoritative display path; opening still revalidates source.
    pub path: String,
    /// Whether the saved path currently contains an openable Creator project.
    pub available: bool,
}

/// Derived width class for Code's initial World-and-source split.
///
/// This presentation value belongs to the current viewport only. It never
/// changes project source or persistent workspace authority.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CodeContextWidth {
    Compact,
    #[default]
    Standard,
    Wide,
}

/// Read-only, non-authoritative facts rendered by the live Creator workspace.
///
/// The editor application assembles this view from its domain-owned session,
/// build, recipe, modeler, and recovery state at a reconciliation boundary.
/// It carries presentation text only and cannot mutate project source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatorWorkspaceView {
    pub project: String,
    pub activity: String,
    pub recovery: String,
    pub build: String,
    pub recipe: String,
    pub model: String,
    /// The application-owned workspace currently rendered by the shell.
    pub workspace: WorkspaceKind,
    /// Whether the active workspace is in its remembered focused layout.
    pub focus_layout: bool,
    /// Presentation-only width class for Code's first-activation split.
    ///
    /// At the narrow review/runtime breakpoint, the lower-priority file browser
    /// yields to the live World and source work surfaces. It is derived from
    /// the current viewport and is never stored as project or workspace source.
    pub code_context_width: CodeContextWidth,
    /// Presentation-only responsive state for the World workspace.
    ///
    /// At the narrow review/runtime breakpoint, the activity rail yields so
    /// the source browser, live viewport, and editable inspector can remain
    /// adjacent without any panel encroaching on another work surface.
    pub compact_world_context: bool,
    /// Presentation-only responsive state for the UI authoring workspace.
    ///
    /// At the compact breakpoint the activity rail yields and the source and
    /// token inspectors tighten so the compiled preview remains a real,
    /// readable work surface instead of a postage stamp.
    pub compact_ui_authoring: bool,
    /// Persisted dock pane currently selected for keyboard pane cycling.
    pub focused_panel: Option<EditorPanelId>,
    /// Bounded, read-only canonical project source shown by Code.
    pub project_source: String,
    /// Bounded, read-only canonical recipe source shown by Alluvium.
    pub recipe_source: String,
    /// Read-only source and derived-preview facts shown by the native Modeler.
    ///
    /// The preview descriptor is generated from the immutable model revision;
    /// it is never editable source and carries no renderer handle.
    pub modeler: Option<CreatorModelerPresentation>,
}

/// A bounded, source-derived presentation of the current editable-model revision.
///
/// This data crosses only the Creator presentation boundary. Model source,
/// history, selection, and mutations stay owned by `meridian-modeler`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatorModelerPresentation {
    pub generation: u64,
    pub document_label: String,
    pub object_label: String,
    pub object_count: usize,
    pub vertex_count: usize,
    pub edge_count: usize,
    pub face_count: usize,
    pub preview: Option<PenumbraPreview>,
}

impl CreatorWorkspaceView {
    /// Builds a conservative view for UI-only fixtures that have no live tool state.
    #[must_use]
    pub fn foundation(session: &EditorSession, activity: impl Into<String>) -> Self {
        Self {
            project: "Meridian Project".to_owned(),
            activity: activity.into(),
            recovery: format!(
                "Source generation {} is open; no recovery detail was supplied.",
                session.document().generation
            ),
            build: "No build status was supplied by the Creator host.".to_owned(),
            recipe: "No recipe detail was supplied by the Creator host.".to_owned(),
            model: "No model detail was supplied by the Creator host.".to_owned(),
            workspace: WorkspaceKind::World,
            focus_layout: false,
            code_context_width: CodeContextWidth::Standard,
            compact_world_context: false,
            compact_ui_authoring: false,
            focused_panel: None,
            project_source: "No canonical project source was supplied by the Creator host."
                .to_owned(),
            recipe_source: "No canonical recipe source was supplied by the Creator host."
                .to_owned(),
            modeler: None,
        }
    }
}

/// Read-only application-preference facts rendered by the Creator Settings surface.
///
/// Preference mutation remains in `meridian-editor`; this view deliberately
/// contains no project or platform-adapter authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatorSettingsView {
    /// Current project label when Settings temporarily covers an open session.
    pub project: Option<String>,
    /// Whether that suspended project owns an isolated Play session.
    pub play_active: bool,
    pub high_contrast: bool,
    pub reduced_motion: bool,
    pub density: String,
    /// Current retained local preference query; never project source.
    pub query: String,
    pub status: String,
}

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
        commands: &[
            "editor.recover",
            "editor.return-hub",
            "editor.play-start",
            "editor.play-apply",
            "editor.play-discard",
        ],
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
        commands: &["editor.preview-command", "editor.edit-placement"],
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
        commands: &["asset.reimport", "asset.inspect-source"],
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
            "procedural.inspect",
            "procedural.validate",
            "procedural.preview",
            "procedural.bake",
            "procedural.explain",
            "procedural.dirty",
            "procedural.migrate",
            "procedural.provenance",
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

fn transparent_group(
    id: UiNodeId,
    name: impl Into<String>,
    layout: UiLayout,
    children: Vec<UiNodeId>,
) -> UiNode {
    UiNode::container(id, name, layout, children).with_style(UiStyle::transparent())
}

fn fixed_height(node: UiNode, height: f32) -> UiNode {
    node.with_layout_hints(UiLayoutHints::fixed_height(height))
}

fn fixed_width(node: UiNode, width: f32) -> UiNode {
    node.with_layout_hints(UiLayoutHints::fixed_width(width))
}

fn fixed_size(node: UiNode, width: f32, height: f32) -> UiNode {
    node.with_layout_hints(UiLayoutHints::fixed_size(width, height))
}

/// The shared Creator vocabulary is deliberately small: each instance keeps a
/// stable node identity and its own semantics, while its visual treatment
/// resolves through the canonical authored source contract.  These IDs are
/// document-source identities, not renderer handles or positional IDs.
fn creator_authored_vocabulary() -> [(UiStyleId, UiComponentId, UiStyleVariant); 15] {
    [
        (
            UiStyleId::new(97_001),
            UiComponentId::new(97_101),
            UiStyleVariant::Panel,
        ),
        (
            UiStyleId::new(97_002),
            UiComponentId::new(97_102),
            UiStyleVariant::Text,
        ),
        (
            UiStyleId::new(97_003),
            UiComponentId::new(97_103),
            UiStyleVariant::Transparent,
        ),
        (
            UiStyleId::new(97_004),
            UiComponentId::new(97_104),
            UiStyleVariant::Canvas,
        ),
        (
            UiStyleId::new(97_005),
            UiComponentId::new(97_105),
            UiStyleVariant::Surface,
        ),
        (
            UiStyleId::new(97_006),
            UiComponentId::new(97_106),
            UiStyleVariant::ElevatedSurface,
        ),
        (
            UiStyleId::new(97_007),
            UiComponentId::new(97_107),
            UiStyleVariant::Heading,
        ),
        (
            UiStyleId::new(97_008),
            UiComponentId::new(97_108),
            UiStyleVariant::SectionHeading,
        ),
        (
            UiStyleId::new(97_009),
            UiComponentId::new(97_109),
            UiStyleVariant::MutedText,
        ),
        (
            UiStyleId::new(97_010),
            UiComponentId::new(97_110),
            UiStyleVariant::PrimaryAction,
        ),
        (
            UiStyleId::new(97_011),
            UiComponentId::new(97_111),
            UiStyleVariant::DestructiveAction,
        ),
        (
            UiStyleId::new(97_012),
            UiComponentId::new(97_112),
            UiStyleVariant::SecondaryAction,
        ),
        (
            UiStyleId::new(97_013),
            UiComponentId::new(97_113),
            UiStyleVariant::CompactAction,
        ),
        (
            UiStyleId::new(97_014),
            UiComponentId::new(97_114),
            UiStyleVariant::TextField,
        ),
        (
            UiStyleId::new(97_015),
            UiComponentId::new(97_115),
            UiStyleVariant::CompactTextField,
        ),
    ]
}

/// Compiles one Creator view from the same canonical authored source that a
/// future UI-authoring workspace inspects.  Existing compatibility treatments
/// that exactly equal a locked variant are upgraded into component instances;
/// Creator's more specific dense-shell treatments remain explicit source nodes
/// and continue through the framework's token-resolution boundary.
fn creator_authored_document(
    root: UiNodeId,
    nodes: Vec<UiNode>,
) -> Result<UiDocument, UiDocumentError> {
    let vocabulary = creator_authored_vocabulary();
    let mut builder = UiDocument::authoring(root);
    for (style, component, variant) in vocabulary {
        builder = builder
            .with_style(UiAuthoredStyle::new(style, variant))
            .with_component(UiComponentDefinition::new(component, style));
    }
    for node in nodes {
        // Elevation is deliberate hierarchy, not a default decoration. The
        // primary canvas or an intentional floating surface opts in at its
        // composition site; making every dock panel raised flattened the
        // workbench into equally loud, unrelated cards.
        // Text weight is part of the authored component vocabulary, not a
        // renderer preference. Give controls their constructor-defined medium
        // face; upgrade only primary headings and primary values so secondary
        // metadata remains quiet at dense editor scale.
        let node = if node.font_weight == UiFontWeight::Normal {
            let is_primary = node.style.foreground == UiColor::text();
            let is_field_label = node.style.foreground == UiColor::secondary_text()
                && (node.style.font_size - 12.0).abs() <= f32::EPSILON;
            let weight = if is_primary && node.style.font_size >= 18.0 {
                Some(UiFontWeight::Semibold)
            } else if (is_primary && node.style.font_size >= 14.0) || is_field_label {
                Some(UiFontWeight::Medium)
            } else {
                None
            };
            weight.map_or(node.clone(), |weight| node.with_font_weight(weight))
        } else {
            node
        };
        let component =
            creator_authored_vocabulary()
                .into_iter()
                .find_map(|(_, component, variant)| {
                    (node.style == variant.compatibility_style()).then_some(component)
                });
        builder = if let Some(component) = component {
            builder.instantiate(node, component)
        } else {
            builder.with_authored_node(node, UiNodeSource::plain())
        };
    }
    builder.build()
}

/// Read-only facts projected from the exact authored document selected by the
/// initial UI-authoring workspace. This is not a second source model: it is a
/// bounded inspection view over canonical [`UiDocument`] source and its
/// derived frame, rebuilt whenever Creator rebuilds the workspace.
#[derive(Clone, Debug, Eq, PartialEq)]
struct CreatorUiAuthoringInspection {
    document_name: String,
    schema_version: u16,
    source_nodes: usize,
    authored_styles: usize,
    component_definitions: usize,
    component_instance_count: usize,
    component_instances: Vec<String>,
    packaged_assets: usize,
    display_primitives: usize,
    semantic_nodes: usize,
    compact_display_primitives: usize,
    compact_semantic_nodes: usize,
    hidpi_display_primitives: usize,
}

/// Builds the initial read-only target for the UI workspace from the real
/// World document. The target remains a `UiDocument` through source recovery
/// and compilation; only the resulting bounded facts cross into Creator's
/// inspection controls.
fn inspect_creator_world_document(
    session: &EditorSession,
    view: &CreatorWorkspaceView,
) -> Result<CreatorUiAuthoringInspection, UiDocumentError> {
    let mut target_view = view.clone();
    target_view.workspace = WorkspaceKind::World;
    target_view.focus_layout = false;
    target_view.focused_panel = None;
    let document = creator_world_workspace_document_with_view(session, &target_view)?;
    let compile = |viewport: UiSize, scale_factor: f32| {
        let mut input = UiFrameInput::new(viewport);
        input.scale_factor = scale_factor;
        UiDocumentCompiler::new(document.clone())
            .compile(input)
            .frame
    };
    let frame = compile(UiSize::new(1280.0, 800.0), 1.0);
    let compact_frame = compile(UiSize::new(1024.0, 720.0), 1.0);
    let hidpi_frame = compile(UiSize::new(1280.0, 800.0), 2.0);
    let source = document.canonical_source_snapshot();
    let component_instance_count = document.component_instances().count();
    let component_instances = document
        .component_instances()
        .map(|instance| {
            let name = document
                .node(instance.root)
                .map_or("Unnamed component", |node| node.semantics.name.as_str());
            let inspection_name = name
                .replace("Selected placement ", "")
                .replace("Selected ", "")
                .replace(" coordinate", "")
                .replace(" in millimetres", "")
                .replace(" workspace", "");
            bounded_text(&inspection_name, 24)
        })
        .take(3)
        .collect();
    Ok(CreatorUiAuthoringInspection {
        document_name: document.node(document.root()).map_or_else(
            || "World workspace".to_owned(),
            |node| node.semantics.name.clone(),
        ),
        schema_version: document.schema().version,
        source_nodes: source.nodes.len(),
        authored_styles: source.styles.len(),
        component_definitions: source.components.len(),
        component_instance_count,
        component_instances,
        packaged_assets: source
            .node_sources
            .iter()
            .filter(|(_, source)| source.asset.is_some())
            .count(),
        display_primitives: frame.display_list.primitives.len(),
        semantic_nodes: frame.semantic_tree.nodes.len(),
        compact_display_primitives: compact_frame.display_list.primitives.len(),
        compact_semantic_nodes: compact_frame.semantic_tree.nodes.len(),
        hidpi_display_primitives: hidpi_frame.display_list.primitives.len(),
    })
}

/// Compiles the real World document selected by the initial UI-authoring
/// workspace. The returned frame is derived inspection input only; it has no
/// source authority and cannot be edited through the preview.
///
/// # Errors
///
/// Returns the normal retained-document diagnostic before any preview can be
/// emitted when the selected World source is invalid.
pub fn creator_ui_authoring_target_frame(
    session: &EditorSession,
    view: &CreatorWorkspaceView,
) -> Result<UiFrameOutput, UiDocumentError> {
    let mut target_view = view.clone();
    target_view.workspace = WorkspaceKind::World;
    target_view.focus_layout = false;
    target_view.focused_panel = None;
    let document = creator_world_workspace_document_with_view(session, &target_view)?;
    Ok(UiDocumentCompiler::new(document)
        .compile(UiFrameInput::new(UiSize::new(1280.0, 800.0)))
        .frame)
}

fn shell_row_style(background: UiColor, padding: f32) -> UiStyle {
    UiStyle {
        background: Some(background),
        // The application shell is a layered frame, not three adjacent
        // rectangles. Reserve one-pixel borders for interactive controls and
        // real work surfaces; the palette change and tab tray establish the
        // shell rows without drawing a box around every band.
        border: None,
        corner_radius: 0.0,
        foreground: UiColor::foreground(),
        padding,
        font_size: 14.0,
    }
}

fn shell_brand_style() -> UiStyle {
    UiStyle {
        background: None,
        border: None,
        corner_radius: 0.0,
        foreground: UiColor::text(),
        padding: 4.0,
        font_size: 16.0,
    }
}

fn shell_project_style() -> UiStyle {
    UiStyle {
        background: None,
        border: None,
        corner_radius: 0.0,
        foreground: UiColor::secondary_text(),
        padding: 4.0,
        font_size: 13.0,
    }
}

fn shell_utility_style(strong: bool, active: bool) -> UiStyle {
    UiStyle {
        background: (strong || active).then_some(UiColor::rgba(
            UiColor::amber().red,
            UiColor::amber().green,
            UiColor::amber().blue,
            0.14,
        )),
        border: Some(UiBorder {
            color: if strong || active {
                UiColor::amber()
            } else {
                UiColor::border()
            },
            width: 1,
        }),
        corner_radius: 6.0,
        foreground: if strong || active {
            UiColor::text()
        } else {
            UiColor::secondary_text()
        },
        padding: 4.0,
        font_size: 12.0,
    }
}

/// Compact icon actions belong to one small command cluster. Keeping their
/// individual surfaces transparent removes the accidental "three empty form
/// fields" look while retaining full target size and semantics.
fn shell_icon_action_style() -> UiStyle {
    UiStyle {
        background: None,
        border: None,
        corner_radius: 4.0,
        foreground: UiColor::secondary_text(),
        padding: 4.0,
        font_size: 12.0,
    }
}

fn shell_icon_cluster_style() -> UiStyle {
    UiStyle {
        background: Some(UiColor::background()),
        border: Some(UiBorder {
            color: UiColor::border(),
            width: 1,
        }),
        corner_radius: 6.0,
        foreground: UiColor::foreground(),
        padding: 4.0,
        font_size: 12.0,
    }
}

fn workspace_tab_style(selected: bool) -> UiStyle {
    UiStyle {
        background: selected.then_some(UiColor::rgba(
            UiColor::amber().red,
            UiColor::amber().green,
            UiColor::amber().blue,
            0.12,
        )),
        border: selected.then_some(UiBorder {
            color: UiColor::amber(),
            width: 1,
        }),
        corner_radius: if selected { 6.0 } else { 0.0 },
        foreground: if selected {
            UiColor::text()
        } else {
            UiColor::secondary_text()
        },
        // The 36 px workspace row reserves 24 px inside its segmented tray.
        // Four-pixel inset keeps the 12 px UI face within that slot at 1x and
        // 2x, instead of clipping tab labels behind oversized chrome padding.
        padding: 4.0,
        font_size: 12.0,
    }
}

/// The workspace strip is a single restrained segmented surface. Individual
/// tabs remain light-weight; this avoids a row of unrelated pills while making
/// the current workspace immediately legible.
fn workspace_tab_strip_style() -> UiStyle {
    UiStyle {
        background: Some(UiColor::surface()),
        border: Some(UiBorder {
            color: UiColor::border(),
            width: 1,
        }),
        corner_radius: 10.0,
        foreground: UiColor::foreground(),
        // The outer row already provides the four-pixel inset. Keeping the
        // segmented tray flush preserves a full 20 px text slot in its 36 px
        // row, rather than double-padding and clipping the tab face.
        padding: 0.0,
        font_size: 12.0,
    }
}

fn world_panel_style(radius: f32) -> UiStyle {
    UiStyle {
        background: Some(UiColor::surface()),
        border: Some(UiBorder {
            // Browser and inspector seams should establish containment without
            // competing with the canvas. The exact border token remains the
            // source colour; reduced opacity only lowers passive chrome weight.
            color: UiColor::rgba(0.160_784_32, 0.176_470_6, 0.172_549_02, 0.72),
            width: 1,
        }),
        corner_radius: radius,
        foreground: UiColor::foreground(),
        padding: 12.0,
        font_size: 14.0,
    }
}

/// Activity rails are deliberately narrower than ordinary tool panels. They
/// need a full icon target inside a quiet opaque surface, so they use the
/// locked four-pixel inset instead of inheriting the twelve-pixel panel inset
/// that would crush a 44 px rail down to a 20 px content column.
fn creator_activity_rail_style() -> UiStyle {
    UiStyle {
        background: Some(UiColor::surface()),
        border: Some(UiBorder {
            color: UiColor::border(),
            width: 1,
        }),
        corner_radius: 10.0,
        foreground: UiColor::foreground(),
        padding: 4.0,
        font_size: 12.0,
    }
}

/// The persistent World shelf is a low-profile status surface by default.
/// It deliberately has less inset than a tool panel so it reads as a status
/// strip, not a second work area competing with the viewport.
fn world_shelf_style() -> UiStyle {
    let mut style = world_panel_style(10.0);
    style.padding = 4.0;
    style
}

fn focused_world_panel_style(radius: f32, focused: bool) -> UiStyle {
    let mut style = world_panel_style(radius);
    if focused {
        style.border = Some(UiBorder {
            color: UiColor::amber(),
            width: 1,
        });
    }
    style
}

fn active_panel(view: &CreatorWorkspaceView, panels: &[EditorPanelId]) -> bool {
    view.focused_panel
        .is_some_and(|focused| panels.contains(&focused))
}

fn activity_shelf_section_style() -> UiStyle {
    UiStyle {
        background: None,
        border: None,
        corner_radius: 0.0,
        foreground: UiColor::foreground(),
        padding: 8.0,
        font_size: 13.0,
    }
}

fn activity_shelf_divider_style() -> UiStyle {
    UiStyle {
        background: Some(UiColor::border()),
        border: None,
        corner_radius: 0.0,
        foreground: UiColor::foreground(),
        padding: 0.0,
        font_size: 1.0,
    }
}

/// Panel headers use a single quiet hairline to separate orientation from
/// dense tools. It is deliberately not a rounded section card or a second
/// shadow, so the browser and inspector stay compact and easy to scan.
fn panel_header_divider_style() -> UiStyle {
    UiStyle {
        background: Some(UiColor::border()),
        border: None,
        corner_radius: 0.0,
        foreground: UiColor::foreground(),
        padding: 0.0,
        font_size: 1.0,
    }
}

fn world_canvas_style() -> UiStyle {
    UiStyle {
        // The canvas is the deep work surface inside the framed viewport. Its
        // contrast carries that boundary; repeating the outer panel border and
        // ten-pixel radius made the centre read as a card inside a card.
        background: Some(UiColor::background()),
        border: None,
        corner_radius: 6.0,
        foreground: UiColor::secondary_text(),
        padding: 0.0,
        font_size: 12.0,
    }
}

fn creator_preview_canvas_style() -> UiStyle {
    UiStyle {
        // The UI-authoring canvas is a stage for a derived frame, not the
        // frame itself. A restrained intermediate surface keeps the compiled
        // preview legible against the outer panel while the preview preserves
        // the authored deep-canvas token inside it.
        background: Some(UiColor::rgba(0.055, 0.063, 0.063, 1.0)),
        border: Some(UiBorder {
            color: UiColor::rgba(0.160_784_32, 0.176_470_6, 0.172_549_02, 0.72),
            width: 1,
        }),
        corner_radius: 6.0,
        foreground: UiColor::secondary_text(),
        padding: 0.0,
        font_size: 12.0,
    }
}

fn creator_authoring_state_style(selected: bool) -> UiStyle {
    UiStyle {
        background: Some(if selected {
            UiColor::rgba(0.752_941_2, 0.588_235_3, 0.305_882_36, 0.12)
        } else {
            UiColor::background()
        }),
        border: Some(UiBorder {
            color: if selected {
                UiColor::amber()
            } else {
                UiColor::border()
            },
            width: 1,
        }),
        corner_radius: 6.0,
        foreground: if selected {
            UiColor::text()
        } else {
            UiColor::secondary_text()
        },
        padding: 8.0,
        font_size: 13.0,
    }
}

fn world_section_style(accent: UiColor) -> UiStyle {
    UiStyle {
        background: Some(UiColor::background()),
        border: Some(UiBorder {
            color: accent,
            width: 1,
        }),
        corner_radius: 6.0,
        foreground: UiColor::foreground(),
        padding: 8.0,
        font_size: 14.0,
    }
}

/// Quiet centre-state treatment for a genuinely unavailable domain. The accent
/// belongs to the eyebrow, not a loud border around the whole message: this
/// keeps an honest unavailable state calm and intentional instead of making it
/// look like an error or a fake feature panel.
fn domain_state_style() -> UiStyle {
    UiStyle {
        background: Some(UiColor::surface()),
        border: Some(UiBorder {
            color: UiColor::border(),
            width: 1,
        }),
        corner_radius: 14.0,
        foreground: UiColor::foreground(),
        padding: 20.0,
        font_size: 14.0,
    }
}

/// An unavailable domain still needs a real work surface, but that surface is
/// a dark stage rather than a second card behind the centred state. The single
/// raised state card then carries the message without creating a stack of
/// competing radii and borders.
fn domain_stage_style() -> UiStyle {
    UiStyle {
        // Keep unavailable workspaces visibly framed without adding a second
        // bordered card around the honest state card. The intermediate stage
        // tone gives the empty surface a deliberate plane against the shell.
        background: Some(UiColor::rgba(0.055, 0.063, 0.063, 1.0)),
        border: None,
        corner_radius: 10.0,
        foreground: UiColor::foreground(),
        padding: 16.0,
        font_size: 14.0,
    }
}

fn domain_state_eyebrow_style(accent: UiColor) -> UiStyle {
    UiStyle {
        background: None,
        border: None,
        corner_radius: 0.0,
        foreground: accent,
        padding: 0.0,
        font_size: 12.0,
    }
}

/// Sparse-domain capability notes are supporting information, not alert
/// banners. The amber type preserves Meridian's emphasis colour without
/// turning an honest unavailable state into a warning-shaped panel.
fn domain_capability_note_style() -> UiStyle {
    UiStyle {
        background: None,
        border: None,
        corner_radius: 0.0,
        foreground: UiColor::amber(),
        padding: 0.0,
        font_size: 13.0,
    }
}

/// Dense source/provenance grouping inside a primary panel. It deliberately
/// avoids a second rounded card: hierarchy and whitespace do the grouping,
/// while the surrounding panel remains the only surface boundary.
fn world_subsection_style() -> UiStyle {
    UiStyle {
        background: None,
        border: None,
        corner_radius: 0.0,
        foreground: UiColor::foreground(),
        padding: 0.0,
        font_size: 14.0,
    }
}

fn world_tree_item_style(selected: bool) -> UiStyle {
    UiStyle {
        background: selected.then_some(UiColor::background()),
        border: selected.then_some(UiBorder {
            color: UiColor::amber(),
            width: 1,
        }),
        corner_radius: if selected { 4.0 } else { 0.0 },
        foreground: if selected {
            UiColor::text()
        } else {
            UiColor::secondary_text()
        },
        padding: 4.0,
        font_size: 13.0,
    }
}

/// Tree hierarchy is carried by explicit labels and retained semantic state,
/// not by text glyphs that impersonate disclosure controls or file icons. The
/// compact group face makes the source browser easy to scan without turning
/// every row into a rounded card.
fn world_tree_group_style() -> UiStyle {
    UiStyle {
        background: None,
        border: None,
        corner_radius: 0.0,
        foreground: UiColor::text(),
        padding: 4.0,
        font_size: 12.0,
    }
}

/// Tree leaves retain the same dense vertical geometry as branches. The
/// runtime supplies their leading column so they align with the text face of a
/// native disclosure icon without using source whitespace or a 30 px all-side
/// padding that could force text through a fixed-height row.
fn world_tree_child_style(selected: bool) -> UiStyle {
    world_tree_item_style(selected)
}

fn world_tree_selected_group_style() -> UiStyle {
    let mut style = world_tree_group_style();
    style.background = Some(UiColor::background());
    style.border = Some(UiBorder {
        color: UiColor::amber(),
        width: 1,
    });
    style.corner_radius = 4.0;
    style
}

/// Native vector disclosure is part of the retained tree component rather
/// than a look-alike Unicode character embedded in display text.
fn creator_tree_group_item(
    id: UiNodeId,
    name: impl Into<String>,
    action: impl Into<String>,
    selected: bool,
    expanded: bool,
) -> UiNode {
    UiNode::tree_branch(id, name, action, selected, expanded).with_style(if selected {
        world_tree_selected_group_style()
    } else {
        world_tree_group_style()
    })
}

fn creator_tree_child_item(
    id: UiNodeId,
    name: impl Into<String>,
    action: impl Into<String>,
    selected: bool,
) -> UiNode {
    UiNode::tree_item(id, name, action, selected, false)
        .with_style(world_tree_child_style(selected))
}

fn status_row_style() -> UiStyle {
    UiStyle {
        background: Some(UiColor::surface()),
        border: None,
        corner_radius: 0.0,
        foreground: UiColor::secondary_text(),
        padding: 0.0,
        font_size: 12.0,
    }
}

const SHELL_APPLICATION_ROW: UiNodeId = UiNodeId::new(92_000);
const SHELL_WORKSPACE_ROW: UiNodeId = UiNodeId::new(92_020);
const SHELL_STATUS_ROW: UiNodeId = UiNodeId::new(92_040);

fn push_shell_utility_cluster(nodes: &mut Vec<UiNode>, project_open: bool) -> UiNodeId {
    let build = UiNodeId::new(92_004);
    let search = UiNodeId::new(92_005);
    let settings = UiNodeId::new(92_006);
    let utilities = UiNodeId::new(92_009);
    let mut utility_children = Vec::new();
    if project_open {
        nodes.push(fixed_width(
            UiNode::icon_button(build, "Build project", "build.submit", IconId::Build)
                .with_style(shell_icon_action_style()),
            32.0,
        ));
        utility_children.push(build);
    }
    nodes.push(fixed_width(
        UiNode::icon_button(search, "Search Meridian", "shell.search", IconId::Search)
            .with_style(shell_icon_action_style()),
        32.0,
    ));
    utility_children.push(search);
    nodes.push(fixed_width(
        UiNode::icon_button(
            settings,
            "Open Meridian settings",
            "shell.settings",
            IconId::Settings,
        )
        .with_style(shell_icon_action_style()),
        32.0,
    ));
    utility_children.push(settings);
    nodes.push(fixed_width(
        UiNode::container(
            utilities,
            "Meridian utility commands",
            UiLayout::HorizontalStack { gap: 0.0 },
            utility_children,
        )
        .with_style(shell_icon_cluster_style()),
        // Each icon keeps the 44px accessible target promised by the runtime;
        // the cluster owns their combined width instead of clipping the last
        // control at the narrow Creator breakpoint.
        if project_open { 140.0 } else { 96.0 },
    ));
    utilities
}

fn push_application_row(
    nodes: &mut Vec<UiNode>,
    project_label: &str,
    project_open: bool,
    play_active: bool,
) -> UiNodeId {
    let brand = UiNodeId::new(92_001);
    let spacer = UiNodeId::new(92_002);
    let play = UiNodeId::new(if play_active { 92_007 } else { 92_003 });
    let project = UiNodeId::new(92_008);
    let utilities = push_shell_utility_cluster(nodes, project_open);
    nodes.push(fixed_width(
        UiNode::button(
            brand,
            "Return to Meridian projects",
            "editor.return-hub",
            "Meridian",
        )
        .with_style(shell_brand_style())
        .with_font_weight(UiFontWeight::Semibold),
        112.0,
    ));
    nodes.push(fixed_width(
        UiNode::label(project, "Active Meridian project", project_label)
            .with_style(shell_project_style()),
        180.0,
    ));
    nodes.push(
        transparent_group(
            spacer,
            "Application command spacer",
            UiLayout::Overlay,
            Vec::new(),
        )
        .with_layout_hints(UiLayoutHints::flexible()),
    );
    if project_open {
        nodes.push(fixed_width(
            UiNode::button(
                play,
                if play_active {
                    "Stop Play"
                } else {
                    "Start Play"
                },
                if play_active {
                    "editor.play-discard"
                } else {
                    "editor.play-start"
                },
                if play_active { "Stop" } else { "Run" },
            )
            .with_style(shell_utility_style(true, play_active)),
            72.0,
        ));
    }
    let mut application_children = vec![brand, project, spacer];
    if project_open {
        application_children.push(play);
    }
    application_children.push(utilities);
    nodes.push(fixed_height(
        UiNode::container(
            SHELL_APPLICATION_ROW,
            "Meridian application commands",
            UiLayout::HorizontalStack { gap: 8.0 },
            application_children,
        )
        .with_style(shell_row_style(UiColor::surface(), 8.0))
        .with_constraints(UiConstraints {
            horizontal_alignment: UiAlignment::Center,
            vertical_alignment: UiAlignment::Center,
            ..UiConstraints::default()
        }),
        44.0,
    ));
    SHELL_APPLICATION_ROW
}

#[allow(clippy::too_many_lines)]
fn push_workspace_row(
    nodes: &mut Vec<UiNode>,
    selected_workspace: Option<WorkspaceKind>,
) -> UiNodeId {
    let leading_spacer = UiNodeId::new(92_019);
    let tab_list = UiNodeId::new(92_030);
    let trailing_spacer = UiNodeId::new(92_029);
    let workspaces = [
        (
            92_021,
            "World",
            "workspace.world",
            WorkspaceKind::World,
            76.0,
        ),
        (
            92_022,
            "Modeler",
            "workspace.modeler",
            WorkspaceKind::Modeler,
            88.0,
        ),
        (
            92_023,
            "UI",
            "workspace.ui",
            WorkspaceKind::UiAuthoring,
            54.0,
        ),
        (92_024, "Code", "workspace.code", WorkspaceKind::Code, 66.0),
        (
            92_025,
            "Materials",
            "workspace.materials",
            WorkspaceKind::Materials,
            92.0,
        ),
        (
            92_026,
            "Alluvium",
            "workspace.alluvium",
            WorkspaceKind::Alluvium,
            94.0,
        ),
        (
            92_027,
            "Build",
            "workspace.build",
            WorkspaceKind::Build,
            66.0,
        ),
        (
            92_028,
            "Profile",
            "workspace.profile",
            WorkspaceKind::Profile,
            76.0,
        ),
    ];
    let mut tabs = Vec::new();
    for (id, label, action, workspace, width) in workspaces {
        let id = UiNodeId::new(id);
        let selected = selected_workspace == Some(workspace);
        nodes.push(fixed_width(
            UiNode::tab(id, label, action, selected)
                .with_style(workspace_tab_style(selected))
                .with_font_weight(if selected {
                    UiFontWeight::Semibold
                } else {
                    UiFontWeight::Medium
                }),
            width,
        ));
        tabs.push(id);
    }
    nodes.push(fixed_width(
        UiNode::tabs(tab_list, "Meridian workspaces", tabs).with_style(workspace_tab_strip_style()),
        660.0,
    ));
    nodes.push(
        transparent_group(
            leading_spacer,
            "Leading workspace navigation spacer",
            UiLayout::Overlay,
            Vec::new(),
        )
        .with_layout_hints(UiLayoutHints::flexible()),
    );
    nodes.push(
        transparent_group(
            trailing_spacer,
            "Trailing workspace navigation spacer",
            UiLayout::Overlay,
            Vec::new(),
        )
        .with_layout_hints(UiLayoutHints::flexible()),
    );
    nodes.push(fixed_height(
        UiNode::container(
            SHELL_WORKSPACE_ROW,
            "Meridian workspace navigation",
            UiLayout::HorizontalStack { gap: 0.0 },
            vec![leading_spacer, tab_list, trailing_spacer],
        )
        // Keep the fixed second row distinct for keyboard and responsive
        // behavior, while the darker band makes the centred workspace tray
        // read as composed navigation rather than a second application bar.
        .with_style(shell_row_style(UiColor::background(), 4.0))
        .with_constraints(UiConstraints {
            horizontal_alignment: UiAlignment::Center,
            vertical_alignment: UiAlignment::Center,
            ..UiConstraints::default()
        }),
        36.0,
    ));
    SHELL_WORKSPACE_ROW
}

fn push_status_row(
    nodes: &mut Vec<UiNode>,
    source: impl Into<String>,
    activity: impl Into<String>,
    play_active: bool,
) -> UiNodeId {
    let source_id = UiNodeId::new(92_041);
    let activity_id = UiNodeId::new(92_042);
    let play_id = UiNodeId::new(92_043);
    nodes.push(fixed_width(
        UiNode::label(source_id, "Source status", source).with_style(creator_meta_style()),
        220.0,
    ));
    nodes.push(
        UiNode::label(activity_id, "Creator activity", activity).with_style(creator_meta_style()),
    );
    let mut children = vec![source_id, activity_id];
    if play_active {
        nodes.push(fixed_width(
            UiNode::button(
                play_id,
                "Apply Play session changes",
                "editor.play-apply",
                "Apply Play changes",
            )
            .with_style(shell_utility_style(true, true)),
            144.0,
        ));
        children.push(play_id);
    }
    nodes.push(fixed_height(
        UiNode::container(
            SHELL_STATUS_ROW,
            "Meridian status",
            UiLayout::HorizontalStack { gap: 8.0 },
            children,
        )
        .with_style(status_row_style())
        .with_constraints(UiConstraints {
            horizontal_alignment: UiAlignment::Center,
            vertical_alignment: UiAlignment::Center,
            ..UiConstraints::default()
        }),
        24.0,
    ));
    SHELL_STATUS_ROW
}

fn workspace_canvas_style() -> UiStyle {
    UiStyle {
        background: Some(UiColor::background()),
        border: None,
        corner_radius: 0.0,
        foreground: UiColor::foreground(),
        padding: 0.0,
        font_size: 16.0,
    }
}

/// Creator workbenches use the locked eight-pixel dock gutter. The generic
/// application canvas intentionally has a roomier 24-pixel inset for welcome
/// and document surfaces, but applying it around a rail/browser/viewport/
/// inspector layout wastes the working canvas and makes the shell feel boxed
/// in at normal desktop widths.
fn creator_workbench_canvas_style() -> UiStyle {
    UiStyle {
        background: Some(UiColor::background()),
        border: None,
        corner_radius: 0.0,
        foreground: UiColor::foreground(),
        padding: 8.0,
        font_size: 16.0,
    }
}

fn creator_title_style() -> UiStyle {
    UiStyle {
        background: None,
        border: None,
        corner_radius: 0.0,
        foreground: UiColor::text(),
        padding: 0.0,
        font_size: 20.0,
    }
}

fn world_panel_title_style() -> UiStyle {
    UiStyle {
        background: None,
        border: None,
        corner_radius: 0.0,
        foreground: UiColor::text(),
        padding: 0.0,
        font_size: 18.0,
    }
}

fn side_panel_title_style() -> UiStyle {
    UiStyle {
        background: None,
        border: None,
        corner_radius: 0.0,
        foreground: UiColor::text(),
        padding: 0.0,
        font_size: 16.0,
    }
}

fn creator_meta_style() -> UiStyle {
    UiStyle {
        background: None,
        border: None,
        corner_radius: 0.0,
        foreground: UiColor::secondary_text(),
        padding: 0.0,
        font_size: 14.0,
    }
}

/// Readable values and first-order supporting sentences. Metadata remains
/// secondary; using it for selected source, topology, and provenance values
/// made those facts collapse into the surrounding chrome at normal density.
fn creator_value_style() -> UiStyle {
    UiStyle {
        background: None,
        border: None,
        corner_radius: 0.0,
        foreground: UiColor::text(),
        padding: 0.0,
        font_size: 14.0,
    }
}

fn activity_shelf_detail_style() -> UiStyle {
    UiStyle {
        background: None,
        border: None,
        corner_radius: 0.0,
        foreground: UiColor::secondary_text(),
        padding: 0.0,
        font_size: 13.0,
    }
}

fn activity_shelf_hint_style() -> UiStyle {
    UiStyle {
        background: None,
        border: None,
        corner_radius: 0.0,
        foreground: UiColor::muted_text(),
        padding: 0.0,
        font_size: 12.0,
    }
}

/// The persistent shelf is a compact control strip, not another form. Its
/// command faces stay transparent and rely on the retained focus/selection
/// treatment; recovery gets restrained amber type rather than a third boxed
/// button competing with the working canvas.
fn activity_shelf_action_style(emphasis: bool) -> UiStyle {
    UiStyle {
        background: None,
        border: None,
        corner_radius: 4.0,
        foreground: if emphasis {
            UiColor::amber()
        } else {
            UiColor::secondary_text()
        },
        padding: 4.0,
        font_size: 12.0,
    }
}

fn creator_hub_status_style() -> UiStyle {
    UiStyle {
        background: None,
        border: None,
        corner_radius: 0.0,
        foreground: UiColor::secondary_text(),
        padding: 0.0,
        font_size: 13.0,
    }
}

fn creator_code_source_style() -> UiStyle {
    UiStyle {
        background: None,
        border: None,
        corner_radius: 0.0,
        foreground: UiColor::text(),
        padding: 0.0,
        // Source rows are intentionally dense. Twelve pixels keeps the
        // bundled monospace face fully inside the retained 16 px line grid
        // at 1x and 2x instead of allowing a fractional glyph row to clip.
        font_size: 12.0,
    }
}

/// Muted, fixed-width gutters make a source excerpt scan like a deliberate
/// editor surface instead of a loose diagnostic paragraph. The actual source
/// remains in the adjacent text node and is never renumbered or rewritten.
fn creator_code_line_number_style() -> UiStyle {
    UiStyle {
        background: None,
        border: None,
        corner_radius: 0.0,
        foreground: UiColor::muted_text(),
        padding: 0.0,
        font_size: 12.0,
    }
}

/// One quiet source well within a Code panel. It has a crisp 4px radius so
/// source reads as an instrument, not another large rounded card.
fn creator_code_listing_style() -> UiStyle {
    UiStyle {
        background: Some(UiColor::background()),
        border: Some(UiBorder {
            color: UiColor::border(),
            width: 1,
        }),
        corner_radius: 4.0,
        foreground: UiColor::text(),
        padding: 8.0,
        font_size: 13.0,
    }
}

/// The hub is the one place where a broad, welcoming surface is useful. Its
/// children stay dense and flat so the application does not turn into nested
/// rounded cards.
fn creator_hub_surface_style() -> UiStyle {
    UiStyle {
        background: Some(UiColor::surface()),
        border: Some(UiBorder {
            color: UiColor::border(),
            width: 1,
        }),
        corner_radius: 14.0,
        foreground: UiColor::foreground(),
        padding: 24.0,
        font_size: 16.0,
    }
}

fn creator_hub_action_style(primary: bool) -> UiStyle {
    UiStyle {
        background: Some(if primary {
            UiColor::rgba(
                UiColor::amber().red,
                UiColor::amber().green,
                UiColor::amber().blue,
                0.16,
            )
        } else {
            UiColor::background()
        }),
        border: Some(UiBorder {
            color: if primary {
                UiColor::amber()
            } else {
                UiColor::border()
            },
            width: 1,
        }),
        corner_radius: 10.0,
        foreground: UiColor::text(),
        padding: 12.0,
        font_size: 15.0,
    }
}

fn creator_hub_field_label_style() -> UiStyle {
    UiStyle {
        background: None,
        border: None,
        corner_radius: 0.0,
        foreground: UiColor::secondary_text(),
        padding: 0.0,
        font_size: 12.0,
    }
}

fn creator_recent_row_style(available: bool) -> UiStyle {
    UiStyle {
        background: Some(UiColor::background()),
        border: Some(UiBorder {
            color: UiColor::border(),
            width: 1,
        }),
        corner_radius: 6.0,
        foreground: if available {
            UiColor::foreground()
        } else {
            UiColor::secondary_text()
        },
        padding: 8.0,
        font_size: 13.0,
    }
}

fn creator_compact_action_style(_panel: EditorPanelId, command: &str) -> UiStyle {
    let primary = matches!(
        command,
        "editor.play-start"
            | "editor.play-apply"
            | "editor.select-placement"
            | "editor.edit-placement"
            | "build.submit"
            | "procedural.validate"
            | "procedural.preview"
            | "model.create-primitive"
    );
    UiStyle {
        // Supporting commands remain quieter than committed actions, but all
        // real controls get a bounded face. Bare text made action rows read as
        // unfinished wireframes and gave no visual anchor for keyboard focus.
        background: Some(if primary {
            UiColor::rgba(
                UiColor::amber().red,
                UiColor::amber().green,
                UiColor::amber().blue,
                0.14,
            )
        } else {
            UiColor::background()
        }),
        border: Some(UiBorder {
            color: if primary {
                UiColor::amber()
            } else {
                UiColor::border()
            },
            width: 1,
        }),
        corner_radius: 6.0,
        foreground: UiColor::text(),
        padding: 8.0,
        font_size: 13.0,
    }
}

fn bounded_text(value: &str, maximum_chars: usize) -> String {
    let mut text = value.chars().take(maximum_chars).collect::<String>();
    if value.chars().nth(maximum_chars).is_some() {
        text.push_str("...");
    }
    text
}

/// Formats canonical source for a narrow read-only code pane without changing
/// the source supplied by the editor host. Long stable IDs and hashes have no
/// ordinary Unicode line-break opportunities, so their display copy receives
/// explicit visual line breaks at character boundaries. The semantic source
/// authority remains `CreatorWorkspaceView::project_source` unchanged.
fn code_pane_display_text(source: &str, maximum_columns: usize) -> String {
    let maximum_columns = maximum_columns.max(12);
    source
        .lines()
        .flat_map(|line| wrap_code_pane_line(line, maximum_columns))
        .collect::<Vec<_>>()
        .join("\n")
}

fn wrap_code_pane_line(line: &str, maximum_columns: usize) -> Vec<String> {
    if line.chars().count() <= maximum_columns {
        return vec![line.to_owned()];
    }

    let indentation = line
        .chars()
        .take_while(|character| character.is_whitespace())
        .collect::<String>();
    let continuation = format!("{indentation}  ");
    let mut remaining = line;
    let mut lines = Vec::new();
    let mut first = true;
    while remaining.chars().count() > maximum_columns {
        let prefix = if first { "" } else { continuation.as_str() };
        let content_columns = maximum_columns
            .saturating_sub(prefix.chars().count())
            .max(1);
        let mut boundary = remaining.len();
        let mut preferred_boundary = None;
        for (index, character) in remaining.char_indices() {
            if remaining[..index].chars().count() >= content_columns {
                boundary = index;
                break;
            }
            if matches!(character, ' ' | ',' | ':' | '{' | '}' | '[' | ']') {
                preferred_boundary = Some(index + character.len_utf8());
            }
        }
        let boundary = preferred_boundary
            .filter(|candidate| *candidate > 0 && *candidate <= boundary)
            .unwrap_or(boundary);
        let (head, tail) = remaining.split_at(boundary);
        lines.push(format!("{prefix}{head}"));
        remaining = tail.trim_start_matches(char::is_whitespace);
        first = false;
    }
    let prefix = if first { "" } else { continuation.as_str() };
    lines.push(format!("{prefix}{remaining}"));
    lines
}

/// Builds a bounded, read-only source listing with a stable visual gutter.
///
/// `source` stays authoritative in the editor host; this function only makes
/// its display excerpt easier to read in the narrow contextual Code panel.
/// The final continuation row is intentionally explicit when the preview is
/// shortened, directing users to the existing source-inspection action.
fn push_code_source_listing(
    nodes: &mut Vec<UiNode>,
    listing: UiNodeId,
    first_line_id: u128,
    source: &str,
    maximum_columns: usize,
    maximum_rows: usize,
) -> UiNodeId {
    let display = code_pane_display_text(&bounded_text(source, 2_000), maximum_columns);
    let all_lines = display.lines().collect::<Vec<_>>();
    let shown_line_count = all_lines.len().min(maximum_rows);
    let mut rows = Vec::new();
    for (index, line) in all_lines.iter().take(shown_line_count).enumerate() {
        let base = first_line_id.saturating_add((index as u128).saturating_mul(3));
        let number = UiNodeId::new(base);
        let text = UiNodeId::new(base + 1);
        let row = UiNodeId::new(base + 2);
        nodes.push(
            UiNode::label(
                number,
                format!("Source line {}", index + 1),
                (index + 1).to_string(),
            )
            .with_style(creator_code_line_number_style())
            .with_font_role(UiFontRole::Monospace)
            .with_layout_hints(UiLayoutHints::fixed_width(24.0)),
        );
        nodes.push(
            UiNode::label(text, format!("Source line {} content", index + 1), *line)
                .with_style(creator_code_source_style())
                .with_font_role(UiFontRole::Monospace),
        );
        nodes.push(fixed_height(
            transparent_group(
                row,
                format!("Source line {}", index + 1),
                UiLayout::HorizontalStack { gap: 8.0 },
                vec![number, text],
            ),
            16.0,
        ));
        rows.push(row);
    }
    if shown_line_count < all_lines.len() {
        let base = first_line_id.saturating_add((shown_line_count as u128).saturating_mul(3));
        let more = UiNodeId::new(base);
        nodes.push(fixed_height(
            UiNode::label(
                more,
                "Source listing continuation",
                "… source continues — Inspect source for the authoritative document",
            )
            .with_style(creator_code_line_number_style()),
            16.0,
        ));
        rows.push(more);
    }
    nodes.push(
        UiNode::container(
            listing,
            "Read-only canonical project source listing",
            UiLayout::VerticalStack { gap: 4.0 },
            rows,
        )
        .with_style(creator_code_listing_style())
        .with_constraints(UiConstraints {
            clip: true,
            ..UiConstraints::default()
        }),
    );
    listing
}

fn preference_matches(query: &str, terms: &[&str]) -> bool {
    query.is_empty()
        || terms
            .iter()
            .any(|term| term.to_ascii_lowercase().contains(query))
}

fn creator_action_label(command: &str) -> &'static str {
    match command {
        "editor.recover" => "Recover session",
        "editor.return-hub" => "Back to projects",
        "editor.play-start" => "Start Play",
        "editor.play-apply" => "Apply Play",
        "editor.play-discard" => "Discard Play",
        "editor.focus-selection" => "Focus",
        "editor.select-placement" => "Select placement",
        "editor.preview-command" => "Preview",
        "editor.edit-placement" => "Save",
        "editor.undo" => "Undo",
        "editor.redo" => "Redo",
        "asset.reimport" => "Reimport source",
        "asset.inspect-source" => "Open source",
        "build.submit" => "Build project",
        "build.inspect" => "Inspect build",
        "procedural.inspect" => "Inspect recipe",
        "procedural.validate" => "Validate recipe",
        "procedural.migrate" => "Migrate recipe",
        "procedural.preview" => "Preview recipe",
        "procedural.bake" => "Bake recipe",
        "procedural.dirty" => "Check changes",
        "procedural.explain" => "Explain output",
        "procedural.provenance" => "View provenance",
        "procedural.license-audit" => "Audit licenses",
        "model.inspect-source" => "Inspect source",
        "model.create-primitive" => "Add primitive",
        "model.transform" => "Transform",
        "model.split-edge" => "Split edge",
        "model.undo" => "Undo model",
        "model.redo" => "Redo model",
        "model.recover" => "Recover model",
        "editor.show-diagnostic" => "Show details",
        _ => "Action",
    }
}

fn creator_action_accessible_name(command: &str) -> &'static str {
    match command {
        "editor.focus-selection" => "Focus selected placement",
        "editor.preview-command" => "Preview placement change",
        "editor.edit-placement" => "Save placement",
        _ => creator_action_label(command),
    }
}

fn creator_command_is_available(command: &str, session: &EditorSession) -> bool {
    match command {
        "editor.play-start" => !session.play_active(),
        "editor.play-apply" | "editor.play-discard" => session.play_active(),
        _ => true,
    }
}

#[allow(clippy::too_many_arguments)]
fn push_creator_action_grid(
    nodes: &mut Vec<UiNode>,
    group: UiNodeId,
    first_action_id: u128,
    name: &str,
    panel: EditorPanelId,
    session: &EditorSession,
    commands: &[&str],
    columns: u8,
    height: f32,
) -> UiNodeId {
    let mut actions = Vec::new();
    for (index, command) in commands
        .iter()
        .copied()
        .filter(|command| creator_command_is_available(command, session))
        .enumerate()
    {
        let id = UiNodeId::new(first_action_id.saturating_add(index as u128));
        let label = creator_action_label(command);
        nodes.push(
            UiNode::button(
                id,
                format!("{name}: {}", creator_action_accessible_name(command)),
                command,
                label,
            )
            .with_style(creator_compact_action_style(panel, command)),
        );
        actions.push(id);
    }
    // Buttons retain the framework's accessible 44 px minimum. The source
    // group must reserve that same space (rather than merely prefer a smaller
    // visual height), otherwise a constrained inspector can place its next
    // sibling through the buttons' semantic hit targets.
    let rows = u8::try_from(actions.len().div_ceil(usize::from(columns.max(1)))).unwrap_or(u8::MAX);
    let minimum_height = (f32::from(rows) * 44.0) + (f32::from(rows.saturating_sub(1)) * 4.0);
    let reserved_height = height.max(minimum_height);
    nodes.push(fixed_height(
        transparent_group(
            group,
            format!("{name} actions"),
            UiLayout::Grid { columns, gap: 4.0 },
            actions,
        )
        .with_constraints(UiConstraints {
            minimum: UiSize::new(0.0, reserved_height),
            ..UiConstraints::default()
        }),
        reserved_height,
    ));
    group
}

fn selected_placement_summary(session: &EditorSession) -> Option<String> {
    let placement = selected_placement(session)?;
    Some(format!(
        "{} at X {} mm, Y {} mm, Z {} mm.",
        bounded_text(&placement.label, 72),
        placement.translation.x_mm,
        placement.translation.y_mm,
        placement.translation.z_mm
    ))
}

fn selected_placement(session: &EditorSession) -> Option<&WorldPlacement> {
    session
        .selection()
        .ids
        .iter()
        .find_map(|id| session.document().placements.get(id))
        .or_else(|| session.document().placements.values().next())
}

fn inspected_translation_values(session: &EditorSession) -> (String, String, String) {
    let (x_mm, y_mm, z_mm) = session
        .selection()
        .ids
        .iter()
        .find_map(|id| session.document().placements.get(id))
        .or_else(|| session.document().placements.values().next())
        .map_or((0, 0, 0), |placement| {
            (
                placement.translation.x_mm,
                placement.translation.y_mm,
                placement.translation.z_mm,
            )
        });
    (x_mm.to_string(), y_mm.to_string(), z_mm.to_string())
}

/// Builds the persistent Creator hub. Project paths are non-authoritative
/// local state: every open action is revalidated by editor-core before use.
///
/// # Errors
///
/// Returns an error if the retained semantic tree is invalid.
#[allow(clippy::too_many_lines)] // The bounded hub owns its complete semantic tree in one place.
pub fn creator_hub_document(
    recents: &[RecentProjectView],
    status: &str,
) -> Result<UiDocument, UiDocumentError> {
    let root = UiNodeId::new(90_000);
    let main = UiNodeId::new(90_020);
    let top_spacer = UiNodeId::new(90_021);
    let center_row = UiNodeId::new(90_022);
    let bottom_spacer = UiNodeId::new(90_023);
    let left_spacer = UiNodeId::new(90_024);
    let content = UiNodeId::new(90_025);
    let right_spacer = UiNodeId::new(90_026);
    let hero = UiNodeId::new(90_027);
    let description = UiNodeId::new(90_028);
    let action_row = UiNodeId::new(90_029);
    let surface = UiNodeId::new(90_030);
    let create = UiNodeId::new(90_003);
    let open = UiNodeId::new(90_004);
    let project_name_label = UiNodeId::new(90_017);
    let recents_title = UiNodeId::new(90_014);
    let recents_list = UiNodeId::new(90_015);
    let mut nodes = Vec::new();

    let application_row = push_application_row(&mut nodes, "Projects", false, false);

    nodes.push(fixed_height(
        UiNode::label(hero, "Meridian project hub", "Start a project.")
            .with_style(UiStyle::heading()),
        40.0,
    ));
    nodes.push(fixed_height(
        UiNode::label(
            description,
            "Meridian project model",
            "Create a local source-backed project or open an existing one.",
        )
        .with_style(creator_meta_style()),
        40.0,
    ));
    nodes.push(fixed_size(
        UiNode::button(
            create,
            "Create a Meridian project",
            "hub.create-project",
            "Create project",
        )
        .with_style(creator_hub_action_style(true)),
        352.0,
        56.0,
    ));
    nodes.push(fixed_size(
        UiNode::button(
            open,
            "Open a Meridian project",
            "hub.open-project",
            "Open project",
        )
        .with_style(creator_hub_action_style(false)),
        352.0,
        56.0,
    ));
    nodes.push(fixed_height(
        transparent_group(
            action_row,
            "Project creation and open actions",
            UiLayout::HorizontalStack { gap: 8.0 },
            vec![create, open],
        ),
        56.0,
    ));
    nodes.push(fixed_height(
        UiNode::label(
            project_name_label,
            "New project name label",
            "NEW PROJECT NAME",
        )
        .with_style(creator_hub_field_label_style()),
        18.0,
    ));
    nodes.push(fixed_height(
        UiNode::text_input(
            CREATOR_HUB_PROJECT_NAME,
            "New project name",
            "Meridian Project",
            UiTextInputOptions::default(),
        )
        .with_style(UiStyle::text_field()),
        40.0,
    ));
    nodes.push(fixed_height(
        UiNode::label(recents_title, "Recent projects", "Recent projects")
            .with_style(UiStyle::section_heading()),
        24.0,
    ));

    let displayed_recent_count = recents.len().min(MAX_CREATOR_HUB_RECENT_ROWS);
    let mut recent_rows = Vec::new();
    for (index, recent) in recents.iter().take(MAX_CREATOR_HUB_RECENT_ROWS).enumerate() {
        let base = 90_100_u128.saturating_add((index as u128).saturating_mul(4));
        let label = UiNodeId::new(base);
        let open = UiNodeId::new(base + 1);
        let remove = UiNodeId::new(base + 2);
        let row = UiNodeId::new(base + 3);
        nodes.push(
            UiNode::label(
                label,
                format!("Recent project {}", index + 1),
                format!(
                    "{}\n{}",
                    bounded_text(&recent.label, 56),
                    if recent.available {
                        bounded_text(&recent.path, 92)
                    } else {
                        "Location unavailable".to_owned()
                    }
                ),
            )
            .with_style(creator_meta_style()),
        );
        nodes.push(fixed_width(
            UiNode::button(
                open,
                if recent.available {
                    format!("Open recent project {}", index + 1)
                } else {
                    format!("Locate missing recent project {}", index + 1)
                },
                if recent.available {
                    format!("hub.open-recent:{index}")
                } else {
                    format!("hub.locate-recent:{index}")
                },
                if recent.available { "Open" } else { "Locate" },
            )
            .with_style(shell_utility_style(false, !recent.available)),
            88.0,
        ));
        nodes.push(fixed_width(
            UiNode::button(
                remove,
                format!("Remove recent project {}", index + 1),
                format!("hub.remove-recent:{index}"),
                "Remove",
            )
            .with_style(shell_utility_style(false, false)),
            88.0,
        ));
        nodes.push(fixed_height(
            UiNode::container(
                row,
                format!("Recent project {} controls", index + 1),
                UiLayout::HorizontalStack { gap: 8.0 },
                vec![label, open, remove],
            )
            .with_style(creator_recent_row_style(recent.available)),
            48.0,
        ));
        recent_rows.push(row);
    }
    if recent_rows.is_empty() {
        let empty = UiNodeId::new(90_016);
        nodes.push(fixed_height(
            UiNode::label(
                empty,
                "No recent projects",
                "No recent projects yet. Meridian never opens a saved path implicitly.",
            )
            .with_style(creator_meta_style()),
            40.0,
        ));
        recent_rows.push(empty);
    }
    let recent_count = f32::from(u8::try_from(displayed_recent_count.max(1)).unwrap_or(u8::MAX));
    let recent_gap_count =
        f32::from(u8::try_from(displayed_recent_count.saturating_sub(1)).unwrap_or(u8::MAX));
    let recent_height = recent_count * 48.0 + recent_gap_count * 6.0;
    nodes.push(fixed_height(
        UiNode::virtual_list(recents_list, "Recent projects list", recent_rows)
            .with_style(UiStyle::transparent()),
        recent_height,
    ));

    nodes.push(
        UiNode::container(
            content,
            "Meridian project hub content",
            UiLayout::VerticalStack { gap: 8.0 },
            vec![
                hero,
                description,
                action_row,
                project_name_label,
                CREATOR_HUB_PROJECT_NAME,
                recents_title,
                recents_list,
            ],
        )
        .with_style(UiStyle::transparent())
        .with_layout_hints(UiLayoutHints::fixed_width(712.0)),
    );
    nodes.push(transparent_group(
        left_spacer,
        "Project hub left margin",
        UiLayout::Overlay,
        Vec::new(),
    ));
    nodes.push(transparent_group(
        right_spacer,
        "Project hub right margin",
        UiLayout::Overlay,
        Vec::new(),
    ));
    let center_height = 322.0 + recent_height;
    nodes.push(fixed_size(
        UiNode::container(
            surface,
            "Meridian project hub surface",
            UiLayout::VerticalStack { gap: 0.0 },
            vec![content],
        )
        .with_style(creator_hub_surface_style())
        .with_elevation(UiElevation::Raised),
        760.0,
        center_height,
    ));
    nodes.push(fixed_height(
        transparent_group(
            center_row,
            "Project hub centered content",
            UiLayout::HorizontalStack { gap: 0.0 },
            vec![left_spacer, surface, right_spacer],
        ),
        center_height,
    ));
    nodes.push(transparent_group(
        top_spacer,
        "Project hub top margin",
        UiLayout::Overlay,
        Vec::new(),
    ));
    nodes.push(transparent_group(
        bottom_spacer,
        "Project hub bottom margin",
        UiLayout::Overlay,
        Vec::new(),
    ));
    nodes.push(
        UiNode::container(
            main,
            "Meridian project hub",
            UiLayout::VerticalStack { gap: 0.0 },
            vec![top_spacer, center_row, bottom_spacer],
        )
        .with_style(UiStyle::canvas()),
    );
    let status_row = push_status_row(
        &mut nodes,
        "No project open",
        bounded_text(status, 120),
        false,
    );
    nodes.push(
        UiNode::container(
            root,
            "Meridian application hub",
            UiLayout::VerticalStack { gap: 0.0 },
            vec![application_row, main, status_row],
        )
        .with_style(workspace_canvas_style()),
    );
    creator_authored_document(root, nodes)
}

/// Builds Meridian's application-owned Settings surface.
///
/// The document offers only typed local preferences supplied by the host. It
/// does not expose project-source, filesystem, or platform-adapter authority.
///
/// # Errors
///
/// Returns an error if the retained semantic tree is invalid.
#[allow(clippy::too_many_lines)] // This bounded surface keeps every preference control auditable.
pub fn creator_settings_document(
    view: &CreatorSettingsView,
) -> Result<UiDocument, UiDocumentError> {
    let root = UiNodeId::new(90_100);
    let main = UiNodeId::new(90_101);
    let navigation = UiNodeId::new(90_102);
    let surface = UiNodeId::new(90_103);
    let inspector = UiNodeId::new(90_104);
    let navigation_title = UiNodeId::new(90_105);
    let navigation_items = UiNodeId::new(90_106);
    let surface_title = UiNodeId::new(90_107);
    let surface_detail = UiNodeId::new(90_108);
    let accessibility_title = UiNodeId::new(90_109);
    let high_contrast = UiNodeId::new(90_110);
    let reduced_motion = UiNodeId::new(90_111);
    let density_title = UiNodeId::new(90_112);
    let compact = UiNodeId::new(90_113);
    let standard = UiNodeId::new(90_114);
    let comfortable = UiNodeId::new(90_115);
    let reset = UiNodeId::new(90_116);
    let return_target = UiNodeId::new(90_117);
    let inspector_title = UiNodeId::new(90_118);
    let inspector_detail = UiNodeId::new(90_119);
    let history_detail = UiNodeId::new(90_120);
    let actions = UiNodeId::new(90_121);
    let density_actions = UiNodeId::new(90_122);
    let no_matches = UiNodeId::new(90_131);
    let high_contrast_detail = UiNodeId::new(90_132);
    let high_contrast_row = UiNodeId::new(90_133);
    let reduced_motion_detail = UiNodeId::new(90_134);
    let reduced_motion_row = UiNodeId::new(90_135);
    let preference_actions = UiNodeId::new(90_136);
    let mut nodes = Vec::new();
    let query = view.query.trim().to_ascii_lowercase();

    let application_row = push_application_row(
        &mut nodes,
        view.project.as_deref().unwrap_or("Preferences"),
        view.project.is_some(),
        view.play_active,
    );
    // Settings is application-owned rather than a project workspace. It still
    // retains the permanent navigation row so the product does not turn into
    // an orphaned preferences page; no project workspace is falsely selected.
    let workspace_row = push_workspace_row(&mut nodes, None);

    nodes.push(fixed_height(
        UiNode::label(navigation_title, "Settings categories", "SETTINGS")
            .with_style(creator_hub_field_label_style()),
        18.0,
    ));
    let categories = [
        (90_123, "Appearance"),
        (90_124, "Accessibility"),
        (90_125, "Workspace"),
    ];
    let mut category_nodes = Vec::new();
    for (id, category) in categories {
        let id = UiNodeId::new(id);
        nodes.push(fixed_height(
            UiNode::label(id, format!("{category} settings category"), category)
                .with_style(creator_meta_style()),
            20.0,
        ));
        category_nodes.push(id);
    }
    nodes.push(
        UiNode::container(
            navigation_items,
            "Settings categories",
            UiLayout::VerticalStack { gap: 8.0 },
            category_nodes,
        )
        .with_style(UiStyle::transparent()),
    );
    nodes.push(
        UiNode::container(
            navigation,
            "Settings navigation",
            UiLayout::VerticalStack { gap: 12.0 },
            vec![navigation_title, navigation_items],
        )
        .with_style(world_panel_style(10.0))
        .with_layout_hints(UiLayoutHints::fixed_width(232.0)),
    );

    nodes.push(fixed_height(
        UiNode::label(
            surface_title,
            "Meridian settings",
            "Appearance & accessibility",
        )
        .with_style(creator_title_style()),
        30.0,
    ));
    nodes.push(fixed_height(
        UiNode::label(
            surface_detail,
            "Settings persistence boundary",
            "Preferences are local to this Meridian installation and never modify project source.",
        )
        .with_style(creator_meta_style()),
        38.0,
    ));
    nodes.push(fixed_height(
        UiNode::search_input(CREATOR_SETTINGS_SEARCH, "Search preferences", &view.query)
            .with_placeholder("Search preferences")
            .with_style(UiStyle::text_field()),
        36.0,
    ));
    let mut action_children = vec![surface_title, surface_detail, CREATOR_SETTINGS_SEARCH];
    let high_contrast_matches =
        preference_matches(&query, &["accessibility", "high contrast", "contrast"]);
    let reduced_motion_matches =
        preference_matches(&query, &["accessibility", "reduced motion", "motion"]);
    let accessibility_matches = high_contrast_matches || reduced_motion_matches;
    let density_matches =
        preference_matches(&query, &["density", "compact", "standard", "comfortable"]);
    if accessibility_matches {
        nodes.push(fixed_height(
            UiNode::label(
                accessibility_title,
                "Accessibility preferences",
                "ACCESSIBILITY",
            )
            .with_style(creator_hub_field_label_style()),
            18.0,
        ));
        action_children.push(accessibility_title);
        if high_contrast_matches {
            nodes.push(fixed_size(
                UiNode::label(
                    high_contrast_detail,
                    "High contrast preference detail",
                    "High contrast\nUse opaque, higher-contrast Meridian surfaces.",
                )
                .with_style(creator_meta_style()),
                288.0,
                36.0,
            ));
            nodes.push(fixed_size(
                UiNode::toggle(
                    high_contrast,
                    "Toggle high contrast",
                    "settings.toggle-high-contrast",
                    view.high_contrast,
                )
                .with_style(shell_utility_style(false, view.high_contrast)),
                84.0,
                44.0,
            ));
            nodes.push(fixed_height(
                UiNode::container(
                    high_contrast_row,
                    "High contrast preference",
                    UiLayout::HorizontalStack { gap: 12.0 },
                    vec![high_contrast_detail, high_contrast],
                )
                .with_style(world_subsection_style()),
                44.0,
            ));
            action_children.push(high_contrast_row);
        }
        if reduced_motion_matches {
            nodes.push(fixed_size(
                UiNode::label(
                    reduced_motion_detail,
                    "Reduced motion preference detail",
                    "Reduced motion\nSettle interface motion immediately.",
                )
                .with_style(creator_meta_style()),
                288.0,
                36.0,
            ));
            nodes.push(fixed_size(
                UiNode::toggle(
                    reduced_motion,
                    "Toggle reduced motion",
                    "settings.toggle-reduced-motion",
                    view.reduced_motion,
                )
                .with_style(shell_utility_style(false, view.reduced_motion)),
                84.0,
                44.0,
            ));
            nodes.push(fixed_height(
                UiNode::container(
                    reduced_motion_row,
                    "Reduced motion preference",
                    UiLayout::HorizontalStack { gap: 12.0 },
                    vec![reduced_motion_detail, reduced_motion],
                )
                .with_style(world_subsection_style()),
                44.0,
            ));
            action_children.push(reduced_motion_row);
        }
    }
    if density_matches {
        nodes.push(fixed_height(
            UiNode::label(density_title, "Interface density", "DENSITY")
                .with_style(creator_hub_field_label_style()),
            18.0,
        ));
        for (id, action, label, selected) in [
            (
                compact,
                "settings.density-compact",
                "Compact",
                view.density == "Compact",
            ),
            (
                standard,
                "settings.density-standard",
                "Standard",
                view.density == "Standard",
            ),
            (
                comfortable,
                "settings.density-comfortable",
                "Comfortable",
                view.density == "Comfortable",
            ),
        ] {
            nodes.push(fixed_size(
                UiNode::button(id, format!("Use {label} interface density"), action, label)
                    .with_style(shell_utility_style(false, selected)),
                104.0,
                44.0,
            ));
        }
        nodes.push(fixed_height(
            transparent_group(
                density_actions,
                "Interface density choices",
                UiLayout::HorizontalStack { gap: 8.0 },
                vec![compact, standard, comfortable],
            ),
            44.0,
        ));
        action_children.extend([density_title, density_actions]);
    }
    if !accessibility_matches && !density_matches {
        nodes.push(
            UiNode::label(
                no_matches,
                "No preference search matches",
                "No local preferences match this search.",
            )
            .with_style(world_section_style(UiColor::amber())),
        );
        action_children.push(no_matches);
    }
    nodes.push(fixed_width(
        UiNode::button(
            reset,
            "Reset Meridian preferences",
            "settings.reset-preferences",
            "Reset local preferences",
        )
        .with_style(shell_utility_style(false, false)),
        180.0,
    ));
    nodes.push(fixed_width(
        UiNode::button(
            return_target,
            if view.project.is_some() {
                "Return to the open project"
            } else {
                "Return to the Meridian project hub"
            },
            "settings.return",
            if view.project.is_some() {
                "Return to project"
            } else {
                "Return to hub"
            },
        )
        .with_style(shell_utility_style(true, false)),
        164.0,
    ));
    nodes.push(fixed_height(
        transparent_group(
            preference_actions,
            "Settings recovery and return actions",
            UiLayout::HorizontalStack { gap: 8.0 },
            vec![reset, return_target],
        ),
        44.0,
    ));
    action_children.push(preference_actions);
    nodes.push(fixed_height(
        UiNode::container(
            actions,
            "Settings controls",
            UiLayout::VerticalStack { gap: 8.0 },
            action_children,
        )
        .with_style(UiStyle::transparent())
        .with_constraints(UiConstraints {
            maximum: Some(UiSize::new(760.0, 408.0)),
            horizontal_alignment: UiAlignment::Center,
            vertical_alignment: UiAlignment::Center,
            ..UiConstraints::default()
        }),
        408.0,
    ));
    nodes.push(
        UiNode::container(
            surface,
            "Settings work surface",
            UiLayout::Overlay,
            vec![actions],
        )
        .with_style(world_panel_style(10.0)),
    );

    nodes.push(fixed_height(
        UiNode::label(inspector_title, "Preference authority", "LOCAL PREFERENCES")
            .with_style(creator_hub_field_label_style()),
        18.0,
    ));
    nodes.push(fixed_height(
        UiNode::label(
            inspector_detail,
            "Applied preference summary",
            format!(
                "High contrast: {}. Reduced motion: {}. Density: {}.",
                if view.high_contrast { "On" } else { "Off" },
                if view.reduced_motion { "On" } else { "Off" },
                view.density,
            ),
        )
        .with_style(creator_meta_style()),
        48.0,
    ));
    nodes.push(fixed_height(
        UiNode::label(
            history_detail,
            "Preference history boundary",
            "Preference changes are atomic local writes. Preference history and platform-owned controls remain explicitly unavailable.",
        )
        .with_style(domain_capability_note_style()),
        64.0,
    ));
    nodes.push(
        UiNode::container(
            inspector,
            "Settings inspector",
            UiLayout::VerticalStack { gap: 12.0 },
            vec![inspector_title, inspector_detail, history_detail],
        )
        .with_style(world_panel_style(10.0))
        .with_layout_hints(UiLayoutHints::fixed_width(300.0)),
    );
    nodes.push(
        UiNode::container(
            main,
            "Meridian settings workspace",
            UiLayout::HorizontalStack { gap: 8.0 },
            vec![navigation, surface, inspector],
        )
        .with_style(creator_workbench_canvas_style()),
    );
    let status_row = push_status_row(
        &mut nodes,
        if view.project.is_some() {
            "Project source retained"
        } else {
            "Local preferences"
        },
        bounded_text(&view.status, 120),
        view.play_active,
    );
    nodes.push(
        UiNode::container(
            root,
            "Meridian Settings",
            UiLayout::VerticalStack { gap: 0.0 },
            vec![application_row, workspace_row, main, status_row],
        )
        .with_style(workspace_canvas_style()),
    );
    creator_authored_document(root, nodes)
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
    creator_workspace_document(session, "Ready")
}

/// Builds the Creator workspace with one application-owned diagnostic summary.
/// Source and command state still come exclusively from editor-core.
///
/// # Errors
///
/// Returns an error if the generated retained document violates UI semantics.
pub fn creator_workspace_document(
    session: &EditorSession,
    status: &str,
) -> Result<UiDocument, UiDocumentError> {
    let view = CreatorWorkspaceView::foundation(session, status);
    creator_workspace_document_with_view(session, &view)
}

/// Builds the Creator workspace from a bounded read-only presentation view.
///
/// Source and command state still come exclusively from editor-core. The view
/// provides only current tool/recovery facts so the UI never substitutes stale
/// placeholder copy for a project that is actually open.
///
/// # Errors
///
/// Returns an error if the generated retained document violates UI semantics.
#[allow(clippy::too_many_lines)] // The workspace composition keeps all bounded panels auditable.
pub fn creator_workspace_document_with_view(
    session: &EditorSession,
    view: &CreatorWorkspaceView,
) -> Result<UiDocument, UiDocumentError> {
    match view.workspace {
        WorkspaceKind::World => creator_world_workspace_document_with_view(session, view),
        WorkspaceKind::Hub => creator_hub_document(&[], "Choose a project from the Meridian hub."),
        WorkspaceKind::Code => creator_code_workspace_document(session, view),
        WorkspaceKind::UiAuthoring => creator_ui_authoring_workspace_document(session, view),
        WorkspaceKind::Modeler => creator_modeler_workspace_document(session, view),
        WorkspaceKind::Materials
        | WorkspaceKind::Alluvium
        | WorkspaceKind::Build
        | WorkspaceKind::Profile
        | WorkspaceKind::Settings
        | WorkspaceKind::Recovery => creator_domain_workspace_document(session, view),
    }
}

#[allow(clippy::too_many_lines)] // The World composition keeps its source authority auditable.
fn creator_world_workspace_document_with_view(
    session: &EditorSession,
    view: &CreatorWorkspaceView,
) -> Result<UiDocument, UiDocumentError> {
    let root = UiNodeId::new(1);
    let main = UiNodeId::new(4);
    let activity_rail = UiNodeId::new(92_060);
    let browser = UiNodeId::new(164);
    let viewport = UiNodeId::new(132);
    let inspector = UiNodeId::new(196);
    let bottom_shelf = UiNodeId::new(5);
    let mut nodes = Vec::new();
    let compact_context = view.compact_world_context;

    let application_row = push_application_row(
        &mut nodes,
        &bounded_text(&view.project, 34),
        true,
        session.play_active(),
    );
    let workspace_row = push_workspace_row(&mut nodes, Some(WorkspaceKind::World));

    if !compact_context {
        nodes.push(fixed_height(
            UiNode::button(
                UiNodeId::new(92_061),
                "World workspace",
                "workspace.world",
                "W",
            )
            .with_style(workspace_tab_style(true)),
            34.0,
        ));
        let rail_items = [
            (92_062, IconId::Build, "Import source", "asset.reimport"),
            (92_063, IconId::Search, "Search World", "shell.search"),
            (92_064, IconId::More, "World favorites", "shell.favorites"),
            (92_065, IconId::Settings, "World panels", "shell.panels"),
        ];
        let mut rail_children = vec![UiNodeId::new(92_061)];
        for (id, icon, name, action) in rail_items {
            let id = UiNodeId::new(id);
            nodes.push(fixed_height(
                UiNode::icon_button(id, name, action, icon).with_style(workspace_tab_style(false)),
                34.0,
            ));
            rail_children.push(id);
        }
        nodes.push(
            UiNode::container(
                activity_rail,
                "World activity rail",
                UiLayout::VerticalStack { gap: 8.0 },
                rail_children,
            )
            .with_style(creator_activity_rail_style())
            .with_layout_hints(UiLayoutHints::fixed_width(44.0)),
        );
    }

    let browser_header = UiNodeId::new(92_051);
    let browser_title = UiNodeId::new(92_052);
    let browser_kind = UiNodeId::new(92_053);
    let browser_header_row = UiNodeId::new(92_087);
    let browser_header_divider = UiNodeId::new(92_088);
    let browser_tree = UiNodeId::new(92_054);
    let placement_item = UiNodeId::new(92_055);
    let source_item = UiNodeId::new(92_056);
    let generated_item = UiNodeId::new(92_057);
    let scene_item = UiNodeId::new(92_083);
    let sources_group = UiNodeId::new(92_084);
    let placements_group = UiNodeId::new(92_085);
    let browser_actions = UiNodeId::new(92_058);
    let browser_footer_divider = UiNodeId::new(92_089);
    let source_status = UiNodeId::new(92_080);
    let source_status_title = UiNodeId::new(92_081);
    let source_status_detail = UiNodeId::new(92_082);
    let browser_spacer = UiNodeId::new(92_086);
    let reimport = UiNodeId::new(262);
    let inspect_source = UiNodeId::new(263);
    nodes.push(fixed_width(
        UiNode::label(browser_title, "World browser title", "World")
            .with_style(world_panel_title_style()),
        104.0,
    ));
    nodes.push(
        UiNode::label(browser_kind, "World browser source mode", "SOURCE")
            .with_style(creator_hub_field_label_style()),
    );
    nodes.push(fixed_height(
        transparent_group(
            browser_header_row,
            "World browser title row",
            UiLayout::HorizontalStack { gap: 8.0 },
            vec![browser_title, browser_kind],
        ),
        24.0,
    ));
    nodes.push(fixed_height(
        transparent_group(
            browser_header_divider,
            "World browser header divider",
            UiLayout::Overlay,
            Vec::new(),
        )
        .with_style(panel_header_divider_style()),
        1.0,
    ));
    nodes.push(fixed_height(
        UiNode::container(
            browser_header,
            "World browser header",
            UiLayout::VerticalStack { gap: 7.0 },
            vec![browser_header_row, browser_header_divider],
        )
        .with_style(UiStyle::transparent()),
        32.0,
    ));
    nodes.push(fixed_height(
        UiNode::search_input(CREATOR_WORLD_SEARCH, "Search World sources", "")
            .with_placeholder("Search sources")
            .with_style(UiStyle::text_field()),
        36.0,
    ));
    let selected_label = selected_placement(session).map_or_else(
        || "No source placement".to_owned(),
        // The enclosing branch already tells the reader this is a placement.
        // Preserve the real source label instead of spending the narrow tree
        // row on a redundant "Placement ·" prefix and an early ellipsis.
        |placement| bounded_text(&placement.label, 28),
    );
    nodes.push(fixed_height(
        creator_tree_group_item(scene_item, "Project", "editor.focus-selection", false, true),
        30.0,
    ));
    nodes.push(fixed_height(
        creator_tree_group_item(
            sources_group,
            "Sources",
            "asset.inspect-source",
            false,
            true,
        ),
        28.0,
    ));
    let source_label = session.document().sources.values().next().map_or_else(
        || "Source · no imported source".to_owned(),
        // Likewise, Sources is already the visible group. A clean source name
        // scans more like a real browser row and keeps its stable identity
        // legible at the supported compact width.
        |source| bounded_text(&source.label, 28),
    );
    nodes.push(fixed_height(
        creator_tree_child_item(source_item, source_label, "asset.inspect-source", false),
        30.0,
    ));
    nodes.push(fixed_height(
        creator_tree_group_item(
            placements_group,
            "Placements",
            "editor.focus-selection",
            false,
            true,
        ),
        28.0,
    ));
    nodes.push(fixed_height(
        creator_tree_child_item(
            placement_item,
            selected_label,
            "editor.select-placement",
            !session.selection().ids.is_empty(),
        ),
        30.0,
    ));
    nodes.push(fixed_height(
        creator_tree_group_item(
            generated_item,
            "Generated",
            "procedural.inspect",
            false,
            false,
        ),
        30.0,
    ));
    nodes.push(
        UiNode::tree(
            browser_tree,
            "World source hierarchy",
            vec![
                scene_item,
                sources_group,
                source_item,
                placements_group,
                placement_item,
                generated_item,
            ],
        )
        .with_style(UiStyle::transparent())
        .with_layout_hints(UiLayoutHints::fixed_height(264.0)),
    );
    nodes.push(transparent_group(
        browser_spacer,
        "World browser content spacer",
        UiLayout::Overlay,
        Vec::new(),
    ));
    nodes.push(fixed_height(
        transparent_group(
            browser_footer_divider,
            "World browser footer divider",
            UiLayout::Overlay,
            Vec::new(),
        )
        .with_style(panel_header_divider_style()),
        1.0,
    ));
    nodes.push(
        UiNode::label(source_status_title, "World source status", "SOURCE STATUS")
            .with_style(creator_hub_field_label_style()),
    );
    nodes.push(
        UiNode::label(
            source_status_detail,
            "World source summary",
            format!(
                "{} source{} · {} editable placement{}",
                session.document().sources.len(),
                if session.document().sources.len() == 1 {
                    ""
                } else {
                    "s"
                },
                session.document().placements.len(),
                if session.document().placements.len() == 1 {
                    ""
                } else {
                    "s"
                },
            ),
        )
        .with_style(creator_meta_style()),
    );
    nodes.push(fixed_height(
        UiNode::container(
            source_status,
            "World source summary",
            UiLayout::VerticalStack { gap: 4.0 },
            vec![source_status_title, source_status_detail],
        )
        .with_style(world_subsection_style()),
        52.0,
    ));
    nodes.push(fixed_width(
        UiNode::button(
            reimport,
            "Reimport selected source",
            "asset.reimport",
            "Reimport",
        )
        .with_style(creator_compact_action_style(
            EditorPanelId::Assets,
            "asset.reimport",
        )),
        96.0,
    ));
    nodes.push(
        UiNode::button(
            inspect_source,
            "Inspect authoritative source",
            "asset.inspect-source",
            "Open source",
        )
        .with_style(creator_compact_action_style(
            EditorPanelId::Assets,
            "asset.inspect-source",
        )),
    );
    nodes.push(fixed_height(
        transparent_group(
            browser_actions,
            "World source actions",
            UiLayout::HorizontalStack { gap: 8.0 },
            vec![reimport, inspect_source],
        )
        .with_constraints(UiConstraints {
            minimum: UiSize::new(0.0, 44.0),
            ..UiConstraints::default()
        }),
        44.0,
    ));
    nodes.push(
        UiNode::container(
            browser,
            "World browser",
            UiLayout::VerticalStack { gap: 8.0 },
            vec![
                browser_header,
                CREATOR_WORLD_SEARCH,
                browser_tree,
                browser_spacer,
                browser_footer_divider,
                source_status,
                browser_actions,
            ],
        )
        .with_style(focused_world_panel_style(
            10.0,
            active_panel(view, &[EditorPanelId::Hierarchy, EditorPanelId::Assets]),
        ))
        .with_layout_hints(UiLayoutHints::fixed_width(if compact_context {
            240.0
        } else {
            264.0
        })),
    );

    let viewport_header = UiNodeId::new(92_070);
    let viewport_title = UiNodeId::new(92_071);
    let viewport_meta = UiNodeId::new(92_072);
    let viewport_revision = UiNodeId::new(92_074);
    let viewport_header_spacer = UiNodeId::new(92_075);
    let focus_selection = UiNodeId::new(134);
    nodes.push(fixed_width(
        UiNode::label(viewport_title, "Live World viewport title", "World")
            .with_style(side_panel_title_style()),
        72.0,
    ));
    nodes.push(fixed_width(
        UiNode::label(
            viewport_meta,
            "Live World viewport mode",
            if compact_context {
                "Perspective"
            } else {
                "Perspective · Lit"
            },
        )
        .with_style(creator_meta_style()),
        if compact_context { 84.0 } else { 104.0 },
    ));
    if !compact_context {
        nodes.push(fixed_width(
            UiNode::label(
                viewport_revision,
                "World source generation",
                format!("r{}", session.document().generation),
            )
            .with_style(activity_shelf_hint_style()),
            36.0,
        ));
        nodes.push(transparent_group(
            viewport_header_spacer,
            "Live World viewport header spacer",
            UiLayout::Overlay,
            Vec::new(),
        ));
    }
    nodes.push(fixed_width(
        UiNode::button(
            focus_selection,
            "Focus selected World source",
            "editor.focus-selection",
            if compact_context {
                "Focus"
            } else {
                "Focus selection"
            },
        )
        .with_style(shell_utility_style(false, false)),
        if compact_context { 64.0 } else { 112.0 },
    ));
    let mut viewport_header_children = vec![viewport_title, viewport_meta];
    if !compact_context {
        viewport_header_children.extend([viewport_revision, viewport_header_spacer]);
    }
    viewport_header_children.push(focus_selection);
    nodes.push(fixed_height(
        transparent_group(
            viewport_header,
            "Live World viewport header",
            UiLayout::HorizontalStack { gap: 8.0 },
            viewport_header_children,
        ),
        32.0,
    ));
    nodes.push(
        UiNode::canvas(
            CREATOR_WORLD_VIEWPORT_CANVAS,
            "Live source-derived World viewport",
            Vec::new(),
        )
        .with_style(world_canvas_style())
        .with_constraints(UiConstraints {
            // The persistent World shell retains its rail, browser, and
            // inspector at a 1024 px width. Keep the canvas drawable there
            // without allowing its focus target to spill into the inspector.
            minimum: UiSize::new(240.0, 240.0),
            clip: true,
            ..UiConstraints::default()
        }),
    );
    nodes.push(
        UiNode::container(
            viewport,
            "World viewport",
            UiLayout::VerticalStack { gap: 8.0 },
            vec![viewport_header, CREATOR_WORLD_VIEWPORT_CANVAS],
        )
        .with_style(focused_world_panel_style(
            10.0,
            active_panel(view, &[EditorPanelId::Viewport]),
        ))
        .with_elevation(UiElevation::Raised),
    );

    let inspector_header = UiNodeId::new(93_000);
    let inspector_title = UiNodeId::new(93_001);
    let inspector_context = UiNodeId::new(93_002);
    let inspector_header_row = UiNodeId::new(93_049);
    let inspector_header_divider = UiNodeId::new(93_050);
    let selection_summary = UiNodeId::new(93_003);
    let transform_title = UiNodeId::new(93_004);
    let transform_fields = UiNodeId::new(93_005);
    let source_title = UiNodeId::new(93_006);
    let source_name = UiNodeId::new(93_007);
    let source_authority = UiNodeId::new(93_008);
    let source_section = UiNodeId::new(93_009);
    let source_origin_row = UiNodeId::new(93_040);
    let source_authority_row = UiNodeId::new(93_041);
    let source_preview_row = UiNodeId::new(93_042);
    let source_origin_label = UiNodeId::new(93_043);
    let source_authority_label = UiNodeId::new(93_044);
    let source_preview_label = UiNodeId::new(93_045);
    let inspector_source_action = UiNodeId::new(93_047);
    let inspector_spacer = UiNodeId::new(93_048);
    let inspector_footer_divider = UiNodeId::new(93_051);
    nodes.push(fixed_width(
        UiNode::label(inspector_title, "World Inspector title", "Inspector")
            .with_style(world_panel_title_style()),
        118.0,
    ));
    nodes.push(
        UiNode::label(inspector_context, "World Inspector context", "PLACEMENT")
            .with_style(creator_hub_field_label_style()),
    );
    nodes.push(fixed_height(
        transparent_group(
            inspector_header_row,
            "World Inspector title row",
            UiLayout::HorizontalStack { gap: 8.0 },
            vec![inspector_title, inspector_context],
        ),
        24.0,
    ));
    nodes.push(fixed_height(
        transparent_group(
            inspector_header_divider,
            "World Inspector header divider",
            UiLayout::Overlay,
            Vec::new(),
        )
        .with_style(panel_header_divider_style()),
        1.0,
    ));
    nodes.push(fixed_height(
        UiNode::container(
            inspector_header,
            "World Inspector header",
            UiLayout::VerticalStack { gap: 7.0 },
            vec![inspector_header_row, inspector_header_divider],
        )
        .with_style(UiStyle::transparent()),
        32.0,
    ));
    nodes.push(fixed_height(
        UiNode::label(
            selection_summary,
            "World Inspector selection",
            selected_placement(session).map_or_else(
                || "Select a source placement to edit it.".to_owned(),
                |placement| {
                    format!(
                        "{} · {}, {}, {} mm",
                        bounded_text(&placement.label, 48),
                        placement.translation.x_mm,
                        placement.translation.y_mm,
                        placement.translation.z_mm
                    )
                },
            ),
        )
        .with_style(creator_value_style()),
        42.0,
    ));
    let source_label = session.document().sources.values().next().map_or_else(
        || "No imported source is registered.".to_owned(),
        |source| format!("Imported · {}", bounded_text(&source.label, 48)),
    );
    nodes.push(fixed_height(
        UiNode::label(source_title, "Selected source section", "SOURCE")
            .with_style(creator_hub_field_label_style()),
        18.0,
    ));
    nodes.push(fixed_width(
        UiNode::label(source_origin_label, "Source origin label", "Origin")
            .with_style(creator_hub_field_label_style()),
        76.0,
    ));
    nodes.push(
        UiNode::label(source_name, "Selected authoritative source", source_label)
            .with_style(creator_value_style()),
    );
    nodes.push(fixed_height(
        transparent_group(
            source_origin_row,
            "Selected source origin",
            UiLayout::HorizontalStack { gap: 8.0 },
            vec![source_origin_label, source_name],
        ),
        20.0,
    ));
    nodes.push(fixed_width(
        UiNode::label(
            source_authority_label,
            "Source authority label",
            "Authority",
        )
        .with_style(creator_hub_field_label_style()),
        76.0,
    ));
    nodes.push(
        UiNode::label(source_authority, "Source authority state", "Source")
            .with_style(creator_value_style()),
    );
    nodes.push(fixed_height(
        transparent_group(
            source_authority_row,
            "Selected source authority",
            UiLayout::HorizontalStack { gap: 8.0 },
            vec![source_authority_label, source_authority],
        ),
        20.0,
    ));
    nodes.push(fixed_width(
        UiNode::label(
            source_preview_label,
            "Viewport derivation label",
            "Viewport",
        )
        .with_style(creator_hub_field_label_style()),
        76.0,
    ));
    let source_preview = UiNodeId::new(93_046);
    nodes.push(
        UiNode::label(source_preview, "Viewport derivation state", "Derived")
            .with_style(creator_value_style()),
    );
    nodes.push(fixed_height(
        transparent_group(
            source_preview_row,
            "Selected source viewport derivation",
            UiLayout::HorizontalStack { gap: 8.0 },
            vec![source_preview_label, source_preview],
        ),
        20.0,
    ));
    nodes.push(fixed_height(
        UiNode::container(
            source_section,
            "World source provenance",
            UiLayout::VerticalStack { gap: 4.0 },
            vec![
                source_title,
                source_origin_row,
                source_authority_row,
                source_preview_row,
            ],
        )
        .with_style(world_subsection_style()),
        90.0,
    ));
    nodes.push(transparent_group(
        inspector_spacer,
        "World Inspector lower spacer",
        UiLayout::Overlay,
        Vec::new(),
    ));
    nodes.push(fixed_height(
        transparent_group(
            inspector_footer_divider,
            "World Inspector footer divider",
            UiLayout::Overlay,
            Vec::new(),
        )
        .with_style(panel_header_divider_style()),
        1.0,
    ));
    nodes.push(fixed_height(
        UiNode::button(
            inspector_source_action,
            "Inspect authoritative source",
            "asset.inspect-source",
            "Inspect source",
        )
        .with_style(creator_compact_action_style(
            EditorPanelId::Assets,
            "asset.inspect-source",
        ))
        .with_constraints(UiConstraints {
            minimum: UiSize::new(0.0, 44.0),
            ..UiConstraints::default()
        }),
        44.0,
    ));
    nodes.push(fixed_height(
        UiNode::label(transform_title, "Transform properties", "TRANSFORM · MM")
            .with_style(creator_hub_field_label_style()),
        18.0,
    ));
    let (x_mm, y_mm, z_mm) = inspected_translation_values(session);
    let axes = [
        (93_010, 93_011, CREATOR_INSPECTOR_X_MM, "X", x_mm),
        (93_012, 93_013, CREATOR_INSPECTOR_Y_MM, "Y", y_mm),
        (93_014, 93_015, CREATOR_INSPECTOR_Z_MM, "Z", z_mm),
    ];
    let mut axis_groups = Vec::new();
    for (group, label, field, axis, value) in axes {
        let group = UiNodeId::new(group);
        let label = UiNodeId::new(label);
        nodes.push(fixed_height(
            UiNode::label(label, format!("{axis} coordinate"), axis)
                .with_style(creator_hub_field_label_style()),
            16.0,
        ));
        nodes.push(fixed_height(
            UiNode::text_input(
                field,
                format!("Selected placement {axis} coordinate in millimetres"),
                value,
                UiTextInputOptions::default(),
            )
            .with_style(UiStyle::compact_text_field()),
            44.0,
        ));
        nodes.push(fixed_height(
            transparent_group(
                group,
                format!("{axis} coordinate field"),
                UiLayout::VerticalStack { gap: 4.0 },
                vec![label, field],
            )
            .with_constraints(UiConstraints {
                minimum: UiSize::new(0.0, 64.0),
                ..UiConstraints::default()
            }),
            64.0,
        ));
        axis_groups.push(group);
    }
    nodes.push(fixed_height(
        transparent_group(
            transform_fields,
            "Selected placement transform",
            UiLayout::HorizontalStack { gap: 8.0 },
            axis_groups,
        )
        .with_constraints(UiConstraints {
            minimum: UiSize::new(0.0, 64.0),
            ..UiConstraints::default()
        }),
        64.0,
    ));
    let placement_actions = push_creator_action_grid(
        &mut nodes,
        UiNodeId::new(93_020),
        93_021,
        "World placement",
        EditorPanelId::Inspector,
        session,
        &[
            "editor.preview-command",
            "editor.edit-placement",
            "editor.focus-selection",
        ],
        if compact_context { 2 } else { 3 },
        if compact_context { 64.0 } else { 30.0 },
    );

    nodes.push(
        UiNode::container(
            inspector,
            "World Inspector",
            UiLayout::VerticalStack { gap: 8.0 },
            vec![
                inspector_header,
                selection_summary,
                transform_title,
                transform_fields,
                placement_actions,
                source_section,
                inspector_spacer,
                inspector_footer_divider,
                inspector_source_action,
            ],
        )
        .with_style(focused_world_panel_style(
            10.0,
            active_panel(view, &[EditorPanelId::Inspector]),
        ))
        .with_layout_hints(UiLayoutHints::fixed_width(if compact_context {
            320.0
        } else {
            344.0
        })),
    );

    let mut main_children = Vec::new();
    if !compact_context {
        main_children.push(activity_rail);
    }
    main_children.extend([browser, viewport, inspector]);
    nodes.push(
        UiNode::container(
            main,
            "World workspace",
            UiLayout::HorizontalStack { gap: 8.0 },
            main_children,
        )
        .with_style(creator_workbench_canvas_style()),
    );

    let shelf_header = UiNodeId::new(93_200);
    let history_panel = UiNodeId::new(93_201);
    let build_panel = UiNodeId::new(93_202);
    let recovery_panel = UiNodeId::new(93_203);
    let history_title = UiNodeId::new(93_204);
    let history_detail = UiNodeId::new(93_205);
    let undo = UiNodeId::new(93_206);
    let redo = UiNodeId::new(93_207);
    let build_title = UiNodeId::new(93_209);
    let build_detail = UiNodeId::new(93_210);
    let inspect_build = UiNodeId::new(93_211);
    let recovery_title = UiNodeId::new(93_212);
    let recovery_detail = UiNodeId::new(93_213);
    let diagnostics = UiNodeId::new(93_214);
    let recover = UiNodeId::new(93_215);
    let recovery_actions = UiNodeId::new(93_216);
    let shelf_body = UiNodeId::new(93_217);
    let shelf_header_row = UiNodeId::new(93_218);
    let shelf_summary = UiNodeId::new(93_219);
    let history_state = UiNodeId::new(93_220);
    let build_scope = UiNodeId::new(93_221);
    let build_artifacts = UiNodeId::new(93_222);
    let recovery_checkpoint = UiNodeId::new(93_223);
    let recovery_state = UiNodeId::new(93_224);
    let history_divider = UiNodeId::new(93_225);
    let build_divider = UiNodeId::new(93_226);
    // The World viewport is the primary work surface. Keep durable
    // history/build/recovery facts visible in the compact summary, and expand
    // their shelf only after an explicit focus/open action. This protects the
    // canvas from a default console-sized region while retaining a keyboard
    // path to every recovery control.
    let shelf_expanded = active_panel(
        view,
        &[
            EditorPanelId::History,
            EditorPanelId::Build,
            EditorPanelId::ProjectRecovery,
            EditorPanelId::Diagnostics,
        ],
    );

    if shelf_expanded {
        nodes.push(fixed_width(
            UiNode::button(undo, "Undo latest source command", "editor.undo", "Undo")
                .with_style(activity_shelf_action_style(false)),
            72.0,
        ));
        nodes.push(fixed_width(
            UiNode::button(redo, "Redo latest source command", "editor.redo", "Redo")
                .with_style(activity_shelf_action_style(false)),
            72.0,
        ));
        nodes.push(fixed_width(
            UiNode::button(
                recover,
                "Recover Creator source session",
                "editor.recover",
                "Recover",
            )
            .with_style(activity_shelf_action_style(true)),
            84.0,
        ));
        nodes.push(fixed_height(
            UiNode::label(history_title, "World history", "History")
                .with_style(world_panel_title_style()),
            20.0,
        ));
        nodes.push(fixed_height(
            UiNode::label(
                history_detail,
                "World history summary",
                format!(
                    "{} undo · {} redo · {} checkpoints",
                    session.undo_depth(),
                    session.redo_depth(),
                    session.checkpoints().len()
                ),
            )
            .with_style(activity_shelf_detail_style()),
            20.0,
        ));
        nodes.push(fixed_height(
            UiNode::label(
                history_state,
                "World source mutation state",
                "Source unchanged",
            )
            .with_style(activity_shelf_hint_style()),
            18.0,
        ));
        nodes.push(
            UiNode::container(
                history_panel,
                "World history panel",
                UiLayout::VerticalStack { gap: 4.0 },
                vec![history_title, history_detail, history_state],
            )
            .with_style(activity_shelf_section_style()),
        );

        nodes.push(fixed_height(
            UiNode::label(build_title, "World build", "Build")
                .with_style(world_panel_title_style()),
            20.0,
        ));
        nodes.push(fixed_height(
            UiNode::label(
                build_detail,
                "World build status",
                bounded_text(&view.build, 72),
            )
            .with_style(activity_shelf_detail_style()),
            20.0,
        ));
        nodes.push(fixed_height(
            UiNode::label(
                build_scope,
                "World build source scope",
                "Input · saved source",
            )
            .with_style(activity_shelf_hint_style()),
            18.0,
        ));
        nodes.push(fixed_height(
            UiNode::label(
                build_artifacts,
                "World build artifact status",
                "Artifacts · none",
            )
            .with_style(activity_shelf_hint_style()),
            18.0,
        ));
        nodes.push(fixed_size(
            UiNode::button(
                inspect_build,
                "Inspect durable build progress and artifacts",
                "build.inspect",
                "Build details",
            )
            .with_style(creator_compact_action_style(
                EditorPanelId::Build,
                "build.inspect",
            )),
            112.0,
            44.0,
        ));
        nodes.push(
            UiNode::container(
                build_panel,
                "World build panel",
                UiLayout::VerticalStack { gap: 4.0 },
                vec![
                    build_title,
                    build_detail,
                    build_scope,
                    build_artifacts,
                    inspect_build,
                ],
            )
            .with_style(activity_shelf_section_style()),
        );

        nodes.push(fixed_height(
            UiNode::label(recovery_title, "World recovery", "Recovery")
                .with_style(world_panel_title_style()),
            20.0,
        ));
        nodes.push(fixed_height(
            UiNode::label(
                recovery_detail,
                "World recovery status",
                bounded_text(&view.recovery, 72),
            )
            .with_style(activity_shelf_detail_style()),
            20.0,
        ));
        nodes.push(fixed_height(
            UiNode::label(
                recovery_checkpoint,
                "World checkpoint summary",
                if session.checkpoints().is_empty() {
                    "No checkpoint available".to_owned()
                } else {
                    format!("{} checkpoint(s) available", session.checkpoints().len())
                },
            )
            .with_style(activity_shelf_hint_style()),
            18.0,
        ));
        nodes.push(fixed_height(
            UiNode::label(
                recovery_state,
                "World recovery source state",
                "Recovery is manual",
            )
            .with_style(activity_shelf_hint_style()),
            18.0,
        ));
        nodes.push(fixed_width(
            UiNode::button(
                diagnostics,
                "Show Creator diagnostics",
                "editor.show-diagnostic",
                "Diagnostics",
            )
            .with_style(creator_compact_action_style(
                EditorPanelId::Diagnostics,
                "editor.show-diagnostic",
            )),
            96.0,
        ));
        nodes.push(fixed_height(
            transparent_group(
                recovery_actions,
                "World recovery actions",
                UiLayout::HorizontalStack { gap: 8.0 },
                vec![diagnostics],
            ),
            44.0,
        ));
        nodes.push(
            UiNode::container(
                recovery_panel,
                "World recovery panel",
                UiLayout::VerticalStack { gap: 4.0 },
                vec![
                    recovery_title,
                    recovery_detail,
                    recovery_checkpoint,
                    recovery_state,
                    recovery_actions,
                ],
            )
            .with_style(activity_shelf_section_style()),
        );
    }

    if shelf_expanded {
        nodes.push(fixed_width(
            transparent_group(
                history_divider,
                "History and build separator",
                UiLayout::Overlay,
                Vec::new(),
            )
            .with_style(activity_shelf_divider_style()),
            1.0,
        ));
        nodes.push(fixed_width(
            transparent_group(
                build_divider,
                "Build and recovery separator",
                UiLayout::Overlay,
                Vec::new(),
            )
            .with_style(activity_shelf_divider_style()),
            1.0,
        ));
    }

    nodes.push(fixed_size(
        UiNode::button(
            shelf_header,
            "Open World history, build, and recovery",
            "shell.open-shelf",
            "Activity",
        )
        .with_style(workspace_tab_style(true)),
        112.0,
        if shelf_expanded { 44.0 } else { 24.0 },
    ));
    let compact_build_state = if view.build.starts_with("Ready") {
        "Build ready".to_owned()
    } else {
        format!("Build · {}", bounded_text(&view.build, 24))
    };
    let compact_recovery_state = if session.checkpoints().is_empty() {
        "Recovery none".to_owned()
    } else {
        format!("Recovery {} checkpoint(s)", session.checkpoints().len())
    };
    nodes.push(
        UiNode::label(
            shelf_summary,
            "World activity summary",
            format!(
                "{} undo · {} redo · {} · {}",
                session.undo_depth(),
                session.redo_depth(),
                compact_build_state,
                compact_recovery_state,
            ),
        )
        .with_style(creator_meta_style()),
    );
    let shelf_header_children = if shelf_expanded {
        vec![shelf_header, shelf_summary, undo, redo, recover]
    } else {
        // The 32 px peek is a calm entry point rather than a compressed row
        // of commands. Undo, redo, and recovery remain one activation away
        // in the expanded shelf, where their full 44 px targets fit.
        vec![shelf_header, shelf_summary]
    };
    nodes.push(fixed_height(
        transparent_group(
            shelf_header_row,
            "World bottom shelf controls",
            UiLayout::HorizontalStack { gap: 8.0 },
            shelf_header_children,
        ),
        if shelf_expanded { 44.0 } else { 24.0 },
    ));
    if shelf_expanded {
        nodes.push(fixed_height(
            UiNode::container(
                shelf_body,
                "World bottom shelf content",
                UiLayout::HorizontalStack { gap: 12.0 },
                vec![
                    history_panel,
                    history_divider,
                    build_panel,
                    build_divider,
                    recovery_panel,
                ],
            )
            .with_style(UiStyle::transparent()),
            184.0,
        ));
    }
    let bottom_shelf_children = if shelf_expanded {
        vec![shelf_header_row, shelf_body]
    } else {
        vec![shelf_header_row]
    };
    nodes.push(fixed_height(
        UiNode::container(
            bottom_shelf,
            "World bottom shelf",
            UiLayout::VerticalStack { gap: 4.0 },
            bottom_shelf_children,
        )
        .with_style(world_shelf_style())
        .with_elevation(UiElevation::Raised),
        if shelf_expanded { 240.0 } else { 32.0 },
    ));

    let status_row = push_status_row(
        &mut nodes,
        if session.play_active() {
            "Source saved · Play fork isolated"
        } else {
            "Source saved"
        },
        bounded_text(&view.activity, 100),
        session.play_active(),
    );
    nodes.push(
        UiNode::container(
            root,
            "Meridian World workspace",
            UiLayout::VerticalStack { gap: 0.0 },
            vec![
                application_row,
                workspace_row,
                main,
                bottom_shelf,
                status_row,
            ],
        )
        .with_style(workspace_canvas_style()),
    );
    creator_authored_document(root, nodes)
}

#[allow(clippy::too_many_lines)] // Code has two intentionally distinct source-backed layouts.
fn creator_code_workspace_document(
    session: &EditorSession,
    view: &CreatorWorkspaceView,
) -> Result<UiDocument, UiDocumentError> {
    let root = UiNodeId::new(95_000);
    let main = UiNodeId::new(95_001);
    let rail = UiNodeId::new(95_002);
    let browser = UiNodeId::new(95_003);
    let viewport = UiNodeId::new(95_004);
    let source = UiNodeId::new(95_005);
    let viewport_title = UiNodeId::new(95_006);
    let viewport_detail = UiNodeId::new(95_007);
    let code_title = UiNodeId::new(95_008);
    let code_detail = UiNodeId::new(95_009);
    let code_source = UiNodeId::new(95_010);
    let browser_title = UiNodeId::new(95_011);
    let browser_tree = UiNodeId::new(95_012);
    let project_source = UiNodeId::new(95_013);
    let selected_source = UiNodeId::new(95_014);
    let code_actions = UiNodeId::new(95_015);
    let inspect_source = UiNodeId::new(95_016);
    let diagnostics = UiNodeId::new(95_017);
    let shelf = UiNodeId::new(95_018);
    let mut nodes = Vec::new();
    let compact_context =
        view.code_context_width == CodeContextWidth::Compact && !view.focus_layout;

    let application_row = push_application_row(
        &mut nodes,
        &bounded_text(&view.project, 34),
        true,
        session.play_active(),
    );
    let workspace_row = push_workspace_row(&mut nodes, Some(WorkspaceKind::Code));

    let rail_items = if compact_context {
        [
            (
                95_020,
                IconId::Search,
                "Inspect project files",
                "asset.inspect-source",
            ),
            (95_021, IconId::More, "Code favorites", "shell.favorites"),
            (95_022, IconId::Settings, "Code panels", "shell.panels"),
        ]
    } else {
        [
            (95_020, IconId::Search, "Search Code", "shell.search"),
            (95_021, IconId::More, "Code favorites", "shell.favorites"),
            (95_022, IconId::Settings, "Code panels", "shell.panels"),
        ]
    };
    let mut rail_children = Vec::new();
    for (id, icon, name, action) in rail_items {
        let id = UiNodeId::new(id);
        nodes.push(fixed_height(
            UiNode::icon_button(id, name, action, icon).with_style(workspace_tab_style(false)),
            34.0,
        ));
        rail_children.push(id);
    }
    nodes.push(
        UiNode::container(
            rail,
            "Code activity rail",
            UiLayout::VerticalStack { gap: 8.0 },
            rail_children,
        )
        .with_style(creator_activity_rail_style())
        .with_layout_hints(UiLayoutHints::fixed_width(44.0)),
    );

    if !compact_context {
        nodes.push(fixed_height(
            UiNode::label(browser_title, "Code project files", "Project files")
                .with_style(side_panel_title_style()),
            22.0,
        ));
        nodes.push(fixed_height(
            UiNode::search_input(CREATOR_DOMAIN_SEARCH, "Search project files", "")
                .with_placeholder("Search project files")
                .with_style(UiStyle::text_field()),
            36.0,
        ));
        nodes.push(fixed_height(
            creator_tree_group_item(
                project_source,
                "Canonical project source",
                "asset.inspect-source",
                false,
                true,
            ),
            32.0,
        ));
        nodes.push(fixed_height(
            creator_tree_child_item(
                selected_source,
                selected_placement(session).map_or_else(
                    || "No selected source".to_owned(),
                    |placement| bounded_text(&placement.label, 32),
                ),
                "editor.select-placement",
                !session.selection().ids.is_empty(),
            ),
            32.0,
        ));
        nodes.push(
            UiNode::tree(
                browser_tree,
                "Code project files",
                vec![project_source, selected_source],
            )
            .with_style(UiStyle::transparent()),
        );
        nodes.push(
            UiNode::container(
                browser,
                "Code project browser",
                UiLayout::VerticalStack { gap: 8.0 },
                vec![browser_title, CREATOR_DOMAIN_SEARCH, browser_tree],
            )
            .with_style(world_panel_style(10.0))
            // The browser is a navigation aid, not one of Code's primary
            // work surfaces. Keep it deliberately slimmer so contextual Code
            // gives equal visual weight to the live World and source panels.
            .with_layout_hints(UiLayoutHints::fixed_width(232.0)),
        );
    }

    nodes.push(fixed_height(
        UiNode::label(viewport_title, "Live World context", "World context")
            .with_style(world_panel_title_style()),
        24.0,
    ));
    nodes.push(fixed_height(
        UiNode::label(
            viewport_detail,
            "World context detail",
            selected_placement_summary(session).map_or_else(
                || "No source placement is selected.".to_owned(),
                |summary| bounded_text(&summary, 96),
            ),
        )
        .with_style(creator_meta_style()),
        40.0,
    ));
    nodes.push(
        UiNode::canvas(
            CREATOR_WORLD_VIEWPORT_CANVAS,
            "Live source-derived World context",
            Vec::new(),
        )
        .with_style(world_canvas_style())
        .with_constraints(UiConstraints {
            minimum: UiSize::new(240.0, 180.0),
            clip: true,
            ..UiConstraints::default()
        }),
    );
    let viewport_node = UiNode::container(
        viewport,
        "Code World context viewport",
        UiLayout::VerticalStack { gap: 8.0 },
        vec![
            viewport_title,
            viewport_detail,
            CREATOR_WORLD_VIEWPORT_CANVAS,
        ],
    )
    .with_style(world_panel_style(10.0))
    .with_elevation(UiElevation::Raised);
    nodes.push(if view.focus_layout {
        viewport_node.with_layout_hints(UiLayoutHints::fixed_width(264.0))
    } else {
        viewport_node
    });

    nodes.push(fixed_height(
        UiNode::label(code_title, "Canonical project source", "Source")
            .with_style(world_panel_title_style()),
        24.0,
    ));
    nodes.push(fixed_height(
        UiNode::label(
            code_detail,
            "Code source authority",
            if view.focus_layout {
                "The full Code layout is active. Press Escape to return to contextual source."
            } else if compact_context {
                "Project files are compact at this width. Source stays beside a live World context."
            } else {
                "Source stays beside a live World context. Activate Code again for the remembered focused layout."
            },
        )
        .with_style(creator_meta_style()),
        36.0,
    ));
    push_code_source_listing(
        &mut nodes,
        code_source,
        95_100,
        &view.project_source,
        if view.focus_layout { 72 } else { 40 },
        if view.focus_layout {
            30
        } else if compact_context {
            // A compact Code split must leave a clear boundary before its
            // source actions. Showing fewer rows is more useful than letting
            // the final source line visually collide with those controls.
            18
        } else {
            24
        },
    );
    nodes.push(fixed_width(
        UiNode::button(
            inspect_source,
            "Inspect canonical project source",
            "asset.inspect-source",
            "Inspect source",
        )
        .with_style(creator_compact_action_style(
            EditorPanelId::Assets,
            "asset.inspect-source",
        )),
        120.0,
    ));
    nodes.push(fixed_width(
        UiNode::button(
            diagnostics,
            "Show Creator diagnostics",
            "editor.show-diagnostic",
            "Diagnostics",
        )
        .with_style(creator_compact_action_style(
            EditorPanelId::Diagnostics,
            "editor.show-diagnostic",
        )),
        104.0,
    ));
    nodes.push(fixed_height(
        transparent_group(
            code_actions,
            "Code source actions",
            UiLayout::HorizontalStack { gap: 8.0 },
            vec![inspect_source, diagnostics],
        ),
        44.0,
    ));
    let source_node = UiNode::container(
        source,
        "Code source surface",
        UiLayout::VerticalStack { gap: 8.0 },
        vec![code_title, code_detail, code_source, code_actions],
    )
    .with_style(world_panel_style(10.0))
    .with_elevation(UiElevation::Raised);
    nodes.push(if view.focus_layout {
        source_node
    } else {
        // Contextual Code is a real split workspace. Keep the canonical source
        // wide enough for identifiers and a readable gutter while retaining a
        // useful live World surface at the smallest supported width.
        // A first Code activation keeps the World live beside source, but the
        // document itself must still receive enough horizontal measure to be
        // legible. The browser is yielded at the compact breakpoint; above it
        // the source column holds a stable 560 px reading measure instead of
        // becoming a narrow diagnostic strip on a wide desktop.
        source_node.with_layout_hints(UiLayoutHints::fixed_width(
            if view.code_context_width == CodeContextWidth::Wide {
                560.0
            } else {
                432.0
            },
        ))
    });

    let mut main_children = vec![rail];
    if !compact_context {
        main_children.push(browser);
    }
    if view.focus_layout {
        main_children.extend([source, viewport]);
    } else {
        main_children.extend([viewport, source]);
    }
    nodes.push(
        UiNode::container(
            main,
            "Meridian Code workspace",
            UiLayout::HorizontalStack { gap: 8.0 },
            main_children,
        )
        // Code shares the same deliberate outer gutter as World, Modeler, and
        // UI authoring. The contextual split may tighten internally at narrow
        // widths, but never reads as a panel accidentally glued to the window.
        .with_style(creator_workbench_canvas_style()),
    );

    let activity = UiNodeId::new(95_023);
    nodes.push(
        UiNode::label(
            activity,
            "Code source shelf guidance",
            "Read-only source preview · Inspect source for the full authoritative document",
        )
        .with_style(creator_meta_style()),
    );
    nodes.push(fixed_height(
        UiNode::container(
            shelf,
            "Code bottom shelf",
            UiLayout::HorizontalStack { gap: 8.0 },
            vec![activity],
        )
        .with_style(shell_row_style(UiColor::surface(), 4.0)),
        32.0,
    ));
    let status_row = push_status_row(
        &mut nodes,
        "Project source is authoritative",
        bounded_text(&view.activity, 100),
        session.play_active(),
    );
    nodes.push(
        UiNode::container(
            root,
            "Meridian Code workspace",
            UiLayout::VerticalStack { gap: 0.0 },
            vec![application_row, workspace_row, main, shelf, status_row],
        )
        .with_style(workspace_canvas_style()),
    );
    creator_authored_document(root, nodes)
}

/// Builds the native modeler from its immutable source revision and derived
/// Penumbra preview descriptor. The work surface intentionally shows no fake
/// mesh, material, or topology tool: every listed count and every preview path
/// comes from the current source-derived presentation.
#[allow(clippy::too_many_lines)]
fn creator_modeler_workspace_document(
    session: &EditorSession,
    view: &CreatorWorkspaceView,
) -> Result<UiDocument, UiDocumentError> {
    let base = workspace_node_base(WorkspaceKind::Modeler);
    let root = UiNodeId::new(base + 1);
    let main = UiNodeId::new(base + 2);
    let rail = UiNodeId::new(base + 3);
    let browser = UiNodeId::new(base + 4);
    let surface = UiNodeId::new(base + 5);
    let inspector = UiNodeId::new(base + 6);
    let shelf = UiNodeId::new(base + 7);
    let presentation = view.modeler.as_ref();
    let document_label =
        presentation.map_or("Model source", |modeler| modeler.document_label.as_str());
    let object_label = presentation.map_or("No editable object", |modeler| {
        modeler.object_label.as_str()
    });
    let generation = presentation.map_or(0, |modeler| modeler.generation);
    let object_count = presentation.map_or(0, |modeler| modeler.object_count);
    let vertex_count = presentation.map_or(0, |modeler| modeler.vertex_count);
    let edge_count = presentation.map_or(0, |modeler| modeler.edge_count);
    let face_count = presentation.map_or(0, |modeler| modeler.face_count);
    let preview_triangles = presentation
        .and_then(|modeler| modeler.preview.as_ref())
        .map_or(0, |preview| preview.triangle_indices.len() / 3);
    let mut nodes = Vec::new();

    let application_row = push_application_row(
        &mut nodes,
        &bounded_text(&view.project, 34),
        true,
        session.play_active(),
    );
    let workspace_row = push_workspace_row(&mut nodes, Some(WorkspaceKind::Modeler));

    let rail_items = [
        (
            base + 10,
            IconId::Search,
            "Search model source",
            "shell.search",
        ),
        (
            base + 11,
            IconId::More,
            "Modeler favorites",
            "shell.favorites",
        ),
        (
            base + 12,
            IconId::Settings,
            "Modeler panels",
            "shell.panels",
        ),
    ];
    let mut rail_children = Vec::new();
    for (id, icon, name, action) in rail_items {
        let id = UiNodeId::new(id);
        nodes.push(fixed_height(
            UiNode::icon_button(id, name, action, icon).with_style(workspace_tab_style(false)),
            34.0,
        ));
        rail_children.push(id);
    }
    nodes.push(
        UiNode::container(
            rail,
            "Modeler activity rail",
            UiLayout::VerticalStack { gap: 8.0 },
            rail_children,
        )
        .with_style(creator_activity_rail_style())
        .with_layout_hints(UiLayoutHints::fixed_width(44.0)),
    );

    let browser_title = UiNodeId::new(base + 20);
    let browser_mode = UiNodeId::new(base + 21);
    let source_root = UiNodeId::new(base + 22);
    let object_item = UiNodeId::new(base + 23);
    let vertices_item = UiNodeId::new(base + 24);
    let edges_item = UiNodeId::new(base + 25);
    let faces_item = UiNodeId::new(base + 26);
    let preview_item = UiNodeId::new(base + 27);
    let model_tree = UiNodeId::new(base + 28);
    nodes.push(fixed_height(
        UiNode::label(
            browser_title,
            "Modeler source browser title",
            "Model structure",
        )
        .with_style(side_panel_title_style()),
        22.0,
    ));
    nodes.push(fixed_height(
        UiNode::label(
            browser_mode,
            "Modeler source browser mode",
            "EDITABLE MODEL SOURCE",
        )
        .with_style(creator_hub_field_label_style()),
        16.0,
    ));
    nodes.push(fixed_height(
        UiNode::search_input(CREATOR_DOMAIN_SEARCH, "Search model source", "")
            .with_placeholder("Search model source")
            .with_style(UiStyle::text_field()),
        36.0,
    ));
    for (id, name, selected, expanded, group) in [
        (
            source_root,
            format!("Source · {}", bounded_text(document_label, 24)),
            false,
            true,
            true,
        ),
        (
            object_item,
            format!("Object · {}", bounded_text(object_label, 24)),
            true,
            false,
            false,
        ),
        (
            vertices_item,
            format!("Vertices · {vertex_count}"),
            false,
            false,
            false,
        ),
        (
            edges_item,
            format!("Edges · {edge_count}"),
            false,
            false,
            false,
        ),
        (
            faces_item,
            format!("Faces · {face_count}"),
            false,
            false,
            false,
        ),
        (
            preview_item,
            format!("Derived preview · {preview_triangles} triangles"),
            false,
            false,
            true,
        ),
    ] {
        nodes.push(fixed_height(
            if group {
                creator_tree_group_item(id, name, "model.inspect-source", selected, expanded)
            } else {
                creator_tree_child_item(id, name, "model.inspect-source", selected)
            },
            28.0,
        ));
    }
    nodes.push(
        UiNode::tree(
            model_tree,
            "Editable model source hierarchy",
            vec![
                source_root,
                object_item,
                vertices_item,
                edges_item,
                faces_item,
                preview_item,
            ],
        )
        .with_style(UiStyle::transparent()),
    );
    nodes.push(
        UiNode::container(
            browser,
            "Modeler source browser",
            UiLayout::VerticalStack { gap: 8.0 },
            vec![
                browser_title,
                browser_mode,
                CREATOR_DOMAIN_SEARCH,
                model_tree,
            ],
        )
        .with_style(world_panel_style(10.0))
        .with_layout_hints(UiLayoutHints::fixed_width(264.0)),
    );

    let preview_title = UiNodeId::new(base + 30);
    let preview_detail = UiNodeId::new(base + 31);
    let preview_status = UiNodeId::new(base + 32);
    let operation_actions = UiNodeId::new(base + 33);
    nodes.push(fixed_height(
        UiNode::label(
            preview_title,
            "Derived Penumbra model preview",
            "Penumbra preview",
        )
        .with_style(world_panel_title_style()),
        24.0,
    ));
    nodes.push(fixed_height(
        UiNode::label(
            preview_detail,
            "Model preview provenance",
            format!("r{generation} · {object_count} object(s) · {preview_triangles} triangles"),
        )
        .with_style(creator_meta_style()),
        20.0,
    ));
    nodes.push(
        UiNode::canvas(
            CREATOR_MODELER_PREVIEW_CANVAS,
            "Derived editable-model wireframe preview",
            Vec::new(),
        )
        .with_style(world_canvas_style())
        .with_constraints(UiConstraints {
            minimum: UiSize::new(320.0, 240.0),
            clip: true,
            ..UiConstraints::default()
        }),
    );
    nodes.push(fixed_height(
        UiNode::label(
            preview_status,
            "Model preview derivation state",
            if presentation
                .and_then(|modeler| modeler.preview.as_ref())
                .is_some()
            {
                "Derived preview · source authoritative."
            } else {
                "No derived preview for this revision."
            },
        )
        .with_style(creator_meta_style()),
        20.0,
    ));
    let actions = push_creator_action_grid(
        &mut nodes,
        operation_actions,
        base + 34,
        "Modeler",
        EditorPanelId::Modeler,
        session,
        &[
            "model.inspect-source",
            "model.create-primitive",
            "model.transform",
            "model.split-edge",
            "model.undo",
            "model.redo",
            "model.recover",
        ],
        4,
        80.0,
    );
    nodes.push(
        UiNode::container(
            surface,
            "Modeler derived preview surface",
            UiLayout::VerticalStack { gap: 8.0 },
            vec![
                preview_title,
                preview_detail,
                CREATOR_MODELER_PREVIEW_CANVAS,
                preview_status,
                actions,
            ],
        )
        .with_style(world_panel_style(10.0))
        .with_elevation(UiElevation::Raised),
    );

    let inspector_title = UiNodeId::new(base + 50);
    let inspector_mode = UiNodeId::new(base + 51);
    let selection_summary = UiNodeId::new(base + 52);
    let topology_summary = UiNodeId::new(base + 53);
    let source_summary = UiNodeId::new(base + 54);
    let capability_boundary = UiNodeId::new(base + 55);
    nodes.push(fixed_height(
        UiNode::label(
            inspector_title,
            "Modeler inspector title",
            "Topology & history",
        )
        .with_style(side_panel_title_style()),
        22.0,
    ));
    nodes.push(fixed_height(
        UiNode::label(inspector_mode, "Modeler inspector mode", "SOURCE REVISION")
            .with_style(creator_hub_field_label_style()),
        16.0,
    ));
    nodes.push(fixed_height(
        UiNode::label(
            selection_summary,
            "Modeler selection summary",
            format!("Selected object · {object_label}"),
        )
        .with_style(creator_value_style()),
        20.0,
    ));
    nodes.push(fixed_height(
        UiNode::label(
            topology_summary,
            "Modeler topology summary",
            format!("{vertex_count} vertices · {edge_count} edges · {face_count} faces"),
        )
        .with_style(creator_value_style()),
        20.0,
    ));
    nodes.push(fixed_height(
        UiNode::label(
            source_summary,
            "Modeler source authority",
            "Typed operations preserve source, semantic undo, and recovery.",
        )
        .with_style(creator_value_style()),
        36.0,
    ));
    nodes.push(fixed_height(
        UiNode::label(
            capability_boundary,
            "Modeler capability boundary",
            "UVs, modifiers, collision, LOD, and interchange remain unavailable in this foundation.",
        )
        .with_style(domain_capability_note_style()),
        64.0,
    ));
    nodes.push(
        UiNode::container(
            inspector,
            "Modeler topology inspector",
            UiLayout::VerticalStack { gap: 8.0 },
            vec![
                inspector_title,
                inspector_mode,
                selection_summary,
                topology_summary,
                source_summary,
                capability_boundary,
            ],
        )
        .with_style(world_panel_style(10.0))
        .with_layout_hints(UiLayoutHints::fixed_width(344.0)),
    );

    nodes.push(
        UiNode::container(
            main,
            "Meridian Modeler workspace",
            UiLayout::HorizontalStack { gap: 8.0 },
            vec![rail, browser, surface, inspector],
        )
        .with_style(creator_workbench_canvas_style()),
    );

    let activity = UiNodeId::new(base + 60);
    nodes.push(
        UiNode::label(
            activity,
            "Modeler source summary",
            format!(
                "r{generation} · {object_count} object(s) · {preview_triangles} derived triangles"
            ),
        )
        .with_style(creator_meta_style()),
    );
    nodes.push(fixed_height(
        UiNode::container(
            shelf,
            "Modeler activity shelf",
            UiLayout::HorizontalStack { gap: 8.0 },
            vec![activity],
        )
        .with_style(shell_row_style(UiColor::surface(), 4.0)),
        32.0,
    ));
    let status_row = push_status_row(
        &mut nodes,
        "Editable model source is authoritative",
        bounded_text(&view.activity, 100),
        session.play_active(),
    );
    nodes.push(
        UiNode::container(
            root,
            "Meridian Modeler workspace",
            UiLayout::VerticalStack { gap: 0.0 },
            vec![application_row, workspace_row, main, shelf, status_row],
        )
        .with_style(workspace_canvas_style()),
    );
    creator_authored_document(root, nodes)
}

/// Builds the first UI-authoring workspace as an honest inspection surface.
///
/// The browser exposes the authored `UiDocument` vocabulary, the centre hosts
/// a dominant renderer-neutral frame preview plus a bounded responsive/state
/// inspection shelf, and the inspector exposes the locked styling contract.
/// Direct manipulation, responsive authoring, and state editing deliberately
/// remain outside this bounded package.
#[allow(clippy::too_many_lines)]
fn creator_ui_authoring_workspace_document(
    session: &EditorSession,
    view: &CreatorWorkspaceView,
) -> Result<UiDocument, UiDocumentError> {
    let base = workspace_node_base(WorkspaceKind::UiAuthoring);
    let root = UiNodeId::new(base + 1);
    let main = UiNodeId::new(base + 2);
    let rail = UiNodeId::new(base + 3);
    let browser = UiNodeId::new(base + 4);
    let surface = UiNodeId::new(base + 5);
    let inspector = UiNodeId::new(base + 6);
    let shelf = UiNodeId::new(base + 7);
    let center = UiNodeId::new(base + 8);
    let mut nodes = Vec::new();
    let inspection = inspect_creator_world_document(session, view)?;
    let compact = view.compact_ui_authoring;

    let application_row = push_application_row(
        &mut nodes,
        &bounded_text(&view.project, 34),
        true,
        session.play_active(),
    );
    let workspace_row = push_workspace_row(&mut nodes, Some(WorkspaceKind::UiAuthoring));

    if !compact {
        let rail_items = [
            (
                base + 10,
                IconId::Search,
                "Search UI source",
                "shell.search",
            ),
            (
                base + 11,
                IconId::More,
                "UI authoring favorites",
                "shell.favorites",
            ),
            (
                base + 12,
                IconId::Settings,
                "UI authoring panels",
                "shell.panels",
            ),
        ];
        let mut rail_children = Vec::new();
        for (id, icon, name, action) in rail_items {
            let id = UiNodeId::new(id);
            nodes.push(fixed_height(
                UiNode::icon_button(id, name, action, icon).with_style(workspace_tab_style(false)),
                34.0,
            ));
            rail_children.push(id);
        }
        nodes.push(
            UiNode::container(
                rail,
                "UI activity rail",
                UiLayout::VerticalStack { gap: 8.0 },
                rail_children,
            )
            .with_style(creator_activity_rail_style())
            .with_layout_hints(UiLayoutHints::fixed_width(44.0)),
        );
    }

    let browser_header = UiNodeId::new(base + 18);
    let browser_header_row = UiNodeId::new(base + 16);
    let browser_header_divider = UiNodeId::new(base + 17);
    let browser_overflow = UiNodeId::new(base + 19);
    let browser_title = UiNodeId::new(base + 20);
    let browser_mode = UiNodeId::new(base + 21);
    let source_schema = UiNodeId::new(base + 22);
    let component_root = UiNodeId::new(base + 23);
    let application_row_component = UiNodeId::new(base + 24);
    let workspace_row_component = UiNodeId::new(base + 25);
    let world_component = UiNodeId::new(base + 26);
    let styles_root = UiNodeId::new(base + 27);
    let assets_root = UiNodeId::new(base + 28);
    let component_tree = UiNodeId::new(base + 29);
    let responsive_root = UiNodeId::new(base + 65);
    let recovery_root = UiNodeId::new(base + 66);
    nodes.push(
        UiNode::label(browser_title, "UI authoring browser title", "Hierarchy")
            .with_style(side_panel_title_style())
            .with_layout_hints(UiLayoutHints::flexible()),
    );
    nodes.push(fixed_width(
        UiNode::label(browser_mode, "UI authoring browser mode", "SOURCE")
            .with_style(creator_hub_field_label_style()),
        if compact { 64.0 } else { 76.0 },
    ));
    nodes.push(fixed_width(
        UiNode::icon_button(
            browser_overflow,
            "UI authoring browser menu",
            "shell.panels",
            IconId::More,
        )
        .with_style(shell_icon_action_style()),
        32.0,
    ));
    nodes.push(fixed_height(
        transparent_group(
            browser_header_row,
            "UI authoring browser title row",
            UiLayout::HorizontalStack { gap: 4.0 },
            vec![browser_title, browser_mode, browser_overflow],
        ),
        24.0,
    ));
    nodes.push(fixed_height(
        transparent_group(
            browser_header_divider,
            "UI authoring browser header divider",
            UiLayout::Overlay,
            Vec::new(),
        )
        .with_style(panel_header_divider_style()),
        1.0,
    ));
    nodes.push(fixed_height(
        transparent_group(
            browser_header,
            "UI authoring browser header",
            UiLayout::VerticalStack { gap: 7.0 },
            vec![browser_header_row, browser_header_divider],
        ),
        32.0,
    ));
    nodes.push(fixed_height(
        UiNode::search_input(CREATOR_DOMAIN_SEARCH, "Search UiDocument", "")
            .with_placeholder("Search UiDocument")
            .with_style(UiStyle::text_field()),
        36.0,
    ));
    nodes.push(fixed_height(
        UiNode::label(
            source_schema,
            "UiDocument schema",
            format!("schema · ui-document/v{}", inspection.schema_version),
        )
        .with_style(creator_code_source_style())
        .with_font_role(UiFontRole::Monospace),
        20.0,
    ));
    let mut source_component_labels = inspection.component_instances.clone();
    while source_component_labels.len() < 3 {
        source_component_labels.push("No component instance in this source".to_owned());
    }
    for (id, name, selected, expanded, group) in [
        (
            component_root,
            format!("Root · {} instances", inspection.component_instance_count),
            true,
            true,
            true,
        ),
        (
            application_row_component,
            format!(
                "Component · {}",
                bounded_text(&source_component_labels[0], 22)
            ),
            false,
            false,
            false,
        ),
        (
            workspace_row_component,
            format!(
                "Component · {}",
                bounded_text(&source_component_labels[1], 22)
            ),
            false,
            false,
            false,
        ),
        (
            world_component,
            format!(
                "Component · {}",
                bounded_text(&source_component_labels[2], 22)
            ),
            false,
            false,
            false,
        ),
        (
            styles_root,
            format!(
                "Styles & tokens · {} authored styles",
                inspection.authored_styles
            ),
            false,
            false,
            true,
        ),
        (
            assets_root,
            format!(
                "Packaged assets · {} references",
                inspection.packaged_assets
            ),
            false,
            false,
            true,
        ),
        (
            responsive_root,
            "Responsive states · 1× / 2×".to_owned(),
            false,
            false,
            true,
        ),
        (
            recovery_root,
            "Source recovery · versioned envelope".to_owned(),
            false,
            false,
            true,
        ),
    ] {
        nodes.push(fixed_height(
            if group {
                creator_tree_group_item(id, name, "asset.inspect-source", selected, expanded)
            } else {
                creator_tree_child_item(id, name, "asset.inspect-source", selected)
            },
            24.0,
        ));
    }
    nodes.push(
        UiNode::tree(
            component_tree,
            "UiDocument component tree",
            vec![
                component_root,
                application_row_component,
                workspace_row_component,
                world_component,
                styles_root,
                assets_root,
                responsive_root,
                recovery_root,
            ],
        )
        .with_style(UiStyle::transparent()),
    );
    nodes.push(
        UiNode::container(
            browser,
            "UI source browser",
            UiLayout::VerticalStack { gap: 4.0 },
            vec![
                browser_header,
                CREATOR_DOMAIN_SEARCH,
                source_schema,
                component_tree,
            ],
        )
        .with_style(world_panel_style(10.0))
        .with_layout_hints(UiLayoutHints::fixed_width(if compact {
            220.0
        } else {
            264.0
        })),
    );

    let preview_title = UiNodeId::new(base + 30);
    let preview_detail = UiNodeId::new(base + 31);
    let preview_toolbar = UiNodeId::new(base + 32);
    let preview_actions = UiNodeId::new(base + 33);
    let inspect_source = UiNodeId::new(base + 34);
    let diagnostics = UiNodeId::new(base + 35);
    let preview_kind = UiNodeId::new(base + 36);
    let preview_state = UiNodeId::new(base + 37);
    let preview_scale = UiNodeId::new(base + 39);
    nodes.push(fixed_height(
        UiNode::label(
            preview_title,
            "Compiled UI frame preview",
            "Compiled preview",
        )
        .with_style(world_panel_title_style()),
        24.0,
    ));
    nodes.push(fixed_height(
        UiNode::label(
            preview_detail,
            "Compiled UI frame detail",
            format!(
                "{} source nodes · {} display primitives",
                inspection.source_nodes, inspection.display_primitives
            ),
        )
        .with_style(creator_meta_style()),
        22.0,
    ));
    nodes.push(fixed_height(
        fixed_width(
            UiNode::label(preview_kind, "Compiled preview mode", "RESPONSIVE PREVIEW")
                .with_style(creator_hub_field_label_style()),
            132.0,
        ),
        24.0,
    ));
    nodes.push(fixed_height(
        fixed_width(
            UiNode::label(preview_state, "Compiled preview state", "WIDE")
                .with_style(creator_meta_style()),
            72.0,
        ),
        24.0,
    ));
    nodes.push(fixed_height(
        fixed_width(
            UiNode::label(preview_scale, "Compiled preview source size", "1280 × 800")
                .with_style(creator_meta_style())
                .with_font_role(UiFontRole::Monospace),
            112.0,
        ),
        24.0,
    ));
    nodes.push(fixed_height(
        transparent_group(
            preview_toolbar,
            "Compiled preview toolbar",
            UiLayout::HorizontalStack { gap: 8.0 },
            vec![preview_kind, preview_state, preview_scale],
        )
        .with_style(shell_row_style(UiColor::surface(), 4.0)),
        24.0,
    ));
    nodes.push(
        UiNode::canvas(
            CREATOR_UI_AUTHORING_PREVIEW_CANVAS,
            "Derived UiDocument frame preview",
            Vec::new(),
        )
        .with_style(creator_preview_canvas_style())
        .with_elevation(UiElevation::Raised)
        .with_constraints(UiConstraints {
            minimum: UiSize::new(260.0, 220.0),
            clip: true,
            ..UiConstraints::default()
        }),
    );
    nodes.push(fixed_width(
        UiNode::button(
            inspect_source,
            "Inspect authored UiDocument source",
            "asset.inspect-source",
            "Inspect source",
        )
        .with_style(creator_compact_action_style(
            EditorPanelId::Assets,
            "asset.inspect-source",
        )),
        128.0,
    ));
    nodes.push(fixed_width(
        UiNode::button(
            diagnostics,
            "Show UI authoring diagnostics",
            "editor.show-diagnostic",
            "Diagnostics",
        )
        .with_style(creator_compact_action_style(
            EditorPanelId::Diagnostics,
            "editor.show-diagnostic",
        )),
        104.0,
    ));
    nodes.push(fixed_height(
        transparent_group(
            preview_actions,
            "UI authoring preview actions",
            UiLayout::HorizontalStack { gap: 8.0 },
            vec![inspect_source, diagnostics],
        ),
        44.0,
    ));
    nodes.push(
        UiNode::container(
            surface,
            "UI compiled preview surface",
            UiLayout::VerticalStack { gap: 8.0 },
            vec![
                preview_title,
                preview_detail,
                preview_toolbar,
                CREATOR_UI_AUTHORING_PREVIEW_CANVAS,
            ],
        )
        .with_style(world_panel_style(10.0)),
    );

    let inspector_title = UiNodeId::new(base + 50);
    let inspector_mode = UiNodeId::new(base + 51);
    let spacing_token = UiNodeId::new(base + 52);
    let radius_token = UiNodeId::new(base + 53);
    let surface_token = UiNodeId::new(base + 54);
    let access_token = UiNodeId::new(base + 55);
    let compiled_states_title = UiNodeId::new(base + 56);
    let compiled_state_wide = UiNodeId::new(base + 57);
    let compiled_state_compact = UiNodeId::new(base + 58);
    let compiled_state_hidpi = UiNodeId::new(base + 59);
    let inspector_header = UiNodeId::new(base + 48);
    let inspector_header_row = UiNodeId::new(base + 46);
    let inspector_header_divider = UiNodeId::new(base + 47);
    let inspector_overflow = UiNodeId::new(base + 49);
    let responsive_title = UiNodeId::new(base + 70);
    let responsive_wide = UiNodeId::new(base + 71);
    let responsive_compact = UiNodeId::new(base + 72);
    let responsive_scale = UiNodeId::new(base + 73);
    let asset_boundary = UiNodeId::new(base + 74);
    let recovery_state = UiNodeId::new(base + 75);
    nodes.push(
        UiNode::label(inspector_title, "UI inspector title", "Tokens")
            .with_style(side_panel_title_style())
            .with_layout_hints(UiLayoutHints::flexible()),
    );
    nodes.push(fixed_width(
        UiNode::label(inspector_mode, "UI inspector mode", "LOCKED")
            .with_style(creator_hub_field_label_style()),
        if compact { 56.0 } else { 68.0 },
    ));
    nodes.push(fixed_width(
        UiNode::icon_button(
            inspector_overflow,
            "UI inspector menu",
            "shell.panels",
            IconId::More,
        )
        .with_style(shell_icon_action_style()),
        32.0,
    ));
    nodes.push(fixed_height(
        transparent_group(
            inspector_header_row,
            "UI inspector title row",
            UiLayout::HorizontalStack { gap: 4.0 },
            vec![inspector_title, inspector_mode, inspector_overflow],
        ),
        24.0,
    ));
    nodes.push(fixed_height(
        transparent_group(
            inspector_header_divider,
            "UI inspector header divider",
            UiLayout::Overlay,
            Vec::new(),
        )
        .with_style(panel_header_divider_style()),
        1.0,
    ));
    nodes.push(fixed_height(
        transparent_group(
            inspector_header,
            "UI inspector header",
            UiLayout::VerticalStack { gap: 7.0 },
            vec![inspector_header_row, inspector_header_divider],
        ),
        32.0,
    ));
    for (id, text) in [
        (spacing_token, "Spacing  ·  4 px steps"),
        (radius_token, "Radii  ·  4 / 6 / 10 / 14"),
        (surface_token, "Surfaces  ·  opaque + 1 px border"),
        (access_token, "High contrast opaque · motion immediate"),
    ] {
        let height = if id == access_token { 32.0 } else { 20.0 };
        nodes.push(fixed_height(
            UiNode::label(id, "UI token value", text).with_style(creator_meta_style()),
            height,
        ));
    }
    nodes.push(fixed_height(
        UiNode::label(
            recovery_state,
            "UI source recovery state",
            "Canonical snapshot revalidates before compile.",
        )
        .with_style(creator_meta_style()),
        32.0,
    ));
    nodes.push(fixed_height(
        UiNode::label(
            compiled_states_title,
            "UI compiled state section",
            "COMPILED STATES",
        )
        .with_style(creator_hub_field_label_style()),
        16.0,
    ));
    for (id, text) in [
        (
            compiled_state_wide,
            format!("Wide · {} prim", inspection.display_primitives),
        ),
        (
            compiled_state_compact,
            format!(
                "Compact · {}/{}",
                inspection.compact_display_primitives, inspection.compact_semantic_nodes
            ),
        ),
        (
            compiled_state_hidpi,
            format!("HiDPI · {} prim", inspection.hidpi_display_primitives),
        ),
    ] {
        nodes.push(fixed_height(
            UiNode::label(id, "UI compiled state value", text).with_style(creator_meta_style()),
            20.0,
        ));
    }
    nodes.push(fixed_height(
        UiNode::label(
            responsive_title,
            "UI responsive inspection title",
            "RESPONSIVE INSPECTION",
        )
        .with_style(creator_hub_field_label_style()),
        16.0,
    ));
    for (id, text) in [
        (
            responsive_wide,
            format!("Wide · {} prim", inspection.display_primitives),
        ),
        (
            responsive_compact,
            format!(
                "Compact · {}/{}",
                inspection.compact_display_primitives, inspection.compact_semantic_nodes
            ),
        ),
        (
            responsive_scale,
            format!("1× / 2× · {} prim", inspection.hidpi_display_primitives),
        ),
    ] {
        nodes.push(fixed_height(
            UiNode::label(id, "UI responsive inspection state", text)
                .with_style(creator_authoring_state_style(id == responsive_wide)),
            36.0,
        ));
    }
    nodes.push(fixed_height(
        UiNode::label(
            asset_boundary,
            "UI authoring capability boundary",
            "Packaged raster assets and audited native vectors are inspectable. Responsive, state, animation, and direct-canvas editing remain later bounded work.",
        )
        .with_style(domain_capability_note_style()),
        64.0,
    ));
    let responsive_controls = UiNodeId::new(base + 80);
    let responsive_controls_title = UiNodeId::new(base + 81);
    let responsive_controls_detail = UiNodeId::new(base + 82);
    let responsive_controls_states = UiNodeId::new(base + 83);
    nodes.push(fixed_height(
        UiNode::label(
            responsive_controls_title,
            "UI responsive state controls title",
            "Responsive · State · Motion",
        )
        .with_style(world_panel_title_style()),
        24.0,
    ));
    nodes.push(fixed_height(
        UiNode::label(
            responsive_controls_detail,
            "UI responsive state controls detail",
            "Read-only variants · source authoritative",
        )
        .with_style(creator_meta_style()),
        28.0,
    ));
    nodes.push(
        UiNode::container(
            responsive_controls_states,
            "UI compiled responsive states",
            UiLayout::HorizontalStack { gap: 8.0 },
            vec![responsive_wide, responsive_compact, responsive_scale],
        )
        .with_style(UiStyle::transparent())
        .with_layout_hints(UiLayoutHints::fixed_height(36.0)),
    );
    nodes.push(
        UiNode::container(
            responsive_controls,
            "UI responsive state inspection surface",
            UiLayout::VerticalStack { gap: 8.0 },
            vec![
                responsive_controls_title,
                responsive_controls_detail,
                responsive_title,
                responsive_controls_states,
                preview_actions,
            ],
        )
        .with_style(world_panel_style(10.0))
        .with_layout_hints(UiLayoutHints::fixed_height(180.0)),
    );
    nodes.push(
        UiNode::container(
            inspector,
            "UI tokens and capability inspector",
            UiLayout::VerticalStack { gap: 4.0 },
            vec![
                inspector_header,
                spacing_token,
                radius_token,
                surface_token,
                access_token,
                recovery_state,
                compiled_states_title,
                compiled_state_wide,
                compiled_state_compact,
                compiled_state_hidpi,
                asset_boundary,
            ],
        )
        .with_style(world_panel_style(10.0))
        .with_layout_hints(UiLayoutHints::fixed_width(if compact {
            260.0
        } else {
            344.0
        })),
    );

    let mut main_children = Vec::with_capacity(4);
    if !compact {
        main_children.push(rail);
    }
    nodes.push(
        UiNode::container(
            center,
            "UI authoring center column",
            UiLayout::VerticalStack { gap: 8.0 },
            vec![surface, responsive_controls],
        )
        .with_style(UiStyle::transparent())
        .with_layout_hints(UiLayoutHints::flexible()),
    );
    main_children.extend([browser, center, inspector]);
    nodes.push(
        UiNode::container(
            main,
            "Meridian UI authoring workspace",
            UiLayout::HorizontalStack { gap: 8.0 },
            main_children,
        )
        .with_style(creator_workbench_canvas_style()),
    );

    let activity = UiNodeId::new(base + 60);
    nodes.push(
        UiNode::label(
            activity,
            "UI authored-source summary",
            format!(
                "UiDocument · {} nodes · {} primitives",
                inspection.source_nodes, inspection.display_primitives
            ),
        )
        .with_style(creator_meta_style()),
    );
    nodes.push(fixed_height(
        UiNode::container(
            shelf,
            "UI authoring activity shelf",
            UiLayout::HorizontalStack { gap: 8.0 },
            vec![activity],
        )
        .with_style(shell_row_style(UiColor::surface(), 4.0)),
        32.0,
    ));
    let status_row = push_status_row(
        &mut nodes,
        "UiDocument source is authoritative",
        bounded_text(&view.activity, 100),
        session.play_active(),
    );
    nodes.push(
        UiNode::container(
            root,
            "Meridian UI authoring workspace",
            UiLayout::VerticalStack { gap: 0.0 },
            vec![application_row, workspace_row, main, shelf, status_row],
        )
        .with_style(workspace_canvas_style()),
    );
    creator_authored_document(root, nodes)
}

fn workspace_node_base(workspace: WorkspaceKind) -> u128 {
    match workspace {
        WorkspaceKind::Code => 96_000,
        WorkspaceKind::Modeler => 96_100,
        WorkspaceKind::UiAuthoring => 96_200,
        WorkspaceKind::Materials => 96_300,
        WorkspaceKind::Alluvium => 96_400,
        WorkspaceKind::Build => 96_500,
        WorkspaceKind::Profile => 96_600,
        WorkspaceKind::Settings => 96_700,
        WorkspaceKind::Recovery => 96_800,
        WorkspaceKind::Hub | WorkspaceKind::World => 96_900,
    }
}

struct DomainWorkspaceContent<'a> {
    title: &'static str,
    eyebrow: &'static str,
    browser_title: &'static str,
    surface_title: &'static str,
    surface_detail: &'static str,
    source_excerpt: &'a str,
    inspector_title: &'static str,
    inspector_detail: &'static str,
    panel: EditorPanelId,
    commands: &'static [&'static str],
    capability_boundary: Option<&'static str>,
}

/// The sparse domains deliberately use one centred state composition instead
/// of a vague paragraph pinned into a large blank work surface. The content is
/// still constrained to only the authority that exists today: an unavailable
/// domain does not gain imitation controls merely to occupy space.
fn centered_domain_state(
    workspace: WorkspaceKind,
    _source_excerpt: &str,
) -> Option<(&'static str, String, &'static str, UiColor)> {
    match workspace {
        WorkspaceKind::Build => Some((
            "BUILD SERVICE READY",
            "Local build ready.".to_owned(),
            "Start a build or inspect the durable service.",
            UiColor::grass(),
        )),
        WorkspaceKind::Materials => Some((
            "NO MATERIAL AUTHORITY",
            "Material authoring is unavailable.".to_owned(),
            "No material source contract is active. Graph editing and live preview remain unavailable.",
            UiColor::amber(),
        )),
        WorkspaceKind::Profile => Some((
            "NO CAPTURED PROFILE",
            "No profile artifact.".to_owned(),
            "Capture data is required before profiling views appear.",
            UiColor::amber(),
        )),
        WorkspaceKind::Recovery => Some((
            "NO RECOVERY SNAPSHOT",
            "No recovery snapshot.".to_owned(),
            "Source remains authoritative until matching recovery context exists.",
            UiColor::grass(),
        )),
        _ => None,
    }
}

/// The persistent status line already carries host activity. Each workspace
/// shelf therefore reports a compact, source-backed fact about the work area
/// rather than repeating the same sentence twice at the bottom of the app.
fn domain_shelf_summary(workspace: WorkspaceKind) -> &'static str {
    match workspace {
        WorkspaceKind::Materials => "No material authority · domain unavailable",
        WorkspaceKind::Alluvium => "Recipe source · generated result is derived",
        WorkspaceKind::Build => "Bounded local build service",
        WorkspaceKind::Profile => "No profile artifact · capture required",
        WorkspaceKind::Recovery => "Source authoritative · recovery on demand",
        WorkspaceKind::Settings => "Local preferences · versioned and recoverable",
        WorkspaceKind::Code => "Read-only source preview",
        WorkspaceKind::Modeler
        | WorkspaceKind::UiAuthoring
        | WorkspaceKind::Hub
        | WorkspaceKind::World => "Source-backed Creator workspace",
    }
}

#[allow(clippy::too_many_lines)] // Each workspace exposes only its backed authority.
fn creator_domain_workspace_document(
    session: &EditorSession,
    view: &CreatorWorkspaceView,
) -> Result<UiDocument, UiDocumentError> {
    let workspace = view.workspace;
    debug_assert!(!matches!(
        workspace,
        WorkspaceKind::Hub | WorkspaceKind::World
    ));
    let base = workspace_node_base(workspace);
    let content = match workspace {
        WorkspaceKind::Code => DomainWorkspaceContent {
            title: "Code",
            eyebrow: if view.focus_layout {
                "FOCUSED IDE LAYOUT"
            } else {
                "CONTEXTUAL SOURCE LAYOUT"
            },
            browser_title: "Project files",
            surface_title: if view.focus_layout {
                "Canonical project source"
            } else {
                "Source beside World context"
            },
            surface_detail: if view.focus_layout {
                "The remembered IDE layout is active. Press Escape to return to contextual source."
            } else {
                "Source stays beside World context. Activate Code again for its remembered focused layout."
            },
            source_excerpt: &view.project_source,
            inspector_title: "Source authority",
            inspector_detail: "Project JSON is authoritative. This bounded source inspection surface does not claim a general-purpose code editor.",
            panel: EditorPanelId::Assets,
            commands: &["asset.inspect-source", "editor.show-diagnostic"],
            capability_boundary: Some(
                "Text editing and language services require their own completed source package.",
            ),
        },
        WorkspaceKind::Modeler => DomainWorkspaceContent {
            title: "Modeler",
            eyebrow: "EDITABLE MODEL SOURCE",
            browser_title: "Model structure",
            surface_title: "Derived Penumbra preview",
            surface_detail: "The preview is derived from immutable revisions; every source mutation uses a typed semantic operation.",
            source_excerpt: &view.model,
            inspector_title: "Topology and history",
            inspector_detail: "Primitive creation, transforms, one bounded edge split, semantic undo, and recovery are live. UVs, modifiers, collision, LOD, and interchange remain unavailable.",
            panel: EditorPanelId::Modeler,
            commands: &[
                "model.inspect-source",
                "model.create-primitive",
                "model.transform",
                "model.split-edge",
                "model.undo",
                "model.redo",
                "model.recover",
            ],
            capability_boundary: Some(
                "MS-03 modeler foundation only; unsupported topology tools are intentionally absent.",
            ),
        },
        WorkspaceKind::UiAuthoring => DomainWorkspaceContent {
            title: "UI",
            eyebrow: "AUTHORING SURFACE",
            browser_title: "Components",
            surface_title: "UiDocument",
            surface_detail: "Versioned UI source resolves stable nodes, named styles, reusable components, semantics, and packaged assets into a derived preview.",
            source_excerpt: "schema: meridian.ui-document/v1\nroot: MeridianCreatorShell\nstyles: shell-row, workspace-tab, dense-panel, source-field\ncomponents: ApplicationRow, WorkspaceRow, PanelHeader, StatusLine\nassets: Meridian package references only\npreview: derived display list and semantic tree",
            inspector_title: "Tokens & states",
            inspector_detail: "4 px spacing · 4/6/10/14 radii · one-pixel borders. High Contrast is opaque; Reduced Motion is immediate.",
            panel: EditorPanelId::Diagnostics,
            commands: &["asset.inspect-source", "editor.show-diagnostic"],
            capability_boundary: Some(
                "Inspect source and compiled preview now. Direct canvas, responsive, state, and animation editing are later bounded work.",
            ),
        },
        WorkspaceKind::Materials => DomainWorkspaceContent {
            title: "Materials",
            eyebrow: "MATERIAL AUTHORING",
            browser_title: "Material assets",
            surface_title: "Material source",
            surface_detail: "A material source contract is required before graph or preview tools can be offered.",
            source_excerpt: "No material source has been registered by a material authority.",
            inspector_title: "Material parameters",
            inspector_detail: "No graph compiler or material-source authority is active, so this workspace does not imitate editable material behavior.",
            panel: EditorPanelId::Diagnostics,
            commands: &[],
            capability_boundary: Some(
                "Material graphs, source editing, and live previews require the later material domain package.",
            ),
        },
        WorkspaceKind::Alluvium => DomainWorkspaceContent {
            title: "Alluvium",
            eyebrow: "PROCEDURAL RECIPE",
            browser_title: "Recipe assets",
            surface_title: "Canonical recipe source",
            surface_detail: "Structured recipe operations use the same typed Alluvium authority as headless commands.",
            source_excerpt: &view.recipe_source,
            inspector_title: "Generated result",
            inspector_detail: "Preview, bake, dirty explanation, provenance, and license audit are real bounded recipe operations. Generated output remains derived cache data.",
            panel: EditorPanelId::Recipe,
            commands: &[
                "procedural.inspect",
                "procedural.validate",
                "procedural.migrate",
                "procedural.preview",
                "procedural.bake",
                "procedural.dirty",
                "procedural.explain",
                "procedural.provenance",
                "procedural.license-audit",
            ],
            capability_boundary: None,
        },
        WorkspaceKind::Build => DomainWorkspaceContent {
            title: "Build",
            eyebrow: "DURABLE LOCAL BUILD",
            browser_title: "Build tasks",
            surface_title: "Build activity and artifacts",
            surface_detail: "Build work is asynchronous and remains owned by the durable one-worker Cargo service.",
            source_excerpt: &view.build,
            inspector_title: "Recovery and artifacts",
            inspector_detail: "Build artifacts and worker-loss recovery are reported from the durable service; the UI cannot execute arbitrary commands.",
            panel: EditorPanelId::Build,
            commands: &["build.submit", "build.inspect"],
            capability_boundary: None,
        },
        WorkspaceKind::Profile => DomainWorkspaceContent {
            title: "Profile",
            eyebrow: "OBSERVABILITY",
            browser_title: "Captured profiles",
            surface_title: "Timelines and traces",
            surface_detail: "Profile data authority is unavailable; traces, counters, comparison, and flame graphs remain disabled.",
            source_excerpt: "No profile artifact is available for this project.",
            inspector_title: "Evidence boundary",
            inspector_detail: "Meridian will not fabricate traces, performance measurements, or renderer qualification data.",
            panel: EditorPanelId::Diagnostics,
            commands: &[],
            capability_boundary: Some(
                "Profiling visualizations stay unavailable until backed by measured, attributable data.",
            ),
        },
        WorkspaceKind::Settings => DomainWorkspaceContent {
            title: "Settings",
            eyebrow: "APPLICATION PREFERENCES",
            browser_title: "Meridian preferences",
            surface_title: "Appearance and accessibility",
            surface_detail: "Theme roles, density, contrast, and reduced motion are Meridian-owned contracts. Platform-owned menus and pickers stay native.",
            source_excerpt: "No mutable preference source is registered by this bounded Creator package.",
            inspector_title: "Availability",
            inspector_detail: "Settings navigation is real; persistence beyond the registered workspace state is not claimed.",
            panel: EditorPanelId::ProjectRecovery,
            commands: &[],
            capability_boundary: Some(
                "Additional preferences require their own source and migration contract.",
            ),
        },
        WorkspaceKind::Recovery => DomainWorkspaceContent {
            title: "Recovery",
            eyebrow: "SOURCE AND SESSION RECOVERY",
            browser_title: "Recovery records",
            surface_title: "Authoritative recovery",
            surface_detail: "Recovery reloads validated source and source-matching local context; it never treats a cache as authoritative.",
            source_excerpt: &view.recovery,
            inspector_title: "Current session",
            inspector_detail: "Use recovery only when it preserves the canonical project document and generation-checked selection boundary.",
            panel: EditorPanelId::ProjectRecovery,
            commands: &["editor.recover", "editor.show-diagnostic"],
            capability_boundary: None,
        },
        WorkspaceKind::Hub | WorkspaceKind::World => unreachable!("domain workspace only"),
    };
    let DomainWorkspaceContent {
        title,
        eyebrow,
        browser_title,
        surface_title,
        surface_detail,
        source_excerpt,
        inspector_title,
        inspector_detail,
        panel,
        commands,
        capability_boundary,
    } = content;
    // Materials, Profile, Build, and Recovery can be genuinely sparse until
    // their owning domains provide source authority. Keep their information
    // truthful, but still give the workbench a continuous, full-height
    // browser/surface/inspector rhythm: three detached little panels on an
    // otherwise black window read as an abandoned prototype rather than an
    // intentionally unavailable workflow.
    let has_centred_domain_state = centered_domain_state(workspace, source_excerpt).is_some();
    let root = UiNodeId::new(base + 1);
    let main = UiNodeId::new(base + 2);
    let rail = UiNodeId::new(base + 3);
    let browser = UiNodeId::new(base + 4);
    let surface = UiNodeId::new(base + 5);
    let inspector = UiNodeId::new(base + 6);
    let shelf = UiNodeId::new(base + 7);
    let mut nodes = Vec::new();
    let application_row = push_application_row(
        &mut nodes,
        &bounded_text(&view.project, 34),
        true,
        session.play_active(),
    );
    let workspace_row = push_workspace_row(&mut nodes, Some(workspace));

    let rail_items = [
        (
            base + 10,
            IconId::Search,
            "Search this workspace",
            "shell.search",
        ),
        (
            base + 11,
            IconId::More,
            "Workspace favorites",
            "shell.favorites",
        ),
        (
            base + 12,
            IconId::Settings,
            "Workspace panels",
            "shell.panels",
        ),
    ];
    let mut rail_children = Vec::new();
    for (id, icon, name, action) in rail_items {
        let id = UiNodeId::new(id);
        nodes.push(fixed_height(
            UiNode::icon_button(id, name, action, icon).with_style(workspace_tab_style(false)),
            34.0,
        ));
        rail_children.push(id);
    }
    let rail_node = UiNode::container(
        rail,
        format!("{title} activity rail"),
        UiLayout::VerticalStack { gap: 8.0 },
        rail_children,
    )
    .with_style(creator_activity_rail_style());
    nodes.push(rail_node.with_layout_hints(UiLayoutHints::fixed_width(44.0)));

    let browser_title_id = UiNodeId::new(base + 20);
    let browser_mode_id = UiNodeId::new(base + 21);
    let browser_tree = UiNodeId::new(base + 22);
    let project_source = UiNodeId::new(base + 23);
    let selected_source = UiNodeId::new(base + 24);
    nodes.push(fixed_height(
        UiNode::label(
            browser_title_id,
            format!("{title} browser title"),
            browser_title,
        )
        .with_style(side_panel_title_style()),
        22.0,
    ));
    nodes.push(fixed_height(
        UiNode::label(browser_mode_id, format!("{title} browser mode"), eyebrow)
            .with_style(creator_hub_field_label_style()),
        16.0,
    ));
    nodes.push(fixed_height(
        UiNode::search_input(CREATOR_DOMAIN_SEARCH, format!("Search {title}"), "")
            .with_placeholder(format!("Search {title}"))
            .with_style(UiStyle::text_field()),
        36.0,
    ));
    let browser_source_label = match workspace {
        WorkspaceKind::Alluvium => "Canonical recipe source",
        WorkspaceKind::Build => "Bounded local build request",
        WorkspaceKind::Materials => "No material source",
        WorkspaceKind::Profile => "No profile artifact",
        _ => "Canonical project source",
    };
    let browser_source_action = match workspace {
        WorkspaceKind::Alluvium => "procedural.inspect",
        WorkspaceKind::Build => "build.inspect",
        WorkspaceKind::Materials | WorkspaceKind::Profile => "editor.show-diagnostic",
        _ => "asset.inspect-source",
    };
    let browser_context_label = match workspace {
        WorkspaceKind::Alluvium => bounded_text(&view.recipe, 96),
        WorkspaceKind::Build => bounded_text(&view.build, 96),
        WorkspaceKind::Materials => "Live preview unavailable".to_owned(),
        WorkspaceKind::Profile => "Trace data unavailable".to_owned(),
        _ => selected_placement_summary(session).unwrap_or_else(|| "No selected source".to_owned()),
    };
    nodes.push(fixed_height(
        creator_tree_group_item(
            project_source,
            browser_source_label,
            browser_source_action,
            false,
            true,
        ),
        32.0,
    ));
    nodes.push(fixed_height(
        creator_tree_child_item(
            selected_source,
            browser_context_label,
            browser_source_action,
            !session.selection().ids.is_empty(),
        ),
        32.0,
    ));
    nodes.push(
        UiNode::tree(
            browser_tree,
            format!("{title} source browser"),
            vec![project_source, selected_source],
        )
        .with_style(UiStyle::transparent()),
    );
    let browser_node = UiNode::container(
        browser,
        format!("{title} browser"),
        UiLayout::VerticalStack { gap: 8.0 },
        vec![
            browser_title_id,
            browser_mode_id,
            CREATOR_DOMAIN_SEARCH,
            browser_tree,
        ],
    )
    .with_style(world_panel_style(10.0));
    nodes.push(browser_node.with_layout_hints(UiLayoutHints::fixed_width(264.0)));

    let surface_title_id = UiNodeId::new(base + 30);
    let surface_detail_id = UiNodeId::new(base + 31);
    let source_excerpt_id = UiNodeId::new(base + 32);
    let mut surface_children = Vec::new();
    nodes.push(fixed_height(
        UiNode::label(
            surface_title_id,
            format!("{title} work surface title"),
            surface_title,
        )
        .with_style(creator_title_style()),
        28.0,
    ));
    surface_children.push(surface_title_id);
    nodes.push(fixed_height(
        UiNode::label(
            surface_detail_id,
            format!("{title} work surface detail"),
            bounded_text(surface_detail, 180),
        )
        .with_style(creator_meta_style()),
        60.0,
    ));
    surface_children.push(surface_detail_id);
    if let Some((state_eyebrow, state_title, state_detail, state_accent)) =
        centered_domain_state(workspace, source_excerpt)
    {
        let state_body = UiNodeId::new(base + 90);
        let state_card = UiNodeId::new(base + 91);
        let state_eyebrow_id = UiNodeId::new(base + 92);
        let state_detail_id = UiNodeId::new(base + 93);
        let mut state_children = Vec::new();
        nodes.push(fixed_height(
            UiNode::label(
                state_eyebrow_id,
                format!("{title} centred state eyebrow"),
                state_eyebrow,
            )
            .with_style(domain_state_eyebrow_style(state_accent)),
            18.0,
        ));
        state_children.push(state_eyebrow_id);
        nodes.push(fixed_height(
            UiNode::label(
                source_excerpt_id,
                format!("{title} centred state title"),
                state_title,
            )
            .with_style(creator_title_style()),
            54.0,
        ));
        state_children.push(source_excerpt_id);
        nodes.push(fixed_height(
            UiNode::label(
                state_detail_id,
                format!("{title} centred state detail"),
                state_detail,
            )
            .with_style(creator_meta_style()),
            56.0,
        ));
        state_children.push(state_detail_id);
        if !commands.is_empty() {
            let actions = push_creator_action_grid(
                &mut nodes,
                UiNodeId::new(base + 94),
                base + 95,
                title,
                panel,
                session,
                commands,
                2,
                40.0,
            );
            state_children.push(actions);
        }
        nodes.push(
            UiNode::container(
                state_card,
                format!("{title} centred source state"),
                UiLayout::VerticalStack { gap: 8.0 },
                state_children,
            )
            .with_style(domain_state_style())
            .with_layout_hints(UiLayoutHints::fixed_height(232.0))
            .with_constraints(UiConstraints {
                maximum: Some(UiSize::new(560.0, 232.0)),
                horizontal_alignment: UiAlignment::Center,
                vertical_alignment: UiAlignment::Center,
                ..UiConstraints::default()
            }),
        );
        nodes.push(
            UiNode::container(
                state_body,
                format!("{title} centred source state region"),
                UiLayout::Overlay,
                vec![state_card],
            )
            .with_style(UiStyle::transparent()),
        );
        surface_children.push(state_body);
    } else if workspace == WorkspaceKind::Alluvium {
        push_code_source_listing(
            &mut nodes,
            source_excerpt_id,
            base + 100,
            source_excerpt,
            // The source panel shares room with the browser and generated
            // result. Keep each visible row inside its real listing bounds;
            // authoritative source remains available through Inspect recipe.
            30,
            15,
        );
    } else {
        let source_excerpt_node = UiNode::label(
            source_excerpt_id,
            format!("{title} authoritative source excerpt"),
            if workspace == WorkspaceKind::UiAuthoring {
                code_pane_display_text(&bounded_text(source_excerpt, 520), 72)
            } else {
                bounded_text(source_excerpt, 520)
            },
        );
        nodes.push(if workspace == WorkspaceKind::UiAuthoring {
            source_excerpt_node
                .with_style(creator_code_source_style())
                .with_font_role(UiFontRole::Monospace)
        } else {
            source_excerpt_node.with_style(creator_hub_status_style())
        });
    }
    if !has_centred_domain_state {
        surface_children.push(source_excerpt_id);
        if !commands.is_empty() {
            let action_height = match commands.len() {
                0..=3 => 40.0,
                4..=6 => 80.0,
                _ => 120.0,
            };
            let actions = push_creator_action_grid(
                &mut nodes,
                UiNodeId::new(base + 33),
                base + 34,
                title,
                panel,
                session,
                commands,
                3,
                action_height,
            );
            surface_children.push(actions);
        }
    }
    let surface_style = if has_centred_domain_state {
        domain_stage_style()
    } else {
        world_panel_style(10.0)
    };
    nodes.push(
        UiNode::container(
            surface,
            format!("{title} work surface"),
            UiLayout::VerticalStack { gap: 8.0 },
            surface_children,
        )
        .with_style(surface_style),
    );

    let inspector_title_id = UiNodeId::new(base + 50);
    let inspector_detail_id = UiNodeId::new(base + 51);
    let mut inspector_children = Vec::new();
    nodes.push(fixed_height(
        UiNode::label(
            inspector_title_id,
            format!("{title} inspector title"),
            inspector_title,
        )
        .with_style(side_panel_title_style()),
        22.0,
    ));
    inspector_children.push(inspector_title_id);
    nodes.push(
        UiNode::label(
            inspector_detail_id,
            format!("{title} inspector detail"),
            bounded_text(inspector_detail, 240),
        )
        .with_style(creator_meta_style()),
    );
    inspector_children.push(inspector_detail_id);
    if let Some(boundary) = capability_boundary {
        let boundary_id = UiNodeId::new(base + 52);
        nodes.push(fixed_height(
            UiNode::label(
                boundary_id,
                format!("{title} capability boundary"),
                bounded_text(boundary, 240),
            )
            .with_style(if has_centred_domain_state {
                domain_capability_note_style()
            } else {
                world_section_style(UiColor::amber())
            }),
            if has_centred_domain_state {
                60.0
            } else {
                104.0
            },
        ));
        inspector_children.push(boundary_id);
    }
    let inspector_node = UiNode::container(
        inspector,
        format!("{title} inspector"),
        UiLayout::VerticalStack { gap: 8.0 },
        inspector_children,
    )
    .with_style(world_panel_style(10.0));
    nodes.push(inspector_node.with_layout_hints(UiLayoutHints::fixed_width(344.0)));

    nodes.push(
        UiNode::container(
            main,
            format!("{title} workspace"),
            UiLayout::HorizontalStack { gap: 8.0 },
            vec![rail, browser, surface, inspector],
        )
        .with_style(creator_workbench_canvas_style()),
    );

    let activity = UiNodeId::new(base + 63);
    nodes.push(fixed_width(
        UiNode::label(
            activity,
            format!("{title} workspace summary"),
            domain_shelf_summary(workspace),
        )
        .with_style(creator_meta_style()),
        460.0,
    ));
    nodes.push(fixed_height(
        UiNode::container(
            shelf,
            format!("{title} bottom shelf"),
            UiLayout::HorizontalStack { gap: 8.0 },
            vec![activity],
        )
        .with_style(shell_row_style(UiColor::surface(), 4.0)),
        // Unavailable domains report their real source-backed fact here.
        // Project commands remain in their owning World/recovery journeys;
        // this is intentionally a quiet 32 px peek, not fake tooling.
        32.0,
    ));
    let status_row = push_status_row(
        &mut nodes,
        if session.play_active() {
            "Source saved · Play fork isolated"
        } else {
            "Source saved"
        },
        bounded_text(&view.activity, 100),
        session.play_active(),
    );
    nodes.push(
        UiNode::container(
            root,
            format!("Meridian {title} workspace"),
            UiLayout::VerticalStack { gap: 0.0 },
            vec![application_row, workspace_row, main, shelf, status_row],
        )
        .with_style(workspace_canvas_style()),
    );
    creator_authored_document(root, nodes)
}

#[derive(Clone, Copy)]
struct WorldViewportGeometry {
    canvas: UiRect,
    left: f32,
    right: f32,
    top: f32,
    horizon: f32,
    bottom: f32,
}

impl WorldViewportGeometry {
    fn from_canvas(canvas: UiRect) -> Option<Self> {
        let left = canvas.origin.x + 18.0;
        let right = canvas.origin.x + canvas.size.width - 18.0;
        // Keep the source-derived placement in a grounded perspective grid
        // without leaving most of a large Creator viewport visually vacant.
        let top = canvas.origin.y + 36.0;
        let bottom = canvas.origin.y + canvas.size.height - 18.0;
        (right > left && bottom > top).then_some(Self {
            canvas,
            left,
            right,
            top,
            horizon: top + (bottom - top) * 0.20,
            bottom,
        })
    }
}

fn push_world_path(
    display: &mut DisplayList,
    commands: Vec<UiPathCommand>,
    color: UiColor,
) -> Result<(), DisplayListError> {
    display.try_push(DisplayPrimitive::Path {
        node: CREATOR_WORLD_VIEWPORT_CANVAS,
        commands,
        fill: None,
        stroke: Some(UiStroke::new(color, 1.0)),
    })
}

fn decorate_world_grid(
    display: &mut DisplayList,
    geometry: WorldViewportGeometry,
) -> Result<(), DisplayListError> {
    decorate_world_reference_grid(display, geometry)
}

fn decorate_world_reference_grid(
    display: &mut DisplayList,
    geometry: WorldViewportGeometry,
) -> Result<(), DisplayListError> {
    let grid = UiColor::rgba(0.161, 0.176, 0.173, 0.19);
    let vanishing = world_reference_vanishing(geometry);
    let horizon = geometry.horizon.max(geometry.top);
    // A small, low-contrast reference grid is sufficient to establish scale
    // for the authoritative placement. The canvas remains a flat working
    // field: simulated sky/floor planes made this source-neutral viewport
    // read like a renderer diagnostic rather than an editor instrument.
    for column in 0_u8..=4 {
        let progress = f32::from(column) / 4.0;
        push_world_path(
            display,
            vec![
                UiPathCommand::MoveTo(UiPoint {
                    x: geometry.left + (geometry.right - geometry.left) * progress,
                    y: geometry.bottom,
                }),
                UiPathCommand::LineTo(vanishing),
            ],
            grid,
        )?;
    }
    for row in 1_u8..=4 {
        let progress = f32::from(row) / 4.0;
        let y = horizon + (geometry.bottom - horizon) * progress * progress;
        push_world_path(
            display,
            vec![
                UiPathCommand::MoveTo(UiPoint {
                    x: geometry.left,
                    y,
                }),
                UiPathCommand::LineTo(UiPoint {
                    x: geometry.right,
                    y,
                }),
            ],
            grid,
        )?;
    }
    push_world_path(
        display,
        vec![
            UiPathCommand::MoveTo(UiPoint {
                x: geometry.left,
                y: horizon,
            }),
            UiPathCommand::LineTo(UiPoint {
                x: geometry.right,
                y: horizon,
            }),
        ],
        UiColor::rgba(0.161, 0.176, 0.173, 0.36),
    )?;
    Ok(())
}

fn world_reference_vanishing(geometry: WorldViewportGeometry) -> UiPoint {
    UiPoint {
        x: (geometry.left + geometry.right) * 0.5,
        y: geometry.horizon,
    }
}

#[allow(clippy::cast_precision_loss)] // Source millimetres clamp to a small display-only range.
fn world_placement_center(geometry: WorldViewportGeometry, placement: &WorldPlacement) -> UiPoint {
    let source_x = placement.translation.x_mm.clamp(-5_000, 5_000) as f32 / 5_000.0;
    let source_z = placement.translation.z_mm.clamp(-5_000, 5_000) as f32 / 5_000.0;
    UiPoint {
        x: (geometry.left + geometry.right) * 0.5
            + (geometry.right - geometry.left) * source_x * 0.18,
        y: geometry.horizon + (geometry.bottom - geometry.horizon) * (0.44 - source_z * 0.16),
    }
}

/// Returns the display-only scale for one authoritative World placement.
///
/// A World canvas can legitimately become much larger on a desktop display.
/// Keep the selected source subject legible without letting the public fixture
/// become a giant debug symbol. Source transforms remain editable in the
/// Inspector, so the viewport needs a calm selection anchor rather than a
/// full transform gizmo.
fn world_placement_visual_size(geometry: WorldViewportGeometry) -> f32 {
    geometry
        .canvas
        .size
        .width
        .min(geometry.canvas.size.height)
        .mul_add(0.12, 0.0)
        .clamp(48.0, 104.0)
}

/// Returns the displayed selection bounds for one World placement.
///
/// The selection follows the derived triangle rather than using a generic
/// square around its pivot, preventing a conspicuous empty lower half while
/// still leaving a clear perimeter for keyboard and pointer selection.
fn world_placement_selection_bounds(center: UiPoint, size: f32) -> UiRect {
    UiRect::new(
        UiPoint {
            x: center.x - size * 0.86 - 12.0,
            y: center.y - size - 12.0,
        },
        UiSize::new(size * 1.72 + 24.0, size * 1.52 + 24.0),
    )
}

fn decorate_world_placement(
    display: &mut DisplayList,
    geometry: WorldViewportGeometry,
    placement: &WorldPlacement,
) -> Result<(), DisplayListError> {
    let center = world_placement_center(geometry, placement);
    // The public fixture has one authoritative placement. Its subtle fill and
    // amber selection brackets establish source context without impersonating
    // unavailable scene content or a viewport-transform gizmo. Brackets make
    // the object read as selected without enclosing it in the prototype-like
    // empty rectangle a full frame would create.
    let size = world_placement_visual_size(geometry);
    let triangle = [
        UiPoint {
            x: center.x,
            y: center.y - size,
        },
        UiPoint {
            x: center.x - size * 0.86,
            y: center.y + size * 0.52,
        },
        UiPoint {
            x: center.x + size * 0.86,
            y: center.y + size * 0.52,
        },
    ];
    display.try_push(DisplayPrimitive::Path {
        node: CREATOR_WORLD_VIEWPORT_CANVAS,
        commands: vec![
            UiPathCommand::MoveTo(triangle[0]),
            UiPathCommand::LineTo(triangle[1]),
            UiPathCommand::LineTo(triangle[2]),
            UiPathCommand::Close,
        ],
        fill: Some(UiColor::rgba(0.553, 0.537, 0.38, 0.26)),
        stroke: Some(UiStroke::new(UiColor::amber(), 1.0)),
    })?;
    let selection_bounds = world_placement_selection_bounds(center, size);
    let corner = (selection_bounds
        .size
        .width
        .min(selection_bounds.size.height)
        * 0.16)
        .clamp(12.0, 20.0);
    let left = selection_bounds.origin.x;
    let right = selection_bounds.origin.x + selection_bounds.size.width;
    let top = selection_bounds.origin.y;
    let bottom = selection_bounds.origin.y + selection_bounds.size.height;
    push_world_path(
        display,
        vec![
            UiPathCommand::MoveTo(UiPoint {
                x: left,
                y: top + corner,
            }),
            UiPathCommand::LineTo(UiPoint { x: left, y: top }),
            UiPathCommand::LineTo(UiPoint {
                x: left + corner,
                y: top,
            }),
            UiPathCommand::MoveTo(UiPoint {
                x: right - corner,
                y: top,
            }),
            UiPathCommand::LineTo(UiPoint { x: right, y: top }),
            UiPathCommand::LineTo(UiPoint {
                x: right,
                y: top + corner,
            }),
            UiPathCommand::MoveTo(UiPoint {
                x: right,
                y: bottom - corner,
            }),
            UiPathCommand::LineTo(UiPoint {
                x: right,
                y: bottom,
            }),
            UiPathCommand::LineTo(UiPoint {
                x: right - corner,
                y: bottom,
            }),
            UiPathCommand::MoveTo(UiPoint {
                x: left + corner,
                y: bottom,
            }),
            UiPathCommand::LineTo(UiPoint { x: left, y: bottom }),
            UiPathCommand::LineTo(UiPoint {
                x: left,
                y: bottom - corner,
            }),
        ],
        UiColor::amber(),
    )?;
    Ok(())
}

/// Adds a bounded source-derived World presentation to the retained canvas.
///
/// The immutable editor source remains authoritative. This decoration uses
/// only renderer-neutral paths and the accepted canvas bounds; it carries no
/// renderer handles and cannot mutate the project.
///
/// # Errors
///
/// Returns a typed display-list error without changing the accepted frame.
pub fn decorate_world_viewport(
    session: &EditorSession,
    frame: &UiFrameOutput,
) -> Result<UiFrameOutput, DisplayListError> {
    let geometry = frame
        .layout
        .iter()
        .find(|entry| entry.node == CREATOR_WORLD_VIEWPORT_CANVAS)
        .and_then(|entry| WorldViewportGeometry::from_canvas(entry.bounds));
    let Some(geometry) = geometry else {
        return Ok(Arc::clone(frame));
    };
    let mut decorated = (**frame).clone();
    decorate_world_grid(&mut decorated.display_list, geometry)?;
    if let Some(placement) = session
        .selection()
        .ids
        .iter()
        .find_map(|id| session.document().placements.get(id))
        .or_else(|| session.document().placements.values().next())
    {
        decorate_world_placement(&mut decorated.display_list, geometry, placement)?;
    }
    decorated.display_list.validate()?;
    Ok(Arc::new(decorated))
}

fn push_ui_authoring_preview_surface(
    display: &mut DisplayList,
    bounds: UiRect,
    color: UiColor,
    radius: f32,
) -> Result<(), DisplayListError> {
    display.try_push(DisplayPrimitive::RoundedRect {
        node: CREATOR_UI_AUTHORING_PREVIEW_CANVAS,
        bounds,
        radii: UiCornerRadii::uniform(radius),
        color,
    })
}

/// Adds a compact, renderer-neutral thumbnail of the compiled component frame.
///
/// The thumbnail deliberately uses only the locked Meridian palette and
/// registered radii. It is a derived inspection aid for the authored
/// `UiDocument`, not an interactive canvas or a substitute source format.
///
/// # Errors
///
/// Returns a typed display-list error without changing the accepted frame.
#[allow(clippy::too_many_lines)] // The thumbnail geometry is intentionally kept auditable.
/// Projects the selected compiled source frame into the bounded inspection
/// canvas. Process-local resources are intentionally omitted at this scale;
/// retained surfaces, borders, native paths, and scaled source text keep the
/// real frame's composition legible without inventing a second preview format.
fn project_ui_authoring_preview(
    display: &mut DisplayList,
    source: &UiFrameOutput,
    destination: UiRect,
    destination_scale_factor: f32,
) -> Result<usize, DisplayListError> {
    let source_bounds = UiRect::new(UiPoint::default(), UiSize::new(1280.0, 800.0));
    let preview_scale = destination.size.width / source_bounds.size.width;
    // Text raster payloads are physical pixels, while the projected preview
    // geometry is logical space. Convert through both frame scale factors so
    // a thumbnail compiled into a 2x destination remains the same logical
    // size and does not become a visibly undersized blur.
    let source_scale_factor = source.scale_factor.max(f32::EPSILON);
    let destination_scale_factor = destination_scale_factor.max(f32::EPSILON);
    let raster_scale = preview_scale * destination_scale_factor / source_scale_factor;
    let map_point = |point: UiPoint| UiPoint {
        x: destination.origin.x
            + (point.x - source_bounds.origin.x) / source_bounds.size.width
                * destination.size.width,
        y: destination.origin.y
            + (point.y - source_bounds.origin.y) / source_bounds.size.height
                * destination.size.height,
    };
    let map_rect = |bounds: UiRect| UiRect {
        origin: map_point(bounds.origin),
        size: UiSize::new(
            bounds.size.width / source_bounds.size.width * destination.size.width,
            bounds.size.height / source_bounds.size.height * destination.size.height,
        ),
    };
    let map_path = |commands: &[UiPathCommand]| {
        commands
            .iter()
            .copied()
            .map(|command| match command {
                UiPathCommand::MoveTo(point) => UiPathCommand::MoveTo(map_point(point)),
                UiPathCommand::LineTo(point) => UiPathCommand::LineTo(map_point(point)),
                UiPathCommand::QuadraticTo { control, end } => UiPathCommand::QuadraticTo {
                    control: map_point(control),
                    end: map_point(end),
                },
                UiPathCommand::CubicTo {
                    control_a,
                    control_b,
                    end,
                } => UiPathCommand::CubicTo {
                    control_a: map_point(control_a),
                    control_b: map_point(control_b),
                    end: map_point(end),
                },
                UiPathCommand::Close => UiPathCommand::Close,
            })
            .collect::<Vec<_>>()
    };
    let mut projected = 0_usize;
    for primitive in &source.display_list.primitives {
        let primitive = match primitive {
            DisplayPrimitive::Rect { bounds, color, .. } => Some(DisplayPrimitive::Rect {
                node: CREATOR_UI_AUTHORING_PREVIEW_CANVAS,
                bounds: map_rect(*bounds),
                color: *color,
            }),
            DisplayPrimitive::Border {
                bounds,
                color,
                width,
                ..
            } => Some(DisplayPrimitive::Border {
                node: CREATOR_UI_AUTHORING_PREVIEW_CANVAS,
                bounds: map_rect(*bounds),
                color: *color,
                width: *width,
            }),
            DisplayPrimitive::RoundedRect {
                bounds,
                radii,
                color,
                ..
            } => Some(DisplayPrimitive::RoundedRect {
                node: CREATOR_UI_AUTHORING_PREVIEW_CANVAS,
                bounds: map_rect(*bounds),
                radii: UiCornerRadii::uniform(
                    radii
                        .top_left
                        .min(radii.top_right)
                        .min(radii.bottom_right)
                        .min(radii.bottom_left)
                        / source_bounds.size.width
                        * destination.size.width,
                ),
                color: *color,
            }),
            DisplayPrimitive::Path {
                commands,
                fill,
                stroke,
                ..
            } => Some(DisplayPrimitive::Path {
                node: CREATOR_UI_AUTHORING_PREVIEW_CANVAS,
                commands: map_path(commands),
                fill: *fill,
                stroke: *stroke,
            }),
            DisplayPrimitive::Text {
                bounds,
                text,
                color,
                layout,
                raster,
                ..
            } => Some(DisplayPrimitive::Text {
                node: CREATOR_UI_AUTHORING_PREVIEW_CANVAS,
                bounds: map_rect(*bounds),
                text: text.clone(),
                color: *color,
                layout: scale_ui_authoring_preview_text_layout(layout, preview_scale),
                raster: scale_ui_authoring_preview_text_raster(raster, raster_scale),
            }),
            DisplayPrimitive::GlyphRun {
                bounds,
                text,
                color,
                layout,
                raster,
                ..
            } => Some(DisplayPrimitive::GlyphRun {
                node: CREATOR_UI_AUTHORING_PREVIEW_CANVAS,
                bounds: map_rect(*bounds),
                text: text.clone(),
                color: *color,
                layout: scale_ui_authoring_preview_text_layout(layout, preview_scale),
                raster: scale_ui_authoring_preview_text_raster(raster, raster_scale),
            }),
            DisplayPrimitive::FocusIndicator { .. }
            | DisplayPrimitive::Image { .. }
            | DisplayPrimitive::Mesh { .. }
            | DisplayPrimitive::PushClip { .. }
            | DisplayPrimitive::PopClip { .. }
            | DisplayPrimitive::BeginLayer { .. }
            | DisplayPrimitive::EndLayer { .. }
            | DisplayPrimitive::Shadow { .. }
            | DisplayPrimitive::Backdrop { .. } => None,
        };
        if let Some(primitive) = primitive {
            display.try_push(primitive)?;
            projected = projected.saturating_add(1);
        }
    }
    Ok(projected)
}

fn scale_ui_authoring_preview_text_layout(layout: &UiTextLayout, scale: f32) -> UiTextLayout {
    let mut scaled = layout.clone();
    scaled.width *= scale;
    scaled.height *= scale;
    scaled
}

fn scale_ui_authoring_preview_text_raster(raster: &UiTextRaster, scale: f32) -> UiTextRaster {
    UiTextRaster {
        glyphs: raster
            .glyphs
            .iter()
            .map(|glyph| scale_ui_authoring_preview_glyph(glyph, scale))
            .collect(),
        has_unrasterized_glyphs: raster.has_unrasterized_glyphs,
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)] // UiTextRaster bounds every glyph bitmap before this display-only thumbnail scaler runs.
fn scale_ui_authoring_preview_glyph(glyph: &UiGlyphBitmap, scale: f32) -> UiGlyphBitmap {
    if glyph.width == 0 || glyph.height == 0 || glyph.alpha.is_empty() {
        return glyph.clone();
    }
    let width = ((glyph.width as f32 * scale).round().max(1.0)) as u32;
    let height = ((glyph.height as f32 * scale).round().max(1.0)) as u32;
    let mut alpha = Vec::new();
    for target_y in 0..height {
        let source_y = ((target_y as f32 + 0.5) * glyph.height as f32 / height as f32)
            .floor()
            .min((glyph.height - 1) as f32) as u32;
        for target_x in 0..width {
            let source_x = ((target_x as f32 + 0.5) * glyph.width as f32 / width as f32)
                .floor()
                .min((glyph.width - 1) as f32) as u32;
            let index = source_y as usize * glyph.width as usize + source_x as usize;
            alpha.push(glyph.alpha[index]);
        }
    }
    UiGlyphBitmap {
        origin: UiPoint {
            x: glyph.origin.x * scale,
            y: glyph.origin.y * scale,
        },
        width,
        height,
        alpha,
    }
}

/// Draws one actual compiled Creator source frame into the read-only
/// UI-authoring canvas. The target frame remains immutable; this only copies
/// bounded renderer-neutral geometry into the current inspection frame.
///
/// # Errors
///
/// Returns a display-list error without changing either accepted frame.
pub fn decorate_ui_authoring_preview(
    frame: &UiFrameOutput,
    target: &UiFrameOutput,
) -> Result<UiFrameOutput, DisplayListError> {
    let canvas = frame
        .layout
        .iter()
        .find(|entry| entry.node == CREATOR_UI_AUTHORING_PREVIEW_CANVAS)
        .map(|entry| entry.bounds);
    let Some(canvas) = canvas else {
        return Ok(Arc::clone(frame));
    };
    if canvas.size.width < 48.0 || canvas.size.height < 48.0 {
        return Ok(Arc::clone(frame));
    }
    let inset = 14.0;
    let available = UiSize::new(
        (canvas.size.width - inset * 2.0).max(0.0),
        (canvas.size.height - inset * 2.0).max(0.0),
    );
    // The inspected World frame is a 16:10 authored document. Preserve that
    // source geometry inside the authoring canvas instead of stretching it to
    // an arbitrary dock size; otherwise the preview lies about responsive
    // layout and reads as an accidental miniature rather than a real frame.
    let source_size = UiSize::new(1280.0, 800.0);
    let scale = (available.width / source_size.width)
        .min(available.height / source_size.height)
        .max(0.0);
    let preview_size = UiSize::new(source_size.width * scale, source_size.height * scale);
    let preview_frame = UiRect {
        origin: UiPoint {
            x: canvas.origin.x + (canvas.size.width - preview_size.width) * 0.5,
            y: canvas.origin.y + (canvas.size.height - preview_size.height) * 0.5,
        },
        size: preview_size,
    };
    let destination = UiRect {
        origin: UiPoint {
            x: preview_frame.origin.x + 1.0,
            y: preview_frame.origin.y + 1.0,
        },
        size: UiSize::new(
            (preview_frame.size.width - 2.0).max(0.0),
            (preview_frame.size.height - 2.0).max(0.0),
        ),
    };
    let mut decorated = (**frame).clone();
    push_ui_authoring_preview_surface(
        &mut decorated.display_list,
        preview_frame,
        UiColor::border(),
        10.0,
    )?;
    if project_ui_authoring_preview(
        &mut decorated.display_list,
        target,
        destination,
        frame.scale_factor,
    )? == 0
    {
        return decorate_ui_authoring_preview_fallback(frame);
    }
    decorated.display_list.validate()?;
    Ok(Arc::new(decorated))
}

#[allow(clippy::too_many_lines)] // Legacy empty-source fallback remains deliberately auditable.
fn decorate_ui_authoring_preview_fallback(
    frame: &UiFrameOutput,
) -> Result<UiFrameOutput, DisplayListError> {
    let canvas = frame
        .layout
        .iter()
        .find(|entry| entry.node == CREATOR_UI_AUTHORING_PREVIEW_CANVAS)
        .map(|entry| entry.bounds);
    let Some(canvas) = canvas else {
        return Ok(Arc::clone(frame));
    };
    if canvas.size.width < 48.0 || canvas.size.height < 48.0 {
        return Ok(Arc::clone(frame));
    }

    let inset = 14.0;
    let preview = UiRect {
        origin: UiPoint {
            x: canvas.origin.x + inset,
            y: canvas.origin.y + inset,
        },
        size: UiSize::new(
            (canvas.size.width - inset * 2.0).max(0.0),
            (canvas.size.height - inset * 2.0).max(0.0),
        ),
    };
    let mut decorated = (**frame).clone();
    push_ui_authoring_preview_surface(
        &mut decorated.display_list,
        preview,
        UiColor::rgba(0.035, 0.043, 0.043, 1.0),
        10.0,
    )?;

    let command_height = preview.size.height.clamp(20.0, 34.0);
    let command_row = UiRect {
        origin: preview.origin,
        size: UiSize::new(preview.size.width, command_height),
    };
    push_ui_authoring_preview_surface(
        &mut decorated.display_list,
        command_row,
        UiColor::surface(),
        10.0,
    )?;
    let selected_tab = UiRect {
        origin: UiPoint {
            x: command_row.origin.x + 12.0,
            y: command_row.origin.y + 8.0,
        },
        size: UiSize::new((command_row.size.width * 0.22).clamp(42.0, 88.0), 8.0),
    };
    push_ui_authoring_preview_surface(
        &mut decorated.display_list,
        selected_tab,
        UiColor::amber(),
        4.0,
    )?;

    let body_top = command_row.origin.y + command_row.size.height + 8.0;
    let body_height = (preview.origin.y + preview.size.height - body_top - 10.0).max(0.0);
    let body_width = preview.size.width - 20.0;
    let sidebar_width = (body_width * 0.23).clamp(42.0, 104.0);
    let sidebar = UiRect {
        origin: UiPoint {
            x: preview.origin.x + 10.0,
            y: body_top,
        },
        size: UiSize::new(sidebar_width, body_height),
    };
    push_ui_authoring_preview_surface(
        &mut decorated.display_list,
        sidebar,
        UiColor::rgba(0.071, 0.082, 0.082, 1.0),
        6.0,
    )?;
    let stage = UiRect {
        origin: UiPoint {
            x: sidebar.origin.x + sidebar.size.width + 8.0,
            y: body_top,
        },
        size: UiSize::new(
            (preview.origin.x + preview.size.width - sidebar.origin.x - sidebar.size.width - 18.0)
                .max(0.0),
            body_height,
        ),
    };
    push_ui_authoring_preview_surface(
        &mut decorated.display_list,
        stage,
        UiColor::rgba(0.055, 0.063, 0.063, 1.0),
        6.0,
    )?;
    for (index, fraction) in [0.18_f32, 0.43, 0.68].into_iter().enumerate() {
        let row = UiRect {
            origin: UiPoint {
                x: sidebar.origin.x + 10.0,
                y: sidebar.origin.y + sidebar.size.height * fraction,
            },
            size: UiSize::new((sidebar.size.width - 20.0).max(0.0), 5.0),
        };
        push_ui_authoring_preview_surface(
            &mut decorated.display_list,
            row,
            if index == 0 {
                UiColor::rgba(0.753, 0.588, 0.306, 0.72)
            } else {
                UiColor::rgba(0.451, 0.478, 0.463, 0.5)
            },
            4.0,
        )?;
    }
    let focal = UiRect {
        origin: UiPoint {
            x: stage.origin.x + stage.size.width * 0.18,
            y: stage.origin.y + stage.size.height * 0.25,
        },
        size: UiSize::new(stage.size.width * 0.58, stage.size.height * 0.42),
    };
    push_ui_authoring_preview_surface(
        &mut decorated.display_list,
        focal,
        UiColor::rgba(0.106, 0.122, 0.12, 1.0),
        10.0,
    )?;
    let focal_padding = 14.0_f32.min((focal.size.width * 0.12).max(4.0));
    let focal_header = UiRect {
        origin: UiPoint {
            x: focal.origin.x + focal_padding,
            y: focal.origin.y + focal_padding,
        },
        size: UiSize::new((focal.size.width * 0.42).max(0.0), 6.0),
    };
    push_ui_authoring_preview_surface(
        &mut decorated.display_list,
        focal_header,
        UiColor::rgba(0.84, 0.83, 0.77, 0.72),
        4.0,
    )?;
    let focal_action = UiRect {
        origin: UiPoint {
            x: focal.origin.x + focal.size.width - focal_padding - (focal.size.width * 0.18),
            y: focal.origin.y + focal_padding - 2.0,
        },
        size: UiSize::new((focal.size.width * 0.18).max(0.0), 10.0),
    };
    push_ui_authoring_preview_surface(
        &mut decorated.display_list,
        focal_action,
        UiColor::rgba(0.753, 0.588, 0.306, 0.82),
        4.0,
    )?;
    let content_top = focal_header.origin.y + 22.0;
    let content_height =
        (focal.origin.y + focal.size.height - content_top - focal_padding).max(0.0);
    let primary_content = UiRect {
        origin: UiPoint {
            x: focal.origin.x + focal_padding,
            y: content_top,
        },
        size: UiSize::new((focal.size.width * 0.56).max(0.0), content_height),
    };
    push_ui_authoring_preview_surface(
        &mut decorated.display_list,
        primary_content,
        UiColor::rgba(0.055, 0.063, 0.063, 1.0),
        6.0,
    )?;
    let secondary_content = UiRect {
        origin: UiPoint {
            x: primary_content.origin.x + primary_content.size.width + 8.0,
            y: content_top,
        },
        size: UiSize::new(
            (focal.origin.x + focal.size.width
                - focal_padding
                - primary_content.origin.x
                - primary_content.size.width
                - 8.0)
                .max(0.0),
            content_height,
        ),
    };
    push_ui_authoring_preview_surface(
        &mut decorated.display_list,
        secondary_content,
        UiColor::rgba(0.071, 0.082, 0.082, 1.0),
        6.0,
    )?;
    for fraction in [0.24_f32, 0.49, 0.74] {
        let source_row = UiRect {
            origin: UiPoint {
                x: primary_content.origin.x + 10.0,
                y: primary_content.origin.y + primary_content.size.height * fraction,
            },
            size: UiSize::new((primary_content.size.width - 20.0).max(0.0), 5.0),
        };
        push_ui_authoring_preview_surface(
            &mut decorated.display_list,
            source_row,
            UiColor::rgba(0.451, 0.478, 0.463, 0.52),
            3.0,
        )?;
    }
    decorated.display_list.validate()?;
    Ok(Arc::new(decorated))
}

#[allow(clippy::cast_precision_loss)] // Source millimetres are projected only into a bounded preview.
fn project_modeler_preview_position(position: meridian_modeler::Millimetres3) -> (f32, f32) {
    let x = position.x as f32;
    let y = position.y as f32;
    let z = position.z as f32;
    // A fixed isometric projection is a renderer-neutral inspection aid. It
    // does not alter model source or claim a camera/tool mode.
    (x - z * 0.58, y * 0.92 + (x + z) * 0.24)
}

/// Renders the current `PenumbraPreview` as a bounded source-derived wireframe.
///
/// The editable model stays authoritative. This merely projects the immutable
/// preview positions and indices that `meridian-modeler` already derives from
/// that source, avoiding a fake mesh or backend-owned viewport state.
///
/// # Errors
///
/// Returns a typed display-list error without changing the accepted frame.
#[allow(clippy::too_many_lines)] // Projection stays together to keep source-derived geometry auditable.
pub fn decorate_modeler_preview(
    modeler: Option<&CreatorModelerPresentation>,
    frame: &UiFrameOutput,
) -> Result<UiFrameOutput, DisplayListError> {
    let canvas = frame
        .layout
        .iter()
        .find(|entry| entry.node == CREATOR_MODELER_PREVIEW_CANVAS)
        .map(|entry| entry.bounds);
    let Some(canvas) = canvas else {
        return Ok(Arc::clone(frame));
    };
    let Some(preview) = modeler.and_then(|presentation| presentation.preview.as_ref()) else {
        return Ok(Arc::clone(frame));
    };
    if preview.positions_mm.is_empty() || preview.triangle_indices.len() < 3 {
        return Ok(Arc::clone(frame));
    }

    let projected = preview
        .positions_mm
        .iter()
        .copied()
        .map(project_modeler_preview_position)
        .collect::<Vec<_>>();
    let (mut minimum_x, mut maximum_x, mut minimum_y, mut maximum_y) = (
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
    );
    for (x, y) in &projected {
        minimum_x = minimum_x.min(*x);
        maximum_x = maximum_x.max(*x);
        minimum_y = minimum_y.min(*y);
        maximum_y = maximum_y.max(*y);
    }
    let padding = 30.0;
    let available_width = (canvas.size.width - padding * 2.0).max(1.0);
    let available_height = (canvas.size.height - padding * 2.0).max(1.0);
    let span_x = (maximum_x - minimum_x).abs().max(1.0);
    let span_y = (maximum_y - minimum_y).abs().max(1.0);
    let scale = (available_width / span_x).min(available_height / span_y);
    let map_position = |(x, y): (f32, f32)| UiPoint {
        x: canvas.origin.x + padding + (x - minimum_x) * scale,
        y: canvas.origin.y + canvas.size.height - padding - (y - minimum_y) * scale,
    };

    let mut decorated = (**frame).clone();
    let axis = UiColor::rgba(0.161, 0.176, 0.173, 0.34);
    let horizontal_axis = canvas.origin.y + canvas.size.height * 0.68;
    decorated.display_list.try_push(DisplayPrimitive::Path {
        node: CREATOR_MODELER_PREVIEW_CANVAS,
        commands: vec![
            UiPathCommand::MoveTo(UiPoint {
                x: canvas.origin.x + 18.0,
                y: horizontal_axis,
            }),
            UiPathCommand::LineTo(UiPoint {
                x: canvas.origin.x + canvas.size.width - 18.0,
                y: horizontal_axis,
            }),
        ],
        fill: None,
        stroke: Some(UiStroke::new(axis, 1.0)),
    })?;
    for indices in preview.triangle_indices.chunks_exact(3) {
        let [first, second, third] = indices else {
            continue;
        };
        let (Some(first), Some(second), Some(third)) = (
            projected.get(*first as usize),
            projected.get(*second as usize),
            projected.get(*third as usize),
        ) else {
            continue;
        };
        decorated.display_list.try_push(DisplayPrimitive::Path {
            node: CREATOR_MODELER_PREVIEW_CANVAS,
            commands: vec![
                UiPathCommand::MoveTo(map_position(*first)),
                UiPathCommand::LineTo(map_position(*second)),
                UiPathCommand::LineTo(map_position(*third)),
                UiPathCommand::Close,
            ],
            fill: Some(UiColor::rgba(0.553, 0.537, 0.38, 0.28)),
            stroke: Some(UiStroke::new(UiColor::amber(), 1.0)),
        })?;
    }
    for position in projected {
        let point = map_position(position);
        decorated
            .display_list
            .try_push(DisplayPrimitive::RoundedRect {
                node: CREATOR_MODELER_PREVIEW_CANVAS,
                bounds: UiRect {
                    origin: UiPoint {
                        x: point.x - 2.0,
                        y: point.y - 2.0,
                    },
                    size: UiSize::new(4.0, 4.0),
                },
                radii: UiCornerRadii::uniform(2.0),
                color: UiColor::text(),
            })?;
    }
    decorated.display_list.validate()?;
    Ok(Arc::new(decorated))
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
    creator_authored_document(
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
    creator_authored_document(
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
    use meridian_editor_core::{
        CommandMetadata, EditorCommand, EditorTransaction, ProjectDocument, Translation,
    };
    use meridian_ui::{
        DisplayPrimitive, SemanticAction, SemanticDelta, SemanticRole, UiEvent, UiFrameInput,
        UiPoint, UiRuntime, UiSize, UiWidgetKind,
    };

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
        let inspector = creator_alpha_document(&session).expect("valid inspector UI document");
        let mut history_view = CreatorWorkspaceView::foundation(&session, "Ready");
        history_view.focused_panel = Some(EditorPanelId::History);
        let history = creator_workspace_document_with_view(&session, &history_view)
            .expect("valid history UI document");
        for (document, action) in [
            (&inspector, "editor.edit-placement"),
            (&history, "editor.undo"),
            (&history, "editor.redo"),
        ] {
            let node = document
                .focus_order()
                .into_iter()
                .find_map(|id| {
                    document
                        .node(id)
                        .filter(|node| node.semantics.action.as_deref() == Some(action))
                })
                .expect("declared action");
            assert_eq!(node.kind, UiWidgetKind::Button);
            assert!(node.focusable);
            assert_eq!(node.semantics.action.as_deref(), Some(action));
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

    #[test]
    fn creator_hub_exposes_named_create_open_and_recent_remediation_actions() {
        let hub = creator_hub_document(
            &[RecentProjectView {
                label: "Unavailable public project".to_owned(),
                path: "/missing/public-project".to_owned(),
                available: false,
            }],
            "Choose a project.",
        )
        .expect("valid Creator hub");
        let project_name = hub
            .node(CREATOR_HUB_PROJECT_NAME)
            .expect("project-name field");
        assert_eq!(project_name.kind, UiWidgetKind::TextInput);
        for id in [90_003_u128, 90_004, 90_101, 90_102] {
            let node = hub.node(UiNodeId::new(id)).expect("hub action");
            assert!(node.focusable);
            assert!(node.semantics.action.is_some());
        }
        assert_eq!(
            hub.node(UiNodeId::new(90_101))
                .and_then(|node| node.semantics.action.as_deref()),
            Some("hub.locate-recent:0")
        );
        assert!(
            hub.node(SHELL_WORKSPACE_ROW).is_none(),
            "project-only navigation must not appear before a project is open"
        );
        assert!(!hub.nodes().any(|node| {
            matches!(
                node.semantics.action.as_deref(),
                Some("shell.play-unavailable" | "shell.build-unavailable")
            )
        }));
    }

    fn public_creator_session() -> EditorSession {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("repository root");
        let source =
            ProjectDocument::read_source(root.join("examples/creator-alpha/project.meridian.json"))
                .expect("public Creator project source");
        EditorSession::open(source).expect("public Creator session")
    }

    fn assert_creator_authored_source_round_trips(document: UiDocument) {
        let source = document.canonical_source_snapshot();
        assert_eq!(source.nodes.len(), source.node_sources.len());
        assert_eq!(source.styles.len(), 15);
        assert_eq!(source.components.len(), 15);
        assert!(source
            .node_sources
            .iter()
            .any(|(_, node_source)| node_source.component.is_some()));

        let encoded = document
            .encode_canonical_source()
            .expect("Creator source encodes deterministically");
        let recovered = UiDocument::decode_canonical_source(&encoded)
            .expect("Creator source recovers before it reaches the frame compiler");
        assert_eq!(recovered, document);

        let input = UiFrameInput::new(UiSize::new(1280.0, 800.0));
        let source_frame = UiRuntime::new(document).reconcile(input.clone());
        let recovered_frame = UiRuntime::new(recovered).reconcile(input);
        assert_eq!(source_frame.layout, recovered_frame.layout);
        assert_eq!(source_frame.display_list, recovered_frame.display_list);
        assert_eq!(source_frame.semantic_tree, recovered_frame.semantic_tree);
    }

    fn assert_creator_controls_fit(document: UiDocument, viewport: UiSize, scale_factor: f32) {
        let expected_focus_order = document.focus_order();
        let node_names = document
            .nodes()
            .map(|node| (node.id, node.semantics.name.clone()))
            .collect::<std::collections::BTreeMap<_, _>>();
        let mut runtime = UiRuntime::new(document);
        let mut input = UiFrameInput::new(viewport);
        input.scale_factor = scale_factor;
        let output = runtime.reconcile(input);
        let tree = match &output.semantic_delta {
            SemanticDelta::Replace(tree) => tree,
            SemanticDelta::Update(_) => panic!("first Creator frame cannot be incremental"),
            SemanticDelta::Unchanged => panic!("first Creator frame must publish semantics"),
        };
        let visible_focusable = tree
            .nodes
            .iter()
            .filter(|node| node.actions.contains(&SemanticAction::Focus))
            .collect::<Vec<_>>();
        assert_eq!(visible_focusable.len(), expected_focus_order.len());
        for entry in &output.layout {
            let bounds = entry.bounds;
            let name = node_names
                .get(&entry.node)
                .map_or("unnamed Creator node", String::as_str);
            assert!(bounds.origin.x.is_finite() && bounds.origin.y.is_finite());
            assert!(bounds.size.width.is_finite() && bounds.size.height.is_finite());
            assert!(bounds.origin.x >= 0.0 && bounds.origin.y >= 0.0);
            assert!(
                bounds.size.width >= 0.0 && bounds.size.height >= 0.0,
                "{name} has a negative layout extent: {bounds:?}"
            );
            assert!(
                bounds.origin.x + bounds.size.width <= viewport.width + 0.1,
                "{name} extends past the visible viewport width"
            );
            assert!(
                bounds.origin.y + bounds.size.height <= viewport.height + 0.1,
                "{name} extends past the visible viewport height"
            );
        }
        for node in &visible_focusable {
            let bounds = node.bounds;
            assert!(bounds.origin.x.is_finite() && bounds.origin.y.is_finite());
            assert!(bounds.size.width.is_finite() && bounds.size.height.is_finite());
            assert!(bounds.origin.x >= 0.0 && bounds.origin.y >= 0.0);
            assert!(bounds.size.width >= 1.0 && bounds.size.height >= 1.0);
            assert!(
                bounds.origin.x + bounds.size.width <= viewport.width + 0.1,
                "{} extends past the visible viewport width",
                node.name
            );
            assert!(
                bounds.origin.y + bounds.size.height <= viewport.height + 0.1,
                "{} extends past the visible viewport height",
                node.name
            );
            match node.role {
                SemanticRole::TextInput | SemanticRole::SearchBox => assert!(
                    bounds.size.height >= 28.0,
                    "{} text field height fell below its declared size",
                    node.name
                ),
                _ => assert!(
                    bounds.size.height >= 20.0,
                    "{} control height fell below the compact-action minimum",
                    node.name
                ),
            }
        }
        for (index, first) in visible_focusable.iter().enumerate() {
            for second in visible_focusable.iter().skip(index + 1) {
                let first_right = first.bounds.origin.x + first.bounds.size.width;
                let first_bottom = first.bounds.origin.y + first.bounds.size.height;
                let second_right = second.bounds.origin.x + second.bounds.size.width;
                let second_bottom = second.bounds.origin.y + second.bounds.size.height;
                let overlaps = first.bounds.origin.x < second_right - 0.1
                    && second.bounds.origin.x < first_right - 0.1
                    && first.bounds.origin.y < second_bottom - 0.1
                    && second.bounds.origin.y < first_bottom - 0.1;
                assert!(
                    !overlaps,
                    "focusable Creator controls overlap: {} at {:?} and {} at {:?}",
                    first.name, first.bounds, second.name, second.bounds
                );
            }
        }
    }

    fn assert_creator_text_fits(document: UiDocument, viewport: UiSize, scale_factor: f32) {
        let mut runtime = UiRuntime::new(document);
        let mut input = UiFrameInput::new(viewport);
        input.scale_factor = scale_factor;
        let output = runtime.reconcile(input);
        for primitive in &output.display_list.primitives {
            let (node, text, bounds, layout) = match primitive {
                DisplayPrimitive::Text {
                    node,
                    text,
                    bounds,
                    layout,
                    ..
                }
                | DisplayPrimitive::GlyphRun {
                    node,
                    text,
                    bounds,
                    layout,
                    ..
                } => (*node, text, bounds, layout),
                _ => continue,
            };
            assert!(
                layout.width <= bounds.size.width + 0.1,
                "Creator text for {node:?} exceeds its authored width: {text:?}"
            );
            assert!(
                layout.height <= bounds.size.height + 0.1,
                "Creator text for {node:?} exceeds its authored height: {text:?}; text {layout:?}, slot {bounds:?}"
            );
        }
    }

    fn semantic_bounds(output: &meridian_ui::UiFrameOutput, id: UiNodeId) -> meridian_ui::UiRect {
        let tree = match &output.semantic_delta {
            SemanticDelta::Replace(tree) => tree,
            SemanticDelta::Update(_) => panic!("first fixture frame cannot be incremental"),
            SemanticDelta::Unchanged => panic!("first fixture frame must publish semantics"),
        };
        tree.nodes
            .iter()
            .find(|node| node.id == id)
            .map(|node| node.bounds)
            .expect("fixture node has semantic bounds")
    }

    #[test]
    fn creator_hub_pointer_reaches_the_visible_create_button() {
        let hub = creator_hub_document(&[], "Choose a project.").expect("valid Creator hub");
        let mut runtime = UiRuntime::new(hub);
        let first = runtime.reconcile(UiFrameInput::new(UiSize::new(1280.0, 800.0)));
        let bounds = semantic_bounds(&first, UiNodeId::new(90_003));
        let point = UiPoint {
            x: bounds.origin.x + bounds.size.width / 2.0,
            y: bounds.origin.y + bounds.size.height / 2.0,
        };
        let mut input = UiFrameInput::new(UiSize::new(1280.0, 800.0));
        input.events = vec![UiEvent::PointerDown(point), UiEvent::PointerUp(point)];
        let output = runtime.reconcile(input);

        assert_eq!(output.commands.len(), 1);
        assert_eq!(output.commands[0].action, "hub.create-project");
    }

    #[test]
    fn hub_host_status_appears_only_in_the_permanent_status_line() {
        let status = "Unique hub status is reported once.";
        let hub = creator_hub_document(&[], status).expect("valid Creator hub");
        let frame = UiRuntime::new(hub).reconcile(UiFrameInput::new(UiSize::new(1024.0, 720.0)));
        let count = frame
            .display_list
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                DisplayPrimitive::Text { text, .. } | DisplayPrimitive::GlyphRun { text, .. } => {
                    Some(text.as_str())
                }
                _ => None,
            })
            .filter(|text| text.contains("Unique hub status"))
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn creator_hub_uses_one_visible_parent_surface_without_nested_recent_cards() {
        let hub = creator_hub_document(
            &[RecentProjectView {
                label: "Unavailable public project".to_owned(),
                path: "/missing/public-project".to_owned(),
                available: false,
            }],
            "Choose a project.",
        )
        .expect("valid Creator hub");
        let mut runtime = UiRuntime::new(hub);
        let output = runtime.reconcile(UiFrameInput::new(UiSize::new(1280.0, 800.0)));
        let hub_surface = UiNodeId::new(90_030);
        let missing_recent = UiNodeId::new(90_103);

        assert!(output.display_list.primitives.iter().any(|primitive| {
            matches!(primitive, DisplayPrimitive::RoundedRect { node, .. } if *node == hub_surface)
        }));
        assert!(!output.display_list.primitives.iter().any(|primitive| {
            matches!(primitive, DisplayPrimitive::Border { node, color, .. }
                if *node == missing_recent && *color == UiColor::red())
        }));
    }

    #[test]
    fn bounded_recent_project_rows_keep_their_declared_height() {
        let recents = (0..MAX_CREATOR_HUB_RECENT_ROWS)
            .map(|index| RecentProjectView {
                label: format!("Public project {index}"),
                path: format!("/public/project-{index}"),
                available: true,
            })
            .collect::<Vec<_>>();
        let hub = creator_hub_document(&recents, "Choose a project.").expect("valid Creator hub");
        let mut runtime = UiRuntime::new(hub);
        let output = runtime.reconcile(UiFrameInput::new(UiSize::new(1280.0, 800.0)));

        for index in 0..MAX_CREATOR_HUB_RECENT_ROWS {
            let row = UiNodeId::new(90_103_u128 + (index as u128) * 4);
            let bounds = semantic_bounds(&output, row);
            assert!(
                (bounds.size.height - 48.0).abs() < 0.1,
                "recent row {index} was squeezed to {} px",
                bounds.size.height
            );
        }
    }

    #[test]
    fn creator_workspace_pointer_and_tab_order_reach_inspector_controls() {
        let session = public_creator_session();
        let document = creator_alpha_document(&session).expect("valid Creator workspace");
        let focus_order = document.focus_order();
        let x = focus_order
            .iter()
            .position(|id| *id == CREATOR_INSPECTOR_X_MM)
            .expect("X field is focusable");
        assert_eq!(focus_order[x + 1], CREATOR_INSPECTOR_Y_MM);
        assert_eq!(focus_order[x + 2], CREATOR_INSPECTOR_Z_MM);
        assert_eq!(
            document
                .node(focus_order[x + 3])
                .and_then(|node| node.semantics.action.as_deref()),
            Some("editor.preview-command")
        );

        let edit = focus_order
            .iter()
            .copied()
            .find(|id| {
                document
                    .node(*id)
                    .and_then(|node| node.semantics.action.as_deref())
                    == Some("editor.edit-placement")
            })
            .expect("edit placement action is focusable");
        let mut runtime = UiRuntime::new(document);
        let first = runtime.reconcile(UiFrameInput::new(UiSize::new(1024.0, 720.0)));
        let bounds = semantic_bounds(&first, edit);
        let point = UiPoint {
            x: bounds.origin.x + bounds.size.width / 2.0,
            y: bounds.origin.y + bounds.size.height / 2.0,
        };
        let mut input = UiFrameInput::new(UiSize::new(1024.0, 720.0));
        input.events = vec![UiEvent::PointerDown(point), UiEvent::PointerUp(point)];
        let output = runtime.reconcile(input);

        assert_eq!(output.commands.len(), 1);
        assert_eq!(output.commands[0].action, "editor.edit-placement");
    }

    #[test]
    fn creator_workspace_uses_curated_copy_and_only_current_play_actions() {
        for panel in creator_alpha_panels() {
            for command in panel.commands {
                assert_ne!(creator_action_label(command), "Action");
            }
        }

        let mut session = public_creator_session();
        let has_action = |document: &UiDocument, action| {
            document.focus_order().iter().any(|id| {
                document
                    .node(*id)
                    .and_then(|node| node.semantics.action.as_deref())
                    == Some(action)
            })
        };
        let edit_document = creator_alpha_document(&session).expect("valid edit workspace");
        let save = edit_document
            .focus_order()
            .into_iter()
            .find_map(|id| {
                edit_document.node(id).filter(|node| {
                    node.semantics.action.as_deref() == Some("editor.edit-placement")
                })
            })
            .expect("save placement control");
        assert_eq!(save.text.as_deref(), Some("Save"));
        assert!(has_action(&edit_document, "editor.play-start"));
        assert!(!has_action(&edit_document, "editor.play-apply"));
        assert!(!has_action(&edit_document, "editor.play-discard"));

        session.start_play().expect("Play session starts");
        let play_document = creator_alpha_document(&session).expect("valid Play workspace");
        assert!(!has_action(&play_document, "editor.play-start"));
        assert!(has_action(&play_document, "editor.play-apply"));
        assert!(has_action(&play_document, "editor.play-discard"));
        assert_creator_controls_fit(play_document, UiSize::new(1024.0, 720.0), 1.0);
    }

    #[test]
    fn filtered_workspace_actions_keep_stable_node_identity() {
        let mut session = public_creator_session();
        let edit_document = creator_alpha_document(&session).expect("valid edit workspace");
        let action_id = |document: &UiDocument, action: &str| {
            document.focus_order().into_iter().find(|id| {
                document
                    .node(*id)
                    .and_then(|node| node.semantics.action.as_deref())
                    == Some(action)
            })
        };
        assert_eq!(
            action_id(&edit_document, "editor.play-start"),
            Some(UiNodeId::new(92_003))
        );

        let mut runtime = UiRuntime::new(edit_document);
        let focused = runtime.reconcile({
            let mut input = UiFrameInput::new(UiSize::new(1024.0, 720.0));
            input.events = vec![UiEvent::AssistiveFocus(UiNodeId::new(92_003))];
            input
        });
        assert_eq!(focused.focused, Some(UiNodeId::new(92_003)));

        session.start_play().expect("Play session starts");
        let play_document = creator_alpha_document(&session).expect("valid Play workspace");
        assert_eq!(
            action_id(&play_document, "editor.play-apply"),
            Some(UiNodeId::new(92_043))
        );
        assert_eq!(
            action_id(&play_document, "editor.play-discard"),
            Some(UiNodeId::new(92_007))
        );
        assert!(play_document.node(UiNodeId::new(92_003)).is_none());
        runtime.replace_document(play_document);
        let output = runtime.reconcile({
            let mut input = UiFrameInput::new(UiSize::new(1024.0, 720.0));
            input.events = vec![UiEvent::Activate];
            input
        });
        assert!(output.commands.is_empty());
    }

    #[test]
    fn creator_hub_contains_no_distribution_status_label() {
        let hub = creator_hub_document(&[], "Choose a project.").expect("valid Creator hub");
        let mut runtime = UiRuntime::new(hub);
        let output = runtime.reconcile(UiFrameInput::new(UiSize::new(1280.0, 800.0)));
        let rendered_text = output
            .display_list
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                DisplayPrimitive::Text { text, .. } | DisplayPrimitive::GlyphRun { text, .. } => {
                    Some(text.as_str())
                }
                DisplayPrimitive::Rect { .. }
                | DisplayPrimitive::Border { .. }
                | DisplayPrimitive::FocusIndicator { .. }
                | DisplayPrimitive::RoundedRect { .. }
                | DisplayPrimitive::Path { .. }
                | DisplayPrimitive::Image { .. }
                | DisplayPrimitive::Mesh { .. }
                | DisplayPrimitive::PushClip { .. }
                | DisplayPrimitive::PopClip { .. }
                | DisplayPrimitive::BeginLayer { .. }
                | DisplayPrimitive::EndLayer { .. }
                | DisplayPrimitive::Shadow { .. }
                | DisplayPrimitive::Backdrop { .. } => None,
            });
        for text in rendered_text {
            assert!(!text.contains("Unsigned"));
            assert!(!text.contains("unsigned"));
            assert!(!text.contains("developer preview"));
            assert!(!text.contains("beta"));
        }
    }

    #[test]
    fn creator_workspace_uses_the_locked_shell_and_world_width_priorities() {
        let session = public_creator_session();
        let document = creator_alpha_document(&session).expect("valid Creator workspace");
        let mut runtime = UiRuntime::new(document);
        let output = runtime.reconcile(UiFrameInput::new(UiSize::new(1440.0, 900.0)));
        let application_row = semantic_bounds(&output, SHELL_APPLICATION_ROW);
        let workspace_row = semantic_bounds(&output, SHELL_WORKSPACE_ROW);
        let status_row = semantic_bounds(&output, SHELL_STATUS_ROW);
        let viewport = semantic_bounds(&output, UiNodeId::new(132));
        let browser = semantic_bounds(&output, UiNodeId::new(164));
        let inspector = semantic_bounds(&output, UiNodeId::new(196));
        let shelf = semantic_bounds(&output, UiNodeId::new(5));
        let project = semantic_bounds(&output, UiNodeId::new(92_083));
        let generated = semantic_bounds(&output, UiNodeId::new(92_057));

        assert!((application_row.size.height - 44.0).abs() < 0.1);
        assert!((workspace_row.size.height - 36.0).abs() < 0.1);
        assert!((status_row.size.height - 24.0).abs() < 0.1);
        assert!((browser.size.width - 264.0).abs() < 0.1);
        assert!((inspector.size.width - 344.0).abs() < 0.1);
        assert!(
            (shelf.size.height - 32.0).abs() < 0.1,
            "the compact shelf remains a clean 32 px peek until its commands are opened"
        );
        assert!(
            generated.origin.y + generated.size.height - project.origin.y < 264.1,
            "World hierarchy should stay dense instead of consuming the browser height: project={project:?}, generated={generated:?}"
        );
        assert!(output
            .semantic_tree
            .nodes
            .iter()
            .any(|node| node.name == "World activity summary"));
        assert!(!output
            .semantic_tree
            .nodes
            .iter()
            .any(|node| node.name == "World bottom shelf content"));
        assert!(viewport.size.width > browser.size.width);
        assert!(viewport.size.width > inspector.size.width);

        let mut canvas_view = CreatorWorkspaceView::foundation(&session, "Ready");
        canvas_view.focused_panel = Some(EditorPanelId::Viewport);
        let canvas_document = creator_workspace_document_with_view(&session, &canvas_view)
            .expect("canvas-focused World workspace is valid");
        let mut canvas_runtime = UiRuntime::new(canvas_document);
        let canvas_output = canvas_runtime.reconcile(UiFrameInput::new(UiSize::new(1440.0, 900.0)));
        assert!(
            (semantic_bounds(&canvas_output, UiNodeId::new(5))
                .size
                .height
                - 32.0)
                .abs()
                < 0.1
        );

        let mut focused_view = CreatorWorkspaceView::foundation(&session, "Ready");
        focused_view.focused_panel = Some(EditorPanelId::Hierarchy);
        let focused = creator_workspace_document_with_view(&session, &focused_view)
            .expect("focused World workspace is valid");
        assert_eq!(
            focused
                .node(UiNodeId::new(164))
                .and_then(|node| node.style.border.as_ref())
                .map(|border| border.color),
            Some(UiColor::amber())
        );

        let mut shelf_view = CreatorWorkspaceView::foundation(&session, "Ready");
        shelf_view.focused_panel = Some(EditorPanelId::History);
        let expanded = creator_workspace_document_with_view(&session, &shelf_view)
            .expect("expanded World shelf is valid");
        let mut expanded_runtime = UiRuntime::new(expanded);
        let expanded_output =
            expanded_runtime.reconcile(UiFrameInput::new(UiSize::new(1440.0, 900.0)));
        assert!(
            (semantic_bounds(&expanded_output, UiNodeId::new(5))
                .size
                .height
                - 240.0)
                .abs()
                < 0.1
        );
    }

    #[test]
    fn creator_workbenches_use_the_locked_eight_pixel_outer_gutter() {
        let session = public_creator_session();
        let mut view = CreatorWorkspaceView::foundation(&session, "Ready");
        let workbenches = [
            (
                WorkspaceKind::World,
                UiNodeId::new(4),
                UiNodeId::new(92_060),
                UiNodeId::new(196),
            ),
            (
                WorkspaceKind::Code,
                UiNodeId::new(95_001),
                UiNodeId::new(95_002),
                UiNodeId::new(95_005),
            ),
            (
                WorkspaceKind::Modeler,
                UiNodeId::new(96_102),
                UiNodeId::new(96_103),
                UiNodeId::new(96_106),
            ),
            (
                WorkspaceKind::UiAuthoring,
                UiNodeId::new(96_202),
                UiNodeId::new(96_203),
                UiNodeId::new(96_206),
            ),
            (
                WorkspaceKind::Materials,
                UiNodeId::new(96_302),
                UiNodeId::new(96_303),
                UiNodeId::new(96_306),
            ),
        ];
        for (workspace, main_id, first_id, last_id) in workbenches {
            view.workspace = workspace;
            let document = creator_workspace_document_with_view(&session, &view)
                .expect("Creator workbench is valid");
            let frame =
                UiRuntime::new(document).reconcile(UiFrameInput::new(UiSize::new(1440.0, 900.0)));
            let bounds = |id| {
                frame
                    .layout
                    .iter()
                    .find(|entry| entry.node == id)
                    .expect("Creator workbench node has layout")
                    .bounds
            };
            let main = bounds(main_id);
            let first = bounds(first_id);
            let last = bounds(last_id);
            assert!((first.origin.x - (main.origin.x + 8.0)).abs() <= 0.1);
            assert!((first.origin.y - (main.origin.y + 8.0)).abs() <= 0.1);
            assert!(
                ((last.origin.x + last.size.width) - (main.origin.x + main.size.width - 8.0)).abs()
                    <= 0.1,
                "{workspace:?} must retain its right-side dock gutter"
            );
        }
    }

    #[test]
    fn creator_shell_uses_flat_bands_and_a_single_navigation_tray() {
        let session = public_creator_session();
        let document = creator_alpha_document(&session).expect("valid Creator workspace");
        let application = document
            .node(SHELL_APPLICATION_ROW)
            .expect("application shell row");
        let workspace = document
            .node(SHELL_WORKSPACE_ROW)
            .expect("workspace shell row");
        let status = document.node(SHELL_STATUS_ROW).expect("status shell row");
        let tray = document
            .node(UiNodeId::new(92_030))
            .expect("workspace navigation tray");
        let utilities = document
            .node(UiNodeId::new(92_009))
            .expect("application utility cluster");

        for row in [application, workspace, status] {
            assert!(row.style.border.is_none());
            assert!((row.style.corner_radius - 0.0).abs() <= f32::EPSILON);
        }
        assert_eq!(application.style.background, Some(UiColor::surface()));
        assert_eq!(workspace.style.background, Some(UiColor::background()));
        assert_eq!(status.style.background, Some(UiColor::surface()));
        assert_eq!(tray.style.background, Some(UiColor::surface()));
        assert!(tray.style.border.is_some());
        assert!((tray.style.corner_radius - 10.0).abs() <= f32::EPSILON);
        assert!((utilities.style.corner_radius - 6.0).abs() <= f32::EPSILON);
    }

    #[test]
    fn world_workbench_separates_the_deep_canvas_from_opaque_tool_chrome() {
        let session = public_creator_session();
        let document = creator_alpha_document(&session).expect("valid Creator workspace");
        for node in [UiNodeId::new(164), UiNodeId::new(132), UiNodeId::new(196)] {
            let panel = document.node(node).expect("World workbench panel exists");
            assert_eq!(panel.style.background, Some(UiColor::surface()));
            assert!((panel.style.corner_radius - 10.0).abs() <= f32::EPSILON);
            assert!(panel.style.border.is_some());
        }
        let canvas = document
            .node(CREATOR_WORLD_VIEWPORT_CANVAS)
            .expect("World canvas exists");
        assert_eq!(canvas.style.background, Some(UiColor::background()));
        assert!(canvas.style.border.is_none());
        assert!((canvas.style.corner_radius - 6.0).abs() <= f32::EPSILON);
        assert_ne!(
            canvas.style.background,
            document
                .node(UiNodeId::new(132))
                .expect("viewport chrome exists")
                .style
                .background,
            "the primary work surface must not blend into its surrounding tool chrome"
        );
    }

    #[test]
    fn world_viewport_header_keeps_status_quiet_and_the_canvas_primary() {
        let session = public_creator_session();
        let document = creator_alpha_document(&session).expect("valid Creator workspace");
        let title = document
            .node(UiNodeId::new(92_071))
            .expect("World viewport title");
        let mode = document
            .node(UiNodeId::new(92_072))
            .expect("World viewport mode");
        assert_eq!(title.text.as_deref(), Some("World"));
        assert_eq!(mode.text.as_deref(), Some("Perspective · Lit"));
        assert!(title.style.background.is_none() && title.style.border.is_none());
        assert!(mode.style.background.is_none() && mode.style.border.is_none());
        assert!(
            document.node(UiNodeId::new(92_073)).is_none(),
            "viewing state is one concise status, not a row of pseudo controls"
        );

        let frame =
            UiRuntime::new(document).reconcile(UiFrameInput::new(UiSize::new(1440.0, 900.0)));
        let header = semantic_bounds(&frame, UiNodeId::new(92_070));
        let canvas = semantic_bounds(&frame, CREATOR_WORLD_VIEWPORT_CANVAS);
        assert!((header.size.height - 32.0).abs() <= 0.1);
        assert!(canvas.size.height > header.size.height * 8.0);
    }

    #[test]
    fn world_tool_panel_headers_use_one_quiet_hairline_divider() {
        let session = public_creator_session();
        let document = creator_alpha_document(&session).expect("valid Creator workspace");
        for id in [92_088, 93_050] {
            let divider = document
                .node(UiNodeId::new(id))
                .expect("World tool header divider");
            assert_eq!(divider.style.background, Some(UiColor::border()));
            assert!(divider.style.border.is_none());
            assert!((divider.style.corner_radius - 0.0).abs() <= f32::EPSILON);
        }

        let frame =
            UiRuntime::new(document).reconcile(UiFrameInput::new(UiSize::new(1440.0, 900.0)));
        let bounds = |id| {
            frame
                .layout
                .iter()
                .find(|entry| entry.node == UiNodeId::new(id))
                .expect("World header node has layout")
                .bounds
        };
        for (header, row, divider) in [(92_051, 92_087, 92_088), (93_000, 93_049, 93_050)] {
            let header = bounds(header);
            let row = bounds(row);
            let divider = bounds(divider);
            assert!((header.size.height - 32.0).abs() <= 0.1);
            assert!((divider.size.height - 1.0).abs() <= 0.1);
            assert!(row.origin.y + row.size.height <= divider.origin.y + 0.1);
        }
    }

    #[test]
    fn world_browser_anchors_source_facts_and_actions_at_its_bottom_edge() {
        let session = public_creator_session();
        let document = creator_alpha_document(&session).expect("valid Creator workspace");
        let frame =
            UiRuntime::new(document).reconcile(UiFrameInput::new(UiSize::new(1440.0, 900.0)));
        let bounds = |id| {
            frame
                .layout
                .iter()
                .find(|entry| entry.node == UiNodeId::new(id))
                .expect("World browser node has layout")
                .bounds
        };
        let browser = bounds(164);
        let tree = bounds(92_054);
        let spacer = bounds(92_086);
        let status = bounds(92_080);
        let actions = bounds(92_058);

        assert!(tree.origin.y + tree.size.height <= spacer.origin.y + 0.1);
        assert!(spacer.origin.y + spacer.size.height <= status.origin.y + 0.1);
        assert!(status.origin.y + status.size.height <= actions.origin.y - 7.9);
        assert!(
            actions.origin.y + actions.size.height <= browser.origin.y + browser.size.height - 11.9,
            "browser actions must retain the panel's bottom inset"
        );
    }

    #[test]
    fn world_inspector_keeps_editing_dense_and_its_source_action_contained() {
        let session = public_creator_session();
        let document = creator_alpha_document(&session).expect("valid Creator workspace");
        let frame =
            UiRuntime::new(document).reconcile(UiFrameInput::new(UiSize::new(1440.0, 900.0)));
        let bounds = |id| {
            frame
                .layout
                .iter()
                .find(|entry| entry.node == UiNodeId::new(id))
                .expect("World inspector node has layout")
                .bounds
        };
        let inspector = bounds(196);
        let selection = bounds(93_003);
        let transform = bounds(93_005);
        let source_action = bounds(93_047);

        assert!((selection.size.height - 42.0).abs() <= 0.1);
        assert!(selection.origin.y + selection.size.height <= transform.origin.y - 7.9);
        assert!((source_action.size.height - 44.0).abs() <= 0.1);
        assert!(
            source_action.origin.y + source_action.size.height
                <= inspector.origin.y + inspector.size.height - 11.9,
            "the source action must not bleed through the inspector's bottom edge"
        );
    }

    #[test]
    fn world_activity_shelf_keeps_one_active_tab_and_quiet_commands() {
        let session = public_creator_session();
        let compact_document = creator_alpha_document(&session).expect("valid Creator workspace");
        let activity = compact_document
            .node(UiNodeId::new(93_200))
            .expect("World activity tab");
        assert_eq!(
            activity.style.background,
            Some(UiColor::rgba(
                UiColor::amber().red,
                UiColor::amber().green,
                UiColor::amber().blue,
                0.12,
            ))
        );
        assert_eq!(
            activity.style.border.as_ref().map(|border| border.color),
            Some(UiColor::amber())
        );

        let mut expanded_view = CreatorWorkspaceView::foundation(&session, "Ready");
        expanded_view.focused_panel = Some(EditorPanelId::History);
        let expanded = creator_workspace_document_with_view(&session, &expanded_view)
            .expect("expanded World shelf is valid");

        for (id, expected_foreground) in [
            (93_206, UiColor::secondary_text()),
            (93_207, UiColor::secondary_text()),
            (93_215, UiColor::amber()),
        ] {
            let command = expanded
                .node(UiNodeId::new(id))
                .expect("World shelf command");
            assert!(command.style.background.is_none());
            assert!(command.style.border.is_none());
            assert_eq!(command.style.foreground, expected_foreground);
            assert!((command.style.corner_radius - 4.0).abs() <= f32::EPSILON);
        }

        let compact = UiRuntime::new(compact_document)
            .reconcile(UiFrameInput::new(UiSize::new(1440.0, 900.0)));
        assert!((semantic_bounds(&compact, UiNodeId::new(5)).size.height - 32.0).abs() <= 0.1);
        assert!(compact.semantic_tree.nodes.iter().all(|node| {
            ![
                UiNodeId::new(93_206),
                UiNodeId::new(93_207),
                UiNodeId::new(93_215),
            ]
            .contains(&node.id)
        }));

        let expanded_frame =
            UiRuntime::new(expanded).reconcile(UiFrameInput::new(UiSize::new(1440.0, 900.0)));
        let shelf = semantic_bounds(&expanded_frame, UiNodeId::new(5));
        let controls = [93_200, 93_206, 93_207, 93_215]
            .into_iter()
            .map(|id| semantic_bounds(&expanded_frame, UiNodeId::new(id)))
            .collect::<Vec<_>>();
        assert!((shelf.size.height - 240.0).abs() <= 0.1);
        for control in controls.iter().skip(1) {
            assert!(
                control.size.height >= 43.9,
                "World shelf commands must retain their full accessible height"
            );
            assert!(
                control.origin.y >= shelf.origin.y + 3.9
                    && control.origin.y + control.size.height
                        <= shelf.origin.y + shelf.size.height - 3.9,
                "World shelf command must remain inside the shelf"
            );
        }
        for pair in controls.windows(2) {
            assert!(
                pair[0].origin.x + pair[0].size.width <= pair[1].origin.x + 0.1,
                "World shelf controls must not overlap"
            );
        }
    }

    #[test]
    fn secondary_creator_actions_have_quiet_bounded_control_faces() {
        let session = public_creator_session();
        let document = creator_alpha_document(&session).expect("valid Creator workspace");
        for id in [262, 263, 93_047] {
            let action = document
                .node(UiNodeId::new(id))
                .expect("World secondary action");
            assert_eq!(action.style.background, Some(UiColor::background()));
            assert_eq!(
                action.style.border.as_ref().map(|border| border.color),
                Some(UiColor::border())
            );
            assert_eq!(action.style.foreground, UiColor::text());
            assert!((action.style.corner_radius - 6.0).abs() <= f32::EPSILON);
        }

        let primary = creator_compact_action_style(EditorPanelId::Build, "build.submit");
        assert_eq!(
            primary.background,
            Some(UiColor::rgba(
                UiColor::amber().red,
                UiColor::amber().green,
                UiColor::amber().blue,
                0.14,
            ))
        );
        assert_eq!(
            primary.border.as_ref().map(|border| border.color),
            Some(UiColor::amber())
        );
    }

    #[test]
    fn compact_world_context_yields_rail_before_primary_work_surfaces() {
        let session = public_creator_session();
        let mut view = CreatorWorkspaceView::foundation(&session, "Ready");
        view.compact_world_context = true;
        let document = creator_workspace_document_with_view(&session, &view)
            .expect("compact World workspace is valid");
        assert!(
            document.node(UiNodeId::new(92_060)).is_none(),
            "the lower-priority World activity rail must yield at compact widths"
        );
        assert_creator_controls_fit(document.clone(), UiSize::new(1024.0, 720.0), 1.0);

        let frame =
            UiRuntime::new(document).reconcile(UiFrameInput::new(UiSize::new(1024.0, 720.0)));
        let browser = semantic_bounds(&frame, UiNodeId::new(164));
        let viewport = semantic_bounds(&frame, UiNodeId::new(132));
        let inspector = semantic_bounds(&frame, UiNodeId::new(196));
        assert!((browser.size.width - 240.0).abs() <= 0.1);
        assert!((inspector.size.width - 320.0).abs() <= 0.1);
        assert!(viewport.size.width >= 390.0);
        assert!(
            viewport.origin.x + viewport.size.width <= inspector.origin.x - 7.9,
            "the compact World viewport must not intrude into the inspector"
        );
        assert!(
            browser.origin.x + browser.size.width <= viewport.origin.x - 7.9,
            "the compact World browser must not intrude into the viewport"
        );
    }

    #[test]
    fn world_placement_scales_with_the_actual_canvas_at_desktop_width() {
        fn canvas_geometry(frame: &UiFrameOutput) -> WorldViewportGeometry {
            let canvas = frame
                .layout
                .iter()
                .find(|entry| entry.node == CREATOR_WORLD_VIEWPORT_CANVAS)
                .expect("World canvas has layout")
                .bounds;
            WorldViewportGeometry::from_canvas(canvas).expect("World canvas has drawable geometry")
        }

        let session = public_creator_session();
        let document = creator_alpha_document(&session).expect("valid Creator workspace");
        let narrow = UiRuntime::new(document.clone())
            .reconcile(UiFrameInput::new(UiSize::new(1024.0, 720.0)));
        let desktop =
            UiRuntime::new(document).reconcile(UiFrameInput::new(UiSize::new(2048.0, 1152.0)));
        let narrow_size = world_placement_visual_size(canvas_geometry(&narrow));
        let desktop_size = world_placement_visual_size(canvas_geometry(&desktop));

        assert!(
            desktop_size > narrow_size * 1.4,
            "the selected source placement must gain visual weight with a much wider World canvas"
        );
        assert!(
            desktop_size <= 144.0,
            "the presentation scale stays bounded even on a large display"
        );
    }

    #[test]
    fn world_selection_bounds_follow_the_source_placement_shape() {
        let geometry = WorldViewportGeometry::from_canvas(UiRect::new(
            UiPoint { x: 0.0, y: 0.0 },
            UiSize::new(900.0, 600.0),
        ))
        .expect("World canvas has drawable geometry");
        let size = world_placement_visual_size(geometry);
        let center = UiPoint { x: 450.0, y: 300.0 };
        let bounds = world_placement_selection_bounds(center, size);

        assert!(
            bounds.size.width > bounds.size.height,
            "the selection frame follows the triangle footprint instead of becoming a square pivot box"
        );
        assert!(
            (bounds.origin.y - (center.y - size - 12.0)).abs() <= 0.1,
            "the selection frame leaves a consistent top selection inset"
        );
        assert!(
            (bounds.origin.y + bounds.size.height - (center.y + size * 0.52 + 12.0)).abs() <= 0.1,
            "the selection frame leaves a consistent base selection inset"
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)] // Canvas geometry, selection chrome, and source movement share one visual contract.
    fn world_viewport_is_a_real_canvas_decorated_from_authoritative_source() {
        let mut session = public_creator_session();
        let document = creator_alpha_document(&session).expect("valid Creator workspace");
        let canvas = document
            .node(CREATOR_WORLD_VIEWPORT_CANVAS)
            .expect("World viewport canvas");
        assert_eq!(canvas.kind, UiWidgetKind::Canvas);
        assert_eq!(canvas.semantics.role, SemanticRole::Canvas);
        assert_eq!(canvas.style.background, Some(UiColor::background()));
        assert!(
            (canvas.style.corner_radius - 6.0).abs() <= f32::EPSILON,
            "the central work surface keeps a quieter inner radius than its enclosing panel"
        );
        assert!(canvas.style.border.is_none());

        let mut runtime = UiRuntime::new(document);
        let frame = runtime.reconcile(UiFrameInput::new(UiSize::new(1440.0, 900.0)));
        let canvas_bounds = frame
            .layout
            .iter()
            .find(|entry| entry.node == CREATOR_WORLD_VIEWPORT_CANVAS)
            .expect("World canvas has layout")
            .bounds;
        let geometry = WorldViewportGeometry::from_canvas(canvas_bounds)
            .expect("World canvas has drawable geometry");
        let placement = session
            .document()
            .placements
            .values()
            .next()
            .expect("public placement");
        let viewport_center = canvas_bounds.origin.x + canvas_bounds.size.width * 0.5;
        assert!(
            (world_reference_vanishing(geometry).x - viewport_center).abs() <= 0.1,
            "the World grid must vanish at the viewport centre"
        );
        assert!(
            (world_placement_center(geometry, placement).x - viewport_center).abs() <= 0.1,
            "an untransformed source placement must render at the viewport centre"
        );
        let undecorated_count = frame.display_list.primitives.len();
        let decorated = decorate_world_viewport(&session, &frame).expect("viewport decorates");
        let first_paths = decorated
            .display_list
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                DisplayPrimitive::Path { commands, .. } => Some(commands.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(first_paths.len() >= 13);
        assert!(
            !decorated.display_list.primitives.iter().any(|primitive| {
                let DisplayPrimitive::Path {
                    node,
                    commands,
                    fill: None,
                    ..
                } = primitive
                else {
                    return false;
                };
                let [UiPathCommand::MoveTo(start), UiPathCommand::LineTo(end)] =
                    commands.as_slice()
                else {
                    return false;
                };
                *node == CREATOR_WORLD_VIEWPORT_CANVAS
                    && (start.y - end.y).abs() <= f32::EPSILON
                    && start.y > geometry.top + 0.1
                    && start.y < geometry.horizon - 0.1
            }),
            "the World reference grid stays on its ground plane rather than adding diagnostic sky bands"
        );
        assert!(
            !decorated.display_list.primitives.iter().any(|primitive| {
                matches!(primitive,
                    DisplayPrimitive::Path {
                        node,
                        commands,
                        stroke: Some(stroke),
                        ..
                    } if *node == CREATOR_WORLD_VIEWPORT_CANVAS
                        && commands.len() == 2
                        && (stroke.color == UiColor::red_hover() || stroke.color == UiColor::grass())
                )
            }),
            "World selection stays calm; transforms remain in the Inspector instead of drawing a debug gizmo"
        );
        let selection_chrome = decorated
            .display_list
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                DisplayPrimitive::Path {
                    node,
                    commands,
                    fill: None,
                    stroke: Some(stroke),
                } if *node == CREATOR_WORLD_VIEWPORT_CANVAS && stroke.color == UiColor::amber() => {
                    Some(commands)
                }
                _ => None,
            });
        assert!(
            selection_chrome.is_some_and(|commands| {
                commands.len() == 12
                    && commands
                        .iter()
                        .filter(|command| matches!(command, UiPathCommand::MoveTo(_)))
                        .count()
                        == 4
                    && !commands
                        .iter()
                        .any(|command| matches!(command, UiPathCommand::Close))
            }),
            "World selection uses quiet corner brackets rather than a full empty rectangle"
        );
        assert_eq!(frame.display_list.primitives.len(), undecorated_count);

        let placement_id = *session
            .document()
            .placements
            .keys()
            .next()
            .expect("public placement");
        session
            .commit(EditorTransaction {
                command: EditorCommand::SetPlacementTranslation {
                    placement_id,
                    translation: Translation {
                        x_mm: 1_000,
                        y_mm: 0,
                        z_mm: -1_000,
                    },
                },
                metadata: CommandMetadata::local("Move public placement", [placement_id]),
            })
            .expect("source edit commits");
        let moved = decorate_world_viewport(&session, &frame).expect("moved viewport decorates");
        let moved_paths = moved
            .display_list
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                DisplayPrimitive::Path { commands, .. } => Some(commands.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_ne!(first_paths, moved_paths);
    }

    #[test]
    fn creator_workspace_renders_truthful_state_in_a_bounded_visible_shell() {
        let session = public_creator_session();
        let view = CreatorWorkspaceView {
            project: "Creator Alpha".to_owned(),
            activity: "Opened authoritative source with validated recovery context.".to_owned(),
            recovery: "Recovery restored the source-matching local selection.".to_owned(),
            build: "No build is running; the durable build service is ready.".to_owned(),
            recipe: "Public placement / v1 / 4 placements every 2500 mm / CC0-1.0.".to_owned(),
            model: "Revision 1 / 1 source object / 0 selected elements / 2 derived triangles."
                .to_owned(),
            workspace: WorkspaceKind::World,
            focus_layout: false,
            code_context_width: CodeContextWidth::Standard,
            compact_world_context: false,
            compact_ui_authoring: false,
            focused_panel: None,
            project_source: "{\n  \"schema\": \"meridian.creator-project/v1\"\n}".to_owned(),
            recipe_source: "{\n  \"schema\": \"meridian.procedural-recipe/v1\"\n}".to_owned(),
            modeler: None,
        };
        let document = creator_workspace_document_with_view(&session, &view)
            .expect("truthful Creator workspace");
        let mut runtime = UiRuntime::new(document.clone());
        let output = runtime.reconcile(UiFrameInput::new(UiSize::new(1280.0, 800.0)));
        let rendered_text = output
            .display_list
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                DisplayPrimitive::Text { text, .. } | DisplayPrimitive::GlyphRun { text, .. } => {
                    Some(text.as_str())
                }
                DisplayPrimitive::Rect { .. }
                | DisplayPrimitive::Border { .. }
                | DisplayPrimitive::FocusIndicator { .. }
                | DisplayPrimitive::RoundedRect { .. }
                | DisplayPrimitive::Path { .. }
                | DisplayPrimitive::Image { .. }
                | DisplayPrimitive::Mesh { .. }
                | DisplayPrimitive::PushClip { .. }
                | DisplayPrimitive::PopClip { .. }
                | DisplayPrimitive::BeginLayer { .. }
                | DisplayPrimitive::EndLayer { .. }
                | DisplayPrimitive::Shadow { .. }
                | DisplayPrimitive::Backdrop { .. } => None,
            })
            .collect::<Vec<_>>();
        assert!(rendered_text
            .iter()
            .any(|text| text.contains("Public triangle placement")));
        assert!(rendered_text
            .iter()
            .any(|text| text.contains("Public triangle source")));
        assert!(!rendered_text
            .iter()
            .any(|text| text.contains("Public placement / v1")));
        assert!(!rendered_text
            .iter()
            .any(|text| text.contains("The world has no editable placements.")));
        assert!(!rendered_text
            .iter()
            .any(|text| text.contains("SOURCE VIEW")));
        assert_creator_controls_fit(document.clone(), UiSize::new(1024.0, 720.0), 1.0);
        assert_creator_controls_fit(document, UiSize::new(1280.0, 800.0), 2.0);
    }

    #[test]
    fn domain_workspaces_use_the_permanent_shell_and_keep_unavailable_domains_truthful() {
        let session = public_creator_session();
        let mut view = CreatorWorkspaceView::foundation(&session, "Ready");
        view.workspace = WorkspaceKind::Alluvium;
        view.recipe = "v1 recipe is ready.".to_owned();
        view.recipe_source = "{\n  \"schema\": \"meridian.procedural-recipe/v1\"\n}".to_owned();
        let alluvium = creator_workspace_document_with_view(&session, &view)
            .expect("Alluvium workspace is valid");
        assert!(alluvium.focus_order().iter().any(|id| {
            alluvium
                .node(*id)
                .and_then(|node| node.semantics.action.as_deref())
                == Some("procedural.validate")
        }));
        let alluvium_frame = UiRuntime::new(alluvium.clone())
            .reconcile(UiFrameInput::new(UiSize::new(1280.0, 800.0)));
        let alluvium_base = workspace_node_base(WorkspaceKind::Alluvium);
        for node in [alluvium_base + 3, alluvium_base + 4, alluvium_base + 6] {
            let bounds = alluvium_frame
                .layout
                .iter()
                .find(|entry| entry.node == UiNodeId::new(node))
                .expect("Alluvium auxiliary panel has layout")
                .bounds;
            assert!(
                bounds.size.height >= 500.0,
                "Alluvium auxiliaries share the continuous full-height workbench"
            );
        }
        assert_creator_controls_fit(alluvium, UiSize::new(1280.0, 800.0), 1.0);

        view.workspace = WorkspaceKind::Materials;
        let materials = creator_workspace_document_with_view(&session, &view)
            .expect("Materials workspace is valid");
        let rendered = UiRuntime::new(materials.clone())
            .reconcile(UiFrameInput::new(UiSize::new(1280.0, 800.0)));
        assert!(rendered.display_list.primitives.iter().any(|primitive| {
            matches!(primitive, DisplayPrimitive::Text { text, .. } | DisplayPrimitive::GlyphRun { text, .. }
                if text.contains("No graph compiler or material-source authority is active"))
        }));
        assert_creator_controls_fit(materials, UiSize::new(1280.0, 800.0), 1.0);
    }

    #[test]
    fn domain_shelves_report_workspace_facts_without_repeating_host_activity() {
        let session = public_creator_session();
        let activity = "Unique host activity remains in the permanent status line.";
        let mut view = CreatorWorkspaceView::foundation(&session, activity);
        for workspace in [
            WorkspaceKind::Materials,
            WorkspaceKind::Alluvium,
            WorkspaceKind::Build,
            WorkspaceKind::Profile,
            WorkspaceKind::Recovery,
        ] {
            view.workspace = workspace;
            let document = creator_workspace_document_with_view(&session, &view)
                .expect("domain workspace is valid");
            let frame =
                UiRuntime::new(document).reconcile(UiFrameInput::new(UiSize::new(1024.0, 720.0)));
            let rendered_text = frame
                .display_list
                .primitives
                .iter()
                .filter_map(|primitive| match primitive {
                    DisplayPrimitive::Text { text, .. }
                    | DisplayPrimitive::GlyphRun { text, .. } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(
                rendered_text
                    .iter()
                    .filter(|text| text.contains("Unique host activity"))
                    .count(),
                1,
                "{workspace:?} must keep host activity in the status line, not repeat it in the shelf"
            );
            assert!(
                rendered_text
                    .iter()
                    .any(|text| text.contains(domain_shelf_summary(workspace))),
                "{workspace:?} shelf must report its own source-backed fact"
            );
        }
    }

    #[test]
    #[allow(clippy::too_many_lines)] // The sparse-state geometry audit names each visible boundary.
    fn sparse_domain_states_are_optically_centred_in_their_work_surface() {
        let session = public_creator_session();
        let mut view =
            CreatorWorkspaceView::foundation(&session, "Ready for a bounded local build.");
        for workspace in [
            WorkspaceKind::Materials,
            WorkspaceKind::Build,
            WorkspaceKind::Profile,
            WorkspaceKind::Recovery,
        ] {
            view.workspace = workspace;
            let document = creator_workspace_document_with_view(&session, &view)
                .expect("sparse domain workspace is valid");
            let base = workspace_node_base(workspace);
            let stage = document
                .node(UiNodeId::new(base + 5))
                .expect("centred domain work stage");
            let state_card = document
                .node(UiNodeId::new(base + 91))
                .expect("centred domain state card");
            assert_eq!(
                stage.style.background,
                Some(UiColor::rgba(0.055, 0.063, 0.063, 1.0))
            );
            assert!(stage.style.border.is_none());
            assert!((stage.style.corner_radius - 10.0).abs() <= f32::EPSILON);
            assert_eq!(state_card.style.background, Some(UiColor::surface()));
            assert!(state_card.style.border.is_some());
            assert!((state_card.style.corner_radius - 14.0).abs() <= f32::EPSILON);
            if matches!(
                workspace,
                WorkspaceKind::Materials | WorkspaceKind::Profile | WorkspaceKind::Recovery
            ) {
                assert_eq!(
                    document
                        .node(UiNodeId::new(base + 52))
                        .and_then(|node| node.style.border),
                    None,
                    "{workspace:?} capability note must not render as a warning card"
                );
            }
            let frame = UiRuntime::new(document.clone())
                .reconcile(UiFrameInput::new(UiSize::new(1280.0, 800.0)));
            let rail = frame
                .layout
                .iter()
                .find(|entry| entry.node == UiNodeId::new(base + 3))
                .expect("sparse activity rail has layout")
                .bounds;
            let browser = frame
                .layout
                .iter()
                .find(|entry| entry.node == UiNodeId::new(base + 4))
                .expect("sparse browser has layout")
                .bounds;
            let inspector = frame
                .layout
                .iter()
                .find(|entry| entry.node == UiNodeId::new(base + 6))
                .expect("sparse inspector has layout")
                .bounds;
            let surface = frame
                .layout
                .iter()
                .find(|entry| entry.node == UiNodeId::new(base + 5))
                .expect("sparse work surface has layout")
                .bounds;
            for (name, bounds) in [
                ("rail", rail),
                ("browser", browser),
                ("surface", surface),
                ("inspector", inspector),
            ] {
                assert!(
                    bounds.size.height >= 500.0,
                    "{workspace:?} {name} must participate in the full-height workbench"
                );
                assert!(
                    (bounds.origin.y - rail.origin.y).abs() <= 0.1,
                    "{workspace:?} {name} must share the workbench's top edge"
                );
            }
            let state_region = frame
                .layout
                .iter()
                .find(|entry| entry.node == UiNodeId::new(base + 90))
                .expect("centred state region has layout")
                .bounds;
            let state_card = frame
                .layout
                .iter()
                .find(|entry| entry.node == UiNodeId::new(base + 91))
                .expect("centred state card has layout")
                .bounds;
            let region_center_x = state_region.origin.x + state_region.size.width / 2.0;
            let region_center_y = state_region.origin.y + state_region.size.height / 2.0;
            let card_center_x = state_card.origin.x + state_card.size.width / 2.0;
            let card_center_y = state_card.origin.y + state_card.size.height / 2.0;
            // The state card is allowed to use all of a narrow work surface,
            // then caps at its readable desktop width. Either way, the runtime
            // must centre it in the remaining work region rather than pinning
            // an oversized fixed card to the left edge.
            assert!(
                (region_center_x - card_center_x).abs() <= 0.5,
                "{workspace:?} state card is not horizontally centred"
            );
            assert!(
                (region_center_y - card_center_y).abs() <= 0.5,
                "{workspace:?} state card is not vertically centred"
            );
            let state_title = semantic_bounds(&frame, UiNodeId::new(base + 32));
            let state_detail = semantic_bounds(&frame, UiNodeId::new(base + 93));
            assert!(
                state_title.origin.y + state_title.size.height <= state_detail.origin.y,
                "{workspace:?} centred-state title must not overlap its detail"
            );
            if document.node(UiNodeId::new(base + 94)).is_some() {
                let actions = semantic_bounds(&frame, UiNodeId::new(base + 94));
                assert!(
                    state_detail.origin.y + state_detail.size.height <= actions.origin.y,
                    "{workspace:?} centred-state detail must not overlap its actions"
                );
            }
            assert_creator_controls_fit(
                creator_workspace_document_with_view(&session, &view)
                    .expect("centred sparse workspace still fits"),
                UiSize::new(1024.0, 720.0),
                1.0,
            );
        }
    }

    #[test]
    fn code_contextual_and_focused_layouts_keep_a_real_world_preview() {
        let session = public_creator_session();
        let mut view = CreatorWorkspaceView::foundation(&session, "Ready");
        view.workspace = WorkspaceKind::Code;
        view.code_context_width = CodeContextWidth::Wide;
        view.project_source = "{\n  \"schema\": \"meridian.creator-project/v1\"\n}".to_owned();
        let contextual = creator_workspace_document_with_view(&session, &view)
            .expect("contextual Code workspace is valid");
        assert!(contextual.node(CREATOR_WORLD_VIEWPORT_CANVAS).is_some());
        for (node, expected) in [
            (UiNodeId::new(95_003), UiElevation::Flat),
            (UiNodeId::new(95_004), UiElevation::Raised),
            (UiNodeId::new(95_005), UiElevation::Raised),
        ] {
            assert_eq!(
                contextual
                    .node(node)
                    .expect("contextual Code outer surface exists")
                    .presentation
                    .elevation,
                expected,
                "contextual Code keeps navigation quiet while raising its working surfaces"
            );
        }
        assert_creator_controls_fit(contextual.clone(), UiSize::new(1440.0, 900.0), 1.0);
        let frame =
            UiRuntime::new(contextual).reconcile(UiFrameInput::new(UiSize::new(1440.0, 900.0)));
        let contextual_source = frame
            .layout
            .iter()
            .find(|entry| entry.node == UiNodeId::new(95_005))
            .expect("contextual Code source has layout")
            .bounds;
        let contextual_rail = frame
            .layout
            .iter()
            .find(|entry| entry.node == UiNodeId::new(95_002))
            .expect("contextual Code rail has layout")
            .bounds;
        let contextual_viewport = frame
            .layout
            .iter()
            .find(|entry| entry.node == UiNodeId::new(95_004))
            .expect("contextual World viewport has layout")
            .bounds;
        assert!(
            (contextual_source.size.width - 560.0).abs() <= 0.1,
            "contextual source must retain its readable split width"
        );
        assert!(
            contextual_viewport.size.width >= 540.0,
            "contextual World viewport must remain useful beside Code"
        );
        assert!(
            (contextual_rail.origin.x - 8.0).abs() <= 0.1,
            "contextual Code must retain the shared workspace gutter"
        );
        let decorated = decorate_world_viewport(&session, &frame).expect("Code context decorates");
        assert!(decorated.display_list.primitives.iter().any(|primitive| {
            matches!(primitive, DisplayPrimitive::Path { node, .. } if *node == CREATOR_WORLD_VIEWPORT_CANVAS)
        }));

        view.focus_layout = true;
        let focused = creator_workspace_document_with_view(&session, &view)
            .expect("focused Code workspace is valid");
        assert!(focused.node(CREATOR_WORLD_VIEWPORT_CANVAS).is_some());
        assert_creator_controls_fit(focused.clone(), UiSize::new(1440.0, 900.0), 1.0);
        let focused_frame =
            UiRuntime::new(focused).reconcile(UiFrameInput::new(UiSize::new(1440.0, 900.0)));
        let focused_source = focused_frame
            .layout
            .iter()
            .find(|entry| entry.node == UiNodeId::new(95_005))
            .expect("focused Code source has layout")
            .bounds;
        assert!(
            focused_source.size.width > contextual_source.size.width,
            "focused Code must give source more room than contextual Code"
        );
    }

    #[test]
    fn compact_code_context_yields_browser_before_source_or_world_preview() {
        let session = public_creator_session();
        let mut view = CreatorWorkspaceView::foundation(&session, "Ready");
        view.workspace = WorkspaceKind::Code;
        view.code_context_width = CodeContextWidth::Compact;
        view.project_source = "{\n  \"schema\": \"meridian.creator-project/v1\"\n}".to_owned();

        let document = creator_workspace_document_with_view(&session, &view)
            .expect("compact Code workspace is valid");
        assert!(
            document.node(UiNodeId::new(95_003)).is_none(),
            "the lower-priority project browser must yield at compact widths"
        );
        assert_eq!(
            document
                .node(UiNodeId::new(95_020))
                .expect("compact Code keeps an inspection path in the activity rail")
                .semantics
                .action
                .as_deref(),
            Some("asset.inspect-source")
        );
        assert_creator_controls_fit(document.clone(), UiSize::new(1024.0, 720.0), 1.0);

        let frame =
            UiRuntime::new(document).reconcile(UiFrameInput::new(UiSize::new(1024.0, 720.0)));
        let rail = frame
            .layout
            .iter()
            .find(|entry| entry.node == UiNodeId::new(95_002))
            .expect("compact Code rail has layout")
            .bounds;
        let viewport = frame
            .layout
            .iter()
            .find(|entry| entry.node == UiNodeId::new(95_004))
            .expect("compact World context has layout")
            .bounds;
        let source = frame
            .layout
            .iter()
            .find(|entry| entry.node == UiNodeId::new(95_005))
            .expect("compact Code source has layout")
            .bounds;
        assert!((rail.origin.x - 8.0).abs() <= 0.1);
        assert!((source.size.width - 432.0).abs() <= 0.1);
        assert!(
            viewport.size.width >= 480.0,
            "the compact layout must preserve a useful live World context"
        );
    }

    #[test]
    fn code_pane_wraps_long_canonical_identifiers_without_changing_source() {
        let source = "{\n  \"stable_id\": \"152035140716559442984034638058563022253\"\n}";
        let display = code_pane_display_text(source, 32);

        assert_eq!(
            source,
            "{\n  \"stable_id\": \"152035140716559442984034638058563022253\"\n}"
        );
        assert!(display.lines().all(|line| line.chars().count() <= 32));
        assert!(display.lines().count() > source.lines().count());
        assert!(display.contains("stable_id"));
    }

    #[test]
    #[allow(clippy::too_many_lines)] // Source, frame, bounds, and preview assertions share one inspection journey.
    fn ui_authoring_exposes_the_authored_document_and_a_derived_frame_preview() {
        let session = public_creator_session();
        let mut view = CreatorWorkspaceView::foundation(&session, "UiDocument frame compiled.");
        view.workspace = WorkspaceKind::UiAuthoring;
        let inspection = inspect_creator_world_document(&session, &view)
            .expect("World UiDocument remains inspectable");
        let document = creator_workspace_document_with_view(&session, &view)
            .expect("UI authoring workspace is valid");
        assert!(document.node(CREATOR_UI_AUTHORING_PREVIEW_CANVAS).is_some());
        let base = workspace_node_base(WorkspaceKind::UiAuthoring);
        let preview_surface = document
            .node(UiNodeId::new(base + 5))
            .expect("UI compiled preview surface");
        let preview_canvas = document
            .node(CREATOR_UI_AUTHORING_PREVIEW_CANVAS)
            .expect("UI compiled preview canvas");
        assert!((preview_surface.style.corner_radius - 10.0).abs() <= f32::EPSILON);
        assert!(preview_surface.style.border.is_some());
        assert!((preview_canvas.style.corner_radius - 6.0).abs() <= f32::EPSILON);
        assert!(preview_canvas.style.border.is_some());
        assert_eq!(
            document
                .node(UiNodeId::new(base + 22))
                .and_then(|node| node.text.as_deref()),
            Some(format!("schema · ui-document/v{}", inspection.schema_version).as_str())
        );
        assert!(document
            .node(UiNodeId::new(base + 23))
            .and_then(|node| node.text.as_deref())
            .is_some_and(|text| {
                text.contains("Root")
                    && text.contains(&inspection.component_instance_count.to_string())
            }));
        assert!(document
            .node(UiNodeId::new(base + 31))
            .and_then(|node| node.text.as_deref())
            .is_some_and(|text| {
                text.contains(&inspection.source_nodes.to_string())
                    && text.contains(&inspection.display_primitives.to_string())
                    && text.contains(&inspection.semantic_nodes.to_string())
            }));
        assert!(document.nodes().any(|node| {
            node.text
                .as_deref()
                .is_some_and(|text| text.contains("Root") && text.contains("instances"))
        }));
        assert!(document.focus_order().iter().any(|id| {
            document
                .node(*id)
                .is_some_and(|node| node.semantics.name.contains("Responsive states"))
        }));
        assert!(document.nodes().any(|node| {
            node.semantics.name == "UI responsive inspection state"
                && node.text.as_deref()
                    == Some(
                        format!(
                            "Compact · {}/{}",
                            inspection.compact_display_primitives,
                            inspection.compact_semantic_nodes
                        )
                        .as_str(),
                    )
        }));
        let capability = document
            .node(UiNodeId::new(
                workspace_node_base(WorkspaceKind::UiAuthoring) + 74,
            ))
            .expect("UI authoring capability note exists");
        assert!(capability.style.border.is_none());
        assert_eq!(capability.layout_hints.preferred_height, Some(64.0));
        assert_creator_controls_fit(document.clone(), UiSize::new(1024.0, 720.0), 1.0);
        assert_creator_controls_fit(document.clone(), UiSize::new(1280.0, 800.0), 2.0);
        let frame = UiRuntime::new(document.clone())
            .reconcile(UiFrameInput::new(UiSize::new(1280.0, 800.0)));
        let bounds_for = |node: UiNodeId| {
            frame
                .layout
                .iter()
                .find(|entry| entry.node == node)
                .map(|entry| entry.bounds)
                .expect("UI authoring header node has layout")
        };
        let browser_header_row_bounds = bounds_for(UiNodeId::new(base + 16));
        let browser_title_bounds = bounds_for(UiNodeId::new(base + 20));
        let browser_mode_bounds = bounds_for(UiNodeId::new(base + 21));
        let browser_overflow_bounds = bounds_for(UiNodeId::new(base + 19));
        assert!(browser_title_bounds.origin.x >= browser_header_row_bounds.origin.x);
        assert!(
            browser_title_bounds.origin.x + browser_title_bounds.size.width
                <= browser_mode_bounds.origin.x + 0.1
        );
        assert!(
            browser_mode_bounds.origin.x + browser_mode_bounds.size.width
                <= browser_overflow_bounds.origin.x + 0.1
        );
        assert!(
            browser_overflow_bounds.origin.x + browser_overflow_bounds.size.width
                <= browser_header_row_bounds.origin.x + browser_header_row_bounds.size.width + 0.1
        );
        let inspector_header_row_bounds = bounds_for(UiNodeId::new(base + 46));
        let inspector_title_bounds = bounds_for(UiNodeId::new(base + 50));
        let inspector_mode_bounds = bounds_for(UiNodeId::new(base + 51));
        let inspector_overflow_bounds = bounds_for(UiNodeId::new(base + 49));
        assert!(
            inspector_title_bounds.origin.x + inspector_title_bounds.size.width
                <= inspector_mode_bounds.origin.x + 0.1
        );
        assert!(
            inspector_mode_bounds.origin.x + inspector_mode_bounds.size.width
                <= inspector_overflow_bounds.origin.x + 0.1
        );
        assert!(
            inspector_overflow_bounds.origin.x + inspector_overflow_bounds.size.width
                <= inspector_header_row_bounds.origin.x
                    + inspector_header_row_bounds.size.width
                    + 0.1
        );
        let rail = semantic_bounds(&frame, UiNodeId::new(base + 3));
        let center = semantic_bounds(&frame, UiNodeId::new(base + 8));
        for node in [base + 3, base + 4, base + 8, base + 6] {
            let bounds = frame
                .layout
                .iter()
                .find(|entry| entry.node == UiNodeId::new(node))
                .expect("UI authoring workbench surface has layout")
                .bounds;
            assert!(
                bounds.size.height >= 500.0,
                "UI authoring primary workbench surface {node} must remain full-height"
            );
            assert!(
                (bounds.origin.y - rail.origin.y).abs() <= 0.1,
                "UI authoring workbench surface {node} must share the visible workbench top edge"
            );
        }
        let preview_surface_bounds = semantic_bounds(&frame, UiNodeId::new(base + 5));
        assert!(
            preview_surface_bounds.size.height >= 300.0,
            "compiled preview must retain a dominant center canvas above the state shelf"
        );
        let state_controls_bounds = semantic_bounds(&frame, UiNodeId::new(base + 80));
        assert!((state_controls_bounds.size.height - 180.0).abs() <= 0.1);
        assert!(
            state_controls_bounds.origin.y > preview_surface_bounds.origin.y
                && state_controls_bounds.origin.y + state_controls_bounds.size.height
                    <= center.origin.y + center.size.height + 0.1,
            "responsive state inspection must sit below the compiled preview inside the center column"
        );
        let browser_bounds = frame
            .layout
            .iter()
            .find(|entry| entry.node == UiNodeId::new(base + 4))
            .expect("UI authoring browser has layout")
            .bounds;
        for node in [
            UiNodeId::new(base + 20),
            UiNodeId::new(base + 21),
            CREATOR_DOMAIN_SEARCH,
            UiNodeId::new(base + 22),
            UiNodeId::new(base + 23),
            UiNodeId::new(base + 24),
            UiNodeId::new(base + 25),
            UiNodeId::new(base + 26),
            UiNodeId::new(base + 27),
            UiNodeId::new(base + 28),
            UiNodeId::new(base + 65),
            UiNodeId::new(base + 66),
        ] {
            let bounds = frame
                .layout
                .iter()
                .find(|entry| entry.node == node)
                .expect("UI authoring browser child has layout")
                .bounds;
            assert!(bounds.origin.y >= browser_bounds.origin.y - 0.1);
            assert!(
                bounds.origin.y + bounds.size.height
                    <= browser_bounds.origin.y + browser_bounds.size.height + 0.1,
                "UI authoring browser child {node:?} escapes its panel"
            );
        }
        let target = creator_ui_authoring_target_frame(&session, &view)
            .expect("World authoring target is valid");
        let target = decorate_world_viewport(&session, &target)
            .expect("World authoring target decoration is valid");
        let projected_primitive_count = target
            .display_list
            .primitives
            .iter()
            .filter(|primitive| {
                matches!(
                    primitive,
                    DisplayPrimitive::Rect { .. }
                        | DisplayPrimitive::Border { .. }
                        | DisplayPrimitive::RoundedRect { .. }
                        | DisplayPrimitive::Path { .. }
                )
            })
            .count();
        assert!(projected_primitive_count > 0);
        let decorated =
            decorate_ui_authoring_preview(&frame, &target).expect("frame preview decorates");
        assert!(decorated.display_list.primitives.iter().any(|primitive| {
            matches!(
                primitive,
                DisplayPrimitive::Rect { node, .. }
                    | DisplayPrimitive::Border { node, .. }
                    | DisplayPrimitive::RoundedRect { node, .. }
                    | DisplayPrimitive::Path { node, .. }
                    if *node == CREATOR_UI_AUTHORING_PREVIEW_CANVAS
            )
        }));
        let review_frame = UiRuntime::new(
            creator_workspace_document_with_view(&session, &view)
                .expect("wide UI authoring workspace remains valid"),
        )
        .reconcile(UiFrameInput::new(UiSize::new(1600.0, 960.0)));
        let wide_preview = decorate_ui_authoring_preview(&review_frame, &target)
            .expect("wide frame preview decorates");
        assert!(
            wide_preview
                .display_list
                .primitives
                .iter()
                .any(|primitive| {
                    matches!(
                        primitive,
                        DisplayPrimitive::Text { node, text, .. }
                            if *node == CREATOR_UI_AUTHORING_PREVIEW_CANVAS && text == "World"
                    )
                }),
            "a readable inspection canvas projects real compiled labels at thumbnail scale"
        );

        let source_document = creator_world_workspace_document_with_view(&session, &view)
            .expect("World source remains available for the HiDPI preview");
        let mut hidpi_target_input = UiFrameInput::new(UiSize::new(1280.0, 800.0));
        hidpi_target_input.scale_factor = 2.0;
        let hidpi_target = UiRuntime::new(source_document).reconcile(hidpi_target_input);
        let hidpi_target = decorate_world_viewport(&session, &hidpi_target)
            .expect("HiDPI World source decoration is valid");
        let mut hidpi_review_input = UiFrameInput::new(UiSize::new(1600.0, 960.0));
        hidpi_review_input.scale_factor = 2.0;
        let hidpi_review_frame = UiRuntime::new(document.clone()).reconcile(hidpi_review_input);
        let hidpi_preview = decorate_ui_authoring_preview(&hidpi_review_frame, &hidpi_target)
            .expect("HiDPI compiled preview decorates");
        let one_x_glyph_width = wide_preview
            .display_list
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                DisplayPrimitive::Text {
                    node, text, raster, ..
                }
                | DisplayPrimitive::GlyphRun {
                    node, text, raster, ..
                } if *node == CREATOR_UI_AUTHORING_PREVIEW_CANVAS && text == "World" => {
                    raster.glyphs.first().map(|glyph| glyph.width)
                }
                _ => None,
            })
            .expect("1x preview retains a World glyph");
        let two_x_glyph_width = hidpi_preview
            .display_list
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                DisplayPrimitive::Text {
                    node, text, raster, ..
                }
                | DisplayPrimitive::GlyphRun {
                    node, text, raster, ..
                } if *node == CREATOR_UI_AUTHORING_PREVIEW_CANVAS && text == "World" => {
                    raster.glyphs.first().map(|glyph| glyph.width)
                }
                _ => None,
            })
            .expect("2x preview retains a World glyph");
        assert!(
            two_x_glyph_width >= one_x_glyph_width.saturating_mul(2).saturating_sub(1)
                && two_x_glyph_width <= one_x_glyph_width.saturating_mul(2).saturating_add(1),
            "projected glyph payload must track destination physical scale: 1x={one_x_glyph_width}, 2x={two_x_glyph_width}"
        );

        let mut compact_view = view.clone();
        compact_view.compact_ui_authoring = true;
        let compact_document = creator_workspace_document_with_view(&session, &compact_view)
            .expect("compact UI authoring workspace remains valid");
        let compact_frame = UiRuntime::new(compact_document)
            .reconcile(UiFrameInput::new(UiSize::new(1024.0, 720.0)));
        let compact_bounds_for = |node: UiNodeId| {
            compact_frame
                .layout
                .iter()
                .find(|entry| entry.node == node)
                .map(|entry| entry.bounds)
                .expect("compact UI authoring header node has layout")
        };
        let compact_browser_row = compact_bounds_for(UiNodeId::new(base + 16));
        let compact_browser_title = compact_bounds_for(UiNodeId::new(base + 20));
        let compact_browser_mode = compact_bounds_for(UiNodeId::new(base + 21));
        let compact_browser_overflow = compact_bounds_for(UiNodeId::new(base + 19));
        assert!(
            compact_browser_title.origin.x + compact_browser_title.size.width
                <= compact_browser_mode.origin.x + 0.1
        );
        assert!(
            compact_browser_mode.origin.x + compact_browser_mode.size.width
                <= compact_browser_overflow.origin.x + 0.1
        );
        assert!(
            compact_browser_overflow.origin.x + compact_browser_overflow.size.width
                <= compact_browser_row.origin.x + compact_browser_row.size.width + 0.1
        );
        let compact_preview = decorate_ui_authoring_preview(&compact_frame, &target)
            .expect("compact frame preview decorates");
        let compact_preview_text_count = compact_preview
            .display_list
            .primitives
            .iter()
            .filter(|primitive| {
                matches!(
                    primitive,
                    DisplayPrimitive::Text { node, .. } | DisplayPrimitive::GlyphRun { node, .. }
                        if *node == CREATOR_UI_AUTHORING_PREVIEW_CANVAS
                )
            })
            .count();
        assert!(
            compact_preview_text_count > 0,
            "a compact inspection canvas must retain readable compiled labels; projected text count={compact_preview_text_count}"
        );
        assert!(
            compact_preview
                .display_list
                .primitives
                .iter()
                .any(|primitive| {
                    matches!(
                        primitive,
                        DisplayPrimitive::Text { node, text, .. }
                            if *node == CREATOR_UI_AUTHORING_PREVIEW_CANVAS && text == "World"
                    )
                }),
            "a compact inspection canvas must retain the World label"
        );
    }

    #[test]
    fn modeler_workspace_projects_only_the_current_derived_model_preview() {
        let session = public_creator_session();
        let model = ModelDocument::from_json(include_str!(
            "../../../examples/creator-alpha/models/public-box.model.json"
        ))
        .expect("public model source");
        let object = model.objects.first().expect("public model object");
        let preview = model
            .penumbra_preview(object.id)
            .expect("derived public preview");
        let mut view = CreatorWorkspaceView::foundation(&session, "Model preview refreshed.");
        view.workspace = WorkspaceKind::Modeler;
        view.modeler = Some(CreatorModelerPresentation {
            generation: model.document_generation,
            document_label: model.label.clone(),
            object_label: object.label.clone(),
            object_count: model.objects.len(),
            vertex_count: object.vertices.len(),
            edge_count: object.edges.len(),
            face_count: object.faces.len(),
            preview: Some(preview),
        });
        let document = creator_workspace_document_with_view(&session, &view)
            .expect("Modeler workspace is valid");
        assert!(document.node(CREATOR_MODELER_PREVIEW_CANVAS).is_some());
        let base = workspace_node_base(WorkspaceKind::Modeler);
        let preview_surface = document
            .node(UiNodeId::new(base + 5))
            .expect("Modeler preview surface");
        let preview_canvas = document
            .node(CREATOR_MODELER_PREVIEW_CANVAS)
            .expect("Modeler preview canvas");
        assert!((preview_surface.style.corner_radius - 10.0).abs() <= f32::EPSILON);
        assert!(preview_surface.style.border.is_some());
        assert!((preview_canvas.style.corner_radius - 6.0).abs() <= f32::EPSILON);
        assert!(preview_canvas.style.border.is_none());
        let capability = document
            .node(UiNodeId::new(base + 55))
            .expect("Modeler capability note exists");
        assert!(capability.style.border.is_none());
        assert_eq!(capability.layout_hints.preferred_height, Some(64.0));
        assert_creator_controls_fit(document.clone(), UiSize::new(1280.0, 800.0), 1.0);
        let frame =
            UiRuntime::new(document).reconcile(UiFrameInput::new(UiSize::new(1280.0, 800.0)));
        let inspector = frame
            .layout
            .iter()
            .find(|entry| {
                entry.node == UiNodeId::new(workspace_node_base(WorkspaceKind::Modeler) + 6)
            })
            .expect("Modeler inspector has layout")
            .bounds;
        let rail = semantic_bounds(
            &frame,
            UiNodeId::new(workspace_node_base(WorkspaceKind::Modeler) + 3),
        );
        assert!(
            inspector.size.height >= 500.0,
            "Modeler inspector must share the full-height workbench"
        );
        assert!(
            (inspector.origin.y - rail.origin.y).abs() <= 0.1,
            "Modeler inspector must share the visible workbench top edge"
        );
        let decorated = decorate_modeler_preview(view.modeler.as_ref(), &frame)
            .expect("model preview decorates");
        assert!(decorated.display_list.primitives.iter().any(|primitive| {
            matches!(primitive, DisplayPrimitive::Path { node, .. }
                if *node == CREATOR_MODELER_PREVIEW_CANVAS)
        }));
    }

    #[test]
    fn all_declared_project_workspaces_keep_their_shell_rows_and_accessible_controls() {
        let session = public_creator_session();
        let mut view = CreatorWorkspaceView::foundation(&session, "Ready");
        for workspace in [
            WorkspaceKind::Code,
            WorkspaceKind::Modeler,
            WorkspaceKind::UiAuthoring,
            WorkspaceKind::Materials,
            WorkspaceKind::Alluvium,
            WorkspaceKind::Build,
            WorkspaceKind::Profile,
            WorkspaceKind::Settings,
            WorkspaceKind::Recovery,
        ] {
            view.workspace = workspace;
            let document = creator_workspace_document_with_view(&session, &view)
                .expect("declared workspace is valid");
            assert_creator_controls_fit(document.clone(), UiSize::new(1024.0, 720.0), 1.0);
            assert_creator_controls_fit(document, UiSize::new(1440.0, 900.0), 2.0);
        }
    }

    #[test]
    fn creator_text_stays_within_its_authored_slots_at_normal_and_hidpi_scales() {
        let session = public_creator_session();
        let settings = CreatorSettingsView {
            project: None,
            play_active: false,
            high_contrast: false,
            reduced_motion: false,
            density: "Standard".to_owned(),
            query: String::new(),
            status: "Preferences are local to Meridian.".to_owned(),
        };
        let mut documents = vec![
            creator_hub_document(&[], "Choose a project.").expect("authored hub"),
            creator_settings_document(&settings).expect("authored settings"),
        ];
        let mut view = CreatorWorkspaceView::foundation(&session, "Ready");
        for workspace in [
            WorkspaceKind::World,
            WorkspaceKind::Code,
            WorkspaceKind::Modeler,
            WorkspaceKind::UiAuthoring,
            WorkspaceKind::Materials,
            WorkspaceKind::Alluvium,
            WorkspaceKind::Build,
            WorkspaceKind::Profile,
            WorkspaceKind::Recovery,
        ] {
            view.workspace = workspace;
            documents.push(
                creator_workspace_document_with_view(&session, &view)
                    .expect("authored Creator workspace"),
            );
        }
        for document in documents {
            assert_creator_text_fits(document.clone(), UiSize::new(1024.0, 720.0), 1.0);
            assert_creator_text_fits(document, UiSize::new(1440.0, 900.0), 2.0);
        }
    }

    #[test]
    fn creator_source_trees_use_semantic_state_instead_of_faux_text_icons() {
        let session = public_creator_session();
        let mut view = CreatorWorkspaceView::foundation(&session, "Ready");
        for workspace in [
            WorkspaceKind::World,
            WorkspaceKind::Modeler,
            WorkspaceKind::UiAuthoring,
        ] {
            view.workspace = workspace;
            let document = creator_workspace_document_with_view(&session, &view)
                .expect("source workspace is valid");
            let tree_rows = document
                .nodes()
                .filter(|node| node.kind == UiWidgetKind::TreeItem)
                .collect::<Vec<_>>();
            assert!(
                tree_rows.iter().any(|node| node.semantics.state.expanded),
                "{workspace:?} retains explicit tree expansion state"
            );
            assert!(tree_rows.iter().all(|node| {
                node.text.as_deref().is_none_or(|text| {
                    !text.contains('▾') && !text.contains('▸') && !text.contains('◇')
                })
            }));
            let groups = tree_rows
                .iter()
                .filter(|node| node.icon.is_some())
                .collect::<Vec<_>>();
            assert!(
                !groups.is_empty(),
                "{workspace:?} uses native disclosure vectors for tree groups"
            );
            assert!(groups.iter().all(|node| {
                matches!(node.icon, Some(IconId::ChevronDown | IconId::ChevronRight))
                    && (node.style.padding - 4.0).abs() <= f32::EPSILON
            }));
            assert!(tree_rows
                .iter()
                .filter(|node| node.icon.is_none())
                .all(|node| {
                    !node.semantics.state.expanded
                        && (node.style.padding - 4.0).abs() <= f32::EPSILON
                }));
            for scale_factor in [1.0, 2.0] {
                let mut input = UiFrameInput::new(UiSize::new(1280.0, 800.0));
                input.scale_factor = scale_factor;
                let frame = UiRuntime::new(document.clone()).reconcile(input);
                assert!(groups.iter().all(|group| {
                    frame.display_list.primitives.iter().any(|primitive| {
                        matches!(primitive, DisplayPrimitive::Path { node, .. } if *node == group.id)
                    })
                }));
                for row in tree_rows.iter().filter(|node| node.icon.is_none()) {
                    let row_bounds = semantic_bounds(&frame, row.id);
                    let text_bounds = frame
                        .display_list
                        .primitives
                        .iter()
                        .find_map(|primitive| match primitive {
                            DisplayPrimitive::Text { node, bounds, .. } if *node == row.id => {
                                Some(*bounds)
                            }
                            _ => None,
                        })
                        .expect("tree leaf emits its text");
                    assert!(
                        text_bounds.origin.y >= row_bounds.origin.y - 0.1
                            && text_bounds.origin.y + text_bounds.size.height
                                <= row_bounds.origin.y + row_bounds.size.height + 0.1,
                        "{} must fit inside its dense tree row at {scale_factor}×",
                        row.semantics.name
                    );
                    assert!(
                    text_bounds.origin.x >= row_bounds.origin.x + 27.9,
                        "{} reserves the native branch disclosure column without oversized padding at {scale_factor}×",
                        row.semantics.name
                    );
                }
            }
        }
    }

    #[test]
    fn creator_source_browsers_keep_tree_rows_crisp_instead_of_card_like() {
        let session = public_creator_session();
        let mut view = CreatorWorkspaceView::foundation(&session, "Ready");
        for workspace in [
            WorkspaceKind::World,
            WorkspaceKind::Code,
            WorkspaceKind::Modeler,
            WorkspaceKind::UiAuthoring,
            WorkspaceKind::Materials,
            WorkspaceKind::Alluvium,
            WorkspaceKind::Build,
        ] {
            view.workspace = workspace;
            let document = creator_workspace_document_with_view(&session, &view)
                .expect("source workspace is valid");
            assert!(document
                .nodes()
                .filter(|node| node.kind == UiWidgetKind::TreeItem)
                .all(|node| {
                    node.style.background != Some(UiColor::surface())
                        && node.style.corner_radius <= 4.0
                },));
        }
    }

    #[test]
    fn creator_activity_rails_keep_full_icon_slots_inside_their_compact_surface() {
        let session = public_creator_session();
        let mut view = CreatorWorkspaceView::foundation(&session, "Ready");
        for workspace in [
            WorkspaceKind::World,
            WorkspaceKind::Code,
            WorkspaceKind::Modeler,
            WorkspaceKind::UiAuthoring,
            WorkspaceKind::Materials,
        ] {
            view.workspace = workspace;
            let document = creator_workspace_document_with_view(&session, &view)
                .expect("workspace with activity rail is valid");
            let rail = document
                .nodes()
                .find(|node| node.semantics.name.ends_with("activity rail"))
                .expect("each workbench exposes one activity rail");
            assert!((rail.style.padding - 4.0).abs() <= f32::EPSILON);
            assert_eq!(rail.style.background, Some(UiColor::surface()));
            assert!((rail.style.corner_radius - 10.0).abs() <= f32::EPSILON);
            let rail_id = rail.id;
            let rail_children = rail.children.clone();

            let frame =
                UiRuntime::new(document).reconcile(UiFrameInput::new(UiSize::new(1440.0, 900.0)));
            let rail_bounds = semantic_bounds(&frame, rail_id);
            assert!((rail_bounds.size.width - 44.0).abs() <= 0.1);
            for child in &rail_children {
                let child_bounds = semantic_bounds(&frame, *child);
                assert!(
                    child_bounds.size.width >= 35.9,
                    "{workspace:?} activity control must retain a full 36 px icon slot"
                );
            }
        }
    }

    #[test]
    fn creator_text_buttons_are_optically_centered_at_normal_and_hidpi_scales() {
        let session = public_creator_session();
        let settings = CreatorSettingsView {
            project: None,
            play_active: false,
            high_contrast: false,
            reduced_motion: false,
            density: "Standard".to_owned(),
            query: String::new(),
            status: "Preferences are local to Meridian.".to_owned(),
        };
        let mut documents = vec![
            creator_hub_document(&[], "Ready").expect("authored hub"),
            creator_settings_document(&settings).expect("authored settings"),
        ];
        let mut view = CreatorWorkspaceView::foundation(&session, "Ready");
        for workspace in [
            WorkspaceKind::World,
            WorkspaceKind::Code,
            WorkspaceKind::Modeler,
            WorkspaceKind::UiAuthoring,
            WorkspaceKind::Materials,
            WorkspaceKind::Alluvium,
            WorkspaceKind::Build,
            WorkspaceKind::Profile,
            WorkspaceKind::Recovery,
        ] {
            view.workspace = workspace;
            documents.push(
                creator_workspace_document_with_view(&session, &view)
                    .expect("authored Creator workspace"),
            );
        }
        for document in documents {
            for scale_factor in [1.0, 2.0] {
                let mut input = UiFrameInput::new(UiSize::new(1440.0, 900.0));
                input.scale_factor = scale_factor;
                let frame = UiRuntime::new(document.clone()).reconcile(input);
                for primitive in &frame.display_list.primitives {
                    let DisplayPrimitive::Text { node, bounds, .. } = primitive else {
                        continue;
                    };
                    let Some(control) = document.node(*node) else {
                        continue;
                    };
                    if control.icon.is_some()
                        || !matches!(
                            control.kind,
                            UiWidgetKind::Button
                                | UiWidgetKind::Toggle
                                | UiWidgetKind::Progress
                                | UiWidgetKind::ComboOption
                                | UiWidgetKind::MenuItem
                                | UiWidgetKind::Tab
                        )
                    {
                        continue;
                    }
                    let control_bounds = semantic_bounds(&frame, *node);
                    let control_center = control_bounds.origin.x + control_bounds.size.width * 0.5;
                    let text_center = bounds.origin.x + bounds.size.width * 0.5;
                    assert!(
                        (text_center - control_center).abs() <= 0.6,
                        "{} control text must stay horizontally centred at {scale_factor}×",
                        control.semantics.name
                    );
                }
            }
        }
    }

    #[test]
    fn creator_surfaces_compile_from_recoverable_authored_source() {
        let settings = CreatorSettingsView {
            project: None,
            play_active: false,
            high_contrast: false,
            reduced_motion: false,
            density: "Standard".to_owned(),
            query: String::new(),
            status: "Preferences are local to Meridian.".to_owned(),
        };
        assert_creator_authored_source_round_trips(
            creator_hub_document(&[], "Choose a project.").expect("authored hub"),
        );
        assert_creator_authored_source_round_trips(
            creator_settings_document(&settings).expect("authored settings"),
        );

        let session = public_creator_session();
        let mut view = CreatorWorkspaceView::foundation(&session, "Ready");
        for workspace in [
            WorkspaceKind::World,
            WorkspaceKind::Code,
            WorkspaceKind::Modeler,
            WorkspaceKind::UiAuthoring,
            WorkspaceKind::Materials,
            WorkspaceKind::Alluvium,
            WorkspaceKind::Build,
            WorkspaceKind::Profile,
            WorkspaceKind::Recovery,
        ] {
            view.workspace = workspace;
            assert_creator_authored_source_round_trips(
                creator_workspace_document_with_view(&session, &view)
                    .expect("authored Creator workspace"),
            );
        }
    }

    #[test]
    #[allow(clippy::too_many_lines)] // Preference semantics and full-height workbench geometry share one rendered contract.
    fn settings_surface_has_typed_preference_controls_without_project_authority() {
        let view = CreatorSettingsView {
            project: None,
            play_active: false,
            high_contrast: true,
            reduced_motion: true,
            density: "Comfortable".to_owned(),
            query: String::new(),
            status: "Preferences saved locally.".to_owned(),
        };
        let document = creator_settings_document(&view).expect("settings document is valid");
        assert!(
            document.node(SHELL_WORKSPACE_ROW).is_some(),
            "Settings retains the permanent workspace row without falsely selecting a project workspace"
        );
        for action in [
            "settings.toggle-high-contrast",
            "settings.toggle-reduced-motion",
            "settings.density-compact",
            "settings.density-standard",
            "settings.density-comfortable",
            "settings.reset-preferences",
            "settings.return",
        ] {
            assert!(document.focus_order().iter().any(|id| {
                document
                    .node(*id)
                    .and_then(|node| node.semantics.action.as_deref())
                    == Some(action)
            }));
        }
        for toggle in [UiNodeId::new(90_110), UiNodeId::new(90_111)] {
            assert_eq!(
                document.node(toggle).expect("settings toggle exists").kind,
                UiWidgetKind::Toggle,
                "settings uses a typed toggle instead of a text-only button"
            );
        }
        let surface_style = &document
            .node(UiNodeId::new(90_103))
            .expect("settings work surface")
            .style;
        assert!((surface_style.corner_radius - 10.0).abs() <= f32::EPSILON);
        assert!(surface_style.border.is_some());
        assert_creator_controls_fit(document, UiSize::new(1024.0, 720.0), 1.0);

        let narrow_frame =
            UiRuntime::new(creator_settings_document(&view).expect("narrow settings rerender"))
                .reconcile(UiFrameInput::new(UiSize::new(1024.0, 720.0)));
        let application_row = semantic_bounds(&narrow_frame, SHELL_APPLICATION_ROW);
        let workspace_row = semantic_bounds(&narrow_frame, SHELL_WORKSPACE_ROW);
        let narrow_main = semantic_bounds(&narrow_frame, UiNodeId::new(90_101));
        assert!((application_row.origin.y - 0.0).abs() <= 0.1);
        assert!((application_row.size.height - 44.0).abs() <= 0.1);
        assert!((workspace_row.origin.y - 44.0).abs() <= 0.1);
        assert!((workspace_row.size.height - 36.0).abs() <= 0.1);
        assert!((narrow_main.origin.y - 80.0).abs() <= 0.1);

        let frame = UiRuntime::new(creator_settings_document(&view).expect("settings rerender"))
            .reconcile(UiFrameInput::new(UiSize::new(1280.0, 800.0)));
        let main = frame
            .layout
            .iter()
            .find(|entry| entry.node == UiNodeId::new(90_101))
            .expect("settings main region has layout")
            .bounds;
        let navigation = semantic_bounds(&frame, UiNodeId::new(90_102));
        let surface = semantic_bounds(&frame, UiNodeId::new(90_103));
        let inspector = semantic_bounds(&frame, UiNodeId::new(90_104));
        let actions = semantic_bounds(&frame, UiNodeId::new(90_121));
        let high_contrast_toggle = semantic_bounds(&frame, UiNodeId::new(90_110));
        let high_contrast_detail = semantic_bounds(&frame, UiNodeId::new(90_132));
        assert!((navigation.origin.x - (main.origin.x + 8.0)).abs() <= 0.1);
        assert!((navigation.origin.y - (main.origin.y + 8.0)).abs() <= 0.1);
        assert!(
            ((inspector.origin.x + inspector.size.width) - (main.origin.x + main.size.width - 8.0))
                .abs()
                <= 0.1,
            "Settings inspector must retain the right-side dock gutter"
        );
        for (name, bounds) in [
            ("navigation", navigation),
            ("surface", surface),
            ("inspector", inspector),
        ] {
            assert!(
                bounds.size.height >= 500.0,
                "Settings {name} must participate in the full-height workbench"
            );
            assert!(
                (bounds.origin.y - (main.origin.y + 8.0)).abs() <= 0.1,
                "Settings {name} must begin after the workbench's dock gutter"
            );
        }
        assert!(
            ((actions.origin.x + actions.size.width / 2.0)
                - (surface.origin.x + surface.size.width / 2.0))
                .abs()
                <= 0.5,
            "Settings controls must stay centred in the working surface"
        );
        assert!(
            ((actions.origin.y + actions.size.height / 2.0)
                - (surface.origin.y + surface.size.height / 2.0))
                .abs()
                <= 0.5,
            "Settings controls must stay vertically centred in the working surface"
        );
        assert!((high_contrast_toggle.size.width - 84.0).abs() <= 0.1);
        assert!(
            high_contrast_detail.origin.x + high_contrast_detail.size.width
                <= high_contrast_toggle.origin.x - 11.9,
            "settings preference copy must not collide with its toggle"
        );

        let motion_only = CreatorSettingsView {
            query: "motion".to_owned(),
            ..view
        };
        let document = creator_settings_document(&motion_only).expect("filtered settings valid");
        assert!(document.node(CREATOR_SETTINGS_SEARCH).is_some());
        assert!(document.focus_order().iter().any(|id| {
            document
                .node(*id)
                .and_then(|node| node.semantics.action.as_deref())
                == Some("settings.toggle-reduced-motion")
        }));
        assert!(!document.focus_order().iter().any(|id| {
            document
                .node(*id)
                .and_then(|node| node.semantics.action.as_deref())
                == Some("settings.toggle-high-contrast")
        }));
    }
}
