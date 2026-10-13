#!/usr/bin/env python3
"""Compose the frozen 0070 case plan (plan.json) from source-derived facts.

CPU-only. The ignored-test listing comes from listing.py; every frame bound
below was counted by reading the test source (counting rule in plan.json).
`--check` verifies that the committed plan.json is exactly what this script
produces from the current source, so root can re-derive it independently.

  python3 -I -B make_plan.py [--check]
"""

import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import listing  # noqa: E402

TOOLS = Path(__file__).resolve().parent
PLAN = TOOLS / "plan.json"

# Existing ignored GPU tests: static encoded-frame bound and the counted calls.
# One frame = one draw_scene/screenshot/retained submit/UI layer submit/game
# tick/output-or-text pass encode, including refused attempts; compute-only
# cluster grid encodes are also charged. Readback copies are not frames.
EXISTING = {
    "lib": {
        "diagnostic_output::diagnostic_vertices_draw_reflected_geometry_and_preserve_display_formats": (12, "4 formats x 3 cases draw_scene"),
        "graph::tests::queued_graph_frames_outlive_scene_versions_and_failed_preparation": (13, "4 x (reference + encoded + refused bad viewport) + empty screenshot"),
        "graph::tests::queued_pixel_frames_retain_logical_targets_and_physical_chrome_after_resizing": (8, "4 x (reference + encoded)"),
        "hdr_output::hdr_values_survive_until_tone_mapping_and_transfer_runs_once": (4, "4 formats x output-pass encode"),
        "hdr_output::output_composes_display_colors_and_reuses_bounded_resize_targets": (12, "3 sizes x 4 formats draw_scene"),
        "lighting::gpu_tests::clustered_viewport_offsets_and_queued_resize_commands_keep_their_own_light_data": (9, "3 configurations x 3 draw_scene"),
        "lighting::gpu_tests::every_depth_slice_selects_its_light_and_reused_clusters_clear_old_membership": (2, "2 compute-only grid encodes"),
        "lighting::gpu_tests::extended::extended_cluster_readback_cameras_viewports_and_retained_motion": (83, "3 cameras x 5 light counts x 5 viewports compute grid encodes + 2 x 4 retained submits"),
        "lighting::gpu_tests::masks_represent_every_light_and_clear_tail_bits_across_word_count_changes": (13, "13 light counts x compute grid encode"),
        "models::animation::tests::retained_skin_matches_independent_static_oracle_and_preserves_rejected_frame": (14, "5 x (submit + oracle) + 2 refused submits + submit + reference"),
        "models::animation::tests::single_sided_reflected_skin_and_shadow_match_static_transform_oracle": (12, "3 x (submit + 3 oracle draws)"),
        "models::retained::flipbook_tests::retained_flipbooks_match_static_frames_without_rebuilding_after_cache_deletion": (67, "initial + 32 x (submit + reference) + refused + final"),
        "models::retained::native_tests::binary_native_motion_and_structural_rebuilds_submit_without_source_or_document": (29, "initial screenshot + 12 x (submit + reference) + 2 screenshots + refused + final screenshot"),
        "models::retained::sprite_tests::source_free_native_atlas_sprites_match_fresh_frames_across_motion_reflection_and_rebuild": (20, "initial + 8 x (submit + reference) + 2 screenshots + refused"),
        "models::retained::tests::motion_staging_reuses_reserved_slots_and_rejects_unsupported_bindings": (0, "CPU staging only"),
        "models::retained::tests::rejected_retained_frame_preserves_accepted_gpu_state_and_can_retry": (11, "2 submits, 3 screenshots, 5 refused submits, reference"),
        "models::retained::tests::retained_flipbook_selection_stages_without_allocation": (0, "CPU staging only"),
        "models::retained::tests::retained_motion_matches_fresh_geometry_lights_camera_and_reflections_after_source_deletion": (17, "initial reference + 8 x (submit + reference)"),
        "models::retained::tests::retained_sprite_cpu_staging_allocates_nothing_and_rejects_changed_bindings": (0, "CPU staging only"),
        "models::retained::tests::retained_tile_cpu_staging_allocates_nothing_without_visiting_binding_cells": (0, "CPU staging only"),
        "models::retained::text_native_tests::source_free_native_text_moves_rebuilds_and_changes_through_the_retained_consumer": (22, "initial + 9 x (submit + reference) + 2 screenshots + refused"),
        "models::retained::text_tests::foreign_or_reset_handles_refuse_preparation_before_any_text_upload": (0, "preparation refusals only"),
        "models::retained::text_tests::opaque_geometry_occludes_text_behind_it_and_text_in_front_draws_over_it": (4, "4 fresh draws"),
        "models::retained::text_tests::retained_motion_rebuild_and_changed_text_reuse_or_replace_exact_identities": (29, "2 + 8 x 2 + 2 screenshots + refused + 2 + 3 sizes x 2"),
        "models::retained::tilemap_tests::cooked_source_free_tile_frames_match_raw_cell_sprite_oracle_and_survive_rejection": (20, "initial + 8 x (submit + reference) + 2 screenshots + refused"),
        "text::draw_tests::drawn_text_matches_layout_baselines_bearings_bidi_order_and_missing_glyphs": (2, "text pass encode + draw_scene"),
        "text::draw_tests::rotated_scaled_minified_and_magnified_text_matches_the_affine_reference": (3, "3 text pass encodes"),
        "ui::draw_tests::editor_viewport_region_offsets_scales_and_bounds_the_ui": (1, "1 UI layer submit"),
        "ui::draw_tests::logical_layout_maps_to_physical_pixels_at_integer_and_fractional_scales": (4, "4 UI layer submits"),
        "ui::draw_tests::nested_clips_cut_actual_glyph_bounds_including_negative_x_rtl_overflow": (2, "2 UI layer submits"),
        "ui::draw_tests::retained_atlas_and_quads_follow_input_identity_and_survive_refused_stages": (6, "6 UI layer submits"),
        "ui::draw_tests::ui_composites_over_the_finished_world_frame_and_leaves_uncovered_pixels": (3, "2 draw_scene + 1 UI layer submit"),
        "version_tests::live_gpu_versions_with_equal_recipe_keys_preserve_old_and_new_geometry": (3, "3 screenshots"),
    },
    "model_gpu": {
        "alpha_modes_and_back_faces_follow_material_semantics": (10, "10 screenshots/captures/pixels"),
        "cameras::authored_camera_changes_projection_clips_geometry_and_retains_its_selected_pose": (12, "12 screenshots/captures"),
        "cameras::inherited_cameras_and_custom_light_clusters_match_the_all_light_reference": (16, "3 poses x 2 sizes x 2 + 4 captures"),
        "cameras::orthographic::orthographic_light_clusters_match_all_lights_at_multiple_sizes_poses_and_aspects": (16, "4 poses x 2 sizes x 2"),
        "cameras::orthographic::orthographic_parallel_view_rays_preserve_specular_lighting_when_camera_moves_back": (2, "2 captures"),
        "cameras::orthographic::orthographic_size_clipping_and_retained_projection_match_authored_world_units": (8, "8 captures/screenshots"),
        "cameras::pixel_perfect::fixed_logical_pixels_replicate_exactly_with_integer_scaling_and_letterboxing": (6, "1 + 3 sizes + 2 refused sizes"),
        "cameras::pixel_perfect::followed_grid_geometry_is_stable_at_half_pixel_ties_and_odd_resolutions": (780, "3 rolls x 4 resolutions x (1 + 8 axes x 8 steps)"),
        "cameras::pixel_perfect::subpixel_camera_motion_is_stable_and_crossing_the_grid_moves_by_one_logical_pixel": (5, "1 + 2 + 1 + 1 screenshots"),
        "cameras::rotated_camera_under_uneven_parent_scale_matches_an_orthonormal_flat_pose": (2, "2 captures"),
        "cameras::transparency_depth_order_follows_the_selected_camera": (2, "2 captures"),
        "color_maps_use_srgb_and_texture_reimport_keeps_retained_materials": (5, "5 screenshots"),
        "color_roles::color_environment_minification_matches_alpha_one_and_retains_prior_versions": (4, "4 screenshots"),
        "color_roles::opaque_and_emissive_minification_ignore_alpha_without_recoloring_coverage_roles": (38, "4 modes x 2 alphas x 4 + 2 modes x 3"),
        "dark_fill_lit_materials_remain_distinguishable": (3, "3 captures"),
        "emissive_maps_and_samplers_retain_color_and_addressing": (7, "7 captures/pixels"),
        "environments::authored_environments_preserve_color_space_versions_and_source_free_loading": (6, "6 captures/screenshots"),
        "environments::environment_intensity_rotation_and_roughness_change_real_reflections": (8, "8 captures/pixels"),
        "environments::malformed_environment_cannot_replace_a_retained_scene": (2, "2 screenshots"),
        "environments::studio_environment_renders_metal_and_dielectric_across_roughness": (12, "3 materials x 4 roughness"),
        "highlight_energy_survives_and_transparency_blends_before_tone_mapping": (2, "2 captures"),
        "imported_geometry_survives_sources_and_retained_versions_survive_reimport": (5, "5 screenshots"),
        "lights::clustered_masks_match_total_light_energy_and_retain_old_scenes": (29, "1 + 9 counts x 3 + 1"),
        "lights::coverage::distinct_colored_lights_preserve_energy_across_mask_words": (4, "oracle: 2 sizes x 2"),
        "lights::coverage::local_range_edges_and_hdr_specular_materials_render_repeatably": (22, "2 + 2 x 4 x 2 + 4"),
        "lights::coverage::visible_depth_boundary_patches_match_all_light_oracle": (4, "oracle: 2 sizes x 2"),
        "lights::directional_point_and_spot_lights_obey_authored_physical_parameters": (8, "7 shots + 1 pixel"),
        "lights::finite_range_light_eases_to_zero_without_a_linear_edge_crease": (2, "2 shots"),
        "lights::masks::all_4096_local_bits_match_unculled_point_and_spot_lighting": (6, "2 sizes x 3"),
        "lights::masks::last_mask_bit_alone_lights_visible_geometry_after_a_directional_prefix": (3, "3 screenshots"),
        "lights::spatial_clusters_match_brute_force_lighting_across_tile_edges_and_resize": (8, "4 sizes x 2"),
        "material_maps_preserve_numeric_channels_normal_scale_and_occlusion_strength": (9, "9 captures/pixels"),
        "pipeline_warmup::all_current_variants_exist_before_first_draw_and_reimport_creates_no_pipelines": (48, "3 alpha x 2 sided x 2 mirrored x 4 formats"),
        "shadows::alpha_masked_and_reflected_casters_preserve_holes_and_blend_casters_fail_explicitly": (5, "5 captures"),
        "shadows::coverage::cascade_transitions_preserve_occlusion_and_fade_at_the_declared_distance": (2, "2 captures"),
        "shadows::coverage::orthographic_cascade_transitions_preserve_occlusion_and_fade_at_the_declared_distance": (2, "2 captures"),
        "shadows::directional_shadow_occlusion_respects_flags_range_and_retained_scene_versions": (9, "8 captures + 1 screenshot"),
        "shadows::lookdev::directional_shadow_lookdev_contact_slopes_and_curves": (4, "4 captures"),
        "shadows::normal_offset::opposing_normal_maps_preserve_the_geometric_shadow_boundary": (2, "2 screenshots"),
        "shadows::offscreen_casters_and_multiple_directional_shadow_slots_remain_visible": (3, "3 captures"),
        "shadows::queued::queued_shadow_maps_retain_camera_light_data_and_resized_atlas_versions": (10, "5 x (reference + encoded)"),
        "sprites::atlas::atlas_frame_ids_survive_repacking_source_removal_and_failed_replacement": (6, "6 screenshots"),
        "sprites::inherited_sprite_transforms_and_queued_reimports_survive_scene_drop": (9, "2 + 3 x (reference + encoded) + 1"),
        "sprites::nine_slice_preserves_pixel_borders_and_linear_sampling_has_no_hidden_color_fringe": (2, "2 screenshots"),
        "sprites::sprite_layers_order_and_alpha_follow_painter_semantics": (7, "7 screenshots"),
        "sprites::sprite_subregions_flips_and_retained_reimports_preserve_exact_pixels": (8, "8 screenshots"),
        "sprites::ties::magnified_nearest_texels_keep_uniform_widths_past_pixel_center_ties": (71, "1 + 7 scales x 5 offsets x 2"),
        "sprites::ties::nearest_texel_ties_preserve_each_texel_through_orientation_and_sheet_resize": (49, "1 + 3 scales x 2 heights x 8 orientations"),
        "tangents::generated_normal_map_basis_matches_explicit_frames_under_uv_and_instance_reflection": (48, "4 mirrors x 3 transforms x 4"),
    },
    "gpu_host-game": {
        "hosted_text_child_draws_with_models_script_and_audio_without_motion_uploads": (16, "8 ticks x 2 games"),
        "observed_device_loss_stops_the_host_before_another_target_is_acquired": (2, "tick + refused tick"),
        "paused_committed_refresh_submits_without_advancing_game_or_audio": (5, "tick + 3 refreshes + refused tick"),
        "script_physics_audio_and_submitted_frames_share_the_source_free_native_tick": (8, "8 ticks"),
        "target_errors_and_presentation_unwind_terminalize_game_and_silence_offline_pcm": (9, "3 modes x 3 ticks"),
    },
    "gpu_host-ui": {
        "composed_ui_draws_over_the_moving_world_and_reuses_unchanged_resources": (26, "11 UI ticks x (world + UI pass) + 4 plain ticks"),
        "refresh_and_viewport_relayout_use_the_same_ui_without_advancing_the_game": (6, "UI tick + 2 refreshes, world + UI pass each"),
    },
}

