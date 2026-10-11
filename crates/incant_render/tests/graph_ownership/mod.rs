//! CPU-only coverage of the same pass dependencies with thread-confined inputs.
use super::*;
use std::{cell::RefCell, rc::Rc};

struct Frame(Rc<()>);
struct Commands(Rc<RefCell<Vec<(Pass, std::thread::ThreadId)>>>);
fn record(pass: Pass, frame: NonSend<Frame>, encoder: NonSendMut<Commands>) {
    assert_eq!(Rc::strong_count(&frame.0), 2);
    encoder
        .0
        .borrow_mut()
        .push((pass, std::thread::current().id()));
}
fn initialize(frame: NonSend<Frame>, encoder: NonSendMut<Commands>) {
    record(Pass::InitializeScene, frame, encoder);
}
fn lights(frame: NonSend<Frame>, encoder: NonSendMut<Commands>) {
    record(Pass::AssignLights, frame, encoder);
}
fn shadow(frame: NonSend<Frame>, encoder: NonSendMut<Commands>) {
    record(Pass::Shadows, frame, encoder);
}
fn model(frame: NonSend<Frame>, encoder: NonSendMut<Commands>) {
    record(Pass::Models, frame, encoder);
}
fn sprite(frame: NonSend<Frame>, encoder: NonSendMut<Commands>) {
    record(Pass::Sprites, frame, encoder);
}
fn output(frame: NonSend<Frame>, encoder: NonSendMut<Commands>) {
    record(Pass::Display, frame, encoder);
}
#[test]
fn non_send_passes_stay_on_owner_thread_and_release_each_frame() {
    let mut world = World::new();
    let mut schedule = pass_schedule();
    schedule.add_systems((
        output.in_set(Pass::Display),
        model.in_set(Pass::Models),
        sprite.in_set(Pass::Sprites),
        shadow.in_set(Pass::Shadows),
        lights.in_set(Pass::AssignLights),
        initialize.in_set(Pass::InitializeScene),
    ));
    for _ in 0..2 {
        let frame = Rc::new(());
        let commands = Rc::new(RefCell::new(Vec::new()));
        world.insert_non_send(Frame(Rc::clone(&frame)));
        world.insert_non_send(Commands(Rc::clone(&commands)));
        schedule.run(&mut world);
        world.remove_non_send::<Frame>().unwrap();
        let encoded = world.remove_non_send::<Commands>().unwrap();
        assert!(!world.contains_non_send::<Frame>());
        assert!(!world.contains_non_send::<Commands>());
        assert_eq!(Rc::strong_count(&frame), 1);
        drop(encoded);
        assert_eq!(Rc::strong_count(&commands), 1);
        let commands = commands.borrow();
        assert_eq!(commands.len(), 6);
        assert!(
            commands
                .iter()
                .all(|(_, thread)| *thread == std::thread::current().id())
        );
        let index = |pass| {
            commands
                .iter()
                .position(|(found, _)| *found == pass)
                .unwrap()
        };
        for earlier in [Pass::InitializeScene, Pass::AssignLights, Pass::Shadows] {
            assert!(index(earlier) < index(Pass::Models));
        }
        assert!(index(Pass::Models) < index(Pass::Sprites));
        assert!(index(Pass::Sprites) < index(Pass::Display));
    }
}
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn desktop_graph_and_renderer_preserve_send_sync_contract() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<FrameGraph>();
    send_sync::<PreparedFrame>();
    send_sync::<Encoder>();
    send_sync::<crate::Renderer>();
    send_sync::<crate::ResourceError>();
}
