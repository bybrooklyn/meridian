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
    DisplayList, DisplayListError, DisplayPrimitive, UiAbsolutePosition, UiBorder, UiColor,
    UiConstraints, UiDocument, UiDocumentError, UiFrameOutput, UiLayout, UiLayoutHints, UiNode,
    UiNodeId, UiPathCommand, UiPoint, UiRect, UiSize, UiStroke, UiStyle, UiTextInputOptions,
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
/// Stable browser search field for the World workspace.
pub const CREATOR_WORLD_SEARCH: UiNodeId = UiNodeId::new(92_050);

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
        }
    }
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

fn shell_row_style(background: UiColor, padding: f32) -> UiStyle {
    UiStyle {
        background: Some(background),
        border: Some(UiBorder {
            color: UiColor::border(),
            width: 1,
        }),
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
        font_size: 15.0,
    }
}

fn shell_utility_style(strong: bool, active: bool) -> UiStyle {
    UiStyle {
        background: active.then_some(UiColor::surface()),
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
        padding: 5.0,
        font_size: 12.0,
    }
}

fn workspace_tab_style(selected: bool) -> UiStyle {
    UiStyle {
        background: selected.then_some(UiColor::surface()),
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
        padding: 4.0,
        font_size: 12.0,
    }
}

fn world_panel_style(radius: f32) -> UiStyle {
    UiStyle {
        background: Some(UiColor::surface()),
        border: Some(UiBorder {
            color: UiColor::border(),
            width: 1,
        }),
        corner_radius: radius,
        foreground: UiColor::foreground(),
        padding: 10.0,
        font_size: 14.0,
    }
}

fn world_canvas_style() -> UiStyle {
    UiStyle {
        background: Some(UiColor::background()),
        border: Some(UiBorder {
            color: UiColor::border(),
            width: 1,
        }),
        corner_radius: 6.0,
        foreground: UiColor::secondary_text(),
        padding: 0.0,
        font_size: 12.0,
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
        font_size: 12.0,
    }
}

fn status_row_style() -> UiStyle {
    UiStyle {
        background: Some(UiColor::surface()),
        border: Some(UiBorder {
            color: UiColor::border(),
            width: 1,
        }),
        corner_radius: 0.0,
        foreground: UiColor::secondary_text(),
        padding: 0.0,
        font_size: 11.0,
    }
}

fn canvas_overlay_style(accent: UiColor) -> UiStyle {
    UiStyle {
        background: Some(UiColor::rgba(0.035, 0.043, 0.043, 0.92)),
        border: Some(UiBorder {
            color: accent,
            width: 1,
        }),
        corner_radius: 6.0,
        foreground: UiColor::secondary_text(),
        padding: 6.0,
        font_size: 11.0,
    }
}

const SHELL_APPLICATION_ROW: UiNodeId = UiNodeId::new(92_000);
const SHELL_WORKSPACE_ROW: UiNodeId = UiNodeId::new(92_020);
const SHELL_STATUS_ROW: UiNodeId = UiNodeId::new(92_040);

fn push_application_row(
    nodes: &mut Vec<UiNode>,
    project_label: &str,
    project_open: bool,
    play_active: bool,
) -> UiNodeId {
    let brand = UiNodeId::new(92_001);
    let spacer = UiNodeId::new(92_002);
    let play = UiNodeId::new(if play_active { 92_007 } else { 92_003 });
    let build = UiNodeId::new(92_004);
    let search = UiNodeId::new(92_005);
    let settings = UiNodeId::new(92_006);
    nodes.push(fixed_width(
        UiNode::button(
            brand,
            "Return to Meridian projects",
            "editor.return-hub",
            format!("Meridian · {project_label}"),
        )
        .with_style(shell_brand_style()),
        320.0,
    ));
    nodes.push(transparent_group(
        spacer,
        "Application command spacer",
        UiLayout::Overlay,
        Vec::new(),
    ));
    nodes.push(fixed_width(
        UiNode::button(
            play,
            if play_active {
                "Stop Play"
            } else {
                "Start Play"
            },
            if project_open {
                if play_active {
                    "editor.play-discard"
                } else {
                    "editor.play-start"
                }
            } else {
                "shell.play-unavailable"
            },
            if play_active { "Stop" } else { "Play" },
        )
        .with_style(shell_utility_style(true, play_active)),
        72.0,
    ));
    nodes.push(fixed_width(
        UiNode::button(
            build,
            "Build project",
            if project_open {
                "build.submit"
            } else {
                "shell.build-unavailable"
            },
            "Build",
        )
        .with_style(shell_utility_style(false, false)),
        72.0,
    ));
    nodes.push(fixed_width(
        UiNode::button(search, "Search Meridian", "shell.search", "Search")
            .with_style(shell_utility_style(false, false)),
        84.0,
    ));
    nodes.push(fixed_width(
        UiNode::button(
            settings,
            "Open Meridian settings",
            "shell.settings",
            "Settings",
        )
        .with_style(shell_utility_style(false, false)),
        88.0,
    ));
    nodes.push(fixed_height(
        UiNode::container(
            SHELL_APPLICATION_ROW,
            "Meridian application commands",
            UiLayout::HorizontalStack { gap: 8.0 },
            vec![brand, spacer, play, build, search, settings],
        )
        .with_style(shell_row_style(UiColor::background(), 6.0)),
        44.0,
    ));
    SHELL_APPLICATION_ROW
}