# New ignored tests of this union: run once inside the lib suite and charged to
# the 128-frame new budget, never to the existing-regression budget.
NEW_IN_SUITE = {
    "ambient_occlusion::device_tests::explicit_opt_in_zero_strength_resize_queued_frames_and_failure_preserve_default_output": (7, "3 sizes x (reference + AO frame) + empty scene"),
    "bloom::tests::device::actual_pyramid_nonzero_halo_resize_and_zero_alpha_control": (6, "3 x (baseline output + bloom output)"),
    "bloom::tests::device::bloom_output_keeps_emitter_position_at_reduced_internal_resolution": (3, "3 scales x output encode (the emitter-position test, exactly once)"),
    "bloom::tests::device::canonical_graph_zero_gain_and_pixel_camera_bypass_and_invalid_replace": (4, "4 draw_scene"),
}

# Over the entire existing ceiling by itself; refused until root revises the packet.
DEFERRED = {
    "model_gpu": {
        "sprites::wide::wide_magnified_nearest_regions_preserve_their_leading_and_trailing_texels": (
            6264,
            "per width/orientation: width/64 palette crops + 14 scales x (10 + 6 x anchors); "
            "widths 512..8192, both orientations; 2 x 3132 64x64 screenshots"),
    },
}

NUMERIC = [
    {"name": "ssao_core", "frames": 20, "renderers": ["off", "ao", "ao_radius_0_5", "ao_strength_0_5"],
     "route": "SceneInput + Renderer::prepare + with_camera + draw_scene, 321x193 Rgba8UnormSrgb, offset fractional viewport",
     "covers": ["perspective and orthographic receiver z=-4 / slab z=-3 with a side boundary",
                "white diffuse on/off pairs and black specular-only (diffuse factor 0) pairs",
                "isolated flat control, reversed normal sign, far slab, radius 0.5, strength 0.5",
                "slab extending outside the frustum (single-layer screen-space limitation)"]},
    {"name": "ssao_materials", "frames": 10, "renderers": ["off", "ao"],
     "route": "SceneInput + draw_scene, perspective",
     "covers": ["direct-only (environment intensity 0, directional light)", "emissive-only",
                "BLEND-only", "opaque receiver with BLEND slab (not an occluder)",
                "opaque receiver/slab then BLEND pane (bind-group/pipeline switch under a validation scope)"]},
    {"name": "ssao_geometry", "frames": 17, "renderers": ["off", "ao"],
     "route": "SceneInput + draw_scene and RetainedScene::submit",
     "covers": ["MASK alpha equal to cutoff, below cutoff and textured cutout",
                "double-sided back-facing receiver", "mirrored receiver instance",
                "nonuniform + mirrored tilted flat analytic control (no self-occlusion)",
                "retained skinned slab deformation vs baked far/near oracles", "retained instance motion"]},
    {"name": "lifecycle_drs", "frames": 18, "renderers": ["off", "ao", "bloom", "both"],
     "route": "draw_scene queued out of order; RetainedScene::set_dynamic_resolution fixed 16/12/8",
     "covers": ["alternating odd sizes, out-of-order submission, dropped scene/assets/source files",
                "invalid viewport and invalid AO/bloom configuration followed by a valid frame",
                "SSAO DRS 100/75/50 vs disabled and nearest expansion of internal-size references",
                "bloom DRS 100/75/50 halo centered on the physical emitter"]},
    {"name": "bloom_numeric", "frames": 8, "renderers": ["off", "bloom_l3", "bloom_l1", "bloom_l4", "bloom_l6"],
     "route": "SceneInput + draw_scene, emissive OPAQUE/BLEND emitters, environment intensity 0",
     "covers": ["coverage alpha 0 / 0.5 / 1", "luminance below / at / above threshold",
                "constant emissive field with 1, 4 and 6 levels", "chrome outside the offset viewport"]},
]

