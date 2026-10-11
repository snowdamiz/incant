//! Bounded thread-affine native host ownership. Session workers are waiters;
//! only this thread constructs, uses and destroys its non-Send native state.
use super::worker;
use incant_agent::{
    AgentError,
    session::{TurnContext, TurnOutcome},
};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};

type Work = Box<dyn FnOnce(&mut worker::State, TurnContext) -> TurnOutcome + Send>;
struct Request {
    turn: TurnContext,
    work: Work,
    reply: mpsc::SyncSender<TurnOutcome>,
}
#[derive(Default)]
struct Control {
    closed: bool,
    sender: Option<mpsc::SyncSender<Request>>,
    thread: Option<JoinHandle<()>>,
}
pub(super) struct Owner {
    client: Client,
}
#[derive(Clone)]
pub(super) struct Client(Arc<Mutex<Control>>);
impl Owner {
    pub(super) fn new() -> Self {
        Self {
            client: Client(Arc::new(Mutex::new(Control::default()))),
        }
    }
    pub(super) fn client(&self) -> Client {
        self.client.clone()
    }
    pub(super) fn close(&self) {
        let mut control = self.client.0.lock().unwrap_or_else(|e| e.into_inner());
        control.closed = true;
        // Disconnect even retained client clones. An accepted request drains;
        // Session cancellation wakes approval and fences its tool publication.
        control.sender = None;
        // Never join provider IO or native teardown on the event thread.
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.close();
    }
}
impl Client {
    /// Only called inside Session's panic/accounting boundary. Lost replies or
    /// a broken owner panic with fixed text: Session then exhausts the uncertain
    /// allowance rather than reporting an unmeasured failed request as free.
    pub(super) fn run(
        &self,
        turn: TurnContext,
        work: impl FnOnce(&mut worker::State, TurnContext) -> TurnOutcome + Send + 'static,
    ) -> TurnOutcome {
        let (reply, result) = mpsc::sync_channel(1);
        {
            let mut control = self.0.lock().expect("native agent owner unavailable");
            // Cancellation before enqueue is known not to have run any work.
            // In particular, window shutdown may close admission before this
            // accepted Session waiter first gets scheduled.
            if turn
                .cancellation()
                .load(std::sync::atomic::Ordering::Acquire)
            {
                return TurnOutcome {
                    budget: turn.budget,
                    result: Err(AgentError::Interrupted),
                };
            }
            assert!(!control.closed, "native agent owner closed");
            if control.sender.is_none() {
                let (send, receive) = mpsc::sync_channel::<Request>(1);
                let thread = thread::Builder::new()
                    .name("incant-agent-native".into())
                    .spawn(move || {
                        let mut state = worker::State::default();
                        while let Ok(request) = receive.recv() {
                            let outcome = (request.work)(&mut state, request.turn);
                            // The waiter normally cannot disappear while running.
                            // If it does, abandon the host instead of reusing it.
                            if request.reply.send(outcome).is_err() {
                                break;
                            }
                        }
                    })
                    .expect("native agent owner could not start");
                control.sender = Some(send);
                control.thread = Some(thread);
            }
            // Session guarantees one accepted turn. A violated invariant never
            // creates an unbounded queue or blocks while holding this mutex.
            assert!(
                control
                    .sender
                    .as_ref()
                    .unwrap()
                    .try_send(Request {
                        turn,
                        work: Box::new(work),
                        reply,
                    })
                    .is_ok(),
                "native agent owner unavailable"
            );
        }
        match result.recv() {
            Ok(outcome) => outcome,
            Err(_) => {
                let thread = {
                    let mut control = self.0.lock().unwrap_or_else(|e| e.into_inner());
                    control.closed = true;
                    control.sender = None;
                    control.thread.take()
                };
                // A waiter may join a failed owner, including native/TLS teardown.
                // Its Session remains physically busy until this join finishes.
                if let Some(thread) = thread {
                    let _ = thread.join();
                }
                panic!("native agent owner failed");
            }
        }
    }
}

#[cfg(test)]
mod tests;
