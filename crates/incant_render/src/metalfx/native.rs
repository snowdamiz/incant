//! The only raw API boundary. All objects remain thread-bound; no unsafe
//! Send/Sync implementation, foreign queue, commit, wait or CPU texture copy.
use super::{Fallback, MetalFxError};
use objc2::{
    rc::Retained,
    runtime::{AnyObject, ProtocolObject},
};
use objc2_metal::{
    MTLCommandBuffer, MTLHazardTrackingMode, MTLPixelFormat, MTLResource, MTLStorageMode,
    MTLTexture, MTLTextureUsage,
};
use objc2_metal_fx::{MTLFXTemporalScaler, MTLFXTemporalScalerBase, MTLFXTemporalScalerDescriptor};
use std::{marker::PhantomData, rc::Rc};

pub(super) struct Scaler {
    raw: Retained<ProtocolObject<dyn MTLFXTemporalScaler>>,
    pub size: [u32; 2],
    usage: [MTLTextureUsage; 5],
    _thread: PhantomData<Rc<()>>,
}
pub(super) fn range(device: &wgpu::Device) -> Result<[f32; 2], Fallback> {
    if !objc2::available!(macos = 14.4, ios = 17.4, ..) {
        return Err(Fallback::OperatingSystem);
    }
    // SAFETY: borrowed device guard outlives every native call, no resource is
    // destroyed or queue/device ownership transferred. Version checked above.
    let d = unsafe { device.as_hal::<wgpu::hal::api::Metal>() }.ok_or(Fallback::Backend)?;
    unsafe {
        if !MTLFXTemporalScalerDescriptor::supportsDevice(d.raw_device()) {
            return Err(Fallback::Device);
        }
        let r = [
            MTLFXTemporalScalerDescriptor::supportedInputContentMinScaleForDevice(d.raw_device()),
            MTLFXTemporalScalerDescriptor::supportedInputContentMaxScaleForDevice(d.raw_device()),
        ];
        super::plan::upscale_range(r)
    }
}
impl Scaler {
    pub fn new(device: &wgpu::Device, size: [u32; 2]) -> Result<Self, Fallback> {
        let range = range(device)?;
        let d = unsafe { device.as_hal::<wgpu::hal::api::Metal>() }.ok_or(Fallback::Backend)?;
        // SAFETY: exclusively owned descriptor and newly created scaler; exact
        // device and bounded dimensions are supplied by the checked native plan.
        // No framework object is shared with another Rust thread.
        unsafe {
            let desc = MTLFXTemporalScalerDescriptor::new();
            desc.setInputWidth(size[0] as usize);
            desc.setInputHeight(size[1] as usize);
            desc.setOutputWidth(size[0] as usize);
            desc.setOutputHeight(size[1] as usize);
            desc.setColorTextureFormat(MTLPixelFormat::RGBA16Float);
            desc.setDepthTextureFormat(MTLPixelFormat::R32Float);
            desc.setMotionTextureFormat(MTLPixelFormat::RG16Float);
            desc.setOutputTextureFormat(MTLPixelFormat::RGBA16Float);
            desc.setAutoExposureEnabled(false);
            // Compile at owner/extent preparation, not lazily during ordinary
            // retained frames. DRS content changes reuse this same scaler.
            desc.setRequiresSynchronousInitialization(true);
            desc.setInputContentPropertiesEnabled(true);
            desc.setInputContentMinScale(range[0]);
            desc.setInputContentMaxScale(range[1]);
            desc.setReactiveMaskTextureEnabled(true);
            desc.setReactiveMaskTextureFormat(MTLPixelFormat::R8Unorm);
            let raw = desc
                .newTemporalScalerWithDevice(d.raw_device())
                .ok_or(Fallback::Device)?;
            raw.setPreExposure(1.);
            raw.setDepthReversed(false);
            raw.setMotionVectorScaleX(1.);
            raw.setMotionVectorScaleY(1.);
            let usage = [
                raw.colorTextureUsage(),
                raw.depthTextureUsage(),
                raw.motionTextureUsage(),
                raw.reactiveTextureUsage(),
                raw.outputTextureUsage(),
            ];
            // Engine textures support sampled/render inputs and sampled/storage
            // output. Unknown native requirements are a typed fallback, never
            // an unchecked extra usage or an imported foreign allocation.
            let read = MTLTextureUsage::ShaderRead | MTLTextureUsage::RenderTarget;
            if usage[..4].iter().any(|u| !read.contains(*u))
                || !(read | MTLTextureUsage::ShaderWrite).contains(usage[4])
            {
                return Err(Fallback::Format);
            }
            Ok(Self {
                raw,
                size,
                usage,
                _thread: PhantomData,
            })
        }
    }
    pub fn encode(
        &mut self,
        device: &wgpu::Device,
        textures: &[wgpu::Texture; 6],
        content: [u32; 2],
        jitter: [f32; 2],
        reset: bool,
    ) -> Result<wgpu::CommandBuffer, MetalFxError> {
        if content.contains(&0)
            || content.iter().zip(self.size).any(|(a, b)| *a > b)
            || !jitter.iter().all(|x| x.is_finite())
        {
            return Err(MetalFxError::Interop);
        }
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("MetalFX raw-only pass"),
        });
        // SAFETY: these read guards fence destruction throughout native encoding;
        // the real graph holds safe Texture clones through submitted completion.
        let guards = textures
            .iter()
            .map(|t| unsafe { t.as_hal::<wgpu::hal::api::Metal>() })
            .collect::<Option<Vec<_>>>()
            .ok_or(MetalFxError::Interop)?;
        let raw = guards.iter().map(|g| g.raw_handle()).collect::<Vec<_>>();
        let formats = [
            MTLPixelFormat::RGBA16Float,
            MTLPixelFormat::R32Float,
            MTLPixelFormat::RG16Float,
            MTLPixelFormat::R8Unorm,
            MTLPixelFormat::RGBA16Float,
            MTLPixelFormat::R16Float,
        ];
        for (i, t) in raw.iter().enumerate() {
            let size = if i == 5 { [1, 1] } else { self.size };
            if t.pixelFormat() != formats[i]
                || t.width() != size[0] as usize
                || t.height() != size[1] as usize
                || t.storageMode() != MTLStorageMode::Private
                || t.hazardTrackingMode() == MTLHazardTrackingMode::Untracked
                || (i < 5 && !t.usage().contains(self.usage[i]))
            {
                return Err(MetalFxError::Interop);
            }
        }
        // SAFETY: raw-only encoder, no open wgpu render/compute pass. It remains
        // owned by wgpu: never ended, committed, destroyed or submitted here.
        // All texture usage/init/transitions are encoded by pre/post wgpu buffers
        // on this same queue. Metal's tracked resources order the native passes.
        unsafe {
            encoder.as_hal_mut::<wgpu::hal::api::Metal, _, _>(|hal| {
                let cb = hal
                    .and_then(|h| h.raw_command_buffer())
                    .ok_or(MetalFxError::Interop)?;
                if !cb.retainedReferences()
                    || raw.iter().any(|t| {
                        !std::ptr::eq::<ProtocolObject<dyn objc2_metal::MTLDevice>>(
                            &*t.device(),
                            &*cb.device(),
                        )
                    })
                {
                    return Err(MetalFxError::Interop);
                }
                self.raw.setInputContentWidth(content[0] as usize);
                self.raw.setInputContentHeight(content[1] as usize);
                self.raw.setColorTexture(Some(raw[0]));
                self.raw.setDepthTexture(Some(raw[1]));
                self.raw.setMotionTexture(Some(raw[2]));
                self.raw.setReactiveMaskTexture(Some(raw[3]));
                self.raw.setOutputTexture(Some(raw[4]));
                self.raw.setExposureTexture(Some(raw[5]));
                self.raw.setJitterOffsetX(jitter[0]);
                self.raw.setJitterOffsetY(jitter[1]);
                self.raw.setReset(reset);
                // The pure Objective-C scaler is retained by its native command
                // buffer until that buffer is destroyed, including abandonment.
                // No Rust data crosses a thread and no callback accesses this owner.
                // The stable private key cannot overwrite an unrelated association.
                static KEY: u8 = 0;
                objc2::ffi::objc_setAssociatedObject(
                    (cb as *const ProtocolObject<dyn objc2_metal::MTLCommandBuffer>)
                        .cast_mut()
                        .cast::<AnyObject>(),
                    (&KEY as *const u8).cast(),
                    Retained::as_ptr(&self.raw).cast_mut().cast::<AnyObject>(),
                    objc2::ffi::OBJC_ASSOCIATION_RETAIN,
                );
                self.raw.encodeToCommandBuffer(cb);
                Ok(())
            })
        }?;
        Ok(encoder.finish())
    }
}
// Fail compilation if the native mutable owner accidentally becomes Send/Sync.
const _: fn() = || {
    trait Ambiguous<A> {
        fn check() {}
    }
    struct SendMarker;
    struct SyncMarker;
    impl<T: ?Sized> Ambiguous<()> for T {}
    impl<T: ?Sized + Send> Ambiguous<SendMarker> for T {}
    impl<T: ?Sized + Sync> Ambiguous<SyncMarker> for T {}
    let _ = <Scaler as Ambiguous<_>>::check;
};

