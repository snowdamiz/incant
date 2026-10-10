use crate::{NavigationError, invalid, limit};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MAX_TILES: usize = 256;
pub const MAX_TRIANGLES: usize = 32_768;
pub const MAX_POLYGONS: usize = 32_768;
pub const MAX_DETAIL_TRIANGLES: usize = 262_144;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NavigationSettings {
    /// World-space build bounds, independent of an entity's local transform.
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub cell_size: f32,
    pub cell_height: f32,
    pub tile_cells: u16,
    pub agent_radius: f32,
    pub agent_height: f32,
    pub max_climb: f32,
    pub max_slope_degrees: f32,
}
impl Default for NavigationSettings {
    fn default() -> Self {
        Self {
            min: [-16., -4., -16.],
            max: [16., 8., 16.],
            cell_size: 0.2,
            cell_height: 0.1,
            tile_cells: 32,
            agent_radius: 0.4,
            agent_height: 1.8,
            max_climb: 0.3,
            max_slope_degrees: 45.,
        }
    }
}
impl NavigationSettings {
    pub fn validate(&self) -> Result<(), NavigationError> {
        if !self
            .min
            .iter()
            .chain(&self.max)
            .all(|x| x.is_finite() && x.abs() <= 100_000.)
            || (0..3).any(|i| self.min[i] >= self.max[i])
        {
            return Err(invalid("bounds must be finite, ordered and within 100 km"));
        }
        for (name, value, low, high) in [
            ("cell_size", self.cell_size, 0.05, 2.),
            ("cell_height", self.cell_height, 0.025, 1.),
            ("agent_radius", self.agent_radius, 0.05, 4.),
            ("agent_height", self.agent_height, 0.1, 10.),
            ("max_climb", self.max_climb, 0., 4.),
            ("max_slope_degrees", self.max_slope_degrees, 1., 85.),
        ] {
            if !value.is_finite() || !(low..=high).contains(&value) {
                return Err(invalid(format!("{name} must be in {low}..={high}")));
            }
        }
        if !(16..=128).contains(&self.tile_cells) || !self.tile_cells.is_power_of_two() {
            return Err(invalid("tile_cells must be 16, 32, 64 or 128"));
        }
        if !(3. ..=255.).contains(&(self.agent_height / self.cell_height))
            || self.max_climb >= self.agent_height
        {
            return Err(invalid(
                "agent height must span 3..=255 cells and exceed max_climb",
            ));
        }
        if self.agent_radius / self.cell_size > 16. {
            return Err(limit("agent radius exceeds 16 horizontal cells"));
        }
        if (self.max[1] - self.min[1]) / self.cell_height > 4096. {
            return Err(limit("vertical bounds exceed 4096 cells"));
        }
        let [x, z] = self.tile_counts();
        if x == 0 || z == 0 || u64::from(x) * u64::from(z) > MAX_TILES as u64 {
            return Err(limit("build requires more than 256 tiles"));
        }
        // At large world coordinates, f32 rounding must not consume a voxel.
        if self
            .min
            .iter()
            .chain(&self.max)
            .any(|x| x.abs() * f32::EPSILON > self.cell_size.min(self.cell_height) / 8.)
        {
            return Err(invalid(
                "bounds are too distant from origin for the requested precision",
            ));
        }
        Ok(())
    }
    pub(crate) fn tile_counts(&self) -> [u32; 2] {
        let size = self.cell_size * f32::from(self.tile_cells);
        [
            ((self.max[0] - self.min[0]) / size).ceil() as u32,
            ((self.max[2] - self.min[2]) / size).ceil() as u32,
        ]
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NavigationGeometry {
    /// Counterclockwise winding viewed from the walkable side. Down-facing faces block space.
    pub vertices: Vec<[f32; 3]>,
    pub triangles: Vec<[u32; 3]>,
}
impl NavigationGeometry {
    pub fn validate(&self) -> Result<(), NavigationError> {
        if self.vertices.len() > MAX_TRIANGLES * 3 || self.triangles.len() > MAX_TRIANGLES {
            return Err(limit(
                "source geometry exceeds 32768 triangles/98304 vertices",
            ));
        }
        if !self
            .vertices
            .iter()
            .flatten()
            .all(|x| x.is_finite() && x.abs() <= 100_000.)
        {
            return Err(invalid("source vertices must be finite and within 100 km"));
        }
        for triangle in &self.triangles {
            if triangle.iter().any(|&i| i as usize >= self.vertices.len()) {
                return Err(invalid("triangle references an absent vertex"));
            }
            let [a, b, c] = triangle.map(|i| glam::Vec3::from_array(self.vertices[i as usize]));
            if (b - a).cross(c - a).length_squared() <= 1e-16 {
                return Err(invalid("degenerate source triangle"));
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NavigationPolygon {
    pub vertices: Vec<[f32; 3]>,
    pub triangles: Vec<[[f32; 3]; 3]>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TileId {
    pub x: u32,
    pub z: u32,
}
