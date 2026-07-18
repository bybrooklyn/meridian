//! Meridian-native Creator Editor panel declarations and accessible UI document.
//!
//! Panel declarations describe presentation and commands only. Project source,
//! selections, history, recipes, and model documents remain owned by their
//! respective Meridian domain crates.

mod workspace;

pub use workspace::*;

use meridian_alluvium::ProceduralRecipe;
use meridian_editor_core::EditorSession;
use meridian_modeler::{ModelDocument, ModelSelection, PenumbraPreview};
use meridian_ui::{
    UiBorder, UiColor, UiDocument, UiDocumentError, UiLayout, UiLayoutHints, UiNode, UiNodeId,
    UiStyle, UiTextInputOptions,
};

/// Stable node for the Creator hub's project-name field.
pub const CREATOR_HUB_PROJECT_NAME: UiNodeId = UiNodeId::new(90_002);

/// Stable editable X-coordinate field for the selected Creator placement.
pub const CREATOR_INSPECTOR_X_MM: UiNodeId = UiNodeId::new(91_001);
/// Stable editable Y-coordinate field for the selected Creator placement.
pub const CREATOR_INSPECTOR_Y_MM: UiNodeId = UiNodeId::new(91_002);
/// Stable editable Z-coordinate field for the selected Creator placement.
pub const CREATOR_INSPECTOR_Z_MM: UiNodeId = UiNodeId::new(91_003);

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

fn workspace_canvas_style() -> UiStyle {
    UiStyle {
        background: Some(UiColor::background()),
        border: None,
        foreground: UiColor::foreground(),
        padding: 14.0,
        font_size: 16.0,
    }
}

fn creator_header_style() -> UiStyle {
    UiStyle {
        background: Some(UiColor::surface()),
        border: Some(UiBorder {
            color: UiColor::border(),
            width: 1,
        }),
        foreground: UiColor::foreground(),
        padding: 12.0,
        font_size: 16.0,
    }
}

fn creator_title_style() -> UiStyle {
    UiStyle {
        background: None,
        border: None,
        foreground: UiColor::text(),
        padding: 0.0,
        font_size: 20.0,
    }
}

fn creator_meta_style() -> UiStyle {
    UiStyle {
        background: None,
        border: None,
        foreground: UiColor::secondary_text(),
        padding: 0.0,
        font_size: 12.0,
    }
}

