//! Main-thread, view-bound display wake. No simulation or GPU work in callbacks.
mod gate;
mod inbox;
use gate::Gate;
use inbox::{Inbox, Pulse};
use objc2::{ClassType, DefinedClass, MainThreadOnly, define_class, msg_send, rc::Retained, sel};
use objc2_app_kit::NSView;
use objc2_foundation::{
    MainThreadMarker, NSObject, NSObjectProtocol, NSRunLoop, NSRunLoopCommonModes,
};
use objc2_quartz_core::{CADisplayLink, CAFrameRateRange};
use std::{cell::Cell, rc::Rc, sync::Arc, time::Instant};
use tao::{event_loop::EventLoopProxy, platform::macos::WindowExtMacOS, window::Window};

struct State {
    inbox: Rc<Cell<Inbox>>,
    proxy: EventLoopProxy<()>,
}
define_class!(
    // SAFETY: NSObject permits subclassing. State and selector run only on the
    // main run loop; neither the callback target nor its inbox retains the link.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = State]
    struct IncantDisplayWake;
    impl IncantDisplayWake {
        #[unsafe(method(displayPulse:))]
        fn display_pulse(&self, link: &CADisplayLink) {
            let state = self.ivars();
            let mut inbox = state.inbox.get();
            let notify = inbox.publish(Pulse {
                arrived: Instant::now(), timestamp: link.timestamp(),
                target: link.targetTimestamp(), duration: link.duration(),
            });
            state.inbox.set(inbox);
            if notify && state.proxy.send_event(()).is_err() {
                inbox.close();
                state.inbox.set(inbox);
            }
        }
    }
    unsafe impl NSObjectProtocol for IncantDisplayWake {}
);

pub(super) fn supported() -> bool {
    MainThreadMarker::new().is_some()
        && NSView::class()
            .instance_method(sel!(displayLinkWithTarget:selector:))
            .is_some()
}

#[cfg(feature = "native-independent-presentation")]
#[derive(Clone, Copy)]
pub(super) struct Receipt {
    pub sequence: u64,
    pub arrived: Instant,
    pub target_seconds: f64,
}

pub(super) struct Host {
    link: Retained<CADisplayLink>,
    _target: Retained<IncantDisplayWake>,
    // The Tao-owned native view stays alive through link invalidation.
    _window: Arc<Window>,
    inbox: Rc<Cell<Inbox>>,
    gate: Gate,
}
impl Host {
    pub fn new(window: Arc<Window>, proxy: EventLoopProxy<()>) -> crate::session::Result<Self> {
        let mtm = MainThreadMarker::new().ok_or("display wake requires the main thread")?;
        if !supported() {
            return Err(crate::wake::Error::Unsupported.into());
        }
        let inbox = Rc::new(Cell::new(Inbox::default()));
        let allocated = IncantDisplayWake::alloc(mtm).set_ivars(State {
            inbox: Rc::clone(&inbox),
            proxy,
        });
        // SAFETY: NSObject designated initialization with initialized Rust ivars.
        let target: Retained<IncantDisplayWake> = unsafe { msg_send![super(allocated), init] };
        // SAFETY: Tao exposes its live NSView; the retained Window outlives use.
        let view =
            unsafe { (window.ns_view() as *const NSView).as_ref() }.ok_or("missing native view")?;
        // SAFETY: target implements the exact CADisplayLink selector signature.
        let link = unsafe { view.displayLinkWithTarget_selector(&target, sel!(displayPulse:)) };
        link.setPreferredFrameRateRange(CAFrameRateRange {
            minimum: 60.0,
            maximum: 60.0,
            preferred: 60.0,
        });
        // SAFETY: callback target is main-thread-only, registered on main run loop.
        unsafe {
            link.addToRunLoop_forMode(&NSRunLoop::mainRunLoop(), NSRunLoopCommonModes);
        }
        Ok(Self {
            link,
            _target: target,
            _window: window,
            inbox,
            gate: Gate::new(Instant::now()),
        })
    }
    pub fn boundary(&mut self, now: Instant) {
        let mut inbox = self.inbox.get();
        self.gate.boundary(&mut inbox, now);
        self.inbox.set(inbox);
    }
    pub fn acknowledge(&mut self) {
        let mut inbox = self.inbox.get();
        self.gate.acknowledge(&mut inbox);
        self.inbox.set(inbox);
    }
    /// OS input/health timers cannot authorize active gameplay. A callback only
    /// permits the existing wall-clock scheduler to run; target time is metadata.
    pub fn poll(&mut self, now: Instant, active: bool) -> crate::session::Result<bool> {
        let mut inbox = self.inbox.get();
        let result = self.gate.poll(&mut inbox, now, active);
        self.inbox.set(inbox);
        Ok(result?)
    }
    /// Exact latest callback consumed by the most recent active poll only.
    #[cfg(feature = "native-independent-presentation")]
    pub fn accepted(&self) -> Option<Receipt> {
        self.gate.accepted.map(|(sequence, pulse)| Receipt {
            sequence,
            arrived: pulse.arrived,
            target_seconds: pulse.target,
        })
    }
    pub fn deadline(&self) -> Instant {
        self.gate.deadline(&self.inbox.get())
    }
    pub fn report(&self) -> serde_json::Value {
        let q = self.inbox.get();
        serde_json::json!({"mode":"view-display-link60", "diagnostic_only":true,
            "requested_hz":60,"simulation_clock":"unchanged-rational-wall-clock",
            "callbacks":q.callbacks,"coalesced":q.coalesced,"consumed":q.consumed,
            "active_pulses":self.gate.gameplay_pulses,"first_target_seconds":q.first.map(|p|p.target),
            "last_target_seconds":q.last.map(|p|p.target),
            "callback_arrival_span_ns":q.first.zip(q.last).map(|(a,b)| b.arrived.saturating_duration_since(a.arrived).as_nanos().to_string()),
            "last_reported_duration_seconds":q.last.map(|p|p.duration),
            "future_target_used_for_simulation":false,"performance_gate_passed":false})
    }
    pub fn stop(&mut self) {
        let mut inbox = self.inbox.get();
        inbox.close();
        self.inbox.set(inbox);
        self.link.invalidate();
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        self.stop();
    }
}
