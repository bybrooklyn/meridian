# WP-V1-CENSUS-003 audit sample

Seed: `source_tree_checkpoint` = `source-tree:a24a...` — the draw is reproducible, so a sample
chosen after the mapping cannot be passed off as a sample.

## Stratum A — all 37 crate rows

| crate | disposition | owner / escalation | next phase |
|---|---|---|---|
| `meridian-alluvium` | retain | — | PH-AUTH-006 |
| `meridian-asset-tools` | retain | — | PH-AUTH-006 |
| `meridian-assets` | retain | — | PH-AUTH-006 |
| `meridian-audio` | remove | — | PH-AUTH-006 |
| `meridian-basalt` | remove | — | PH-AUTH-006 |
| `meridian-benchmark` | retain | — | PH-AUTH-006 |
| `meridian-build` | retain | — | PH-AUTH-006 |
| `meridian-core` | retain | — | PH-AUTH-006 |
| `meridian-diagnostics` | retain | — | PH-AUTH-006 |
| `meridian-ecs` | retain | — | PH-AUTH-006 |
| `meridian-editor` | retain | — | PH-AUTH-006 |
| `meridian-editor-core` | retain | — | PH-AUTH-006 |
| `meridian-input` | retain | — | PH-AUTH-006 |
| `meridian-isobar` | remove | — | PH-AUTH-006 |
| `meridian-modeler` | retain | — | PH-AUTH-006 |
| `meridian-package` | retain | — | PH-AUTH-006 |
| `meridian-physics` | — | OD-014 | — |
| `meridian-platform` | retain | — | PH-AUTH-006 |
| `meridian-render-graph` | retain | — | PH-AUTH-006 |
| `meridian-renderer` | retain | — | PH-AUTH-006 |
| `meridian-rhi` | retain | — | PH-AUTH-006 |
| `meridian-rt` | retain | — | PH-AUTH-006 |
| `meridian-save` | retain | — | PH-AUTH-006 |
| `meridian-shader-tools` | — | OD-015 | — |
| `meridian-spec` | retain | — | PH-AUTH-006 |
| `meridian-streaming` | retain | — | PH-AUTH-006 |
| `meridian-tasks` | retain | — | PH-AUTH-006 |
| `meridian-ui` | — | OD-016 | — |
| `meridian-ui-core` | retain | — | PH-AUTH-006 |
| `meridian-ui-editor` | retain | — | PH-AUTH-006 |
| `meridian-ui-render` | retain | — | PH-AUTH-006 |
| `meridian-ui-runtime` | retain | — | PH-AUTH-006 |
| `meridian-ui-semantics` | — | OD-017 | — |
| `meridian-ui-text` | retain | — | PH-AUTH-006 |
| `meridian-vegetation` | remove | — | PH-AUTH-006 |
| `meridian-world` | retain | — | PH-AUTH-006 |
| `meridian-world-tools` | retain | — | PH-AUTH-006 |

## Stratum B — one row per (file, module) group of ≥10 tests (23 groups)

