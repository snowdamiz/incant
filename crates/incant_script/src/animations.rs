//! Committed playback copied before numeric loans. Native scratch is retained;
//! published arrays own VM memory and cannot mutate playback or later snapshots.
use incant_runtime::{AnimationSnapshot, NativeWorld};
use rquickjs::{Array, Ctx, Object, TypedArray, object::Property};

#[derive(Default)]
pub(crate) struct Scratch {
    graphs: crate::animation_graphs::Scratch,
    ik: crate::animation_ik::Scratch,
    rows: Vec<AnimationSnapshot>,
    clips: Vec<i32>,
    seconds: Vec<f64>,
    speeds: Vec<f64>,
    flags: Vec<u32>,
    pose_offsets: Vec<u32>,
    pose_clips: Vec<i32>,
    pose_seconds: Vec<f64>,
    pose_weights: Vec<f64>,
    pose_kinds: Vec<u8>,
    pose_splits: Vec<u8>,
    pose_outer_weights: Vec<f64>,

    transition_flags: Vec<u32>,
    source_clips: Vec<i32>,
    source_seconds: Vec<f64>,
    source_speeds: Vec<f64>,
    transition_elapsed: Vec<f64>,
    transition_duration: Vec<f64>,
}

impl Scratch {
    pub fn capture(&mut self, world: &mut NativeWorld) -> Result<(), incant_runtime::RuntimeError> {
        world.snapshot_animations(&mut self.rows)?;
        self.graphs.capture(world)?;
        self.ik.capture(world)?;
        self.clips.clear();
        self.seconds.clear();
        self.speeds.clear();
        self.flags.clear();
        self.pose_offsets.clear();
        self.pose_clips.clear();
        self.pose_seconds.clear();
        self.pose_weights.clear();
        self.pose_kinds.clear();
        self.pose_splits.clear();
        self.pose_outer_weights.clear();
        self.pose_offsets.push(0);
        self.transition_flags.clear();
        self.source_clips.clear();
        self.source_seconds.clear();
        self.source_speeds.clear();
        self.transition_elapsed.clear();
        self.transition_duration.clear();
        for row in &self.rows {
            self.pose_kinds.extend([0, 1].map(|index| {
                row.pose
                    .and_then(|p| p.group(index).map(|g| g.interpolation as u8))
                    .unwrap_or(255)
            }));
            self.pose_splits.push(
                row.pose
                    .filter(|p| p.outer_weight().is_some())
                    .map_or(0, |p| {
                        p.group(0).expect("retained group").samples.len() as u8
                    }),
            );
            self.pose_outer_weights
                .push(row.pose.and_then(|p| p.outer_weight()).unwrap_or(0.0));
            if let Some(pose) = row.pose {
                for entry in pose.samples() {
                    self.pose_clips
                        .push(entry.sample.clip.map_or(-1, i32::from));
                    self.pose_seconds.push(entry.sample.seconds);
                    self.pose_weights.push(entry.weight);
                }
            }
            self.pose_offsets.push(self.pose_clips.len() as u32);
            let state = row.state;
            self.clips.push(state.sample.clip.map_or(-1, i32::from));
            self.seconds.push(state.sample.seconds);
            self.speeds.push(state.speed);
            self.flags.push(
                u32::from(state.playing)
                    | (u32::from(state.looping) << 1)
                    | (u32::from(state.completed) << 2),
            );
            let transition = state.transition;
            self.transition_flags.push(transition.map_or(0, |t| {
                1 | (u32::from(t.playing) << 1) | (u32::from(t.looping) << 2)
            }));
            self.source_clips
                .push(transition.map_or(0, |t| t.source.clip.map_or(-1, i32::from)));
            self.source_seconds
                .push(transition.map_or(0., |t| t.source.seconds));
            self.source_speeds.push(transition.map_or(0., |t| t.speed));
            self.transition_elapsed
                .push(transition.map_or(0., |t| t.elapsed));
            self.transition_duration
                .push(transition.map_or(0., |t| t.duration));
        }
        Ok(())
    }
    pub fn expose<'js>(&self, ctx: &Ctx<'js>) -> rquickjs::Result<Object<'js>> {
        let result = Object::new_proto(ctx.clone(), None)?;
        result.prop("ik", Property::from(self.ik.expose(ctx)?).enumerable())?;
        result.prop(
            "graphs",
            Property::from(self.graphs.expose(ctx)?).enumerable(),
        )?;
        let ids = Array::new(ctx.clone())?;
        for (index, row) in self.rows.iter().enumerate() {
            ids.as_object().prop(
                index as u32,
                Property::from(ulid::Ulid::from_bytes(row.id.0).to_string()).enumerable(),
            )?;
        }
        result.prop("ids", Property::from(ids).enumerable())?;
        result.prop(
            "clips",
            Property::from(TypedArray::new_copy(ctx.clone(), &self.clips)?).enumerable(),
        )?;
        result.prop(
            "seconds",
            Property::from(TypedArray::new_copy(ctx.clone(), &self.seconds)?).enumerable(),
        )?;
        result.prop(
            "speeds",
            Property::from(TypedArray::new_copy(ctx.clone(), &self.speeds)?).enumerable(),
        )?;
        result.prop(
            "flags",
            Property::from(TypedArray::new_copy(ctx.clone(), &self.flags)?).enumerable(),
        )?;
        result.prop(
            "transitionFlags",
            Property::from(TypedArray::new_copy(ctx.clone(), &self.transition_flags)?).enumerable(),
        )?;
        result.prop(
            "sourceClips",
            Property::from(TypedArray::new_copy(ctx.clone(), &self.source_clips)?).enumerable(),
        )?;
        result.prop(
            "sourceSeconds",
            Property::from(TypedArray::new_copy(ctx.clone(), &self.source_seconds)?).enumerable(),
        )?;
        result.prop(
            "sourceSpeeds",
            Property::from(TypedArray::new_copy(ctx.clone(), &self.source_speeds)?).enumerable(),
        )?;
        result.prop(
            "transitionElapsed",
            Property::from(TypedArray::new_copy(ctx.clone(), &self.transition_elapsed)?)
                .enumerable(),
        )?;
        result.prop(
            "transitionDuration",
            Property::from(TypedArray::new_copy(
                ctx.clone(),
                &self.transition_duration,
            )?)
            .enumerable(),
        )?;
        result.prop(
            "poseOffsets",
            Property::from(TypedArray::new_copy(ctx.clone(), &self.pose_offsets)?).enumerable(),
        )?;
        result.prop(
            "poseClips",
            Property::from(TypedArray::new_copy(ctx.clone(), &self.pose_clips)?).enumerable(),
        )?;
        result.prop(
            "poseSeconds",
            Property::from(TypedArray::new_copy(ctx.clone(), &self.pose_seconds)?).enumerable(),
        )?;
        for (name, data) in [
            ("poseKinds", &self.pose_kinds),
            ("poseSplits", &self.pose_splits),
        ] {
            result.prop(
                name,
                Property::from(TypedArray::new_copy(ctx.clone(), data)?).enumerable(),
            )?;
        }
        result.prop(
            "poseOuterWeights",
            Property::from(TypedArray::new_copy(ctx.clone(), &self.pose_outer_weights)?)
                .enumerable(),
        )?;
        result.prop(
            "poseWeights",
            Property::from(TypedArray::new_copy(ctx.clone(), &self.pose_weights)?).enumerable(),
        )?;
        Ok(result)
    }
}