fn push_workspace_row(nodes: &mut Vec<UiNode>, selected_world: bool) -> UiNodeId {
    let workspaces = [
        (92_021, "World", "workspace.world", 76.0),
        (92_022, "Modeler", "workspace.modeler", 88.0),
        (92_023, "UI", "workspace.ui", 54.0),
        (92_024, "Code", "workspace.code", 66.0),
        (92_025, "Materials", "workspace.materials", 92.0),
        (92_026, "Alluvium", "workspace.alluvium", 94.0),
        (92_027, "Build", "workspace.build", 66.0),
        (92_028, "Profile", "workspace.profile", 76.0),
    ];
    let mut tabs = Vec::new();
    for (id, label, action, width) in workspaces {
        let id = UiNodeId::new(id);
        let selected = selected_world && label == "World";
        nodes.push(fixed_width(
            UiNode::tab(id, format!("{label} workspace"), action, selected)
                .with_style(workspace_tab_style(selected)),
            width,
        ));
        tabs.push(id);
    }
    nodes.push(fixed_height(
        UiNode::tabs(SHELL_WORKSPACE_ROW, "Meridian workspaces", tabs)
            .with_style(shell_row_style(UiColor::surface(), 4.0)),
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
        .with_style(status_row_style()),
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

fn creator_meta_style() -> UiStyle {
    UiStyle {
        background: None,
        border: None,
        corner_radius: 0.0,
        foreground: UiColor::secondary_text(),
        padding: 0.0,
        font_size: 12.0,
    }
}

fn creator_panel_accent(panel: EditorPanelId) -> UiColor {
    match panel {
        EditorPanelId::ProjectRecovery | EditorPanelId::Recipe => UiColor::amber(),
        EditorPanelId::Viewport | EditorPanelId::Build | EditorPanelId::Modeler => UiColor::grass(),
        EditorPanelId::Hierarchy
        | EditorPanelId::Assets
        | EditorPanelId::Inspector
        | EditorPanelId::History => UiColor::secondary_text(),
        EditorPanelId::Diagnostics => UiColor::red(),
    }
}

fn creator_hub_status_style() -> UiStyle {
    UiStyle {
        background: Some(UiColor::background()),
        border: Some(UiBorder {
            color: UiColor::border(),
            width: 1,
        }),
        corner_radius: 6.0,
        foreground: UiColor::secondary_text(),
        padding: 8.0,
        font_size: 12.0,
    }
}

fn creator_hub_field_label_style() -> UiStyle {
    UiStyle {
        background: None,
        border: None,
        corner_radius: 0.0,
        foreground: UiColor::muted_text(),
        padding: 0.0,
        font_size: 11.0,
    }
}

fn creator_recent_row_style(available: bool) -> UiStyle {
    UiStyle {
        background: Some(if available {
            UiColor::background()
        } else {
            UiColor::surface()
        }),
        border: Some(UiBorder {
            color: if available {
                UiColor::border()
            } else {
                UiColor::red()
            },
            width: 1,
        }),
        corner_radius: 10.0,
        foreground: UiColor::foreground(),
        padding: 10.0,
        font_size: 16.0,
    }
}

fn creator_compact_action_style(panel: EditorPanelId, command: &str) -> UiStyle {
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
    let accent = creator_panel_accent(panel);
    UiStyle {
        background: Some(if primary {
            UiColor::surface()
        } else {
            UiColor::background()
        }),
        border: Some(UiBorder {
            color: if primary { accent } else { UiColor::border() },
            width: 1,
        }),
        corner_radius: 4.0,
        foreground: if primary {
            UiColor::text()
        } else {
            UiColor::secondary_text()
        },
        padding: 4.0,
        font_size: 12.0,
    }
}

fn bounded_text(value: &str, maximum_chars: usize) -> String {
    let mut text = value.chars().take(maximum_chars).collect::<String>();
    if value.chars().nth(maximum_chars).is_some() {
        text.push_str("...");
    }
    text
}

fn creator_action_label(command: &str) -> &'static str {
    match command {
        "editor.recover" => "Recover session",
        "editor.return-hub" => "Back to projects",
        "editor.play-start" => "Start Play",
        "editor.play-apply" => "Apply Play",
        "editor.play-discard" => "Discard Play",
        "editor.focus-selection" => "Focus selection",
        "editor.select-placement" => "Select placement",
        "editor.preview-command" => "Preview change",
        "editor.edit-placement" => "Save placement",
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
            UiNode::button(id, format!("{name}: {label}"), command, label)
                .with_style(creator_compact_action_style(panel, command)),
        );
        actions.push(id);
    }
    nodes.push(fixed_height(
        transparent_group(
            group,
            format!("{name} actions"),
            UiLayout::Grid { columns, gap: 4.0 },
            actions,
        ),
        height,
    ));
    group
}

fn selected_placement_summary(session: &EditorSession) -> Option<String> {
    let placement = session
        .selection()
        .ids
        .iter()
        .find_map(|id| session.document().placements.get(id))
        .or_else(|| session.document().placements.values().next())?;
    Some(format!(
        "{} at X {} mm, Y {} mm, Z {} mm.",
        bounded_text(&placement.label, 72),
        placement.translation.x_mm,
        placement.translation.y_mm,
        placement.translation.z_mm
    ))
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
    let create = UiNodeId::new(90_003);
    let open = UiNodeId::new(90_004);
    let project_name_label = UiNodeId::new(90_017);
    let status_id = UiNodeId::new(90_001);
    let recents_title = UiNodeId::new(90_014);
    let recents_list = UiNodeId::new(90_015);
    let mut nodes = Vec::new();

    let application_row = push_application_row(&mut nodes, "Projects", false, false);
    let workspace_row = push_workspace_row(&mut nodes, false);

    nodes.push(fixed_height(
        UiNode::label(
            hero,
            "Meridian project hub",
            "Create something worth keeping.",
        )
        .with_style(UiStyle::heading()),
        44.0,
    ));
    nodes.push(fixed_height(
        UiNode::label(
            description,
            "Meridian project model",
            "Projects are local, source-authoritative, and recoverable.",
        )
        .with_style(creator_meta_style()),
        24.0,
    ));
    nodes.push(fixed_size(
        UiNode::button(
            create,
            "Create a Meridian project",
            "hub.create-project",
            "Create · Start a new Meridian project",
        )
        .with_style(UiStyle {
            background: Some(UiColor::surface()),
            border: Some(UiBorder {
                color: UiColor::grass(),
                width: 1,
            }),
            corner_radius: 14.0,
            foreground: UiColor::text(),
            padding: 16.0,
            font_size: 16.0,
        }),
        380.0,
        68.0,
    ));
    nodes.push(fixed_size(
        UiNode::button(
            open,
            "Open a Meridian project",
            "hub.open-project",
            "Open · Choose an existing project",
        )
        .with_style(UiStyle {
            background: Some(UiColor::surface()),
            border: Some(UiBorder {
                color: UiColor::amber(),
                width: 1,
            }),
            corner_radius: 14.0,
            foreground: UiColor::text(),
            padding: 16.0,
            font_size: 16.0,
        }),
        380.0,
        68.0,
    ));
    nodes.push(fixed_height(
        transparent_group(
            action_row,
            "Project creation and open actions",
            UiLayout::HorizontalStack { gap: 12.0 },
            vec![create, open],
        ),
        68.0,
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
        44.0,
    ));
    nodes.push(fixed_height(
        UiNode::toast(
            status_id,
            "Meridian project status",
            bounded_text(status, 220),
        )
        .with_style(creator_hub_status_style()),
        34.0,
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
            .with_style(if recent.available {
                creator_meta_style()
            } else {
                UiStyle {
                    foreground: UiColor::red_hover(),
                    ..creator_meta_style()
                }
            }),
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
            54.0,
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
    let recent_height = recent_count * 54.0 + recent_gap_count * 8.0;
    nodes.push(fixed_height(
        UiNode::virtual_list(recents_list, "Recent projects list", recent_rows)
            .with_style(UiStyle::transparent()),
        recent_height,
    ));

    nodes.push(
        UiNode::container(
            content,
            "Meridian project hub content",
            UiLayout::VerticalStack { gap: 10.0 },
            vec![
                hero,
                description,
                action_row,
                project_name_label,
                CREATOR_HUB_PROJECT_NAME,
                status_id,
                recents_title,
                recents_list,
            ],
        )
        .with_style(UiStyle::transparent())
        .with_layout_hints(UiLayoutHints::fixed_width(780.0)),
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
    let center_height = 286.0 + recent_height;
    nodes.push(fixed_height(
        transparent_group(
            center_row,
            "Project hub centered content",
            UiLayout::HorizontalStack { gap: 0.0 },
            vec![left_spacer, content, right_spacer],
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
            vec![application_row, workspace_row, main, status_row],
        )
        .with_style(workspace_canvas_style()),
    );
    UiDocument::new(root, nodes)
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
    let root = UiNodeId::new(1);
    let main = UiNodeId::new(4);
    let activity_rail = UiNodeId::new(92_060);
    let browser = UiNodeId::new(164);
    let viewport = UiNodeId::new(132);
    let inspector = UiNodeId::new(196);
    let bottom_shelf = UiNodeId::new(5);
    let mut nodes = Vec::new();

    let application_row = push_application_row(
        &mut nodes,
        &bounded_text(&view.project, 34),
        true,
        session.play_active(),
    );
    let workspace_row = push_workspace_row(&mut nodes, true);

    let rail_items = [
        (92_061, "W", "World workspace", "workspace.world", true),
        (92_062, "+", "Import source", "asset.reimport", false),
        (92_063, "⌕", "Search World", "shell.search", false),
        (92_064, "★", "World favorites", "shell.favorites", false),
        (92_065, "▥", "World panels", "shell.panels", false),
    ];
    let mut rail_children = Vec::new();
    for (id, label, name, action, selected) in rail_items {
        let id = UiNodeId::new(id);
        nodes.push(fixed_height(
            UiNode::button(id, name, action, label).with_style(workspace_tab_style(selected)),
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
        .with_style(world_panel_style(10.0))
        .with_layout_hints(UiLayoutHints::fixed_width(44.0)),
    );

    let browser_header = UiNodeId::new(92_051);
    let browser_title = UiNodeId::new(92_052);
    let browser_kind = UiNodeId::new(92_053);
    let browser_tree = UiNodeId::new(92_054);
    let placement_item = UiNodeId::new(92_055);
    let source_item = UiNodeId::new(92_056);
    let generated_item = UiNodeId::new(92_057);
    let browser_actions = UiNodeId::new(92_058);
    let reimport = UiNodeId::new(262);
    let inspect_source = UiNodeId::new(263);
    nodes.push(fixed_width(
        UiNode::label(browser_title, "World browser title", "World")
            .with_style(creator_title_style()),
        104.0,
    ));
    nodes.push(
        UiNode::label(browser_kind, "World browser source mode", "SOURCE")
            .with_style(creator_hub_field_label_style()),
    );
    nodes.push(fixed_height(
        transparent_group(
            browser_header,
            "World browser header",
            UiLayout::HorizontalStack { gap: 8.0 },
            vec![browser_title, browser_kind],
        ),
        28.0,
    ));
    nodes.push(fixed_height(
        UiNode::search_input(CREATOR_WORLD_SEARCH, "Search World sources", "")
            .with_style(UiStyle::text_field()),
        36.0,
    ));
    let selected_label =
        selected_placement_summary(session).unwrap_or_else(|| "No source placement".to_owned());
    nodes.push(fixed_height(
        UiNode::tree_item(
            placement_item,
            bounded_text(&selected_label, 48),
            "editor.select-placement",
            !session.selection().ids.is_empty(),
            true,
        )
        .with_style(if session.selection().ids.is_empty() {
            UiStyle::secondary_action()
        } else {
            workspace_tab_style(true)
        }),
        34.0,
    ));
    let source_label = session.document().sources.values().next().map_or_else(
        || "Imported · none".to_owned(),
        |source| format!("Imported · {}", bounded_text(&source.label, 34)),
    );
    nodes.push(fixed_height(
        UiNode::tree_item(
            source_item,
            source_label,
            "asset.inspect-source",
            false,
            true,
        )
        .with_style(UiStyle::secondary_action()),
        34.0,
    ));
    nodes.push(fixed_height(
        UiNode::tree_item(
            generated_item,
            "Generated · Alluvium",
            "procedural.inspect",
            false,
            false,
        )
        .with_style(UiStyle::secondary_action()),
        34.0,
    ));
    nodes.push(
        UiNode::tree(
            browser_tree,
            "World source hierarchy",
            vec![placement_item, source_item, generated_item],
        )
        .with_style(UiStyle::transparent()),
    );
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
            UiLayout::HorizontalStack { gap: 6.0 },
            vec![reimport, inspect_source],
        ),
        30.0,
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
                browser_actions,
            ],
        )
        .with_style(world_panel_style(10.0))
        .with_layout_hints(UiLayoutHints::fixed_width(264.0)),
    );

    let viewport_header = UiNodeId::new(92_070);
    let viewport_title = UiNodeId::new(92_071);
    let viewport_meta = UiNodeId::new(92_072);
    let focus_selection = UiNodeId::new(134);
    let canvas_mode = UiNodeId::new(92_080);
    let canvas_status = UiNodeId::new(92_081);
    nodes.push(fixed_width(
        UiNode::label(viewport_title, "Live World viewport title", "LIVE WORLD")
            .with_style(creator_hub_field_label_style()),
        112.0,
    ));
    nodes.push(
        UiNode::label(
            viewport_meta,
            "Live World viewport mode",
            format!(
                "Perspective · Lit · source r{}",
                session.document().generation
            ),
        )
        .with_style(creator_meta_style()),
    );
    nodes.push(fixed_width(
        UiNode::button(
            focus_selection,
            "Focus selected World source",
            "editor.focus-selection",
            "Focus selection",
        )
        .with_style(shell_utility_style(false, false)),
        112.0,
    ));
    nodes.push(fixed_height(
        transparent_group(
            viewport_header,
            "Live World viewport header",
            UiLayout::HorizontalStack { gap: 8.0 },
            vec![viewport_title, viewport_meta, focus_selection],
        ),
        28.0,
    ));
    nodes.push(
        UiNode::label(canvas_mode, "World view projection", "SOURCE VIEW")
            .with_style(canvas_overlay_style(UiColor::grass()))
            .with_absolute_position(UiAbsolutePosition {
                left: 14.0,
                top: 14.0,
                width: Some(104.0),
                height: Some(28.0),
            }),
    );
    nodes.push(
        UiNode::label(
            canvas_status,
            "Selected source in World viewport",
            selected_placement_summary(session).map_or_else(
                || "Select a source placement in World.".to_owned(),
                |summary| bounded_text(&summary, 72),
            ),
        )
        .with_style(canvas_overlay_style(UiColor::border()))
        .with_absolute_position(UiAbsolutePosition {
            left: 14.0,
            top: 50.0,
            width: Some(330.0),
            height: Some(34.0),
        }),
    );
    nodes.push(
        UiNode::canvas(
            CREATOR_WORLD_VIEWPORT_CANVAS,
            "Live source-derived World viewport",
            vec![canvas_mode, canvas_status],
        )
        .with_style(world_canvas_style())
        .with_constraints(UiConstraints {
            minimum: UiSize::new(320.0, 240.0),
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
        .with_style(world_panel_style(10.0)),
    );

    let inspector_header = UiNodeId::new(93_000);
    let inspector_title = UiNodeId::new(93_001);
    let inspector_context = UiNodeId::new(93_002);
    let selection_summary = UiNodeId::new(93_003);
    let transform_title = UiNodeId::new(93_004);
    let transform_fields = UiNodeId::new(93_005);
    nodes.push(fixed_width(
        UiNode::label(inspector_title, "World Inspector title", "Inspector")
            .with_style(creator_title_style()),
        118.0,
    ));
    nodes.push(
        UiNode::label(
            inspector_context,
            "World Inspector context",
            "WORLD PLACEMENT",
        )
        .with_style(creator_hub_field_label_style()),
    );
    nodes.push(fixed_height(
        transparent_group(
            inspector_header,
            "World Inspector header",
            UiLayout::HorizontalStack { gap: 8.0 },
            vec![inspector_title, inspector_context],
        ),
        28.0,
    ));
    nodes.push(fixed_height(
        UiNode::label(
            selection_summary,
            "World Inspector selection",
            selected_placement_summary(session).map_or_else(
                || "Select a source placement to edit it.".to_owned(),
                |summary| bounded_text(&summary, 88),
            ),
        )
        .with_style(creator_meta_style()),
        34.0,
    ));
    nodes.push(fixed_height(
        UiNode::label(
            transform_title,
            "Transform properties",
            "TRANSFORM · MILLIMETRES",
        )
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
            14.0,
        ));
        nodes.push(fixed_height(
            UiNode::text_input(
                field,
                format!("Selected placement {axis} coordinate in millimetres"),
                value,
                UiTextInputOptions::default(),
            )
            .with_style(UiStyle::compact_text_field()),
            28.0,
        ));
        nodes.push(fixed_height(
            transparent_group(
                group,
                format!("{axis} coordinate field"),
                UiLayout::VerticalStack { gap: 2.0 },
                vec![label, field],
            ),
            44.0,
        ));
        axis_groups.push(group);
    }
    nodes.push(fixed_height(
        transparent_group(
            transform_fields,
            "Selected placement transform",
            UiLayout::HorizontalStack { gap: 6.0 },
            axis_groups,
        ),
        44.0,
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
        3,
        30.0,
    );

    let recipe_section = UiNodeId::new(93_040);
    let recipe_title = UiNodeId::new(93_041);
    let recipe_status = UiNodeId::new(93_042);
    nodes.push(fixed_height(
        UiNode::label(recipe_title, "Alluvium section", "ALLUVIUM")
            .with_style(creator_hub_field_label_style()),
        18.0,
    ));
    nodes.push(fixed_height(
        UiNode::label(
            recipe_status,
            "Alluvium World status",
            bounded_text(&view.recipe, 66),
        )
        .with_style(creator_meta_style()),
        20.0,
    ));
    let recipe_actions = push_creator_action_grid(
        &mut nodes,
        UiNodeId::new(93_043),
        93_044,
        "Alluvium",
        EditorPanelId::Recipe,
        session,
        &[
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
        3,
        84.0,
    );
    nodes.push(fixed_height(
        UiNode::container(
            recipe_section,
            "Alluvium World tools",
            UiLayout::VerticalStack { gap: 4.0 },
            vec![recipe_title, recipe_status, recipe_actions],
        )
        .with_style(world_section_style(UiColor::amber())),
        146.0,
    ));

    let model_section = UiNodeId::new(93_080);
    let model_title = UiNodeId::new(93_081);
    let model_status = UiNodeId::new(93_082);
    nodes.push(fixed_height(
        UiNode::label(model_title, "Model source section", "EDITABLE MODEL")
            .with_style(creator_hub_field_label_style()),
        18.0,
    ));
    nodes.push(fixed_height(
        UiNode::label(
            model_status,
            "Editable model status",
            bounded_text(&view.model, 66),
        )
        .with_style(creator_meta_style()),
        20.0,
    ));
    let model_actions = push_creator_action_grid(
        &mut nodes,
        UiNodeId::new(93_083),
        93_084,
        "Editable model",
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
        3,
        84.0,
    );
    nodes.push(fixed_height(
        UiNode::container(
            model_section,
            "Editable model World tools",
            UiLayout::VerticalStack { gap: 4.0 },
            vec![model_title, model_status, model_actions],
        )
        .with_style(world_section_style(UiColor::grass())),
        146.0,
    ));
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
                recipe_section,
                model_section,
            ],
        )
        .with_style(world_panel_style(10.0))
        .with_layout_hints(UiLayoutHints::fixed_width(344.0)),
    );

    nodes.push(
        UiNode::container(
            main,
            "World workspace",
            UiLayout::HorizontalStack { gap: 8.0 },
            vec![activity_rail, browser, viewport, inspector],
        )
        .with_style(UiStyle::canvas()),
    );

    let shelf_items = [
        (93_200, "Undo", "editor.undo", 72.0),
        (93_201, "Redo", "editor.redo", 72.0),
        (93_202, "Build details", "build.inspect", 104.0),
        (93_203, "Diagnostics", "editor.show-diagnostic", 96.0),
        (93_204, "Recover", "editor.recover", 84.0),
    ];
    let mut shelf_children = Vec::new();
    for (id, label, action, width) in shelf_items {
        let id = UiNodeId::new(id);
        nodes.push(fixed_width(
            UiNode::button(id, label, action, label)
                .with_style(creator_compact_action_style(EditorPanelId::History, action)),
            width,
        ));
        shelf_children.push(id);
    }
    let shelf_spacer = UiNodeId::new(93_205);
    let shelf_activity = UiNodeId::new(93_206);
    nodes.push(transparent_group(
        shelf_spacer,
        "Bottom shelf spacer",
        UiLayout::Overlay,
        Vec::new(),
    ));
    nodes.push(fixed_width(
        UiNode::label(
            shelf_activity,
            "Build and recovery summary",
            format!(
                "{} · {}",
                bounded_text(&view.build, 46),
                bounded_text(&view.recovery, 46)
            ),
        )
        .with_style(creator_meta_style()),
        420.0,
    ));
    shelf_children.push(shelf_spacer);
    shelf_children.push(shelf_activity);
    nodes.push(fixed_height(
        UiNode::container(
            bottom_shelf,
            "World bottom shelf",
            UiLayout::HorizontalStack { gap: 6.0 },
            shelf_children,
        )
        .with_style(shell_row_style(UiColor::surface(), 3.0)),
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
    UiDocument::new(root, nodes)
}

#[derive(Clone, Copy)]
struct WorldViewportGeometry {
    canvas: UiRect,
    left: f32,
    right: f32,
    horizon: f32,
    bottom: f32,
}

impl WorldViewportGeometry {
    fn from_canvas(canvas: UiRect) -> Option<Self> {
        let left = canvas.origin.x + 18.0;
        let right = canvas.origin.x + canvas.size.width - 18.0;
        let top = canvas.origin.y + 96.0;
        let bottom = canvas.origin.y + canvas.size.height - 18.0;
        (right > left && bottom > top).then_some(Self {
            canvas,
            left,
            right,
            horizon: top + (bottom - top) * 0.42,
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

fn push_world_filled_path(
    display: &mut DisplayList,
    commands: Vec<UiPathCommand>,
    color: UiColor,
) -> Result<(), DisplayListError> {
    display.try_push(DisplayPrimitive::Path {
        node: CREATOR_WORLD_VIEWPORT_CANVAS,
        commands,
        fill: Some(color),
        stroke: Some(UiStroke::new(color, 1.0)),
    })
}

fn decorate_world_grid(
    display: &mut DisplayList,
    geometry: WorldViewportGeometry,
) -> Result<(), DisplayListError> {
    let grid = UiColor::rgba(0.161, 0.176, 0.173, 0.42);
    let vanishing = UiPoint {
        x: geometry.left + (geometry.right - geometry.left) * 0.56,
        y: geometry.horizon,
    };
    for column in 0_u8..=8 {
        let progress = f32::from(column) / 8.0;
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
    for row in 0_u8..=5 {
        let progress = f32::from(row) / 5.0;
        let y = geometry.horizon + (geometry.bottom - geometry.horizon) * progress * progress;
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
    Ok(())
}

#[allow(clippy::cast_precision_loss)] // Source millimetres clamp to a small display-only range.
fn decorate_world_placement(
    display: &mut DisplayList,
    geometry: WorldViewportGeometry,
    placement: &WorldPlacement,
) -> Result<(), DisplayListError> {
    let source_x = placement.translation.x_mm.clamp(-5_000, 5_000) as f32 / 5_000.0;
    let source_z = placement.translation.z_mm.clamp(-5_000, 5_000) as f32 / 5_000.0;
    let center = UiPoint {
        x: geometry.left + (geometry.right - geometry.left) * (0.56 + source_x * 0.18),
        y: geometry.horizon + (geometry.bottom - geometry.horizon) * (0.52 - source_z * 0.16),
    };
    let size = geometry
        .canvas
        .size
        .width
        .min(geometry.canvas.size.height)
        .mul_add(0.11, 0.0)
        .clamp(28.0, 84.0);
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
    push_world_filled_path(
        display,
        vec![
            UiPathCommand::MoveTo(triangle[0]),
            UiPathCommand::LineTo(triangle[1]),
            UiPathCommand::LineTo(triangle[2]),
            UiPathCommand::Close,
        ],
        UiColor::rgba(0.553, 0.537, 0.38, 0.82),
    )?;
    let radius = size + 12.0;
    push_world_path(
        display,
        vec![
            UiPathCommand::MoveTo(UiPoint {
                x: center.x - radius,
                y: center.y - radius,
            }),
            UiPathCommand::LineTo(UiPoint {
                x: center.x + radius,
                y: center.y - radius,
            }),
            UiPathCommand::LineTo(UiPoint {
                x: center.x + radius,
                y: center.y + radius,
            }),
            UiPathCommand::LineTo(UiPoint {
                x: center.x - radius,
                y: center.y + radius,
            }),
            UiPathCommand::Close,
        ],
        UiColor::amber(),
    )?;
    for (end, color) in [
        (
            UiPoint {
                x: center.x + size * 1.35,
                y: center.y,
            },
            UiColor::red_hover(),
        ),
        (
            UiPoint {
                x: center.x,
                y: center.y - size * 1.35,
            },
            UiColor::grass(),
        ),
    ] {
        push_world_path(
            display,
            vec![UiPathCommand::MoveTo(center), UiPathCommand::LineTo(end)],
            color,
        )?;
    }
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
        let document = creator_alpha_document(&session).expect("valid UI document");
        for action in ["editor.edit-placement", "editor.undo", "editor.redo"] {
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

    fn assert_creator_controls_fit(document: UiDocument, viewport: UiSize, scale_factor: f32) {
        let expected_focus_order = document.focus_order();
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
        for node in visible_focusable {
            let bounds = node.bounds;
            assert!(bounds.origin.x.is_finite() && bounds.origin.y.is_finite());
            assert!(bounds.size.width.is_finite() && bounds.size.height.is_finite());
            assert!(bounds.origin.x >= 0.0 && bounds.origin.y >= 0.0);
            assert!(bounds.size.width >= 1.0 && bounds.size.height >= 1.0);
            assert!(bounds.origin.x + bounds.size.width <= viewport.width + 0.1);
            assert!(bounds.origin.y + bounds.size.height <= viewport.height + 0.1);
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
                (bounds.size.height - 54.0).abs() < 0.1,
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
        assert_eq!(save.text.as_deref(), Some("Save placement"));
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

        assert!((application_row.size.height - 44.0).abs() < 0.1);
        assert!((workspace_row.size.height - 36.0).abs() < 0.1);
        assert!((status_row.size.height - 24.0).abs() < 0.1);
        assert!((browser.size.width - 264.0).abs() < 0.1);
        assert!((inspector.size.width - 344.0).abs() < 0.1);
        assert!(viewport.size.width > browser.size.width);
        assert!(viewport.size.width > inspector.size.width);
    }

    #[test]
    fn world_viewport_is_a_real_canvas_decorated_from_authoritative_source() {
        let mut session = public_creator_session();
        let document = creator_alpha_document(&session).expect("valid Creator workspace");
        let canvas = document
            .node(CREATOR_WORLD_VIEWPORT_CANVAS)
            .expect("World viewport canvas");
        assert_eq!(canvas.kind, UiWidgetKind::Canvas);
        assert_eq!(canvas.semantics.role, SemanticRole::Canvas);

        let mut runtime = UiRuntime::new(document);
        let frame = runtime.reconcile(UiFrameInput::new(UiSize::new(1440.0, 900.0)));
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
        assert!(first_paths.len() >= 19);
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
        assert!(rendered_text
            .iter()
            .any(|text| text.contains("Public placement / v1")));
        assert!(!rendered_text
            .iter()
            .any(|text| text.contains("The world has no editable placements.")));
        assert_creator_controls_fit(document.clone(), UiSize::new(1024.0, 720.0), 1.0);
        assert_creator_controls_fit(document, UiSize::new(1280.0, 800.0), 2.0);
    }
}
