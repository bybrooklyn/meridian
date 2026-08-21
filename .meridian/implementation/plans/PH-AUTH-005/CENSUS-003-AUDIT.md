# WP-V1-CENSUS-003 audit sample (round 2)

Seed: `source-tree:e41e665ddb78` from `source_tree_checkpoint`. Regenerated after the review escalated 124
mappings; the previous sample certified 49 ids of which 16 had no test that served them.

## Stratum A — all 37 crate rows

| crate | disposition | owner / escalation |
|---|---|---|
| `meridian-alluvium` | retain | — |
| `meridian-asset-tools` | retain | — |
| `meridian-assets` | retain | — |
| `meridian-audio` | remove | — |
| `meridian-basalt` | remove | — |
| `meridian-benchmark` | retain | — |
| `meridian-build` | retain | — |
| `meridian-core` | retain | — |
| `meridian-diagnostics` | retain | — |
| `meridian-ecs` | — | OD-021 |
| `meridian-editor` | — | OD-021 |
| `meridian-editor-core` | — | OD-021 |
| `meridian-input` | retain | — |
| `meridian-isobar` | remove | — |
| `meridian-modeler` | — | OD-021 |
| `meridian-package` | retain | — |
| `meridian-physics` | — | OD-014 |
| `meridian-platform` | — | OD-021 |
| `meridian-render-graph` | retain | — |
| `meridian-renderer` | retain | — |
| `meridian-rhi` | retain | — |
| `meridian-rt` | retain | — |
| `meridian-save` | — | OD-021 |
| `meridian-shader-tools` | — | OD-015 |
| `meridian-spec` | retain | — |
| `meridian-streaming` | — | OD-021 |
| `meridian-tasks` | retain | — |
| `meridian-ui` | — | OD-016 |
| `meridian-ui-core` | retain | — |
| `meridian-ui-editor` | retain | — |
| `meridian-ui-render` | retain | — |
| `meridian-ui-runtime` | retain | — |
| `meridian-ui-semantics` | — | OD-017 |
| `meridian-ui-text` | — | OD-021 |
| `meridian-vegetation` | remove | — |
| `meridian-world` | — | OD-021 |
| `meridian-world-tools` | retain | — |

## Stratum B — one row per (file, module) group of ≥10 tests (23)

| test | owner |
|---|---|
| `evidence_status_is_non_promoting_for_declared_source_prove` | PROTECT-009 |
| `fixture_manifest_rejects_duplicate_cases_and_invalid_metad` | UI-008 |
| `metadata_process_failure_returns_a_typed_diagnostic` | BUILD-001 |
| `argument_parser_accepts_bounded_smoke_configuration` | ESC OD-021 |
| `recovery_round_trip_rebuilds_source_without_history` | AUTHOR-001 |
| `every_test_row_has_a_module` | SPEC-003 |
| `retired_v05_identifiers_are_segregated` | SPEC-002 |
| `one_heading_can_declare_a_stem_and_its_letter_suffixed_sib` | SPEC-002 |
| `stale_phase_refs_are_rejected` | SPEC-003 |
| `world_placement_scales_with_the_actual_canvas_at_desktop_w` | UI-005 |
| `invalid_tree_and_failed_mutation_roll_back_without_losing_` | ED-AOT-003 |
| `load_request_cancels_before_decode_and_during_decode` | AUTHOR-003 |
| `history_retains_pipeline_state_for_benchmark_export` | PROTECT-014 |
| `split_edge_preserves_lineage_and_updates_face_boundary` | ESC OD-021 |
| `rejected_stale_unknown_and_malformed_actions_preserve_proj` | ESC OD-017 |
| `platform_context_coalesces_delayed_redraw_without_blocking` | ESC OD-021 |
| `atlas_uvs_use_final_height_and_half_texel_gutters` | UI-008 |
| `curved_path_is_flattened_with_bounded_structural_coverage` | UI-006 |
| `capture_layout_enforces_alignment_dimensions_bytes_and_zer` | PEN-007 |
| `professional_controls_publish_roles_states_and_keyboard_dr` | UI-004 |
| `nested_scopes_and_bounded_backdrop_validate` | UI-005 |
| `focus_owning_transient_does_not_cancel_a_background_proper` | UI-004 |
| `cache_activity_accumulates_saturating_phase_durations` | ESC OD-021 |

