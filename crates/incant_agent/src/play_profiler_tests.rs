use super::*;
use incant_cmd::CommandBus;
use incant_doc::Project;
use std::{cell::Cell, sync::Mutex};

struct Host<'a> {
    available: bool,
    calls: Cell<usize>,
    bus: &'a Mutex<CommandBus>,
    cancel: &'a AtomicBool,
    switch: bool,
    interrupt: bool,
}
impl EngineHost for Host<'_> {
    fn native_play_available(&self) -> bool {
        true
    }
    fn shared_play_available(&self) -> bool {
        true
    }
    fn profiler_available(&self) -> bool {
        self.available
    }
    fn profiler_read(
        &self,
        project: &str,
        revision: u64,
        frames: usize,
    ) -> Result<Value, AgentError> {
        self.calls.set(self.calls.get() + 1);
        let mut bus = self
            .bus
            .try_lock()
            .expect("reporting must release project lock");
        assert_eq!(project, bus.project().id);
        assert_eq!(revision, bus.revision());
        assert_eq!(frames, 1);
        if self.switch {
            *bus = CommandBus::new(Project::empty("Replacement")).unwrap();
        }
        if self.interrupt {
            self.cancel.store(true, Ordering::Relaxed);
        }
        Ok(json!({"transport_fixture":true}))
    }
    fn screenshot(
        &mut self,
        _: &Project,
        _: Option<&str>,
        _: u32,
        _: u32,
    ) -> Result<Value, AgentError> {
        panic!("profiler must never call image capture")
    }
}
#[test]
fn profiler_transport_refuses_malformed_or_unavailable_requests_before_host() {
    let bus = Mutex::new(CommandBus::new(Project::empty("Owned")).unwrap());
    let cancel = AtomicBool::new(false);
    let mut access = ProjectAccess::shared(&bus, &cancel).unwrap();
    let mut host = Host {
        available: true,
        calls: Cell::new(0),
        bus: &bus,
        cancel: &cancel,
        switch: false,
        interrupt: false,
    };
    for args in [
        json!({}),
        json!({"frames":0}),
        json!({"frames":257}),
        json!({"frames":-1}),
        json!({"frames":null}),
        json!({"frames":1,"source":"other"}),
    ] {
        assert!(dispatch(&mut access, &mut host, "profiler_read", args, &cancel).is_err());
    }
    host.available = false;
    assert!(
        dispatch(
            &mut access,
            &mut host,
            "profiler_read",
            json!({"frames":1}),
            &cancel
        )
        .is_err()
    );
    assert_eq!(host.calls.get(), 0);
}
#[test]
fn shared_profiler_rechecks_project_and_cancellation_after_unlocked_read() {
    for switch in [false, true] {
        let bus = Mutex::new(CommandBus::new(Project::empty("Owned")).unwrap());
        let cancel = AtomicBool::new(false);
        let mut access = ProjectAccess::shared(&bus, &cancel).unwrap();
        let mut host = Host {
            available: true,
            calls: Cell::new(0),
            bus: &bus,
            cancel: &cancel,
            switch,
            interrupt: !switch,
        };
        let error = dispatch(
            &mut access,
            &mut host,
            "profiler_read",
            json!({"frames":1}),
            &cancel,
        )
        .unwrap_err();
        if switch {
            assert!(matches!(error, AgentError::ProjectChanged));
        } else {
            assert!(matches!(error, AgentError::Interrupted));
        }
        assert_eq!(host.calls.get(), 1);
    }
}
