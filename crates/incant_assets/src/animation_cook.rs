//! Authoring-only writer for the typed animation suffix. Reader is runtime-only.
use crate::{AssetError, Result};
use incant_assets_runtime::animation::*;
pub(crate) fn encode(data: &AnimationData) -> Result<Vec<u8>> {
    let size = data.encoded_len()?;
    let mut out = Vec::with_capacity(size);
    out.extend_from_slice(b"INCANM01");
    count(&mut out, data.rest.len());
    for rest in &data.rest {
        match rest {
            None => out.push(0),
            Some(t) => {
                out.push(1);
                floats(&mut out, &t.translation);
                floats(&mut out, &t.rotation);
                floats(&mut out, &t.scale);
            }
        }
    }
    count(&mut out, data.node_skins.len());
    for &s in &data.node_skins {
        optional(&mut out, s);
    }
    count(&mut out, data.skins.len());
    for skin in &data.skins {
        name(&mut out, &skin.name);
        optional(&mut out, skin.skeleton);
        count(&mut out, skin.joints.len());
        for (&joint, bind) in skin.joints.iter().zip(&skin.inverse_bind) {
            count(&mut out, joint);
            for column in bind {
                floats(&mut out, column);
            }
        }
    }
    count(&mut out, data.influences.len());
    for stream in &data.influences {
        count(&mut out, stream.len());
        for v in stream {
            for j in v.joints {
                out.extend_from_slice(&j.to_le_bytes());
            }
            floats(&mut out, &v.weights);
        }
    }
    count(&mut out, data.clips.len());
    for clip in &data.clips {
        name(&mut out, &clip.name);
        count(&mut out, clip.channels.len());
        for channel in &clip.channels {
            count(&mut out, channel.node);
            out.push(match channel.property {
                Property::Translation => 0,
                Property::Rotation => 1,
                Property::Scale => 2,
            });
            out.push(match channel.interpolation {
                Interpolation::Linear => 0,
                Interpolation::Step => 1,
            });
            count(&mut out, channel.times.len());
            for (&time, value) in channel.times.iter().zip(&channel.values) {
                floats(&mut out, &[time]);
                floats(&mut out, value);
            }
        }
    }
    if out.len() != size {
        return Err(AssetError::Invalid(
            "animation encoded length mismatch".into(),
        ));
    }
    Ok(out)
}
fn count(out: &mut Vec<u8>, n: usize) {
    out.extend_from_slice(&(n as u32).to_le_bytes());
}
fn optional(out: &mut Vec<u8>, n: Option<usize>) {
    out.extend_from_slice(&n.map_or(u32::MAX, |n| n as u32).to_le_bytes());
}
fn floats(out: &mut Vec<u8>, values: &[f32]) {
    for &v in values {
        out.extend_from_slice(&v.to_le_bytes());
    }
}
fn name(out: &mut Vec<u8>, v: &Option<String>) {
    match v {
        None => optional(out, None),
        Some(v) => {
            count(out, v.len());
            out.extend_from_slice(v.as_bytes());
        }
    }
}
