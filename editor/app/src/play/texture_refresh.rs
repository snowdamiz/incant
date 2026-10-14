//! One cold revision-checked authoring coordinator and one opaque staged ticket.
use super::*;
use crate::ConsoleEvent;

pub(super) struct Pending<T> {
    serial: u64,
    value: Option<(u64, u32, u64, T)>,
}
impl<T> Default for Pending<T> {
    fn default() -> Self {
        Self {
            serial: 0,
            value: None,
        }
    }
}
impl<T> Pending<T> {
    pub fn clear(&mut self) {
        self.value.take();
    }
    pub fn stage(
        &mut self,
        generation: u32,
        revision: u64,
        prepare: impl FnOnce() -> Result<T, String>,
    ) -> Result<u64, String> {
        self.clear();
        let token = self
            .serial
            .checked_add(1)
            .ok_or("Texture refresh ticket exhausted")?;
        let value = prepare()?;
        self.serial = token;
        self.value = Some((token, generation, revision, value));
        Ok(token)
    }
    pub fn take(&mut self, token: u64, generation: u32, revision: u64) -> Result<T, String> {
        if !self
            .value
            .as_ref()
            .is_some_and(|(t, g, r, _)| (*t, *g, *r) == (token, generation, revision))
        {
            return Err(
                "Texture refresh ticket is stale or belongs to another session/revision".into(),
            );
        }
        Ok(self.value.take().expect("checked pending ticket").3)
    }
    pub fn cancel(&mut self, token: u64, generation: u32) {
        if self
            .value
            .as_ref()
            .is_some_and(|(t, g, _, _)| (*t, *g) == (token, generation))
        {
            self.clear();
        }
    }
}
pub(super) fn publish_at_revision<T>(
    bus: &Mutex<project::LoadState>,
    service: &Service,
    revision: u64,
    generation: u32,
    publish: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let state = bus.lock().map_err(|_| "Project lock is unavailable")?;
    if !matches!(&*state, project::LoadState::Ready(bus) if bus.revision() == revision) {
        drop(state);
        return Err(
            "Project changed while texture resources were prepared; prior live catalog retained"
                .into(),
        );
    }
    let play = service
        .status
        .lock()
        .map_err(|_| "Play state is unavailable")?;
    if play.status.generation != generation || !matches!(play.status.phase, "running" | "paused") {
        drop(play);
        drop(state);
        return Err("Play session changed while texture resources were prepared; prior live catalog retained".into());
    }
    // Both short guards remain held until publication returns: an asynchronous
    // session change cannot slip between the last generation check and swap.
    // Only already-prepared handles move here. The returned retirement owner is
    // disposed by the caller after this guard has been released.
    // Native panic remains terminal for Play, but must not poison the editor's
    // authoring/status mutexes. The borrowed callback owns no prepared handles.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(publish));
    drop(play);
    drop(state);
    match outcome {
        Ok(result) => result,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}
struct Refreshing(Arc<Service>);
impl Drop for Refreshing {
    fn drop(&mut self) {
        self.0.refreshing.store(false, Ordering::Release);
    }
}
pub fn schedule_refresh(editor: Arc<Editor>, revision: u64) {
    let Ok(status) = editor.play.snapshot() else {
        return;
    };
    if !status.spatial || !matches!(status.phase, "running" | "paused") {
        return;
    }
    if !editor
        .play
        .basis
        .lock()
        .is_ok_and(|basis| basis.as_ref().is_some_and(|(g, _)| *g == status.generation))
    {
        return;
    }
    if editor
        .play
        .refresh_attempt
        .lock()
        .is_ok_and(|last| *last == Some((status.generation, revision)))
    {
        return;
    }
    if editor
        .play
        .refreshing
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }
    std::thread::spawn(move || {
        let _refreshing = Refreshing(editor.play.clone());
        let result = refresh(&editor, status.generation, revision);
        if let Err(message) = result
            && editor.alive.load(Ordering::Relaxed)
        {
            if let Ok(mut console) = editor.console.lock() {
                console.push(ConsoleEvent::new(
                    "error",
                    format!("Native Play texture refresh: {message}"),
                ));
            }
            editor.play.refresh_event.store(true, Ordering::Release);
        }
        drop(_refreshing);
        // A newer commit can arrive while one refresh is in flight. Coalesce it
        // into the next single attempt rather than lose its notification.
        let latest = editor.bus.lock().ok().and_then(|state| match &*state {
            project::LoadState::Ready(bus) => Some(bus.revision()),
            _ => None,
        });
        if editor.alive.load(Ordering::Relaxed)
            && let Some(latest) = latest
        {
            schedule_refresh(editor, latest);
        }
    });
}
fn refresh(editor: &Editor, generation: u32, revision: u64) -> Result<(), String> {
    *editor
        .play
        .refresh_attempt
        .lock()
        .map_err(|_| "Play refresh state is unavailable")? = Some((generation, revision));
    let basis = editor
        .play
        .basis
        .lock()
        .map_err(|_| "Play authoring basis is unavailable")?
        .as_ref()
        .filter(|(g, _)| *g == generation)
        .map(|(_, basis)| basis.clone())
        .ok_or("Play authoring basis belongs to another session")?;
    let document = {
        let state = editor
            .bus
            .lock()
            .map_err(|_| "Project lock is unavailable")?;
        match &*state {
            project::LoadState::Ready(bus) if bus.revision() == revision => bus.project().clone(),
            _ => return Err("Project changed before texture preparation".into()),
        }
    };
    let (tx, rx) = mpsc::channel();
    editor.play.call(Request::TextureTicket {
        generation,
        reply: tx,
    })?;
    let ticket = rx
        .recv()
        .map_err(|_| "Native Play stopped before texture ticket delivery")??;
    let root = editor.asset_root.as_deref().ok_or(UNSAVED_PROJECT)?;
    let Some(prepared) = basis
        .prepare(&document, root, ticket)
        .map_err(|e| e.to_string())?
    else {
        return Ok(());
    };
    if !editor.alive.load(Ordering::Relaxed) {
        return Err("Editor closed before texture preparation completed".into());
    }
    let stage = editor.play.call(Request::StageTextures {
        generation,
        revision,
        prepared,
    })?;
    let token = stage
        .get("textureTicket")
        .and_then(Value::as_u64)
        .ok_or("Invalid native texture ticket reply")?;
    if !editor.alive.load(Ordering::Relaxed) {
        let _ = editor
            .play
            .call(Request::CancelTextures { generation, token });
        return Err("Editor closed before texture publication".into());
    }
    let receipt = editor.play.call(Request::PublishTextures {
        generation,
        revision,
        token,
        bus: editor.bus.clone(),
    })?;
    if receipt.get("changed").and_then(Value::as_u64).unwrap_or(0) != 0
        && let Ok(mut console) = editor.console.lock()
    {
        console.push(ConsoleEvent::new(
            "info",
            format!("Native Play texture refresh accepted: {receipt}"),
        ));
    }
    editor.play.refresh_event.store(true, Ordering::Release);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use incant_cmd::{Actor, Command, CommandBus};
    use incant_doc::Project;
    use std::sync::atomic::AtomicUsize;
    #[test]
    fn an_edit_between_prepare_and_commit_refuses_without_publication_or_locked_disposal() {
        let bus = Arc::new(Mutex::new(project::LoadState::Ready(Box::new(
            CommandBus::new(Project::empty("Refresh")).unwrap(),
        ))));
        let revision = 0;
        let (service, _) = Service::new();
        service
            .edit(|s| {
                s.phase = "running";
                Ok(())
            })
            .unwrap();
        bus.lock()
            .unwrap()
            .bus_mut()
            .unwrap()
            .execute(
                vec![Command::SetMemory {
                    section: "control".into(),
                    text: "newer edit".into(),
                }],
                Actor::user("test"),
                "edit between stages",
                Some(revision),
            )
            .unwrap();
        let published = Arc::new(AtomicBool::new(false));
        let publication_flag = published.clone();
        let disposed_unlocked = Arc::new(AtomicBool::new(false));
        struct Candidate {
            bus: Arc<Mutex<project::LoadState>>,
            disposed: Arc<AtomicBool>,
        }
        impl Drop for Candidate {
            fn drop(&mut self) {
                self.disposed
                    .store(self.bus.try_lock().is_ok(), Ordering::SeqCst);
            }
        }
        let candidate = Candidate {
            bus: bus.clone(),
            disposed: disposed_unlocked.clone(),
        };
        assert!(
            publish_at_revision(&bus, &service, revision, 0, move || {
                publication_flag.store(true, Ordering::SeqCst);
                Ok(candidate)
            })
            .is_err()
        );
        assert!(!published.load(Ordering::SeqCst));
        assert!(disposed_unlocked.load(Ordering::SeqCst));
        assert_eq!(bus.lock().unwrap().bus_mut().unwrap().revision(), 1);
    }
    #[test]
    fn a_session_change_between_prepare_and_commit_refuses_before_the_swap() {
        let bus = Mutex::new(project::LoadState::Ready(Box::new(
            CommandBus::new(Project::empty("Refresh")).unwrap(),
        )));
        let (service, _) = Service::new();
        service
            .edit(|s| {
                s.phase = "running";
                s.generation = 1;
                Ok(())
            })
            .unwrap();
        assert!(
            publish_at_revision(&bus, &service, 0, 0, || -> Result<(), String> {
                panic!("stale generation must not publish")
            })
            .is_err()
        );
        assert!(service.status.try_lock().is_ok());
        assert!(bus.try_lock().is_ok());
        assert_eq!(bus.lock().unwrap().bus_mut().unwrap().revision(), 0);
    }
    #[test]
    fn a_native_publication_panic_releases_unpoisoned_editor_locks_before_retirement() {
        use std::panic::{AssertUnwindSafe, catch_unwind};
        let bus = Arc::new(Mutex::new(project::LoadState::Ready(Box::new(
            CommandBus::new(Project::empty("Refresh")).unwrap(),
        ))));
        let (service, _) = Service::new();
        service
            .edit(|s| {
                s.phase = "running";
                Ok(())
            })
            .unwrap();
        struct Candidate {
            bus: Arc<Mutex<project::LoadState>>,
            service: Arc<Service>,
            unlocked: Arc<AtomicBool>,
        }
        impl Drop for Candidate {
            fn drop(&mut self) {
                self.unlocked.store(
                    self.bus.try_lock().is_ok() && self.service.status.try_lock().is_ok(),
                    Ordering::SeqCst,
                );
            }
        }
        let unlocked = Arc::new(AtomicBool::new(false));
        let candidate = Candidate {
            bus: bus.clone(),
            service: service.clone(),
            unlocked: unlocked.clone(),
        };
        assert!(
            catch_unwind(AssertUnwindSafe(|| publish_at_revision(
                &bus,
                &service,
                0,
                0,
                || -> Result<(), String> {
                    let _borrow = &candidate;
                    panic!("native publication control");
                }
            )))
            .is_err()
        );
        assert!(!unlocked.load(Ordering::SeqCst));
        drop(candidate);
        assert!(unlocked.load(Ordering::SeqCst));
        bus.lock()
            .unwrap()
            .bus_mut()
            .unwrap()
            .execute(
                vec![Command::SetMemory {
                    section: "control".into(),
                    text: "still usable".into(),
                }],
                Actor::user("test"),
                "edit after native panic",
                Some(0),
            )
            .unwrap();
        assert_eq!(bus.lock().unwrap().bus_mut().unwrap().revision(), 1);
        service.failed(0, "native panic".into());
        assert_eq!(service.snapshot().unwrap().phase, "error");
    }
    #[test]
    fn one_pending_candidate_replacement_cancellation_and_foreign_tokens_are_observable() {
        struct Candidate(Arc<AtomicUsize>);
        impl Drop for Candidate {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        let dropped = Arc::new(AtomicUsize::new(0));
        let mut pending = Pending::default();
        let first = pending
            .stage(3, 7, || Ok(Candidate(dropped.clone())))
            .unwrap();
        let second = pending
            .stage(3, 8, || Ok(Candidate(dropped.clone())))
            .unwrap();
        assert_eq!(dropped.load(Ordering::SeqCst), 1);
        assert!(pending.take(first, 3, 7).is_err());
        pending.cancel(first, 3);
        assert_eq!(dropped.load(Ordering::SeqCst), 1);
        assert!(pending.take(second, 4, 8).is_err());
        assert!(pending.take(second, 3, 7).is_err());
        pending.cancel(second, 3);
        assert_eq!(dropped.load(Ordering::SeqCst), 2);
        assert!(pending.take(second, 3, 8).is_err());
        pending.serial = u64::MAX;
        assert!(
            pending
                .stage(3, 9, || -> Result<Candidate, String> {
                    panic!("overflow must refuse before preparation")
                })
                .is_err()
        );
    }
}
