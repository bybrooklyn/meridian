//! Bounded real-assistive-client review runner for Meridian UI semantics.
//!
//! The runner keeps a native AccessKit-backed window alive long enough for a
//! screen reader to inspect it and records only actions delivered by the real
//! platform adapter. It cannot observe spoken output and never converts a
//! projected tree, timeout, or synthetic event into screen-reader qualification.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use meridian_benchmark::write_evidence_json;
use meridian_platform::{
    run, PlatformAccessibilityActionData, PlatformAccessibilityActionRequest, PlatformApplication,
    PlatformConfig, PlatformContext, PlatformEvent, PlatformEventEnvelope, WindowSize,
};
use meridian_ui_core::{SemanticRole, UiControlState, UiNodeId, UiPoint, UiRect, UiSize};
use meridian_ui_semantics::{
    SemanticAction, SemanticLive, SemanticNode, SemanticRelationships, SemanticTree,
};
use serde::Serialize;

const REPORT_SCHEMA: &str = "meridian.ui-accessibility-review/v1";
const DEFAULT_TIMEOUT_SECONDS: u64 = 120;
const MAX_TIMEOUT_SECONDS: u64 = 300;
const POLL_INTERVAL: Duration = Duration::from_millis(100);
const MAX_RECORDED_ACTIONS: usize = 64;
const MAX_RECORDED_REJECTIONS: usize = 64;
const MAX_FAILURE_DETAIL_CHARS: usize = 240;

const ROOT: UiNodeId = UiNodeId::new(1);
const PROJECT_NAME: UiNodeId = UiNodeId::new(2);
const WORLD_TREE: UiNodeId = UiNodeId::new(3);
const WORLD_ITEM: UiNodeId = UiNodeId::new(4);
const BUILD: UiNodeId = UiNodeId::new(5);
const PROGRESS: UiNodeId = UiNodeId::new(6);
const STATUS: UiNodeId = UiNodeId::new(7);

#[derive(Clone, Debug, Eq, PartialEq)]
struct RunnerOptions {
    evidence_directory: PathBuf,
    timeout: Duration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SourceState {
    CleanCommit,
    WorkingTree,
}

impl SourceState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::CleanCommit => "CleanCommit",
            Self::WorkingTree => "WorkingTree",
        }
    }
}

#[derive(Clone, Debug)]
struct SourceProvenance {
    checkpoint: String,
    state: SourceState,
}

