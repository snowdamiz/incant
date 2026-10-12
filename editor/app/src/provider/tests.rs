use super::*;
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

fn fixture() -> (
    Arc<ProviderRuntime>,
    auth::LoginAttempt,
    Arc<auth::BrowserAuthorization>,
) {
    let runtime = Arc::new(ProviderRuntime::new(Arc::new(crate::agent::Service::new())));
    let attempt =
        auth::LoginAttempt::new("urn:uuid:11111111-1111-4111-8111-111111111111", None).unwrap();
    let browser = Arc::new(attempt.browser_authorization());
    *runtime.browser.lock().unwrap() = Some(browser.clone());
    *runtime.state.lock().unwrap() =
        json!({"status":"connecting","provider":"openai","method":"oauth","phase":"browser"});
    runtime.busy.store(true, Ordering::Release);
    (runtime, attempt, browser)
}
fn settled(runtime: &ProviderRuntime) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while runtime.browser_reopening.load(Ordering::Acquire) {
        assert!(Instant::now() < deadline, "browser reopen did not finish");
        std::thread::sleep(Duration::from_millis(2));
    }
}
#[test]
fn blocked_browser_reopen_does_not_hold_admission_or_state_and_is_single_flight() {
    let (runtime, _attempt, browser) = fixture();
    let (started, waiting) = mpsc::channel();
    let (release, resume) = mpsc::channel();
    let (changed, notified) = mpsc::channel();
    runtime
        .agent
        .account_operation(|| {
            runtime.reopen_with(
                browser.clone(),
                move |_| {
                    started.send(()).unwrap();
                    resume.recv_timeout(Duration::from_secs(5)).unwrap();
                    Err(AgentError::Provider("private launcher detail".into()))
                },
                move || changed.send(()).unwrap(),
            )
        })
        .unwrap();
    waiting.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(runtime.browser.try_lock().is_ok());
    assert!(runtime.state.try_lock().is_ok());
    runtime
        .reopen_with(browser, |_| panic!("duplicate launch"), || {})
        .unwrap();
    // Completes while the injected OS opener is still blocked.
    runtime.agent.shutdown();
    release.send(()).unwrap();
    notified.recv_timeout(Duration::from_secs(5)).unwrap();
    settled(&runtime);
    let state = runtime.snapshot();
    assert_eq!(state["status"], "connecting");
    assert!(
        state["message"]
            .as_str()
            .unwrap()
            .contains("Could not reopen")
    );
    assert!(!state.to_string().contains("private launcher detail"));
}
#[test]
fn late_browser_reopen_failure_cannot_replace_terminal_or_new_attempt() {
    for replace in [false, true] {
        let (runtime, _attempt, browser) = fixture();
        let (started, waiting) = mpsc::channel();
        let (release, resume) = mpsc::channel();
        runtime
            .reopen_with(
                browser,
                move |_| {
                    started.send(()).unwrap();
                    resume.recv_timeout(Duration::from_secs(5)).unwrap();
                    Err(AgentError::Provider("private failure".into()))
                },
                || panic!("a stale completion must not notify"),
            )
            .unwrap();
        waiting.recv_timeout(Duration::from_secs(5)).unwrap();
        let later =
            auth::LoginAttempt::new("urn:uuid:22222222-2222-4222-8222-222222222222", None).unwrap();
        *runtime.browser.lock().unwrap() = replace.then(|| Arc::new(later.browser_authorization()));
        if !replace {
            *runtime.state.lock().unwrap() = json!({"status":"connected","method":"oauth","provider":"openai","activeAccount":"new"});
        }
        let before = runtime.snapshot();
        release.send(()).unwrap();
        settled(&runtime);
        assert_eq!(runtime.snapshot(), before);
    }
}
