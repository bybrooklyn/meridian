#[cfg(not(feature = "accessibility"))]
fn main() {
    println!(
        "native_accessibility_smoke outcome=Unavailable native_adapter_projection=Unavailable surface_presentation=Unavailable adapter_reactivation=NotRun assistive_action=NotRun explicit_exit=false screen_reader_qualification=false reason=accessibility-feature-disabled"
    );
}

#[cfg(feature = "accessibility")]
mod enabled {
    use std::error::Error;
    use std::sync::{Arc, Mutex};

    use meridian_platform::{
        run, PlatformAccessibilityActionRequest, PlatformApplication, PlatformConfig,
        PlatformContext, PlatformError, PlatformErrorKind, PlatformEvent, WindowSize,
    };
    use meridian_ui_core::{SemanticRole, UiControlState, UiNodeId, UiPoint, UiRect, UiSize};
    use meridian_ui_semantics::{SemanticAction, SemanticLive, SemanticNode, SemanticTree};

    const ROOT: UiNodeId = UiNodeId::new(1);
    const PROJECT_NAME: UiNodeId = UiNodeId::new(2);
    const BUILD: UiNodeId = UiNodeId::new(3);
    const STATUS: UiNodeId = UiNodeId::new(4);
    const EXIT_AFTER_REDRAWS: u8 = 4;
    const SYNTHETIC_ACTION_LIMITATION: &str =
        "AccessKit/winit has no supported synthetic native assistive-client activation/action API";
    const REACTIVATION_LIMITATION: &str =
        "native reactivation is assistive-client driven and not observable through the Meridian application boundary";

    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    enum NativeOutcome {
        Presented,
        #[default]
        Unavailable,
    }

    #[derive(Clone, Debug, Default, PartialEq)]
    struct SmokeObservation {
        window: NativeOutcome,
        redraw: NativeOutcome,
        adapter_projection: NativeOutcome,
        assistive_action: Option<PlatformAccessibilityActionRequest>,
        rejected_action: Option<String>,
        explicit_exit_requested: bool,
    }

    struct NativeAccessibilitySmoke {
        tree: SemanticTree,
        redraws: u8,
        observation: Arc<Mutex<SmokeObservation>>,
    }

    impl NativeAccessibilitySmoke {
        fn new(observation: Arc<Mutex<SmokeObservation>>) -> Self {
            Self {
                tree: semantic_tree("Native projection ready", SemanticLive::Polite),
                redraws: 0,
                observation,
            }
        }

        fn observe(&self, update: impl FnOnce(&mut SmokeObservation)) {
            if let Ok(mut observation) = self.observation.lock() {
                update(&mut observation);
            }
        }

        fn apply_assistive_action(&mut self, request: &PlatformAccessibilityActionRequest) {
            if request.target == BUILD && request.action == SemanticAction::Activate {
                self.tree = semantic_tree("Build action accepted", SemanticLive::Assertive);
            } else if request.target == PROJECT_NAME && request.action == SemanticAction::Focus {
                self.tree.focus = Some(PROJECT_NAME);
            }
        }
    }

    impl PlatformApplication for NativeAccessibilitySmoke {
        fn on_event(&mut self, event: PlatformEvent, context: &mut PlatformContext<'_>) {
            match event {
                PlatformEvent::WindowCreated { .. } => {
                    self.observe(|observation| observation.window = NativeOutcome::Presented);
                    context.request_redraw();
                }
                PlatformEvent::RedrawRequested => {
                    self.redraws = self.redraws.saturating_add(1);
                    self.observe(|observation| observation.redraw = NativeOutcome::Presented);

                    if self.redraws == 1 {
                        self.tree = semantic_tree("Adapter cache refreshed", SemanticLive::Polite);
                        self.observe(|observation| {
                            observation.adapter_projection = NativeOutcome::Presented;
                        });
                    }

                    if self.redraws >= EXIT_AFTER_REDRAWS {
                        self.observe(|observation| observation.explicit_exit_requested = true);
                        context.exit();
                    } else {
                        context.request_redraw();
                    }
                }
                PlatformEvent::AccessibilityAction(request) => {
                    self.apply_assistive_action(&request);
                    self.observe(|observation| observation.assistive_action = Some(request));
                    context.request_redraw();
                }
                PlatformEvent::AccessibilityRejected(error) => {
                    self.observe(|observation| {
                        observation.rejected_action = Some(error.to_string());
                    });
                    context.request_redraw();
                }
                PlatformEvent::CloseRequested => {
                    self.observe(|observation| observation.explicit_exit_requested = true);
                    context.exit();
                }
                _ => {}
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
            live: SemanticLive::Off,
            collection_item: None,
            bounds,
            focused: false,
        }
    }