| test | owner | requirement heading |
|---|---|---|
| `nearest_rank_is_deterministic_for_small_samples` | UI-008 | UI renderer qualification `UI-008` — *Normative* |
| `fixture_profile_key_is_stable_and_path_safe` | UI-008 | UI renderer qualification `UI-008` — *Normative* |
| `cargo_stderr_reuses_the_existing_service_bound` | BUILD-001 | Build experience `BUILD-001` — *Normative* |
| `headless_ms01_builds_streams_activates_and_recovers` | EDUX-001 | Curated task workspaces with optional freeform customization `EDUX-001 |
| `reimport_is_typed_and_undoable_without_changing_source_i` | AUTHOR-001 | Stable source and recovery `AUTHOR-001` — *Normative* |
| `every_test_row_has_a_module` | SPEC-003 | Existing code treatment `SPEC-003` — *Normative* |
| `a_v05_research_gate_cited_by_v1_prose_is_retired_not_und` | SPEC-002 | Single-root specoment plus derived projections `SPEC-002` — *Normative |
| `a_longer_number_does_not_yield_a_shorter_identifier` | SPEC-002 | Single-root specoment plus derived projections `SPEC-002` — *Normative |
| `stale_v04_suite_versions_are_rejected` | SPEC-002 | Single-root specoment plus derived projections `SPEC-002` — *Normative |
| `all_declared_project_workspaces_keep_their_shell_rows_an` | ESC OD-017 | — |
| `invalid_tree_and_failed_mutation_roll_back_without_losin` | ED-AOT-003 | Three-column World workspace with viewport priority `ED-AOT-003` |
| `failed_assets_expose_a_visible_placeholder` | AUTHOR-003 | Central authoritative asset catalog `AUTHOR-003` — *Normative* |
| `scales_stutter_thresholds_for_one_twenty_fps` | PROTECT-014 | Diagnostics and explainability `PROTECT-014` — *Normative* |
| `source_writes_atomically_to_a_regular_project_path` | MODELER-003 | Exact source and game mesh `MODELER-003` — *Normative* |
| `reactivation_handler_returns_the_latest_complete_tree` | ESC OD-017 | — |
| `ime_cursor_area_rejects_untrusted_geometry_and_retains_l` | PLATFORM-001 | Meridian 1.0 required platform floor `PLATFORM-001` — *Normative* |
| `empty_clip_scope_skips_children_and_resumes_after_pop` | UI-008 | UI renderer qualification `UI-008` — *Normative* |
| `direct_renderer_state_drops_surface_caches_without_losin` | UI-006 | Game UI `UI-006` — *Normative* |
| `depth_formats_and_stencil_contract_are_backend_neutral` | PEN-007 | Backend portfolio and Vulkan requirement `PEN-007` — *Normative* |
| `retained_nodes_select_locked_typography_by_meridian_role` | UI-004 | Components and binding `UI-004` — *Normative* |
| `nested_scopes_and_bounded_backdrop_validate` | UI-005 | Layout `UI-005` — *Normative direction* |
| `post_event_validation_cannot_bypass_the_aggregate_effect` | UI-004 | Components and binding `UI-004` — *Normative* |
| `shaping_cache_evicts_fifo_within_shared_count_and_byte_b` | UI-002 | `.mui` source `UI-002` — *Normative* |

## Stratum C — id coverage: every one of the 49 distinct ids used, with one assertion

| requirement | heading | a test that serves it |
|---|---|---|
| `ALLU-001` | Task-first normal authoring with graph/source underneath | `budget_and_cancellation_are_typed_failures` |
| `ALLU-002` | Manual edits automatically become non-destructive overri | `overrides_report_applied_conflicted_and_orphaned_wit` |
| `AUTHOR-001` | Stable source and recovery `AUTHOR-001` — *Normative* | `reimport_is_typed_and_undoable_without_changing_sour` |
| `AUTHOR-003` | Central authoritative asset catalog `AUTHOR-003` — *Norm | `manifest_canonicalization_is_independent_of_input_or` |
| `AUTHOR-004` | Isolated importer execution `AUTHOR-004` — *Normative* | `transaction_rejects_duplicates_cancel_and_escape_wit` |
| `AUTHOR-005` | Artifact and blob identity `AUTHOR-005` — *Normative* | `artifact_store_detects_a_corrupted_existing_object` |
| `AUTHOR-010` | Automatic stable scene partitioning `AUTHOR-010` — *Norm | `activation_queue_enforces_item_and_byte_budgets` |
| `BUILD-001` | Build experience `BUILD-001` — *Normative* | `cargo_worker_start_failure_maps_to_a_typed_build_err` |
| `BUILD-002` | Toolchain acquisition and provenance `BUILD-002` — *Norm | `cargo_environment_accepts_bounded_toolchain_search_p` |
| `ED-AOT-002` | Persistent top workspace strip `ED-AOT-002` | `shell_panels_cycles_and_persists_the_active_workspac` |
| `ED-AOT-003` | Three-column World workspace with viewport priority `ED- | `creator_shell_uses_flat_bands_and_a_single_navigatio` |
| `EDUX-001` | Curated task workspaces with optional freeform customiza | `headless_ms01_builds_streams_activates_and_recovers` |
| `EDUX-002` | Baseline schema Inspector plus deliberate task interface | `project_store_create_does_not_replace_existing_autho` |
| `EXEC-009` | Debugger and profiler contract `EXEC-009` — *Normative d | `history_aggregates_retained_gpu_timings_for_benchmar` |
| `IMPL-STATE-001` | Persistent implementation state `IMPL-STATE-001` — *Norm | `a_missing_source_digest_is_reported` |
| `INP-001` | Constrained input semantics; gameplay stays out of Input | `scrolling_preserves_line_pixel_and_gesture_phase` |
| `INP-007` | Portable player binding profile plus optional device-loc | `opposing_bindings_cancel_axis_but_remain_active` |
| `MODELER-001` | Product direction `MODELER-001` — *Normative* | `semantic_undo_redo_and_recovery_restore_accepted_rev` |
| `MODELER-003` | Exact source and game mesh `MODELER-003` — *Normative* | `preview_is_derived_and_cannot_mutate_source` |
| `PEN-002` | Primary 3D architecture `PEN-002` — *Normative* | `compiles_resource_hazards_and_lifetimes` |
| `PEN-006` | Extension seams `PEN-006` — *Normative* | `rejects_duplicate_names_and_same_pass_read_write` |
| `PEN-007` | Backend portfolio and Vulkan requirement `PEN-007` — *No | `pipeline_config_validates_stencil_and_depth_combinat` |
| `PEN-021` | Visual-quality acceptance and vertical-slice doctrine `P | `timing_sample_preserves_frame_submission_pass_and_de` |
| `PKG-001` | Small platform-native built product `PKG-001` — *Normati | `malformed_version_truncation_hash_and_duplicates_are` |
| `PLATFORM-001` | Meridian 1.0 required platform floor `PLATFORM-001` — *N | `zero_size_is_detected_for_minimized_surface_handling` |
| `PROTECT-009` | Behavioral and impossible-state evidence `PROTECT-009` — | `failure_detail_is_bounded_and_path_safe` |
| `PROTECT-014` | Diagnostics and explainability `PROTECT-014` — *Normativ | `classifies_sixty_fps_stutter_thresholds` |
| `RUNTIME-001` | Renderer-free architecture `RUNTIME-001` — *Normative* | `one_second_produces_exactly_sixty_steps_without_peri` |
| `RUNTIME-002` | Aggressively automatic async `RUNTIME-002` — *Normative* | `panicking_tasks_report_their_id` |
| `RUNTIME-003` | Interactive-first startup contract `RUNTIME-003` — *Norm | `platform_context_records_redraw_and_exit_requests` |
| `RUNTIME-005` | One task contract with specialized executors `RUNTIME-00 | `duplicate_render_instance_ids_fail_the_whole_extract` |
| `RUNTIME-006` | Executor selection is automatic `RUNTIME-006` — *Normati | `fixed_schedule_runs_exactly_requested_steps` |
| `RUNTIME-007` | Structured concurrency `RUNTIME-007` — *Normative* | `dropping_pool_drains_submitted_work_before_shutdown` |
| `RUNTIME-008` | Deterministic gameplay-result barriers `RUNTIME-008` — * | `reset_clears_accumulation_and_sets_tick` |
| `SAVE-001` | Per-game Player Preferences/Profile is separate from gam | `version_mismatch_is_rejected_and_payload_limit_is_en` |
| `SAVE-003` | Typed language-neutral persistence schemas `SAVE-003` —  | `schema_aware_transaction_round_trips_and_applies_sta` |
| `SPEC-002` | Single-root specoment plus derived projections `SPEC-002 | `a_v05_research_gate_cited_by_v1_prose_is_retired_not` |
| `SPEC-003` | Existing code treatment `SPEC-003` — *Normative* | `every_axis_meets_its_floor` |
| `UI-002` | `.mui` source `UI-002` — *Normative* | `text_validation_completion_cut_and_paste_remain_type` |
| `UI-003` | Styling `UI-003` — *Normative direction* | `creator_activity_rails_keep_full_icon_slots_inside_t` |
| `UI-004` | Components and binding `UI-004` — *Normative* | `post_event_validation_cannot_bypass_the_aggregate_ef` |
| `UI-005` | Layout `UI-005` — *Normative direction* | `preferred_stack_and_grid_keep_controls_in_distinct_v` |
| `UI-006` | Game UI `UI-006` — *Normative* | `text_raster_is_clipped_to_its_retained_text_bounds` |
| `UI-007` | UI animation `UI-007` — *Normative* | `shared_element_motion_handoffs_between_distinct_cros` |
| `UI-008` | UI renderer qualification `UI-008` — *Normative* | `prepared_rect_vertices_use_snapped_physical_edges_at` |
| `UI-011` | Native nine-slice / nine-patch rendering `UI-011` — *Nor | `every_registered_icon_generates_bounded_runtime_geom` |
| `UI-SRC-001` | Compact brace/block `.mui` syntax `UI-SRC-001` — *Normat | `virtual_collection_contract_validates_only_realized_` |
| `WORLD-001` | World environment, terrain, weather, water, vegetation,  | `cell_membership_handles_boundaries_and_negative_coor` |
| `WORLD-002` | Unified World Environment authoring `WORLD-002` — *Norma | `missing_visual_and_duplicate_stable_ids_are_rejected` |

## Stratum D — one row per non-test judgement-bearing section

| section | row | disposition |
|---|---|---|
| `public_types` | `DropReason` | retain |
| `dependencies` | `wgpu` | retain |
| `features` | `None` | retain |
| `examples` | `ui_direct_qualification` | retain |
| `evidence_runners` | `ci.yml:73` | retain |
| `formats` | `ui-source` | retain |
| `generated_files` | `governance/generated/index.md` | retain |
| `ci_rows` | `ci.yml:governance` | retain |
