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
    fn set_tangent_encoded(&mut self, t: [f32; 4], f: usize, v: usize) {
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
    mesh.validate()
}