APPEARANCE = {
    "renders": 16, "pngs": 16, "pairs": 11, "max_extent": [960, 540],
    "route": "canonical composed host (HostedOfflineGameSession + NativeRenderHost + GpuRenderConsumer), offscreen target, tick 1",
    "effects": {"ao": {"radius": 2.0, "bias": 0.01, "strength": 1.0},
                "bloom": {"threshold": 1.0, "intensity": 0.5, "levels": 6}},
    "views": [
        {"view": "v1", "projection": "perspective", "size": [959, 539], "viewport": "offset", "ui": True,
         "variants": ["off", "ssao", "bloom", "both"]},
        {"view": "v2", "projection": "orthographic", "size": [960, 540], "viewport": "none", "ui": False,
         "variants": ["off", "ssao", "bloom", "both"]},
        {"view": "v3", "projection": "perspective", "size": [959, 539], "viewport": "offset", "ui": True,
         "variants": ["off@12", "both@12", "off@8", "both@8"]},
        {"view": "v4", "projection": "close-up", "size": [640, 480], "viewport": "none", "ui": False,
         "variants": ["off", "ssao", "bloom", "both"]},
    ],
}


def build():
    prediction = listing.predict()
    suites = {}
    existing_total = new_total = deferred_total = 0
    for binary, entry in prediction.items():
        names = entry["ignored"]
        bounds, deferred, new = {}, {}, {}
        for name in names:
            if name in DEFERRED.get(binary, {}):
                frames, note = DEFERRED[binary][name]
                deferred[name] = {"frames": frames, "counted": note}
                deferred_total += frames
            elif binary == "lib" and name in NEW_IN_SUITE:
                frames, note = NEW_IN_SUITE[name]
                new[name] = {"frames": frames, "counted": note}
                new_total += frames
            elif name in EXISTING[binary]:
                frames, note = EXISTING[binary][name]
                bounds[name] = {"frames": frames, "counted": note}
                existing_total += frames
            else:
                raise SystemExit(f"{binary}: no frame bound for {name}")
        stale = (set(EXISTING[binary]) | set(DEFERRED.get(binary, {}))
                 | (set(NEW_IN_SUITE) if binary == "lib" else set())) - set(names)
        if stale:
            raise SystemExit(f"{binary}: bounds for tests that no longer exist: {sorted(stale)}")
        suites[binary] = {
            "root": entry["root"], "listed_tests": entry["tests"], "ignored": names,
            "existing": bounds, "new_in_suite": new, "deferred": deferred,
            "run": sorted(set(bounds) | set(new)),
            "existing_frames": sum(v["frames"] for v in bounds.values()),
            "new_frames": sum(v["frames"] for v in new.values()),
        }
    numeric_total = sum(c["frames"] for c in NUMERIC)
    plan = {
        "format": "incant-0070-plan-v1",
        "packet": "0070-ambient-occlusion",
        "source_commit": "b633f92d9d20cd8ed4c328b0b670ab7935797801",
        "default_features": "incant_render and incant_render_gpu_host default (physics-deterministic); no cooked-* family",
        "frame_rule": ("One charged frame = one call that encodes a scene graph, display/output pass, "
                       "text or UI pass, compute cluster grid, retained submit or composed game tick, "
                       "including refused attempts. Readbacks/copies are not frames. Existing-test "
                       "bounds are static counts from source review; helper cases count actual calls "
                       "in-process and refuse beyond their declaration."),
        "budgets": {
            "gpu_children_max": 16, "children_planned": {"listings": 4, "suites": 4, "numeric": len(NUMERIC),
                                                        "appearance": 1, "focused_retries_reserved": 2},
            "existing_frames_max": 2048, "new_frames_max": 128, "combined_frames_max": 2176,
            "existing_frames_planned": existing_total,
            "existing_headroom_unallocated": 2048 - existing_total,
            "new_frames_planned": {"suite_new_tests": new_total, "numeric": numeric_total,
                                   "appearance": APPEARANCE["renders"],
                                   "retry_reserve": 128 - new_total - numeric_total - APPEARANCE["renders"]},
            "pngs_max": 32, "pairs_max": 16, "max_extent": [960, 540],
            "execution_seconds_max": 2700, "build_jobs_max": 2,
        },
        "timeouts_s": {"listing": 30, "suite": {"lib": 240, "model_gpu": 300, "gpu_host-game": 180,
                                                 "gpu_host-ui": 180},
                       "numeric": 150, "appearance": 300, "retry": 150},
        "suites": suites,
        "deferred_frames": deferred_total,
        "numeric_cases": NUMERIC,
        "appearance": APPEARANCE,
        "environment": {
            "build": {"PATH": "/opt/homebrew/opt/rustup/bin:/usr/bin:/bin:/usr/sbin:/sbin",
                      "forwarded_unchanged_if_set": ["HOME", "CARGO_HOME", "RUSTUP_HOME"],
                      "granted": ["CARGO_TARGET_DIR"], "private": ["TMPDIR"],
                      "fixed": {"CARGO_TERM_COLOR": "never", "CARGO_INCREMENTAL": "0"}},
            "run": {"PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "private": ["TMPDIR"],
                    "fixed": {"RUST_BACKTRACE": "1"}, "granted": ["INCANT_EXPECT_GPU_BACKEND"]},
            "never": ["INCANT_* (except INCANT_EXPECT_GPU_BACKEND)", "WGPU_*", "TRACY_*", "RUSTFLAGS",
                      "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER",
                      "CARGO_BUILD_*", "CARGO_FEATURE_*", "CARGO_PROFILE_*", "account/provider variables"],
        },
        "reviewed_repo_cargo_config_sha256": None,
        "grants": {
            "build": {"format": "incant-0070-build-grant-v1", "required": [
                "packet", "kind=build", "source_commit", "plan_sha256", "helper_digest", "evidence_dir",
                "cargo_target_dir", "max_jobs<=2", "approved_cargo_configs {absolute path: sha256}"]},
            "gpu": {"format": "incant-0070-gpu-grant-v1", "required": [
                "packet", "kind=gpu", "source_commit", "plan_sha256", "helper_digest", "evidence_dir",
                "build_record_sha256", "expected_backend (Metal|Vulkan|Dx12)",
                "allow_deferred_tests [] (root revision only)"]},
        },
    }
    return plan


def main():
    plan = build()
    repo_config = listing.ROOT / ".cargo" / "config.toml"
    import hashlib
    plan["reviewed_repo_cargo_config_sha256"] = hashlib.sha256(repo_config.read_bytes()).hexdigest()
    text = json.dumps(plan, indent=1, sort_keys=False) + "\n"
    if "--check" in sys.argv[1:]:
        if PLAN.read_text() != text:
            raise SystemExit("plan.json differs from the source-derived plan")
        print("plan.json matches the source-derived plan")
        return
    PLAN.write_text(text)
    b = plan["budgets"]
    print(json.dumps({"existing_frames_planned": b["existing_frames_planned"],
                      "existing_headroom_unallocated": b["existing_headroom_unallocated"],
                      "new_frames_planned": b["new_frames_planned"],
                      "deferred_frames": plan["deferred_frames"]}, indent=1))


if __name__ == "__main__":
    main()
