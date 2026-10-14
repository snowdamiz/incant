//! Host staging/prewarm controls; acquiring a presentation target is forbidden.
use super::*;
use incant_render::metalfx::{Config, Fallback, MetalFxError, Preference};
use std::{
    convert::Infallible,
    future::Future,
    task::{Context, Poll, Wake, Waker},
};

struct NoTargets;
struct NoFrame;
impl TargetProvider for NoTargets {
    type Error = Infallible;
    type Frame = NoFrame;
    fn acquire(&mut self, _: &Renderer) -> Result<NoFrame, Infallible> {
        panic!("prewarm/staging must not acquire a presentation target")
    }
}
impl RenderFrame for NoFrame {
    type Error = Infallible;
    fn target(&self) -> RetainedTarget<'_> {
        panic!("no frame exists")
    }
    fn present(self) -> Result<(), Infallible> {
        panic!("no frame exists")
    }
}

// Bounded test executor avoids adding a runtime dependency for startup futures.
fn startup<F: Future>(future: F) -> F::Output {
    struct Thread(std::thread::Thread);
    impl Wake for Thread {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Waker::from(Arc::new(Thread(std::thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    let deadline = Instant::now() + std::time::Duration::from_secs(30);
    loop {
        if let Poll::Ready(result) = future.as_mut().poll(&mut cx) {
            return result;
        }
        assert!(
            Instant::now() < deadline,
            "startup future exceeded 30 seconds"
        );
        std::thread::park_timeout(std::time::Duration::from_millis(10));
    }
}

#[test]
#[ignore = "requires coordinated Mac MetalFX; startup/staging only, zero presentation submissions/maps"]
fn prepared_extent_change_refuses_stale_publication_without_acquiring_target() {
    let mut renderer = Renderer::headless().unwrap();
    startup(renderer.enable_metalfx()).unwrap();
    let renderer = Arc::new(renderer);
    let input = SceneInput {
        nodes: vec![incant_render_data::SceneNode {
            id: "camera".into(),
            world: Default::default(),
            camera: Some(incant_types::Camera {
                fov_degrees: 60.,
                near: 0.1,
                far: 50.,
                projection: incant_types::CameraProjection::Orthographic { vertical_size: 8. },
            }),
            light: None,
            mesh: None,
            sprite: None,
            tilemap: None,
            diagnostic: false,
        }],
        ..Default::default()
    };
    let (settings, _reset) = GpuRenderSettings {
        renderer: Arc::clone(&renderer),
        targets: NoTargets,
    }
    .with_metalfx(
        Preference::RequireMetalFx,
        Config {
            relative_depth: 0.01,
            absolute_depth: 0.001,
        },
        [64, 64],
    )
    .unwrap();
    let mut host = GpuRenderConsumer::prepare(&input, "camera", None, settings).unwrap();
    let old = host.prepare_textures(&input, "camera").unwrap();
    host.validate_textures(&old).unwrap();
    // An invalid resize preserves both the current extent and staged authority.
    assert!(host.prepare_metalfx_extent([0, 96]).is_err());
    host.validate_textures(&old).unwrap();
    host.prepare_metalfx_extent([96, 96]).unwrap();
    let error = host.validate_textures(&old).unwrap_err();
    let GpuHostError::Render(error) = error else {
        panic!("wrong stale-stage error")
    };
    assert_eq!(
        error.downcast_ref::<MetalFxError>(),
        Some(&MetalFxError::Unavailable(Fallback::UnpreparedExtent))
    );
    assert!(!host.is_failed());
    let mut current = host.prepare_textures(&input, "camera").unwrap();
    host.validate_textures(&current).unwrap();
    host.publish_textures(&mut current);
    assert_eq!(
        host.live.as_ref().unwrap().metalfx.unwrap().output_extent,
        [96, 96]
    );
    assert_eq!(host.accepted_frames(), 0);
    assert!(host.motion_frame().is_none());
    assert_eq!(host.metalfx_selection(), None);
    assert!(!host.is_failed());
    drop((old, current, host));
    renderer.check_device().unwrap();
    println!("stale/fresh publication: acquisitions=0 presentation_submissions=0 maps=0");
}