    fn semantic_tree(status: &str, live: SemanticLive) -> SemanticTree {
        let mut root = semantic_node(
            ROOT,
            None,
            SemanticRole::Dialog,
            "Meridian native accessibility smoke",
            UiRect::new(UiPoint::default(), UiSize::new(640.0, 360.0)),
        );
        root.description =
            Some("Native adapter projection smoke; not screen-reader qualification".to_owned());

        let mut project_name = semantic_node(
            PROJECT_NAME,
            Some(ROOT),
            SemanticRole::TextInput,
            "Project name",
            UiRect::new(UiPoint { x: 24.0, y: 28.0 }, UiSize::new(280.0, 36.0)),
        );
        project_name.description = Some("Name used by this bounded smoke".to_owned());
        project_name.command = Some("project.rename".to_owned());
        project_name.actions = vec![
            SemanticAction::Focus,
            SemanticAction::SetValue,
            SemanticAction::ReplaceSelectedText,
        ];
        project_name.value = Some("Native Accessibility Fixture".to_owned());
        project_name.focused = true;

        let mut build = semantic_node(
            BUILD,
            Some(ROOT),
            SemanticRole::Button,
            "Build project",
            UiRect::new(UiPoint { x: 24.0, y: 84.0 }, UiSize::new(152.0, 44.0)),
        );
        build.description = Some("Exercises one Meridian semantic action when invoked".to_owned());
        build.command = Some("build.start".to_owned());
        build.actions = vec![SemanticAction::Focus, SemanticAction::Activate];

        let mut status_node = semantic_node(
            STATUS,
            Some(ROOT),
            SemanticRole::LiveRegion,
            "Accessibility smoke status",
            UiRect::new(UiPoint { x: 24.0, y: 148.0 }, UiSize::new(360.0, 32.0)),
        );
        status_node.value = Some(status.to_owned());
        status_node.live = live;

        SemanticTree {
            root: Some(ROOT),
            focus: Some(PROJECT_NAME),
            nodes: vec![root, project_name, build, status_node],
        }
    }

    fn report(outcome: NativeOutcome, observation: &SmokeObservation, reason: Option<&str>) {
        let native_action = if observation.assistive_action.is_some() {
            "Presented"
        } else {
            "NotRun"
        };
        let rejected_action = observation.rejected_action.as_deref().unwrap_or("none");
        let reason = reason.unwrap_or(if observation.assistive_action.is_some() {
            "none"
        } else {
            SYNTHETIC_ACTION_LIMITATION
        });
        println!(
            "native_accessibility_smoke outcome={outcome:?} window={:?} redraw={:?} native_adapter_projection={:?} surface_presentation=Unavailable adapter_reactivation=NotRun reactivation_reason={REACTIVATION_LIMITATION:?} assistive_action={native_action} action_reason={reason:?} rejected_action={rejected_action:?} explicit_exit={} screen_reader_qualification=false",
            observation.window,
            observation.redraw,
            observation.adapter_projection,
            observation.explicit_exit_requested,
        );
    }

    pub fn main() -> Result<(), Box<dyn Error>> {
        let observation = Arc::new(Mutex::new(SmokeObservation::default()));
        let application = NativeAccessibilitySmoke::new(Arc::clone(&observation));
        let config = PlatformConfig {
            title: "Meridian Native Accessibility Smoke".to_owned(),
            initial_size: WindowSize::new(640, 360),
            resizable: false,
            ..PlatformConfig::default()
        };

        match run(config, application) {
            Ok(()) => {
                let observation = observation
                    .lock()
                    .map_err(|_| "native accessibility smoke observation was poisoned")?;
                let outcome = if observation.window == NativeOutcome::Presented
                    && observation.redraw == NativeOutcome::Presented
                    && observation.adapter_projection == NativeOutcome::Presented
                    && observation.explicit_exit_requested
                {
                    NativeOutcome::Presented
                } else {
                    NativeOutcome::Unavailable
                };
                report(outcome, &observation, None);
                Ok(())
            }
            Err(error)
                if matches!(
                    error.kind(),
                    PlatformErrorKind::EventLoopCreation | PlatformErrorKind::WindowCreation
                ) =>
            {
                let observation = observation
                    .lock()
                    .map_err(|_| "native accessibility smoke observation was poisoned")?;
                report(
                    NativeOutcome::Unavailable,
                    &observation,
                    Some(&error.to_string()),
                );
                Ok(())
            }
            Err(error) => Err(Box::<PlatformError>::new(error)),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn smoke_tree_has_focus_value_actions_and_live_region() {
            let tree = semantic_tree("Native projection ready", SemanticLive::Polite);

            tree.validate().expect("native smoke tree validates");
            assert_eq!(tree.root, Some(ROOT));
            assert_eq!(tree.focus, Some(PROJECT_NAME));
            assert_eq!(
                tree.nodes[1].value.as_deref(),
                Some("Native Accessibility Fixture")
            );
            assert!(tree.nodes[1].actions.contains(&SemanticAction::SetValue));
            assert!(tree.nodes[2].actions.contains(&SemanticAction::Activate));
            assert_eq!(tree.nodes[3].live, SemanticLive::Polite);
        }

        #[test]
        fn meridian_action_updates_the_live_status_without_changing_focus() {
            let observation = Arc::new(Mutex::new(SmokeObservation::default()));
            let mut smoke = NativeAccessibilitySmoke::new(observation);
            smoke.apply_assistive_action(&PlatformAccessibilityActionRequest {
                target: BUILD,
                action: SemanticAction::Activate,
                data: None,
            });

            assert_eq!(smoke.tree.focus, Some(PROJECT_NAME));
            assert_eq!(
                smoke.tree.nodes[3].value.as_deref(),
                Some("Build action accepted")
            );
            assert_eq!(smoke.tree.nodes[3].live, SemanticLive::Assertive);
        }
    }
}

#[cfg(feature = "accessibility")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    enabled::main()
}
