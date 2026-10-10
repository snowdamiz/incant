use crate::{NavigationError, NavigationGeometry, invalid, limit};
use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock},
};

/// Immutable host resources. A lazy converter receives retained cooked data,
/// never project paths. Its bounded result or failure is memoized across ticks.
#[derive(Clone)]
pub struct NavigationAsset {
    pub fingerprint: String,
    geometry: Arc<dyn Fn() -> Result<Arc<NavigationGeometry>, NavigationError> + Send + Sync>,
}
impl NavigationAsset {
    pub fn new(fingerprint: String, geometry: Arc<NavigationGeometry>) -> Self {
        Self {
            fingerprint,
            geometry: Arc::new(move || Ok(geometry.clone())),
        }
    }
    pub fn lazy(
        fingerprint: String,
        convert: impl Fn() -> Result<NavigationGeometry, NavigationError> + Send + Sync + 'static,
    ) -> Self {
        let cached = OnceLock::new();
        Self {
            fingerprint,
            geometry: Arc::new(move || cached.get_or_init(|| convert().map(Arc::new)).clone()),
        }
    }
    pub fn geometry(&self) -> Result<Arc<NavigationGeometry>, NavigationError> {
        (self.geometry)()
    }
}
pub type NavigationResources = BTreeMap<String, NavigationAsset>;

impl NavigationGeometry {
    /// Compose hierarchy transforms in f64 before the bounded f32 bake. Mirrored
    /// instances preserve outward winding just as the renderer's front-face does.
    pub fn transformed(&self, matrix: [[f64; 4]; 4]) -> Result<Self, NavigationError> {
        self.validate()?;
        if matrix[0][3] != 0. || matrix[1][3] != 0. || matrix[2][3] != 0. || matrix[3][3] != 1. {
            return Err(invalid("navigation source transform must be affine"));
        }
        let matrix = glam::DMat4::from_cols_array_2d(&matrix);
        let determinant = matrix.determinant();
        if !matrix.is_finite() || !determinant.is_finite() || determinant.abs() < 1e-18 {
            return Err(invalid("invalid navigation source transform"));
        }
        let mut out = self.clone();
        for point in &mut out.vertices {
            *point = matrix
                .transform_point3(glam::DVec3::from_array(point.map(f64::from)))
                .as_vec3()
                .to_array();
        }
        if determinant < 0. {
            for triangle in &mut out.triangles {
                triangle.swap(1, 2);
            }
        }
        out.validate()?;
        Ok(out)
    }
    pub fn append(&mut self, other: Self) -> Result<(), NavigationError> {
        if self.triangles.len() + other.triangles.len() > crate::MAX_TRIANGLES
            || self.vertices.len() + other.vertices.len() > crate::MAX_TRIANGLES * 3
        {
            return Err(limit("combined navigation source geometry"));
        }
        other.validate()?;
        let offset = self.vertices.len() as u32;
        self.vertices.extend(other.vertices);
        self.triangles
            .extend(other.triangles.into_iter().map(|t| t.map(|i| i + offset)));
        Ok(())
    }
}
