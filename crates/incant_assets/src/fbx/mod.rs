//! Bounded authoring-only static FBX conversion. The C parser never opens files;
//! dependencies are snapshotted through SourceSet and cooked by the shared writer.
mod geometry;
mod materials;
use crate::{AssetError, ImportedModel, ModelNode, Result, SourceSet, invalid};
use std::{collections::BTreeMap, path::Path};

const MAX_NODES: usize = 100_000;
const MAX_PRIMITIVES: usize = 10_000;
const RECIPE: &[u8] =
    b"incant-static-fbx-v3-ufbx-0.11.5-c0.23.1-meters-yup-meshopt-0.6.2-color-roles-v1-normal-basis-v1\0";
fn unsupported(feature: impl std::fmt::Display) -> AssetError {
    AssetError::Unsupported(format!("static FBX: {feature}"))
}

/// Import static triangle/polygon meshes, hierarchy/instances and the supported
/// material subset into the existing model format. Unsupported authored content
/// rejects the whole source; empty animation containers carry no animation data.
pub fn import_fbx(root: &Path, source: &Path) -> Result<ImportedModel> {
    let mut sources = SourceSet::new(root)?;
    let bytes = sources.read(source)?;
    let scene = ufbx::load_memory(&bytes, options())
        .map_err(|e| AssetError::Fbx(format!("{:?}: {}", e.type_, e.description)))?;
    validate_scene(&scene)?;
    let mut material_import = materials::Materials::new(&mut sources, source);
    let materials = scene
        .materials
        .iter()
        .map(|m| material_import.import(m))
        .collect::<Result<Vec<_>>>()?;
    let mut meshes = Vec::new();
    let mut mesh_materials = Vec::new();
    let mut variants = BTreeMap::new();
    let mut nodes = Vec::with_capacity(scene.nodes.len());
    let mut vertices = 0;
    let mut indices = 0;
    let mut instance_references = 0usize;
    for (index, node) in scene.nodes.iter().enumerate() {
        if node.element.typed_id as usize != index {
            return Err(invalid("noncanonical FBX node identity"));
        }
        if !node.visible {
            return Err(unsupported("hidden nodes"));
        }
        let primitives = if let Some(mesh) = &node.mesh {
            let key = (
                mesh.element.typed_id,
                node.materials
                    .iter()
                    .map(|m| m.element.typed_id)
                    .collect::<Vec<_>>(),
            );
            if let Some(ids) = variants.get(&key) {
                Vec::clone(ids)
            } else {
                let parts = geometry::import(mesh, node, &mut vertices, &mut indices)?;
                if meshes.len() + parts.len() > MAX_PRIMITIVES {
                    return Err(AssetError::Limit("FBX mesh primitives"));
                }
                let mut ids = Vec::with_capacity(parts.len());
                for (mesh, material) in parts {
                    ids.push(meshes.len());
                    meshes.push(mesh);
                    mesh_materials.push(material);
                }
                variants.insert(key, ids.clone());
                ids
            }
        } else {
            Vec::new()
        };
        instance_references = instance_references
            .checked_add(primitives.len() + node.materials.len())
            .filter(|n| *n <= 1_000_000)
            .ok_or(AssetError::Limit("FBX instance references"))?;
        let m = node.node_to_parent;
        let transform = [
            [m.m00, m.m10, m.m20, 0.],
            [m.m01, m.m11, m.m21, 0.],
            [m.m02, m.m12, m.m22, 0.],
            [m.m03, m.m13, m.m23, 1.],
        ]
        .map(|column| column.map(|v| v as f32));
        if transform.iter().flatten().any(|v| !v.is_finite()) {
            return Err(invalid("FBX transform outside finite f32 range"));
        }
        if node.element.name.len() > 4096 {
            return Err(AssetError::Limit("FBX node name"));
        }
        nodes.push(ModelNode {
            name: (!node.element.name.is_empty()).then(|| node.element.name.to_string()),
            children: node
                .children
                .iter()
                .map(|c| c.element.typed_id as usize)
                .collect(),
            meshes: primitives,
            transform,
        });
    }
    if meshes.is_empty() {
        return Err(invalid("FBX has no triangle meshes"));
    }
    incant_assets_runtime::model_node::validate_nodes(&nodes)?;
    let (textures, images) = material_import.finish();
    crate::material::resolve(&materials, &textures, &images)?;
    Ok(ImportedModel {
        meshes,
        textures,
        images,
        nodes,
        materials,
        mesh_materials,
        scenes: vec![vec![scene.root_node.element.typed_id as usize]],
        default_scene: Some(0),
        dependencies: sources.dependencies(),
        fingerprint: sources.fingerprint_with_salt(RECIPE),
    })
}