#[derive(Clone, Debug, Serialize)]
struct ObservedAssistiveAction {
    sequence: u64,
    monotonic_nanoseconds: u64,
    target: String,
    target_name: String,
    action: &'static str,
    data: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct AccessibilityReviewReport {
    schema: &'static str,
    runner_status: &'static str,
    evidence_status: &'static str,
    review_status: &'static str,
    screen_reader_qualification: bool,
    source_checkpoint: String,
    source_state: &'static str,
    source_provenance_verification: &'static str,
    promotion_eligibility: &'static str,
    requirement_ids: [&'static str; 2],
    work_package_id: &'static str,
    adapter_projection: &'static str,
    actual_assistive_action_count: usize,
    dropped_assistive_action_count: u64,
    build_activation_observed: bool,
    actions: Vec<ObservedAssistiveAction>,
    rejected_actions: Vec<String>,
    dropped_rejected_action_count: u64,
    timeout_seconds: u64,
    elapsed_milliseconds: u64,
    evidence_directory: &'static str,
    required_human_checks: [&'static str; 5],
    limits: [&'static str; 4],
}

#[derive(Clone, Debug, Serialize)]
struct AccessibilityReviewFailureReport {
    schema: &'static str,
    runner_status: &'static str,
    evidence_status: &'static str,
    review_status: &'static str,
    screen_reader_qualification: bool,
    source_checkpoint: String,
    source_state: &'static str,
    error: String,
}

struct AccessibilityReviewRunner {
    evidence_directory: PathBuf,
    source: SourceProvenance,
    timeout: Duration,
    started: Instant,
    tree: SemanticTree,
    adapter_projected: bool,
    actions: Vec<ObservedAssistiveAction>,
    dropped_actions: u64,
    rejected_actions: Vec<String>,
    dropped_rejections: u64,
    build_activation_observed: bool,
    report_written: bool,
    failure: Arc<Mutex<Option<String>>>,
}

impl AccessibilityReviewRunner {
    fn new(
        evidence_directory: PathBuf,
        source: SourceProvenance,
        timeout: Duration,
        failure: Arc<Mutex<Option<String>>>,
    ) -> Self {
        Self {
            evidence_directory,
            source,
            timeout,
            started: Instant::now(),
            tree: semantic_tree(false, "Screen-reader review ready", SemanticLive::Polite),
            adapter_projected: false,
            actions: Vec::new(),
            dropped_actions: 0,
            rejected_actions: Vec::new(),
            dropped_rejections: 0,
            build_activation_observed: false,
            report_written: false,
            failure,
        }
    }

    fn fail(&mut self, message: impl Into<String>, context: &mut PlatformContext<'_>) {
        let message = sanitize_detail(&message.into());
        let report = AccessibilityReviewFailureReport {
            schema: REPORT_SCHEMA,
            runner_status: "Fail",
            evidence_status: "Inconclusive",
            review_status: "NotRun",
            screen_reader_qualification: false,
            source_checkpoint: self.source.checkpoint.clone(),
            source_state: self.source.state.as_str(),
            error: message.clone(),
        };
        let report_error = write_evidence_json(
            self.evidence_directory
                .join("accessibility-review-failure.json"),
            &report,
        )
        .err();
        let final_message = report_error.map_or(message.clone(), |error| {
            format!("{message}; additionally failed to write failure evidence: {error}")
        });
        let mut failure = self
            .failure
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if failure.is_none() {
            *failure = Some(final_message);
        }
        context.exit();
    }

    fn apply_action(
        &mut self,
        request: &PlatformAccessibilityActionRequest,
        sequence: u64,
        monotonic_nanoseconds: u64,
    ) {
        if self.actions.len() < MAX_RECORDED_ACTIONS {
            self.actions.push(ObservedAssistiveAction {
                sequence,
                monotonic_nanoseconds,
                target: request.target.stable_id().get().to_string(),
                target_name: semantic_name(&self.tree, request.target).to_owned(),
                action: semantic_action_name(request.action),
                data: action_data_name(request.data.as_ref()),
            });
        } else {
            self.dropped_actions = self.dropped_actions.saturating_add(1);
        }
        match (request.target, request.action, request.data.as_ref()) {
            (BUILD, SemanticAction::Activate, None) => {
                self.build_activation_observed = true;
                self.tree = semantic_tree(
                    world_item_expanded(&self.tree),
                    "Build action reached Meridian through the native assistive adapter",
                    SemanticLive::Assertive,
                );
                set_focus(&mut self.tree, BUILD);
            }
            (WORLD_ITEM, SemanticAction::Expand, None) => {
                self.tree = semantic_tree(true, "World item expanded", SemanticLive::Polite);
                set_focus(&mut self.tree, WORLD_ITEM);
            }
            (WORLD_ITEM, SemanticAction::Collapse, None) => {
                self.tree = semantic_tree(false, "World item collapsed", SemanticLive::Polite);
                set_focus(&mut self.tree, WORLD_ITEM);
            }
            (
                PROJECT_NAME,
                SemanticAction::SetValue | SemanticAction::ReplaceSelectedText,
                Some(PlatformAccessibilityActionData::Text(value)),
            ) => {
                set_value(&mut self.tree, PROJECT_NAME, value.clone());
                set_status(&mut self.tree, "Project name updated", SemanticLive::Polite);
                set_focus(&mut self.tree, PROJECT_NAME);
            }
            (target, SemanticAction::Focus, None) => set_focus(&mut self.tree, target),
            _ => {}
        }
    }

    fn finish(&mut self, context: &mut PlatformContext<'_>) -> Result<(), Box<dyn Error>> {
        if self.report_written {
            context.exit();
            return Ok(());
        }
        let elapsed = self.started.elapsed();
        let evidence_status = if self.build_activation_observed {
            "Inconclusive"
        } else {
            "NotRun"
        };
        let report = AccessibilityReviewReport {
            schema: REPORT_SCHEMA,
            runner_status: "Pass",
            evidence_status,
            review_status: "AwaitingHumanReview",
            screen_reader_qualification: false,
            source_checkpoint: self.source.checkpoint.clone(),
            source_state: self.source.state.as_str(),
            source_provenance_verification: "CallerDeclaredNotVerified",
            promotion_eligibility: if self.source.state == SourceState::WorkingTree {
                "NotEligibleWorkingTree"
            } else {
                "NotEligiblePendingHumanAndCrossPlatformReview"
            },
            requirement_ids: ["REQ-UI-001", "REQ-UI-002"],
            work_package_id: "WP-UI-005",
            adapter_projection: if self.adapter_projected {
                "Pass"
            } else {
                "NotRun"
            },
            actual_assistive_action_count: self.actions.len(),
            dropped_assistive_action_count: self.dropped_actions,
            build_activation_observed: self.build_activation_observed,
            actions: self.actions.clone(),
            rejected_actions: self.rejected_actions.clone(),
            dropped_rejected_action_count: self.dropped_rejections,
            timeout_seconds: self.timeout.as_secs(),
            elapsed_milliseconds: duration_millis(elapsed),
            evidence_directory: ".",
            required_human_checks: [
                "Confirm the root dialog and Project name field are announced in reading order.",
                "Confirm the project value and editable state are announced.",
                "Confirm World item expanded or collapsed state is announced after the action.",
                "Confirm Build project is announced as a button and activation updates the live status.",
                "Confirm focus remains visible and recoverable through keyboard and screen-reader navigation.",
            ],
            limits: [
                "Only actions delivered by the native AccessKit/winit adapter are recorded.",
                "The runner cannot observe speech output or approve the required human checks.",
                "No action before timeout is NotRun, not a fabricated accessibility failure or pass.",
                "One local assistive client cannot establish cross-platform accessibility qualification.",
            ],
        };
        write_evidence_json(
            self.evidence_directory.join("accessibility-review.json"),
            &report,
        )?;
        println!(
            "Meridian accessibility review: actions={} build_activation={} evidence={} at {}",
            self.actions.len(),
            self.build_activation_observed,
            evidence_status,
            self.evidence_directory.display()
        );
        self.report_written = true;
        context.exit();
        Ok(())
    }

    fn handle_event(
        &mut self,
        event: &PlatformEvent,
        sequence: u64,
        monotonic_nanoseconds: u64,
        context: &mut PlatformContext<'_>,
    ) -> Result<(), Box<dyn Error>> {
        match event {
            PlatformEvent::WindowCreated { .. } => {
                if let Some(window) = context.window() {
                    window.set_visible(true);
                    window.request_focus();
                }
                context.request_redraw();
            }
            PlatformEvent::RedrawRequested => {
                self.adapter_projected = true;
                if self.build_activation_observed || self.started.elapsed() >= self.timeout {
                    self.finish(context)?;
                } else {
                    context.request_redraw_after(POLL_INTERVAL);
                }
            }
            PlatformEvent::AccessibilityAction(request) => {
                self.apply_action(request, sequence, monotonic_nanoseconds);
                context.request_redraw();
            }
            PlatformEvent::AccessibilityRejected(error) => {
                if self.rejected_actions.len() < MAX_RECORDED_REJECTIONS {
                    self.rejected_actions
                        .push(sanitize_detail(&error.to_string()));
                } else {
                    self.dropped_rejections = self.dropped_rejections.saturating_add(1);
                }
                context.request_redraw();
            }
            PlatformEvent::CloseRequested => self.finish(context)?,
            _ => {}
        }
        Ok(())
    }
}

impl PlatformApplication for AccessibilityReviewRunner {
    fn on_event(&mut self, event: PlatformEvent, context: &mut PlatformContext<'_>) {
        if let Err(error) = self.handle_event(&event, 0, 0, context) {
            self.fail(error.to_string(), context);
        }
    }

    fn on_event_envelope(
        &mut self,
        envelope: PlatformEventEnvelope,
        context: &mut PlatformContext<'_>,
    ) {
        if let Err(error) = self.handle_event(
            &envelope.event,
            envelope.metadata.sequence,
            envelope.metadata.monotonic_ns.get(),
            context,
        ) {
            self.fail(error.to_string(), context);
        }
    }

    fn accessibility_tree(&self) -> Option<SemanticTree> {
        Some(self.tree.clone())
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

fn semantic_tree(expanded: bool, status: &str, live: SemanticLive) -> SemanticTree {
    let root = review_root();

    let mut project_name = semantic_node(
        PROJECT_NAME,
        Some(ROOT),
        SemanticRole::TextInput,
        "Project name",
        rect(24.0, 28.0, 280.0, 36.0),
    );
    project_name.description = Some("Editable Creator project name".to_owned());
    project_name.command = Some("project.rename".to_owned());
    project_name.actions = vec![
        SemanticAction::Focus,
        SemanticAction::SetValue,
        SemanticAction::ReplaceSelectedText,
    ];
    project_name.value = Some("Accessibility Fixture".to_owned());
    project_name.focused = true;
    project_name.relationships.described_by = vec![STATUS];

    let world_tree = semantic_node(
        WORLD_TREE,
        Some(ROOT),
        SemanticRole::Tree,
        "World hierarchy",
        rect(24.0, 84.0, 280.0, 88.0),
    );

    let mut world_item = semantic_node(
        WORLD_ITEM,
        Some(WORLD_TREE),
        SemanticRole::TreeItem,
        "Environment",
        rect(36.0, 96.0, 240.0, 32.0),
    );
    world_item.description = Some("One expandable world hierarchy item".to_owned());
    world_item.command = Some(
        if expanded {
            "hierarchy.collapse"
        } else {
            "hierarchy.expand"
        }
        .to_owned(),
    );
    world_item.actions = vec![
        SemanticAction::Focus,
        if expanded {
            SemanticAction::Collapse
        } else {
            SemanticAction::Expand
        },
    ];
    world_item.state.expanded = expanded;

    let mut build = semantic_node(
        BUILD,
        Some(ROOT),
        SemanticRole::Button,
        "Build project",
        rect(336.0, 28.0, 152.0, 44.0),
    );
    build.description = Some("Activates a typed build proposal in this fixture".to_owned());
    build.command = Some("build.start".to_owned());
    build.actions = vec![SemanticAction::Focus, SemanticAction::Activate];
    build.relationships.described_by = vec![STATUS];

    let mut progress = semantic_node(
        PROGRESS,
        Some(ROOT),
        SemanticRole::ProgressIndicator,
        "Build progress",
        rect(336.0, 92.0, 240.0, 28.0),
    );
    progress.value = Some("Not started".to_owned());
    progress.relationships.described_by = vec![STATUS];

    let mut status_node = semantic_node(
        STATUS,
        Some(ROOT),
        SemanticRole::LiveRegion,
        "Accessibility review status",
        rect(336.0, 140.0, 280.0, 56.0),
    );
    status_node.value = Some(status.to_owned());
    status_node.live = live;

    SemanticTree {
        root: Some(ROOT),
        focus: Some(PROJECT_NAME),
        nodes: vec![
            root,
            project_name,
            world_tree,
            world_item,
            build,
            progress,
            status_node,
        ],
    }
}

fn review_root() -> SemanticNode {
    let mut root = semantic_node(
        ROOT,
        None,
        SemanticRole::Dialog,
        "Meridian UI accessibility review",
        rect(0.0, 0.0, 640.0, 360.0),
    );
    root.description = Some(
        "Bounded native review fixture; use the active screen reader to inspect and activate controls"
            .to_owned(),
    );
    root
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> UiRect {
    UiRect::new(UiPoint { x, y }, UiSize::new(width, height))
}

fn set_focus(tree: &mut SemanticTree, target: UiNodeId) {
    tree.focus = Some(target);
    for node in &mut tree.nodes {
        node.focused = node.id == target;
    }
}

fn set_value(tree: &mut SemanticTree, target: UiNodeId, value: String) {
    if let Some(node) = tree.nodes.iter_mut().find(|node| node.id == target) {
        node.value = Some(value);
    }
}

fn set_status(tree: &mut SemanticTree, status: &str, live: SemanticLive) {
    if let Some(node) = tree.nodes.iter_mut().find(|node| node.id == STATUS) {
        node.value = Some(status.to_owned());
        node.live = live;
    }
}

fn world_item_expanded(tree: &SemanticTree) -> bool {
    tree.nodes
        .iter()
        .find(|node| node.id == WORLD_ITEM)
        .is_some_and(|node| node.state.expanded)
}

fn semantic_name(tree: &SemanticTree, target: UiNodeId) -> &str {
    tree.nodes
        .iter()
        .find(|node| node.id == target)
        .map_or("Unknown Meridian node", |node| node.name.as_str())
}

const fn semantic_action_name(action: SemanticAction) -> &'static str {
    match action {
        SemanticAction::Activate => "Activate",
        SemanticAction::Focus => "Focus",
        SemanticAction::Expand => "Expand",
        SemanticAction::Collapse => "Collapse",
        SemanticAction::Increment => "Increment",
        SemanticAction::Decrement => "Decrement",
        SemanticAction::ReplaceSelectedText => "ReplaceSelectedText",
        SemanticAction::SetValue => "SetValue",
        SemanticAction::ScrollIntoView => "ScrollIntoView",
        SemanticAction::ShowContextMenu => "ShowContextMenu",
    }
}

const fn action_data_name(data: Option<&PlatformAccessibilityActionData>) -> &'static str {
    match data {
        None => "None",
        Some(PlatformAccessibilityActionData::Text(_)) => "TextRedacted",
        Some(PlatformAccessibilityActionData::Numeric(_)) => "NumericRedacted",
        Some(PlatformAccessibilityActionData::Custom(_)) => "CustomRedacted",
    }
}

fn runner_options_from_values(values: Vec<std::ffi::OsString>) -> Result<RunnerOptions, String> {
    let mut values = values.into_iter();
    let mut evidence_directory = None;
    let mut timeout = Duration::from_secs(DEFAULT_TIMEOUT_SECONDS);
    let mut timeout_supplied = false;
    while let Some(argument) = values.next() {
        if argument == "--evidence" {
            if evidence_directory.is_some() {
                return Err("--evidence may be supplied only once".to_owned());
            }
            let value = values
                .next()
                .ok_or_else(|| "--evidence requires a path".to_owned())?;
            let path = PathBuf::from(value);
            if path.as_os_str().is_empty() {
                return Err("--evidence path cannot be empty".to_owned());
            }
            evidence_directory = Some(path);
        } else if argument == "--timeout-seconds" {
            if timeout_supplied {
                return Err("--timeout-seconds may be supplied only once".to_owned());
            }
            let value = values
                .next()
                .ok_or_else(|| "--timeout-seconds requires an integer".to_owned())?;
            let seconds = value
                .to_string_lossy()
                .parse::<u64>()
                .map_err(|_| "--timeout-seconds requires an integer".to_owned())?;
            if seconds == 0 || seconds > MAX_TIMEOUT_SECONDS {
                return Err(format!(
                    "--timeout-seconds must be within 1..={MAX_TIMEOUT_SECONDS}"
                ));
            }
            timeout = Duration::from_secs(seconds);
            timeout_supplied = true;
        } else {
            return Err(format!("unknown argument: {}", argument.to_string_lossy()));
        }
    }
    Ok(RunnerOptions {
        evidence_directory: evidence_directory
            .ok_or_else(|| "--evidence PATH is required".to_owned())?,
        timeout,
    })
}

fn source_provenance_from_environment() -> Result<SourceProvenance, String> {
    let state = match std::env::var("MERIDIAN_SOURCE_STATE").as_deref() {
        Ok("working-tree") => SourceState::WorkingTree,
        Ok("clean-commit") => SourceState::CleanCommit,
        Ok(value) => {
            return Err(format!(
                "MERIDIAN_SOURCE_STATE must be working-tree or clean-commit, got {value:?}"
            ))
        }
        Err(_) => return Err("MERIDIAN_SOURCE_STATE is required".to_owned()),
    };
    let checkpoint = std::env::var("MERIDIAN_SOURCE_CHECKPOINT")
        .map_err(|_| "MERIDIAN_SOURCE_CHECKPOINT is required".to_owned())?;
    validate_checkpoint(&checkpoint)?;
    Ok(SourceProvenance { checkpoint, state })
}

fn validate_checkpoint(checkpoint: &str) -> Result<(), String> {
    if checkpoint.is_empty() || checkpoint.len() > 96 {
        return Err("source checkpoint must contain 1..=96 characters".to_owned());
    }
    if !checkpoint
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err("source checkpoint must be path-free ASCII metadata".to_owned());
    }
    Ok(())
}

fn prepare_evidence_directory(path: &Path) -> Result<(), Box<dyn Error>> {
    if path.exists() && fs::read_dir(path)?.next().transpose()?.is_some() {
        return Err(format!("evidence directory is not empty: {}", path.display()).into());
    }
    fs::create_dir_all(path)?;
    Ok(())
}

fn sanitize_detail(detail: &str) -> String {
    detail
        .chars()
        .filter(|character| !character.is_control() || *character == ' ')
        .take(MAX_FAILURE_DETAIL_CHARS)
        .collect()
}

fn duration_millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = runner_options_from_values(std::env::args_os().skip(1).collect())?;
    let source = source_provenance_from_environment()?;
    prepare_evidence_directory(&options.evidence_directory)?;
    let tree = semantic_tree(false, "Screen-reader review ready", SemanticLive::Polite);
    tree.validate()?;
    println!(
        "Meridian accessibility review evidence: {} (timeout {}s)",
        options.evidence_directory.display(),
        options.timeout.as_secs()
    );
    let failure = Arc::new(Mutex::new(None));
    run(
        PlatformConfig {
            title: "Meridian UI Accessibility Review".to_owned(),
            initial_size: WindowSize::new(640, 360),
            resizable: false,
            visible: true,
            ..PlatformConfig::default()
        },
        AccessibilityReviewRunner::new(
            options.evidence_directory,
            source,
            options.timeout,
            Arc::clone(&failure),
        ),
    )?;
    if let Some(message) = failure
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
    {
        return Err(message.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_tree_has_reading_order_values_relationships_actions_and_live_status() {
        let tree = semantic_tree(false, "Ready", SemanticLive::Polite);
        tree.validate().expect("review tree validates");
        assert_eq!(tree.root, Some(ROOT));
        assert_eq!(tree.focus, Some(PROJECT_NAME));
        assert_eq!(tree.nodes.len(), 7);
        assert!(tree.nodes[1].relationships.described_by.contains(&STATUS));
        assert_eq!(
            tree.nodes[3].actions,
            vec![SemanticAction::Focus, SemanticAction::Expand]
        );
        assert_eq!(
            tree.nodes[4].actions,
            vec![SemanticAction::Focus, SemanticAction::Activate]
        );
        assert_eq!(tree.nodes[6].live, SemanticLive::Polite);
    }

    #[test]
    fn arguments_are_bounded_and_require_explicit_evidence() {
        assert!(runner_options_from_values(Vec::new()).is_err());
        assert!(runner_options_from_values(vec![
            "--evidence".into(),
            "target/review".into(),
            "--timeout-seconds".into(),
            "0".into(),
        ])
        .is_err());
        assert_eq!(
            runner_options_from_values(vec![
                "--evidence".into(),
                "target/review".into(),
                "--timeout-seconds".into(),
                "30".into(),
            ]),
            Ok(RunnerOptions {
                evidence_directory: PathBuf::from("target/review"),
                timeout: Duration::from_secs(30),
            })
        );
    }

    #[test]
    fn action_records_redact_payloads() {
        assert_eq!(
            action_data_name(Some(&PlatformAccessibilityActionData::Text(
                "private".to_owned()
            ))),
            "TextRedacted"
        );
        assert_eq!(semantic_action_name(SemanticAction::Activate), "Activate");
    }
}