#[cfg(all(test, feature = "compiler"))]
mod tests {
    use super::*;
    use incant_runtime::{AnimationSample, AnimationState, StableId};

    #[test]
    fn owned_aligned_arrays_preserve_rest_pose_reverse_speed_and_bypass_guest_setters() {
        let runtime = rquickjs::Runtime::new().unwrap();
        let context = rquickjs::Context::full(&runtime).unwrap();
        let scratch = Scratch {
            graphs: Default::default(),
            ik: Default::default(),
            rows: vec![AnimationSnapshot {
                id: StableId([0; 16]),
                blend: None,
                pose: None,
                state: AnimationState {
                    sample: AnimationSample {
                        clip: None,
                        seconds: 0.,
                    },
                    speed: -2.,
                    playing: false,
                    looping: true,
                    completed: false,
                    transition: None,
                },
            }],
            clips: vec![-1],
            seconds: vec![0.],
            speeds: vec![-2.],
            flags: vec![2],
            pose_offsets: vec![0, 0],
            pose_clips: vec![],
            pose_seconds: vec![],
            pose_weights: vec![],
            pose_kinds: vec![255, 255],
            pose_splits: vec![0],
            pose_outer_weights: vec![0.0],
            transition_flags: vec![0],
            source_clips: vec![0],
            source_seconds: vec![0.],
            source_speeds: vec![0.],
            transition_elapsed: vec![0.],
            transition_duration: vec![0.],
        };
        context.with(|ctx| {
            ctx.eval::<(), _>("Object.defineProperty(Array.prototype,'0',{set(){throw Error('array setter')}});for(const key of ['clips','seconds','speeds','flags','transitionFlags','sourceClips','sourceSeconds','sourceSpeeds','transitionElapsed','transitionDuration'])Object.defineProperty(Object.prototype,key,{set(){throw Error('object setter')}});").unwrap();
            ctx.globals().set("snapshot", scratch.expose(&ctx).unwrap()).unwrap();
            ctx.eval::<(), _>("if(snapshot.ids[0]!=='00000000000000000000000000'||snapshot.clips[0]!==-1||snapshot.seconds[0]!==0||snapshot.speeds[0]!==-2||snapshot.flags[0]!==2)throw Error('snapshot');snapshot.clips[0]=55;snapshot.seconds[0]=9;snapshot.speeds[0]=9;snapshot.flags[0]=9;for(const key of ['transitionFlags','sourceClips','sourceSeconds','sourceSpeeds','transitionElapsed','transitionDuration']){if(snapshot[key][0]!==0)throw Error('inactive');snapshot[key][0]=9;}").unwrap();
        });
        assert_eq!(scratch.clips, [-1]);
        assert_eq!(scratch.seconds, [0.]);
        assert_eq!(scratch.speeds, [-2.]);
        assert_eq!(scratch.flags, [2]);
        assert_eq!(scratch.transition_flags, [0]);
        assert_eq!(scratch.source_clips, [0]);
        assert_eq!(scratch.source_seconds, [0.]);
        assert_eq!(scratch.source_speeds, [0.]);
        assert_eq!(scratch.transition_elapsed, [0.]);
        assert_eq!(scratch.transition_duration, [0.]);
    }
}