fn options() -> ufbx::LoadOpts<'static> {
    ufbx::LoadOpts {
        temp_allocator: ufbx::AllocatorOpts {
            memory_limit: 128 * 1024 * 1024,
            allocation_limit: 1_000_000,
            ..Default::default()
        },
        result_allocator: ufbx::AllocatorOpts {
            memory_limit: 256 * 1024 * 1024,
            allocation_limit: 1_000_000,
            ..Default::default()
        },
        // load_memory + explicit FBX + no external/cache loading prevents parser IO.
        file_format: ufbx::FileFormat::Fbx,
        load_external_files: false,
        strict: true,
        node_depth_limit: 256,
        index_error_handling: ufbx::IndexErrorHandling::AbortLoading,
        unicode_error_handling: ufbx::UnicodeErrorHandling::AbortLoading,
        generate_missing_normals: true,
        normalize_normals: true,
        target_axes: ufbx::CoordinateAxes::right_handed_y_up(),
        target_unit_meters: 1.,
        // Keep conversion on a real root, and express FBX-only geometry/inherit
        // transforms as ordinary helper nodes. Instances remain shared geometry.
        space_conversion: ufbx::SpaceConversion::TransformRoot,
        geometry_transform_handling: ufbx::GeometryTransformHandling::HelperNodes,
        inherit_mode_handling: ufbx::InheritModeHandling::HelperNodes,
        use_blender_pbr_material: true,
        ..Default::default()
    }
}
fn validate_scene(s: &ufbx::Scene) -> Result<()> {
    // ufbx intentionally skips unit conversion for nonpositive source units.
    // Accepting that fallback would silently claim meter-space geometry here.
    if !s.settings.unit_meters.is_finite() || s.settings.unit_meters <= 0. {
        return Err(invalid("FBX source units must be finite and positive"));
    }
    if s.nodes.len() > MAX_NODES
        || s.meshes.len() > MAX_PRIMITIVES
        || s.materials.len() > MAX_PRIMITIVES
        || s.textures.len() > 1024
    {
        return Err(AssetError::Limit("FBX objects"));
    }
    if !s.metadata.warnings.is_empty() {
        return Err(invalid(format!(
            "FBX parser warning: {:?}",
            s.metadata.warnings[0].type_
        )));
    }
    for (present, name) in [
        (
            !s.anim_curves.is_empty() || s.anim_layers.iter().any(|l| !l.anim_props.is_empty()),
            "animation channels",
        ),
        (
            !s.skin_deformers.is_empty() || !s.bones.is_empty(),
            "skinning/bones",
        ),
        (!s.blend_deformers.is_empty(), "morph targets"),
        (!s.cache_deformers.is_empty(), "vertex caches"),
        (
            !s.lights.is_empty() || !s.cameras.is_empty() || !s.stereo_cameras.is_empty(),
            "cameras/lights",
        ),
        (
            !s.line_curves.is_empty()
                || !s.nurbs_curves.is_empty()
                || !s.nurbs_surfaces.is_empty()
                || !s.nurbs_trim_surfaces.is_empty()
                || !s.procedural_geometries.is_empty(),
            "curves/NURBS/procedural geometry",
        ),
        (
            !s.constraints.is_empty() || !s.characters.is_empty() || !s.lod_groups.is_empty(),
            "constraints/characters/LOD groups",
        ),
        (
            !s.audio_clips.is_empty() || !s.unknowns.is_empty(),
            "audio/unknown objects",
        ),
    ] {
        if present {
            return Err(unsupported(name));
        }
    }
    for mesh in &s.meshes {
        geometry::validate(mesh)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn parser_allocation_limits_stop_real_fbx_before_conversion() {
        let bytes = include_bytes!("../../tests/fixtures/fbx/maya_cube_7500_ascii.fbx");
        let mut options = super::options();
        options.result_allocator.memory_limit = 1024;
        let error = ufbx::load_memory(bytes, options)
            .err()
            .expect("bounded parser refusal");
        assert_eq!(error.type_, ufbx::ErrorType::MemoryLimit);
    }
}
