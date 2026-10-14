//! One-entry memo for a single numeric-validation pass, never native state.
use glam::{DMat4, DVec3, DVec4};
use incant_types::Transform;

#[derive(Default)]
pub(crate) struct MatrixBasisCache {
    last: Option<([u64; 7], [DVec4; 3])>,
}

impl MatrixBasisCache {
    /// Call only after the ordinary component checks. A hit bypasses no
    /// validation: it reuses the pure rotation/scale portion of `scene::matrix`.
    /// glam's pinned constructor forms these three columns independently of
    /// translation. Its fourth column copies the current translation and 1.0.
    /// Exact bit keys distinguish signed zeros and do not normalize input.
    #[inline]
    pub(crate) fn matrix(&mut self, transform: &Transform) -> DMat4 {
        let [x, y, z, w] = transform.rotation.map(f64::to_bits);
        let [sx, sy, sz] = transform.scale.map(f64::to_bits);
        let key = [x, y, z, w, sx, sy, sz];
        if let Some((previous, columns)) = &self.last
            && *previous == key
        {
            return DMat4::from_cols(
                columns[0],
                columns[1],
                columns[2],
                DVec4::from((DVec3::from_array(transform.translation), 1.0)),
            );
        }
        let matrix = crate::scene::matrix(transform);
        self.last = Some((key, [matrix.x_axis, matrix.y_axis, matrix.z_axis]));
        matrix
    }
}

#[cfg(test)]
mod tests;
