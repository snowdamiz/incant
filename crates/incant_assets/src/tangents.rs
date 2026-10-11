use crate::{AssetError, MAX_VERTICES, Mesh, Result, invalid};
use mikktspace::Geometry;
struct Faces(Vec<crate::Vertex>);
impl Geometry for Faces {
    fn num_faces(&self) -> usize {
        self.0.len() / 3
    }
    fn num_vertices_of_face(&self, _: usize) -> usize {
        3
    }
    fn position(&self, f: usize, v: usize) -> [f32; 3] {
        self.0[f * 3 + v][..3].try_into().unwrap()
    }
    fn normal(&self, f: usize, v: usize) -> [f32; 3] {
        self.0[f * 3 + v][3..6].try_into().unwrap()
    }
    fn tex_coord(&self, f: usize, v: usize) -> [f32; 2] {
        self.0[f * 3 + v][6..8].try_into().unwrap()
    }
    fn set_tangent_encoded(&mut self, mut t: [f32; 4], f: usize, v: usize) {
        // Mikk's sign follows increasing texture V. Our imported UVs use glTF's
        // top-left image origin and +Y normal maps, so their bitangent is the
        // opposite direction. Only generated signs change; authored glTF
        // TANGENT values already encode their own basis and bypass this path.
        t[3] = -t[3];
        self.0[f * 3 + v][8..12].copy_from_slice(&t);
    }
}
pub(crate) fn generate(mesh: &mut Mesh) -> Result<()> {
    if mesh.indices.len() > MAX_VERTICES {
        return Err(AssetError::Limit("tangent vertices"));
    }
    // MikkTSpace returns per-face-vertex data. Keep separate vertices across
    // tangent discontinuities; averaging back into the old indices loses seams.
    let mut faces = Faces(
        mesh.indices
            .iter()
            .map(|&i| mesh.vertices[i as usize])
            .collect(),
    );
    if !mikktspace::generate_tangents(&mut faces) {
        return Err(invalid("unable to generate tangent space"));
    }
    mesh.vertices = faces.0;
    mesh.indices = (0..mesh.vertices.len() as u32).collect();
    mesh.validate().map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn generated_basis_matches_gltf_normal_axes_including_mirrored_uvs() {
        // glTF images have a top-left origin. On this +Z triangle, the base
        // mapping has +U along +X and the normal map's +Y along +Y (decreasing V).
        // These four analytic bases also distinguish mirroring one or both axes.
        for (mirror_u, mirror_v, expected_t, expected_b, expected_w) in [
            (false, false, Vec3::X, Vec3::Y, 1.),
            (true, false, -Vec3::X, Vec3::Y, -1.),
            (false, true, Vec3::X, -Vec3::Y, -1.),
            (true, true, -Vec3::X, -Vec3::Y, 1.),
        ] {
            let vertices =
                [[0., 0., 0., 1.], [1., 0., 1., 1.], [0., 1., 0., 0.]].map(|[x, y, u, v]| {
                    let u = if mirror_u { 1. - u } else { u };
                    let v = if mirror_v { 1. - v } else { v };
                    [x, y, 0., 0., 0., 1., u, v, 0., 0., 0., 0.]
                });
            let mut mesh = Mesh {
                vertices: vertices.to_vec(),
                indices: vec![0, 1, 2],
            };
            generate(&mut mesh).unwrap();
            for (actual, source) in mesh.vertices.iter().zip(vertices) {
                assert_eq!(
                    &actual[..8],
                    &source[..8],
                    "positions/normals/UVs are unchanged"
                );
                let t = Vec3::from_slice(&actual[8..11]);
                let b = Vec3::Z.cross(t) * actual[11];
                assert!(
                    (t - expected_t).length() < 1e-6,
                    "mirror={mirror_u}/{mirror_v}: {t:?}"
                );
                assert!(
                    (b - expected_b).length() < 1e-6,
                    "mirror={mirror_u}/{mirror_v}: {b:?}"
                );
                assert_eq!(actual[11], expected_w);
            }
        }
    }
}