#[cfg(test)]
#[test]
#[ignore = "coordinated actual Metal encoder lifetime control; no queue submission/map"]
fn abandoned_raw_buffers_retain_native_owner_until_drop() {
    let r = crate::test_support::checked(crate::Renderer::headless().unwrap());
    let range = range(&r.device).expect("actual native capability required");
    let budget = super::lifecycle::Budget::default();
    let mut buffers = Vec::new();
    for size in [64, 96] {
        let p = super::plan::Plan::new(
            [0., 0., (size / 2) as f32, (size / 2) as f32],
            [0., 0., size as f32, size as f32],
            range,
        )
        .unwrap();
        let targets =
            super::targets::Targets::new(&r.device, p, budget.reserve(p.bytes().unwrap()).unwrap())
                .unwrap();
        let mut scaler = Scaler::new(&r.device, p.size()).unwrap();
        let command = scaler
            .encode(&r.device, &targets.textures, [size / 2; 2], [0.; 2], true)
            .unwrap();
        // Staging supersession and caller teardown may drop both owners before
        // the unsubmitted buffer. Native retainedReferences + association own
        // the encoded references; there is intentionally no execute/commit.
        drop(scaler);
        drop(targets);
        buffers.push(command);
    }
    drop(buffers);
    budget.check(super::MAX_METALFX_TRANSITION_BYTES).unwrap();
    r.device.poll(wgpu::PollType::Poll).unwrap();
    r.check_device().unwrap();
    println!("two superseded native encodes dropped after scaler/targets; submissions0 maps0");
}