## Stratum C — every one of the 33 surviving ids, with the heading and a test

| id | heading | test |
|---|---|---|
| `ALLU-001` | Task-first normal authoring with graph/source unde | `scalar_evaluation_is_deterministic_and_cacheab` |
| `ALLU-002` | Manual edits automatically become non-destructive  | `overrides_report_applied_conflicted_and_orphan` |
| `AUTHOR-001` | Stable source and recovery `AUTHOR-001` — *Normati | `checkpoint_is_retained_at_bounded_interval` |
| `AUTHOR-003` | Central authoritative asset catalog `AUTHOR-003` — | `file_pack_reader_loads_a_real_indexed_range` |
| `AUTHOR-004` | Isolated importer execution `AUTHOR-004` — *Normat | `invalid_geometry_and_cache_authority_are_rejec` |
| `AUTHOR-005` | Artifact and blob identity `AUTHOR-005` — *Normati | `source_and_artifact_identity_are_deterministic` |
| `BUILD-001` | Build experience `BUILD-001` — *Normative* | `cargo_process_failure_is_bounded_and_redacted` |
| `BUILD-002` | Toolchain acquisition and provenance `BUILD-002` — | `cargo_environment_accepts_bounded_toolchain_se` |
| `ED-AOT-002` | Persistent top workspace strip `ED-AOT-002` | `shell_settings_and_favorites_are_contextual_an` |
| `ED-AOT-003` | Three-column World workspace with viewport priorit | `workspace_history_undo_redo_branch_and_reset_u` |
| `IMPL-STATE-001` | Persistent implementation state `IMPL-STATE-001` — | `a_stale_digest_is_reported_with_both_values` |
| `INP-001` | Constrained input semantics; gameplay stays out of | `scrolling_preserves_line_pixel_and_gesture_pha` |
| `INP-007` | Portable player binding profile plus optional devi | `gamepad_events_drive_bindings_and_disconnect_c` |
| `MODELER-003` | Exact source and game mesh `MODELER-003` — *Normat | `preview_is_derived_and_cannot_mutate_source` |
| `PEN-002` | Primary 3D architecture `PEN-002` — *Normative* | `environment_light_validates_diffuse_intensity` |
| `PEN-006` | Extension seams `PEN-006` — *Normative* | `rejects_duplicate_names_and_same_pass_read_wri` |
| `PEN-007` | Backend portfolio and Vulkan requirement `PEN-007` | `buffer_write_validation_rejects_misaligned_and` |
| `PKG-001` | Small platform-native built product `PKG-001` — *N | `malformed_version_truncation_hash_and_duplicat` |
| `PROTECT-009` | Behavioral and impossible-state evidence `PROTECT- | `review_case_has_exact_bounded_two_x_surface` |
| `PROTECT-014` | Diagnostics and explainability `PROTECT-014` — *No | `history_is_bounded_and_summarizes_retained_sam` |
| `RUNTIME-001` | Renderer-free architecture `RUNTIME-001` — *Normat | `runtime_records_frame_diagnostics_and_accepts_` |
| `RUNTIME-002` | Aggressively automatic async `RUNTIME-002` — *Norm | `panicking_tasks_report_their_id` |
| `RUNTIME-007` | Structured concurrency `RUNTIME-007` — *Normative* | `correlated_submission_preserves_context_withou` |
| `SAVE-003` | Typed language-neutral persistence schemas `SAVE-0 | `schema_aware_transaction_round_trips_and_appli` |
| `SPEC-002` | Single-root specoment plus derived projections `SP | `check_command_and_github_output_are_supported` |
| `SPEC-003` | Existing code treatment `SPEC-003` — *Normative* | `broken_links_and_fences_are_rejected` |
| `UI-003` | Styling `UI-003` — *Normative direction* | `authored_delta_reports_style_component_and_sou` |
| `UI-004` | Components and binding `UI-004` — *Normative* | `aggregate_route_limit_rolls_back_deep_dispatch` |
| `UI-005` | Layout `UI-005` — *Normative direction* | `recovered_frame_diagnostics_keep_the_snapshot_` |
| `UI-006` | Game UI `UI-006` — *Normative* | `text_raster_is_clipped_to_its_retained_text_bo` |
| `UI-007` | UI animation `UI-007` — *Normative* | `shared_element_motion_handoffs_between_distinc` |
| `UI-008` | UI renderer qualification `UI-008` — *Normative* | `preflight_failure_keeps_output_paths_relative` |
| `WORLD-002` | Unified World Environment authoring `WORLD-002` —  | `missing_visual_and_duplicate_stable_ids_are_re` |

## Stratum D — sections with more than one outcome

The previous sample drew one row from each of eight sections that are 100% `retain`; a draw from
a uniform section cannot fail. These are the sections that can discriminate.

| section | row | disposition |
|---|---|---|
| `public_types` | `UiTextInputState` | retain |
| `public_types` | `AssetDatabaseSnapshot` | retain |
| `public_types` | `scan` | refactor |
| `public_types` | `MAX_ASSISTIVE_ACTION_BINDINGS` | refactor |
| `crates` | `meridian-modeler` | ESC OD-021 |
| `crates` | `meridian-rt` | retain |
| `crates` | `meridian-rhi` | retain |
| `crates` | `meridian-physics` | ESC OD-014 |
| `tests` | `secondary_pointer_release_routes_an_explicit` | ESC OD-021 |
| `tests` | `nested_layers_preserve_parent_order_and_rese` | retain |
| `tests` | `duplicate_render_instance_ids_fail_the_whole` | ESC OD-021 |
| `tests` | `first_snapshot_uploads_sorted_instances_and_` | retain |

## Stratum E — seeded random remainder (20 rows)

The previous sample quoted a seed and drew nothing from it.

| test | owner |
|---|---|
| `reduced_motion_snaps_physical_presentation_to_authoritativ` | UI-007 |
| `settings_preferences_persist_locally_and_apply_to_retained` | ESC OD-021 |
| `cascade_layout_uses_monotonic_practical_splits_and_selects` | PEN-002 |
| `missing_recent_location_is_explicit_transactional_and_canc` | ESC OD-021 |
| `rejects_unknown_resource_identifier` | PEN-002 |
| `invalid_unnamed_focusable_node_is_rejected` | UI-004 |
| `aggregate_effect_bytes_roll_back_repeated_large_commands` | UI-004 |
| `zero_delta_still_publishes_a_render_snapshot_for_the_frame` | RUNTIME-001 |
| `image_mesh_and_geometry_bounds_are_typed_before_growth` | UI-008 |
| `duplicate_ids_are_rejected_without_replacing_existing_meta` | AUTHOR-003 |
| `a_longer_number_does_not_yield_a_shorter_identifier` | SPEC-002 |
| `root_declared_items_exclude_impl_methods_and_private_modul` | SPEC-003 |
| `duplicate_component_delta_is_rejected_without_mutating_sta` | ESC OD-021 |
| `complete_panel_contract_has_unique_ids_and_accessible_comm` | ESC OD-017 |
| `a_stale_digest_is_reported_with_both_values` | IMPL-STATE-001 |
| `airborne_gravity_is_deterministic_and_landing_clears_verti` | ESC OD-014 |
| `assistive_scroll_into_view_moves_the_nearest_scroll_ancest` | UI-004 |
| `compact_world_context_yields_rail_before_primary_work_surf` | ESC OD-021 |
| `layer_inside_empty_parent_clip_never_becomes_an_unscissore` | UI-008 |
| `failed_empty_projection_keeps_the_last_accepted_identity_a` | ESC OD-017 |
