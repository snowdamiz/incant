use super::{CompileFailure, FailureKind, MaterialPrewarmError, worker::Worker};
use crate::{Result, materials::PipelineKey, shader::Stages};
use incant_render_data::{
    material_keys::{MATERIAL_KEY_COUNT, MaterialKey},
    prewarm::{AdmittedMaterials, MaterialProgram},
};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

#[derive(serde::Serialize)]
pub struct MaterialKeyReport {
    pub key: MaterialKey,
    pub requested: bool,
    pub ready: bool,
    pub failure: Option<CompileFailure>,
}
#[derive(serde::Serialize)]
pub struct MaterialPrewarmReport {
    pub worker_thread: String,
    pub prewarmed: u32,
    pub fallback_selections: u64,
    pub real_selections: u64,
    pub keys: Vec<MaterialKeyReport>,
}
pub(crate) struct System {
    fallback: [Option<wgpu::RenderPipeline>; MATERIAL_KEY_COUNT],
    worker: Worker<wgpu::RenderPipeline>,
    prewarmed: u32,
    fallback_selections: AtomicU64,
    real_selections: AtomicU64,
}
struct Builder {
    device: wgpu::Device,
    shader: Stages,
    layouts: [wgpu::BindGroupLayout; 4],
    cache: Option<wgpu::PipelineCache>,
    #[cfg(all(feature = "native-compilation", target_os = "macos"))]
    observer: Option<crate::compilation::Observer>,
}
impl Builder {
    fn build(&self, key: MaterialKey) -> std::result::Result<wgpu::RenderPipeline, CompileFailure> {
        // This scope is entered and dropped by the worker itself. Other threads
        // remain explicitly unscoped; the diagnostic does not claim a frame or
        // driver-stall bound merely because this call has a background owner.
        #[cfg(all(feature = "native-compilation", target_os = "macos"))]
        let _scope = self
            .observer
            .as_ref()
            .map(|observer| {
                observer
                    .scope(crate::compilation::Context {
                        session_generation: 1,
                        segment: key.index() as u64 + 1,
                        phase: crate::compilation::Phase::Background,
                        ..Default::default()
                    })
                    .map_err(|error| {
                        CompileFailure::new(FailureKind::Observation, format_args!("{error:?}"))
                    })
            })
            .transpose()?;
        // Guard push/create/pop/future completion all run on this same caller
        // thread (cold prewarm or the single worker). Guards never cross threads.
        let validation = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let memory = self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let internal = self.device.push_error_scope(wgpu::ErrorFilter::Internal);
        let pipeline = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::material_pipeline::build(
                &self.device,
                &self.shader,
                PipelineKey::from_material(key),
                &self.layouts.each_ref(),
                self.cache.as_ref(),
            )
        }));
        let mut failure = None;
        for (scope, kind) in [
            (internal, FailureKind::Internal),
            (memory, FailureKind::OutOfMemory),
            (validation, FailureKind::Validation),
        ] {
            if let Some(error) = pollster::block_on(scope.pop()) {
                failure = Some(CompileFailure::new(kind, error));
            }
        }
        if let Some(failure) = failure {
            return Err(failure);
        }
        pipeline.map_err(|_| CompileFailure::new(FailureKind::Panic, "pipeline creation panic"))
    }
}
impl System {
    pub(crate) fn new(
        device: &wgpu::Device,
        shader: Stages,
        layouts: [&wgpu::BindGroupLayout; 4],
        backend: wgpu::Backend,
        cache: Option<&wgpu::PipelineCache>,
        admitted: AdmittedMaterials,
        #[cfg(all(feature = "native-compilation", target_os = "macos"))] observer: Option<
            crate::compilation::Observer,
        >,
    ) -> Result<Self> {
        let program = if cfg!(feature = "cooked-materials") {
            MaterialProgram::Cooked
        } else {
            MaterialProgram::Wgsl
        };
        if admitted.profile().program != program {
            return Err(MaterialPrewarmError::Profile.into());
        }
        let fallback_shader = super::fallback_shader(device, backend)?;
        let fallback_builder = Builder {
            device: device.clone(),
            shader: fallback_shader,
            layouts: layouts.map(Clone::clone),
            cache: cache.cloned(),
            #[cfg(all(feature = "native-compilation", target_os = "macos"))]
            observer: None,
        };
        let mut fallback: [Option<wgpu::RenderPipeline>; MATERIAL_KEY_COUNT] =
            std::array::from_fn(|_| None);
        for (index, slot) in fallback.iter_mut().enumerate() {
            let key = MaterialKey::try_from(index as u8).expect("closed catalog");
            if admitted.profile().admits(key) {
                *slot = Some(fallback_builder.build(key)?);
            }
        }
        let builder = Builder {
            device: device.clone(),
            shader,
            layouts: layouts.map(Clone::clone),
            cache: cache.cloned(),
            #[cfg(all(feature = "native-compilation", target_os = "macos"))]
            observer: None,
        };
        let mut ready = Vec::new();
        for key in admitted.keys() {
            ready.push((key, builder.build(key)?));
        }
        let prewarmed = ready.len() as u32;
        #[cfg(all(feature = "native-compilation", target_os = "macos"))]
        let builder = Builder {
            observer,
            ..builder
        };
        let worker = Worker::new(ready, move |key| builder.build(key))?;
        Ok(Self {
            fallback,
            worker,
            prewarmed,
            fallback_selections: AtomicU64::new(0),
            real_selections: AtomicU64::new(0),
        })
    }
    pub(crate) fn pipeline(&self, key: PipelineKey) -> Result<wgpu::RenderPipeline> {
        let key = key.material_key().ok_or(MaterialPrewarmError::Profile)?;
        let fallback = self.fallback[key.index()]
            .as_ref()
            .ok_or(MaterialPrewarmError::Profile)?;
        if let Some(Ok(pipeline)) = self.worker.get_or_request(key)? {
            self.real_selections.fetch_add(1, Ordering::Relaxed);
            return Ok(pipeline.clone());
        }
        // Pending and failed keys keep the admitted exact alpha/depth/coverage
        // fallback. A failed key is not requeued; its bounded error is observable.
        self.fallback_selections.fetch_add(1, Ordering::Relaxed);
        Ok(fallback.clone())
    }
    pub(crate) fn report(&self) -> MaterialPrewarmReport {
        let requested = self.worker.requested();
        MaterialPrewarmReport {
            worker_thread: self.worker.thread_identity.clone(),
            prewarmed: self.prewarmed,
            fallback_selections: self.fallback_selections.load(Ordering::Relaxed),
            real_selections: self.real_selections.load(Ordering::Relaxed),
            keys: (0..MATERIAL_KEY_COUNT)
                .filter(|&i| self.fallback[i].is_some())
                .map(|i| {
                    let key = MaterialKey::try_from(i as u8).unwrap();
                    MaterialKeyReport {
                        key,
                        requested: requested & (1 << i) != 0,
                        ready: self.worker.get(key).is_some_and(|v| v.is_ok()),
                        failure: self.worker.get(key).and_then(|v| v.as_ref().err()).cloned(),
                    }
                })
                .collect(),
        }
    }
    pub(crate) fn close(&mut self, timeout: Duration) -> Result<()> {
        Ok(self.worker.close(timeout)?)
    }
    #[cfg(test)]
    pub(super) fn pause_worker(&self, paused: bool) {
        self.worker.pause(paused);
    }
}
