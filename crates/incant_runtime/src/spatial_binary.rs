use crate::binary::Reader;
use crate::{
    CookedSpatial, NativeEnvironmentLight, NativeMeshRenderer, SceneError, StableId,
    VisualAssetKind, VisualAssetRef,
};
use incant_types::{
    Camera, CameraProjection, DirectionalLight, DirectionalShadows, PointLight, SpotLight,
};

pub(crate) const ASSET_BYTES: usize = 49;
pub(crate) fn encoded_size(n: &CookedSpatial) -> usize {
    17 + n.camera.as_ref().map_or(0, |c| {
        25 + match c.projection {
            CameraProjection::Perspective {} => 0,
            CameraProjection::Orthographic { .. } => 8,
            CameraProjection::PixelPerfect { .. } => 16,
        }
    }) + n.mesh.as_ref().map_or(0, |_| 17)
        + n.directional
            .as_ref()
            .map_or(0, |l| 33 + usize::from(l.shadows.is_some()) * 8)
        + n.point.as_ref().map_or(0, |_| 40)
        + n.spot.as_ref().map_or(0, |_| 56)
        + n.environment.as_ref().map_or(0, |_| 32)
        + n.sprite
            .as_deref()
            .map_or(0, crate::sprite_binary::encoded_size)
}
pub(crate) fn write_asset(bytes: &mut Vec<u8>, asset: &VisualAssetRef) {
    bytes.extend(asset.id.0);
    bytes.push(match asset.kind {
        VisualAssetKind::Model => 0,
        VisualAssetKind::Texture => 1,
    });
    bytes.extend(asset.fingerprint);
}
pub(crate) fn read_asset(r: &mut Reader<'_>) -> Result<VisualAssetRef, SceneError> {
    let id = StableId(r.array()?);
    let kind = match r.byte()? {
        0 => VisualAssetKind::Model,
        1 => VisualAssetKind::Texture,
        _ => return Err(SceneError::Invalid("visual asset kind")),
    };
    Ok(VisualAssetRef {
        id,
        kind,
        fingerprint: r.array()?,
    })
}
fn floats(bytes: &mut Vec<u8>, values: impl IntoIterator<Item = f64>) {
    for value in values {
        bytes.extend(value.to_le_bytes());
    }
}
pub(crate) fn write(bytes: &mut Vec<u8>, n: &CookedSpatial) {
    bytes.extend(n.entity.0);
    bytes.push(
        u8::from(n.camera.is_some())
            | u8::from(n.mesh.is_some()) << 1
            | u8::from(n.directional.is_some()) << 2
            | u8::from(n.point.is_some()) << 3
            | u8::from(n.spot.is_some()) << 4
            | u8::from(n.environment.is_some()) << 5
            | u8::from(n.sprite.is_some()) << 6,
    );
    if let Some(c) = &n.camera {
        floats(bytes, [c.fov_degrees, c.near, c.far]);
        match c.projection {
            CameraProjection::Perspective {} => bytes.push(0),
            CameraProjection::Orthographic { vertical_size } => {
                bytes.push(1);
                floats(bytes, [vertical_size]);
            }
            CameraProjection::PixelPerfect {
                pixels_per_unit,
                reference_resolution,
            } => {
                bytes.push(2);
                floats(bytes, [pixels_per_unit]);
                for size in reference_resolution {
                    bytes.extend(size.to_le_bytes());
                }
            }
        }
    }
    if let Some(m) = &n.mesh {
        bytes.extend(m.mesh.0);
        bytes.push(u8::from(m.cast_shadows));
    }
    if let Some(l) = &n.directional {
        floats(bytes, l.color);
        floats(bytes, [l.intensity]);
        bytes.push(u8::from(l.shadows.is_some()));
        if let Some(s) = &l.shadows {
            floats(bytes, [s.distance]);
        }
    }
    if let Some(l) = &n.point {
        floats(bytes, l.color);
        floats(bytes, [l.intensity, l.range]);
    }
    if let Some(l) = &n.spot {
        floats(bytes, l.color);
        floats(
            bytes,
            [l.intensity, l.range, l.inner_degrees, l.outer_degrees],
        );
    }
    if let Some(e) = &n.environment {
        bytes.extend(e.texture.0);
        floats(bytes, [e.intensity, e.rotation_degrees]);
    }
    if let Some(s) = &n.sprite {
        crate::sprite_binary::write(bytes, s);
    }
}
fn flag(r: &mut Reader<'_>) -> Result<bool, SceneError> {
    match r.byte()? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(SceneError::Invalid("spatial boolean flag")),
    }
}
pub(crate) fn read(r: &mut Reader<'_>, version: u32) -> Result<CookedSpatial, SceneError> {
    let mut n = CookedSpatial::empty(StableId(r.array()?));
    let mask = r.byte()?;
    if mask == 0 || mask > if version >= 5 { 127 } else { 63 } {
        return Err(SceneError::Invalid("spatial component mask"));
    }
    if mask & 1 != 0 {
        let [fov_degrees, near, far] = r.floats()?;
        let projection = match r.byte()? {
            0 => CameraProjection::Perspective {},
            1 => CameraProjection::Orthographic {
                vertical_size: r.floats::<1>()?[0],
            },
            2 => CameraProjection::PixelPerfect {
                pixels_per_unit: r.floats::<1>()?[0],
                reference_resolution: [r.u32()?, r.u32()?],
            },
            _ => return Err(SceneError::Invalid("camera projection tag")),
        };
        n.camera = Some(Camera {
            fov_degrees,
            near,
            far,
            projection,
        });
    }
    if mask & 2 != 0 {
        n.mesh = Some(NativeMeshRenderer {
            mesh: StableId(r.array()?),
            cast_shadows: flag(r)?,
        });
    }
    if mask & 4 != 0 {
        let color = r.floats()?;
        let [intensity] = r.floats()?;
        let shadows = if flag(r)? {
            Some(DirectionalShadows {
                distance: r.floats::<1>()?[0],
            })
        } else {
            None
        };
        n.directional = Some(DirectionalLight {
            color,
            intensity,
            shadows,
        });
    }
    if mask & 8 != 0 {
        let color = r.floats()?;
        let [intensity, range] = r.floats()?;
        n.point = Some(PointLight {
            color,
            intensity,
            range,
        });
    }
    if mask & 16 != 0 {
        let color = r.floats()?;
        let [intensity, range, inner_degrees, outer_degrees] = r.floats()?;
        n.spot = Some(SpotLight {
            color,
            intensity,
            range,
            inner_degrees,
            outer_degrees,
        });
    }
    if mask & 32 != 0 {
        let texture = StableId(r.array()?);
        let [intensity, rotation_degrees] = r.floats()?;
        n.environment = Some(NativeEnvironmentLight {
            texture,
            intensity,
            rotation_degrees,
        });
    }
    if mask & 64 != 0 {
        n.sprite = Some(Box::new(crate::sprite_binary::read(r)?));
    }
    Ok(n)
}
