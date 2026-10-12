use crate::{Animation, AnimationGraph, Entity};
use incant_types::AnimationMotion;

pub(crate) fn validate(entity: &Entity) -> Result<(), String> {
    let Some(graph) = entity.components.get("AnimationGraph") else {
        return Ok(());
    };
    let graph: AnimationGraph = serde_json::from_value(graph.clone()).map_err(|e| e.to_string())?;
    graph.validate().map_err(str::to_owned)?;
    let animation = entity
        .components
        .get("Animation")
        .ok_or("AnimationGraph requires Animation")?;
    let animation: Animation =
        serde_json::from_value(animation.clone()).map_err(|e| e.to_string())?;
    let entry = &graph.states[&graph.initial];
    let clip = match entry.motion {
        AnimationMotion::Clip { clip } => clip,
        AnimationMotion::Blend1d { .. } => None,
    };
    if animation.clip != clip || animation.seconds != 0.0 || animation.looping != entry.looping {
        return Err("AnimationGraph initial Animation must match its entry clip/loop, with zero seconds; blend entry uses a null initial clip".into());
    }
    Ok(())
}