fn creator_mode_style(play_active: bool) -> UiStyle {
    let (background, border, foreground) = if play_active {
        (UiColor::surface(), UiColor::amber(), UiColor::amber())
    } else {
        (UiColor::surface(), UiColor::grass(), UiColor::grass())
    };
    UiStyle {
        background: Some(background),
        border: Some(UiBorder {
            color: border,
            width: 1,
        }),
        foreground,
        padding: 6.0,
        font_size: 11.0,
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

fn creator_panel_style(panel: EditorPanelId) -> UiStyle {
    let background = match panel {
        EditorPanelId::Viewport => UiColor::background(),
        _ => UiColor::surface(),
    };
    UiStyle {
        background: Some(background),
        border: Some(UiBorder {
            color: creator_panel_accent(panel),
            width: if panel == EditorPanelId::Viewport {
                2
            } else {
                1
            },
        }),
        foreground: UiColor::foreground(),
        padding: if panel == EditorPanelId::Viewport {
            14.0
        } else {
            12.0
        },
        font_size: 16.0,
    }
}

fn creator_panel_heading_style(panel: EditorPanelId) -> UiStyle {
    UiStyle {
        background: None,
        border: None,
        foreground: creator_panel_accent(panel),
        padding: 0.0,
        font_size: 15.0,
    }
}

fn creator_preview_style() -> UiStyle {
    UiStyle {
        background: Some(UiColor::background()),
        border: Some(UiBorder {
            color: UiColor::border(),
            width: 1,
        }),
        foreground: UiColor::text(),
        padding: 18.0,
        font_size: 16.0,
    }
}

fn creator_hub_card_style() -> UiStyle {
    UiStyle {
        background: Some(UiColor::surface()),
        border: Some(UiBorder {
            color: UiColor::border(),
            width: 1,
        }),
        foreground: UiColor::foreground(),
        padding: 20.0,
        font_size: 16.0,
    }
}

fn creator_hub_status_style() -> UiStyle {
    UiStyle {
        background: Some(UiColor::background()),
        border: Some(UiBorder {
            color: UiColor::border(),
            width: 1,
        }),
        foreground: UiColor::secondary_text(),
        padding: 8.0,
        font_size: 12.0,
    }
}

fn creator_hub_field_label_style() -> UiStyle {
    UiStyle {
        background: None,
        border: None,
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

fn panel_status(
    panel: EditorPanelId,
    session: &EditorSession,
    view: &CreatorWorkspaceView,
) -> String {
    match panel {
        EditorPanelId::ProjectRecovery if session.play_active() => format!(
            "Play · {} pending change(s)",
            session.pending_play_change_count()
        ),
        EditorPanelId::ProjectRecovery => bounded_text(&view.recovery, 34),
        EditorPanelId::Viewport => selected_placement_summary(session).map_or_else(
            || "No source placement".to_owned(),
            |_| "Derived source preview".to_owned(),
        ),
        EditorPanelId::Hierarchy => {
            format!(
                "{} source placement(s)",
                session.document().placements.len()
            )
        }
        EditorPanelId::Inspector => {
            let selected = session.selection().ids.len();
            format!(
                "{selected} selected · source r{}",
                session.document().generation
            )
        }
        EditorPanelId::History => format!(
            "{} undo · {} redo{}",
            session.undo_depth(),
            session.redo_depth(),
            if session.play_active() {
                " · Play"
            } else {
                ""
            }
        ),
        EditorPanelId::Assets => session.document().sources.values().next().map_or_else(
            || "No imported source".to_owned(),
            |source| {
                format!(
                    "{} imported · {}",
                    session.document().sources.len(),
                    bounded_text(&source.label, 30)
                )
            },
        ),
        EditorPanelId::Build => bounded_text(&view.build, 34),
        EditorPanelId::Recipe => bounded_text(&view.recipe, 42),
        EditorPanelId::Modeler => bounded_text(&view.model, 42),
        EditorPanelId::Diagnostics => bounded_text(&view.activity, 34),
    }
}

fn viewport_preview_text(session: &EditorSession) -> String {
    selected_placement_summary(session).map_or_else(
        || "No placement is selected. Use Hierarchy to choose the first public source placement."
            .to_owned(),
        |placement| {
            format!(
                "DERIVED SOURCE PREVIEW\n\n{placement}\n\nAuthoritative project source · use Focus Selection to reframe."
            )
        },
    )
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
    let status_id = UiNodeId::new(90_001);
    let top_spacer = UiNodeId::new(90_005);
    let center_row = UiNodeId::new(90_006);
    let bottom_spacer = UiNodeId::new(90_007);
    let left_spacer = UiNodeId::new(90_008);
    let card = UiNodeId::new(90_009);
    let right_spacer = UiNodeId::new(90_010);
    let title = UiNodeId::new(90_011);
    let description = UiNodeId::new(90_012);
    let action_row = UiNodeId::new(90_013);
    let recents_title = UiNodeId::new(90_014);
    let recents_list = UiNodeId::new(90_015);
    let project_name_label = UiNodeId::new(90_017);
    let create = UiNodeId::new(90_003);
    let open = UiNodeId::new(90_004);
    let displayed_recent_count = recents.len().min(MAX_CREATOR_HUB_RECENT_ROWS);
    // Keep every bounded recent row at its declared 54 px height plus the
    // list gap. The old estimate squeezed rows as recents accumulated.
    let recent_count = f32::from(u8::try_from(displayed_recent_count).unwrap_or(u8::MAX));
    let recent_gap_count =
        f32::from(u8::try_from(displayed_recent_count.saturating_sub(1)).unwrap_or(u8::MAX));
    let recent_list_height = if displayed_recent_count == 0 {
        40.0
    } else {
        recent_count * 54.0 + recent_gap_count * 8.0
    };
    let hub_height = 366.0 + recent_list_height;
    let mut nodes = vec![
        fixed_height(
            UiNode::label(title, "Meridian Creator", "Meridian Creator")
                .with_style(UiStyle::heading()),
            40.0,
        ),
        fixed_height(
            UiNode::label(
                description,
                "Creator Alpha introduction",
                "Create a public project or open a validated project directory. Your project source remains authoritative.",
            )
            .with_style(UiStyle::muted_text()),
            44.0,
        ),
        fixed_height(
            UiNode::label(
                status_id,
                "Meridian Creator status",
                bounded_text(status, 220),
            )
            .with_style(creator_hub_status_style()),
            34.0,
        ),
        fixed_height(
            UiNode::label(project_name_label, "New project name label", "NEW PROJECT NAME")
                .with_style(creator_hub_field_label_style()),
            18.0,
        ),
        fixed_height(
            UiNode::text_input(
                CREATOR_HUB_PROJECT_NAME,
                "New project name",
                "Meridian Project",
                UiTextInputOptions::default(),
            )
            .with_style(UiStyle::text_field()),
            48.0,
        ),
        UiNode::button(
            create,
            "Create project",
            "hub.create-project",
            "Create project",
        )
        .with_style(UiStyle::primary_action()),
        UiNode::button(open, "Open project", "hub.open-project", "Open project")
            .with_style(UiStyle::secondary_action()),
    ];
    let mut recent_rows = Vec::new();
    for (index, recent) in recents.iter().take(MAX_CREATOR_HUB_RECENT_ROWS).enumerate() {
        let base = 90_100_u128.saturating_add((index as u128).saturating_mul(4));
        let label = UiNodeId::new(base);
        let open = UiNodeId::new(base + 1);
        let remove = UiNodeId::new(base + 2);
        let row = UiNodeId::new(base + 3);
        let availability = if recent.available {
            "available"
        } else {
            "unavailable"
        };
        nodes.push(
            UiNode::label(
                label,
                format!("Recent project {}", index + 1),
                format!(
                    "{} — {} ({availability})",
                    bounded_text(&recent.label, 56),
                    bounded_text(&recent.path, 96)
                ),
            )
            .with_style(UiStyle::muted_text()),
        );
        nodes.push(
            UiNode::button(
                open,
                format!("Open recent project {}", index + 1),
                format!("hub.open-recent:{index}"),
                "Open recent",
            )
            .with_style(UiStyle::secondary_action())
            .with_layout_hints(UiLayoutHints::fixed_width(108.0)),
        );
        nodes.push(
            UiNode::button(
                remove,
                format!("Remove recent project {}", index + 1),
                format!("hub.remove-recent:{index}"),
                "Remove recent",
            )
            .with_style(UiStyle::secondary_action())
            .with_layout_hints(UiLayoutHints::fixed_width(108.0)),
        );
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
        let empty_recent = UiNodeId::new(90_016);
        nodes.push(
            UiNode::label(
                empty_recent,
                "No recent projects",
                "No recent projects yet. Recent paths stay local and are never opened implicitly.",
            )
            .with_style(UiStyle::muted_text()),
        );
        recent_rows.push(empty_recent);
    }
    nodes.push(fixed_height(
        UiNode::label(recents_title, "Recent projects", "Recent projects")
            .with_style(UiStyle::section_heading()),
        24.0,
    ));
    nodes.push(transparent_group(
        recents_list,
        "Recent projects list",
        UiLayout::VerticalStack { gap: 8.0 },
        recent_rows,
    ));
    nodes.push(fixed_height(
        transparent_group(
            action_row,
            "Project actions",
            UiLayout::HorizontalStack { gap: 10.0 },
            vec![create, open],
        ),
        48.0,
    ));
    nodes.push(
        UiNode::container(
            card,
            "Meridian Creator hub",
            UiLayout::VerticalStack { gap: 10.0 },
            vec![
                title,
                description,
                status_id,
                project_name_label,
                CREATOR_HUB_PROJECT_NAME,
                action_row,
                recents_title,
                recents_list,
            ],
        )
        .with_style(creator_hub_card_style())
        .with_layout_hints(UiLayoutHints::fixed_width(760.0)),
    );
    nodes.push(transparent_group(
        left_spacer,
        "Creator hub left margin",
        UiLayout::Overlay,
        Vec::new(),
    ));
    nodes.push(transparent_group(
        right_spacer,
        "Creator hub right margin",
        UiLayout::Overlay,
        Vec::new(),
    ));
    nodes.push(fixed_height(
        transparent_group(
            center_row,
            "Creator hub content",
            UiLayout::HorizontalStack { gap: 0.0 },
            vec![left_spacer, card, right_spacer],
        ),
        hub_height,
    ));
    nodes.push(transparent_group(
        top_spacer,
        "Creator hub top margin",
        UiLayout::Overlay,
        Vec::new(),
    ));
    nodes.push(transparent_group(
        bottom_spacer,
        "Creator hub bottom margin",
        UiLayout::Overlay,
        Vec::new(),
    ));
    nodes.push(
        UiNode::container(
            root,
            "Meridian Creator hub",
            UiLayout::VerticalStack { gap: 0.0 },
            vec![top_spacer, center_row, bottom_spacer],
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
    let header = UiNodeId::new(2);
    let content = UiNodeId::new(3);
    let footer = UiNodeId::new(4);
    let left_column = UiNodeId::new(5);
    let center_column = UiNodeId::new(6);
    let right_column = UiNodeId::new(7);
    let header_title = UiNodeId::new(8);
    let header_summary = UiNodeId::new(9);
    let header_mode = UiNodeId::new(10);
    let mut nodes = Vec::new();
    let selected = session.selection().ids.len();
    let project_status = format!(
        "Source r{} · {} source(s) · {} placement(s) · {selected} selected",
        session.document().generation,
        session.document().sources.len(),
        session.document().placements.len(),
    );
    nodes.push(
        UiNode::label(
            header_title,
            "Meridian Creator workspace",
            "Meridian Creator",
        )
        .with_style(creator_title_style())
        .with_layout_hints(UiLayoutHints::fixed_width(220.0)),
    );
    nodes.push(
        UiNode::label(
            header_summary,
            "Creator project summary",
            bounded_text(&project_status, 220),
        )
        .with_style(creator_meta_style()),
    );
    nodes.push(
        UiNode::label(
            header_mode,
            "Creator interaction mode",
            if session.play_active() {
                "PLAY MODE"
            } else {
                "EDIT MODE"
            },
        )
        .with_style(creator_mode_style(session.play_active()))
        .with_layout_hints(UiLayoutHints::fixed_width(94.0)),
    );
    nodes.push(fixed_height(
        UiNode::container(
            header,
            "Creator project status bar",
            UiLayout::HorizontalStack { gap: 12.0 },
            vec![header_title, header_summary, header_mode],
        )
        .with_style(creator_header_style()),
        56.0,
    ));

    let mut panel_ids = Vec::new();
    for (index, panel) in creator_alpha_panels().iter().enumerate() {
        let base = 100_u128 + (index as u128 * 32);
        let panel_id = UiNodeId::new(base);
        let status_id = UiNodeId::new(base + 1);
        let heading_id = UiNodeId::new(base + 28);
        let action_group_id = UiNodeId::new(base + 29);
        let preview_id = UiNodeId::new(base + 30);
        let inspector_fields_id = UiNodeId::new(base + 27);
        let mut children = vec![heading_id, status_id];
        nodes.push(fixed_height(
            UiNode::label(heading_id, panel.title, panel.title)
                .with_style(creator_panel_heading_style(panel.id)),
            20.0,
        ));
        nodes.push(fixed_height(
            UiNode::label(
                status_id,
                format!("{} current state", panel.title),
                panel_status(panel.id, session, view),
            )
            .with_style(creator_meta_style()),
            20.0,
        ));
        if panel.id == EditorPanelId::Viewport {
            children.push(preview_id);
            nodes.push(
                UiNode::label(
                    preview_id,
                    "Derived source placement preview",
                    viewport_preview_text(session),
                )
                .with_style(creator_preview_style()),
            );
        }
        if panel.id == EditorPanelId::Inspector {
            let (x_mm, y_mm, z_mm) = inspected_translation_values(session);
            children.push(inspector_fields_id);
            let axis_fields = [
                (
                    UiNodeId::new(base + 21),
                    UiNodeId::new(base + 24),
                    CREATOR_INSPECTOR_X_MM,
                    "X",
                    x_mm,
                ),
                (
                    UiNodeId::new(base + 22),
                    UiNodeId::new(base + 25),
                    CREATOR_INSPECTOR_Y_MM,
                    "Y",
                    y_mm,
                ),
                (
                    UiNodeId::new(base + 23),
                    UiNodeId::new(base + 26),
                    CREATOR_INSPECTOR_Z_MM,
                    "Z",
                    z_mm,
                ),
            ];
            let mut axis_groups = Vec::new();
            for (group, label, field, axis, value) in axis_fields {
                nodes.push(fixed_height(
                    UiNode::label(
                        label,
                        format!("{axis} axis millimetres"),
                        format!("{axis} (mm)"),
                    )
                    .with_style(creator_meta_style()),
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
                        format!("Selected placement {axis} coordinate"),
                        UiLayout::VerticalStack { gap: 2.0 },
                        vec![label, field],
                    ),
                    44.0,
                ));
                axis_groups.push(group);
            }
            nodes.push(fixed_height(
                transparent_group(
                    inspector_fields_id,
                    "Selected placement X Y Z millimetre fields",
                    UiLayout::HorizontalStack { gap: 6.0 },
                    axis_groups,
                ),
                44.0,
            ));
        }
        let mut action_ids = Vec::new();
        for (command_index, command) in panel
            .commands
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, command)| creator_command_is_available(command, session))
        {
            let command_node_id = UiNodeId::new(base + 2 + command_index as u128);
            action_ids.push(command_node_id);
            let action_label = creator_action_label(command);
            nodes.push(
                UiNode::button(
                    command_node_id,
                    format!("{}: {action_label}", panel.title),
                    command,
                    action_label,
                )
                .with_style(creator_compact_action_style(panel.id, command)),
            );
        }
        let action_grid = transparent_group(
            action_group_id,
            format!("{} actions", panel.title),
            UiLayout::Grid {
                columns: 3,
                gap: 6.0,
            },
            action_ids,
        );
        nodes.push(match panel.id {
            EditorPanelId::Viewport => fixed_height(action_grid, 26.0),
            EditorPanelId::Inspector => fixed_height(action_grid, 24.0),
            _ => action_grid,
        });
        children.push(action_group_id);
        let panel_node = UiNode::container(
            panel_id,
            panel.title,
            UiLayout::VerticalStack { gap: 6.0 },
            children,
        )
        .with_style(creator_panel_style(panel.id));
        nodes.push(match panel.id {
            EditorPanelId::ProjectRecovery => fixed_height(panel_node, 176.0),
            EditorPanelId::Inspector => fixed_height(panel_node, 178.0),
            _ => panel_node,
        });
        panel_ids.push(panel_id);
    }
    nodes.push(
        transparent_group(
            left_column,
            "Creator project hierarchy and assets",
            UiLayout::VerticalStack { gap: 8.0 },
            vec![panel_ids[0], panel_ids[2], panel_ids[5]],
        )
        .with_layout_hints(UiLayoutHints::fixed_width(230.0)),
    );
    nodes.push(transparent_group(
        center_column,
        "Creator source placement preview",
        UiLayout::VerticalStack { gap: 8.0 },
        vec![panel_ids[1]],
    ));
    nodes.push(
        transparent_group(
            right_column,
            "Creator inspector and tools",
            UiLayout::VerticalStack { gap: 8.0 },
            vec![panel_ids[3], panel_ids[7], panel_ids[8]],
        )
        .with_layout_hints(UiLayoutHints::fixed_width(280.0)),
    );
    nodes.push(transparent_group(
        content,
        "Creator main workspace",
        UiLayout::HorizontalStack { gap: 8.0 },
        vec![left_column, center_column, right_column],
    ));
    nodes.push(fixed_height(
        transparent_group(
            footer,
            "Creator history build and diagnostics",
            UiLayout::HorizontalStack { gap: 10.0 },
            vec![panel_ids[4], panel_ids[6], panel_ids[9]],
        ),
        112.0,
    ));
    nodes.push(
        UiNode::container(
            root,
            "Creator Editor Alpha workspace",
            UiLayout::VerticalStack { gap: 10.0 },
            vec![header, content, footer],
        )
        .with_style(workspace_canvas_style()),
    );
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
    use meridian_ui::{
        DisplayPrimitive, SemanticDelta, SemanticRole, UiEvent, UiFrameInput, UiPoint, UiRuntime,
        UiSize, UiWidgetKind,
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
        for id in [UiNodeId::new(198), UiNodeId::new(230), UiNodeId::new(231)] {
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
            SemanticDelta::Unchanged => panic!("first Creator frame must publish semantics"),
        };
        let visible_focusable = tree
            .nodes
            .iter()
            .filter(|node| matches!(node.role, SemanticRole::Button | SemanticRole::TextInput))
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
                SemanticRole::Button => assert!(
                    bounds.size.height >= 20.0,
                    "{} button height fell below the compact-action minimum",
                    node.name
                ),
                SemanticRole::TextInput => assert!(
                    bounds.size.height >= 28.0,
                    "{} text field height fell below its declared size",
                    node.name
                ),
                SemanticRole::Group
                | SemanticRole::Status
                | SemanticRole::ToggleButton
                | SemanticRole::ProgressIndicator
                | SemanticRole::SearchBox
                | SemanticRole::ComboBox
                | SemanticRole::Option
                | SemanticRole::MenuBar
                | SemanticRole::Menu
                | SemanticRole::MenuItem
                | SemanticRole::Tooltip
                | SemanticRole::LiveRegion
                | SemanticRole::TabList
                | SemanticRole::Tab
                | SemanticRole::Tree
                | SemanticRole::TreeItem
                | SemanticRole::Table
                | SemanticRole::Row
                | SemanticRole::Cell
                | SemanticRole::PropertyGrid
                | SemanticRole::List
                | SemanticRole::ListItem
                | SemanticRole::Timeline
                | SemanticRole::Splitter
                | SemanticRole::Dialog
                | SemanticRole::Graph
                | SemanticRole::Canvas => {
                    unreachable!("focusable filter is exact")
                }
            }
        }
    }

    fn semantic_bounds(output: &meridian_ui::UiFrameOutput, id: UiNodeId) -> meridian_ui::UiRect {
        let tree = match &output.semantic_delta {
            SemanticDelta::Replace(tree) => tree,
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
            Some(UiNodeId::new(104))
        );

        let mut runtime = UiRuntime::new(edit_document);
        let focused = runtime.reconcile({
            let mut input = UiFrameInput::new(UiSize::new(1024.0, 720.0));
            input.events = vec![UiEvent::AssistiveFocus(UiNodeId::new(104))];
            input
        });
        assert_eq!(focused.focused, Some(UiNodeId::new(104)));

        session.start_play().expect("Play session starts");
        let play_document = creator_alpha_document(&session).expect("valid Play workspace");
        assert_eq!(
            action_id(&play_document, "editor.play-apply"),
            Some(UiNodeId::new(105))
        );
        assert_eq!(
            action_id(&play_document, "editor.play-discard"),
            Some(UiNodeId::new(106))
        );
        assert!(play_document.node(UiNodeId::new(104)).is_none());
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
                | DisplayPrimitive::FocusRing { .. }
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
    fn creator_workspace_gives_the_source_preview_and_header_clear_priority() {
        let session = public_creator_session();
        let document = creator_alpha_document(&session).expect("valid Creator workspace");
        let mut runtime = UiRuntime::new(document);
        let output = runtime.reconcile(UiFrameInput::new(UiSize::new(1024.0, 720.0)));
        let header = semantic_bounds(&output, UiNodeId::new(2));
        let viewport = semantic_bounds(&output, UiNodeId::new(132));
        let project_panel = semantic_bounds(&output, UiNodeId::new(100));
        let inspector = semantic_bounds(&output, UiNodeId::new(196));

        assert!((header.size.height - 56.0).abs() < 0.1);
        assert!(viewport.size.width > project_panel.size.width);
        assert!(viewport.size.width > inspector.size.width);
        assert!(viewport.size.height > project_panel.size.height);
    }

    #[test]
    fn creator_workspace_renders_truthful_state_in_a_bounded_visible_shell() {
        let session = public_creator_session();
        let view = CreatorWorkspaceView {
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
                | DisplayPrimitive::FocusRing { .. }
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
