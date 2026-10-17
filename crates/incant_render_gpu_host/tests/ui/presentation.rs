use super::*;
use incant_render_gpu_host::PresentationMode;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

struct Batched {
    inner: Targets,
    defer: Arc<AtomicBool>,
    presents: Arc<AtomicUsize>,
}
impl TargetProvider for Batched {
    type Error = TargetError;
    type Frame = Frame;
    fn acquire(&mut self, renderer: &Renderer) -> Result<Frame, TargetError> {
        assert!(!self.defer.load(Ordering::SeqCst));
        self.presents.fetch_add(1, Ordering::SeqCst);
        self.inner.acquire(renderer)
    }
    fn presentation_mode(&self) -> PresentationMode {
        if self.defer.load(Ordering::SeqCst) {
            PresentationMode::AdmitOnly
        } else {
            PresentationMode::Present
        }
    }
}

#[test]
#[ignore = "requires coordinated native GPU; no performance claim"]
fn intermediate_ui_receipts_commit_without_display_and_final_ui_matches_ordinary() {
    let size = [320, 192];
    let (mut ordinary, reference) = game(true, size);
    let defer = Arc::new(AtomicBool::new(false));
    let acquisitions = Arc::new(AtomicUsize::new(0));
    let (mut batched, probe) = game_with_targets(true, size, |s| GpuRenderSettings {
        renderer: s.renderer,
        targets: Batched {
            inner: s.targets,
            defer: Arc::clone(&defer),
            presents: Arc::clone(&acquisitions),
        },
    });
    let step = |game: &mut Hosted<Batched>, events: &[UiEvent]| {
        let mut pcm = vec![0.; game.next_output_frames().unwrap() * 2];
        game.tick_with_ui(&[], events, &mut pcm).unwrap();
    };
    tick(&mut ordinary, &[], &[]);
    step(&mut batched, &[]);
    let displayed = probe.pixels();
    assert_eq!(displayed, reference.pixels());
    defer.store(true, Ordering::SeqCst);
    tick(&mut ordinary, &[], &[UiEvent::Focus(id(FIELD))]);
    step(&mut batched, &[UiEvent::Focus(id(FIELD))]);
    let target = batched.host().unwrap().ui().unwrap().focus().unwrap();
    assert_eq!(
        target,
        ordinary.host().unwrap().ui().unwrap().focus().unwrap()
    );
    let edit = UiEvent::Commit {
        target,
        text: " 日本".into(),
    };
    tick(&mut ordinary, &[], std::slice::from_ref(&edit));
    step(&mut batched, std::slice::from_ref(&edit));
    assert_eq!(batched.host().unwrap().render_input().tick(), 3);
    assert_eq!(acquisitions.load(Ordering::SeqCst), 1);
    assert_eq!(batched.host().unwrap().consumer().accepted_frames(), 1);
    assert_eq!(
        probe.pixels(),
        displayed,
        "intermediate UI must not overwrite display"
    );
    assert!(
        batched
            .logs()
            .unwrap()
            .entries()
            .iter()
            .any(|entry| entry.message.contains("日本"))
    );
    defer.store(false, Ordering::SeqCst);
    tick(&mut ordinary, &[], &[]);
    step(&mut batched, &[]);
    assert_eq!(probe.pixels(), reference.pixels());
    assert_ne!(probe.pixels(), displayed);
    assert_eq!(acquisitions.load(Ordering::SeqCst), 2);
    assert_eq!(batched.host().unwrap().consumer().accepted_frames(), 2);
    assert_eq!(
        ordinary.world().unwrap().snapshot(),
        batched.world().unwrap().snapshot()
    );
    assert_eq!(
        ordinary.rendered_frames().unwrap(),
        batched.rendered_frames().unwrap()
    );
}
