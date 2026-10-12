use incant_anim::{RootMotionCatalog, RootMotionClip, RootMotionDelta, RootMotionTrack};
use incant_assets_runtime::{
    CookedModel,
    animation::{Channel, Interpolation, Property},
};

pub(super) fn catalog(
    model: &CookedModel,
    fingerprint: [u8; 32],
    selected: u16,
) -> Result<RootMotionCatalog, &'static str> {
    let data = model
        .animation
        .as_ref()
        .ok_or("root motion requires animation data")?;
    // The loader checks the complete cooked model. This boundary also verifies
    // the animation catalog before following topology or channel indices.
    data.validate(&model.metadata.nodes, &model.meshes, &model.metadata.scenes)
        .map_err(|_| "invalid cooked animation data")?;
    let node = usize::from(selected);
    let rest = data
        .rest
        .get(node)
        .and_then(Option::as_ref)
        .ok_or("root motion requires an existing TRS root")?;
    let mut parents = vec![None; model.metadata.nodes.len()];
    for (parent, source) in model.metadata.nodes.iter().enumerate() {
        for &child in &source.children {
            parents[child] = Some(parent);
        }
    }
    let scene = model
        .metadata
        .scenes
        .get(model.metadata.default_scene.unwrap_or(0))
        .ok_or("model has no selected scene")?;
    let scene_root = |mut index: usize| {
        while let Some(parent) = parents[index] {
            index = parent;
        }
        index
    };
    if !scene.contains(&scene_root(node)) {
        return Err("selected root is absent from the rendered model scene");
    }
    let contains = |mut joint: usize| {
        loop {
            if joint == node {
                return true;
            }
            match parents[joint] {
                Some(parent) => joint = parent,
                None => return false,
            }
        }
    };
    for (index, skin) in data.node_skins.iter().enumerate() {
        if scene.contains(&scene_root(index))
            && let Some(skin) = skin
            && data.skins[*skin]
                .joints
                .iter()
                .any(|&joint| !contains(joint))
        {
            return Err("selected root must dominate every bound skin joint");
        }
    }
    let mut reference = RootMotionDelta::IDENTITY;
    let mut cursor = Some(node);
    while let Some(index) = cursor {
        let local = data.rest[index]
            .as_ref()
            .ok_or("root path requires TRS nodes")?;
        if local.scale != [1.; 3] {
            return Err("root path requires unit rest scale");
        }
        if data
            .clips
            .iter()
            .flat_map(|clip| &clip.channels)
            .any(|channel| {
                channel.node == index && (index != node || channel.property == Property::Scale)
            })
        {
            return Err("root scale tracks and animated ancestors are unsupported");
        }
        reference = RootMotionDelta::from_rest(local.translation, local.rotation)
            .and_then(|local| local.compose(reference))
            .map_err(|_| "nonfinite root reference")?;
        cursor = parents[index];
    }
    let mut clips = Vec::with_capacity(data.clips.len());
    for clip in &data.clips {
        let duration = clip
            .channels
            .iter()
            .filter_map(|channel| channel.times.last().copied())
            .fold(0.0f32, f32::max);
        let channel = |property| {
            clip.channels
                .iter()
                .find(|c| c.node == node && c.property == property)
                .map(track)
                .transpose()
        };
        clips.push(
            RootMotionClip::new(
                duration,
                channel(Property::Translation)?,
                channel(Property::Rotation)?,
            )
            .map_err(|_| "root clip duration or track admission")?,
        );
    }
    RootMotionCatalog::new(
        fingerprint,
        selected,
        rest.translation,
        rest.rotation,
        reference,
        clips,
    )
    .map_err(|_| "root catalog capacity or numerical admission")
}

fn track(channel: &Channel) -> Result<RootMotionTrack, &'static str> {
    use incant_anim::track::{Interpolation as I, Kind};
    RootMotionTrack::new(
        channel.times.clone(),
        channel.values.clone(),
        match channel.interpolation {
            Interpolation::Linear => I::Linear,
            Interpolation::Step => I::Step,
        },
        if channel.property == Property::Rotation {
            Kind::Rotation
        } else {
            Kind::Vector
        },
    )
    .map_err(|_| "root track admission")
}
