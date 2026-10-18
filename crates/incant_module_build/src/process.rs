//! macOS direct-child group ownership. Observe exit without reaping so killpg
//! cannot target a reused group ID. All Python-requested children are ours.
use crate::{Error, Result};
use std::sync::Mutex;
use std::{
    fs::File,
    os::{fd::RawFd, unix::process::CommandExt},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
#[link(name = "proc")]
unsafe extern "C" {
    fn proc_listpgrppids(group: u32, buffer: *mut libc::c_void, size: i32) -> i32;
}
pub(crate) struct Owned {
    child: Option<Child>,
    queue: RawFd,
    closed: bool,
    attempted: bool,
    already_exited: bool,
    observer_error: Option<String>,
}
impl Owned {
    pub fn spawn(mut command: Command, stdout: File, stderr: File) -> Result<Self> {
        command
            .process_group(0)
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(stderr);
        // Acquire the observer before launch: failure cannot leave a child unowned.
        let queue = unsafe { libc::kqueue() };
        if queue < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                unsafe { libc::close(queue) };
                return Err(error.into());
            }
        };
        let mut owned = Self {
            child: Some(child),
            queue,
            closed: false,
            attempted: false,
            already_exited: false,
            observer_error: None,
        };
        // SAFETY: the initialized event points to live stack storage for this call.
        let event = libc::kevent {
            ident: owned.pid() as _,
            filter: libc::EVFILT_PROC,
            flags: libc::EV_ADD | libc::EV_ENABLE | libc::EV_ONESHOT,
            fflags: libc::NOTE_EXIT,
            data: 0,
            udata: std::ptr::null_mut(),
        };
        if unsafe { libc::kevent(queue, &event, 1, std::ptr::null_mut(), 0, std::ptr::null()) } < 0
        {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ESRCH) {
                owned.already_exited = true;
            } else {
                owned.observer_error = Some(error.to_string());
            }
        }
        Ok(owned)
    }
    pub fn pid(&self) -> u32 {
        self.child.as_ref().expect("owned child retained").id()
    }
    pub fn exited(&mut self) -> Result<bool> {
        if let Some(error) = &self.observer_error {
            return Err(Error::Cleanup(format!("exit observer: {error}")));
        }
        if self.already_exited {
            return Ok(true);
        }
        // SAFETY: output and timeout point to initialized writable/live storage.
        let mut event: libc::kevent = unsafe { std::mem::zeroed() };
        let timeout = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        let count =
            unsafe { libc::kevent(self.queue, std::ptr::null(), 0, &mut event, 1, &timeout) };
        if count < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        self.already_exited = count > 0;
        Ok(self.already_exited)
    }
    pub fn close(&mut self) -> Result<i32> {
        if self.closed {
            return Err(Error::Cleanup("group already closed".into()));
        }
        self.attempted = true;
        let pid = self.pid();
        // SAFETY: this owned direct child remains unreaped and reserves its group ID.
        let signalled = unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };
        let signal_error = if signalled < 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ESRCH)
                || (error.raw_os_error() == Some(libc::EPERM) && empty(pid).unwrap_or(false))
            {
                None
            } else {
                Some(error)
            }
        } else {
            None
        };
        // Never call blocking wait while the child may still execute. Kqueue observes
        // NOTE_EXIT without reaping, retaining the reserved group ID on failure.
        let exit_deadline = Instant::now() + Duration::from_secs(2);
        while !self.exited()? {
            if Instant::now() >= exit_deadline {
                return Err(Error::Cleanup(
                    "owned child exit unproven; retained unreaped".into(),
                ));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let waited = self
            .child
            .as_mut()
            .expect("owned child retained")
            .wait()
            .map_err(|error| Error::Cleanup(format!("wait after observed exit: {error}")))?;
        self.closed = true;
        if self.queue >= 0 {
            unsafe { libc::close(self.queue) };
            self.queue = -1;
        }
        if let Some(error) = signal_error {
            return Err(Error::Cleanup(format!("killpg: {error}; wait: {waited:?}")));
        }
        let status = waited;
        let deadline = Instant::now() + Duration::from_secs(2);
        while !empty(pid)? {
            if Instant::now() >= deadline {
                return Err(Error::Cleanup("group remains after kill/reap".into()));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        use std::os::unix::process::ExitStatusExt;
        Ok(status
            .code()
            .unwrap_or_else(|| -status.signal().unwrap_or(1)))
    }
}
fn empty(group: u32) -> Result<bool> {
    census(
        group,
        |pids| {
            // SAFETY: libproc writes at most this live initialized PID buffer's size.
            // The NULL-buffer call is only a capacity estimate, never an absence proof.
            unsafe {
                *libc::__error() = 0;
            }
            let count = unsafe {
                proc_listpgrppids(
                    group,
                    pids.as_mut_ptr().cast(),
                    std::mem::size_of_val(pids) as i32,
                )
            };
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            if count < 0 || (count == 0 && errno != 0) {
                return Err(Error::Cleanup(format!(
                    "libproc group census errno {errno}"
                )));
            }
            Ok(count as usize)
        },
        |pid| {
            // SAFETY: getpgid is a read-only process query. ESRCH can race a census.
            let actual = unsafe { libc::getpgid(pid) };
            if actual >= 0 {
                return Ok(Some(actual as u32));
            }
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ESRCH) {
                Ok(None)
            } else {
                Err(Error::Cleanup(format!("getpgid: {error}")))
            }
        },
    )
}
fn census(
    group: u32,
    mut list: impl FnMut(&mut [i32]) -> Result<usize>,
    mut pgid: impl FnMut(i32) -> Result<Option<u32>>,
) -> Result<bool> {
    let mut capacity = 64;
    while capacity <= 1_048_576 {
        let mut pids = vec![0; capacity];
        let count = list(&mut pids)?;
        if count < capacity {
            for pid in pids.into_iter().take(count).filter(|pid| *pid > 0) {
                if pgid(pid)? == Some(group) {
                    return Ok(false);
                }
            }
            return Ok(true);
        }
        capacity *= 2;
    }
    Err(Error::Cleanup(
        "owned process group exceeds census bound".into(),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn census_requires_actual_buffer_and_identity() {
        assert!(
            !census(
                7,
                |out| {
                    out[0] = 3;
                    Ok(1)
                },
                |pid| {
                    assert_eq!(pid, 3);
                    Ok(Some(7))
                }
            )
            .unwrap()
        );
        assert!(
            census(
                7,
                |out| {
                    out[0] = 3;
                    Ok(1)
                },
                |_| Ok(None)
            )
            .unwrap()
        );
        assert!(
            census(
                7,
                |out| {
                    out[0] = 3;
                    Ok(1)
                },
                |_| Ok(Some(8))
            )
            .unwrap()
        );
        assert!(census(7, |_| Err(Error::Cleanup("EPERM".into())), |_| Ok(None)).is_err());
        assert!(census(7, |_| Ok(1), |_| Err(Error::Cleanup("unknown".into()))).is_ok()); // Zero slots are not PIDs.
        assert!(
            census(
                7,
                |out| {
                    out[0] = 3;
                    Ok(1)
                },
                |_| Err(Error::Cleanup("unknown".into()))
            )
            .is_err()
        );
        assert!(census(7, |out| Ok(out.len()), |_| Ok(None)).is_err());
    }
    fn child(root: &std::path::Path, script: &str) -> Owned {
        let mut command = Command::new("/bin/sh");
        command
            .args(["-c", script])
            .current_dir(root)
            .env_clear()
            .env("PATH", "/bin:/usr/bin");
        Owned::spawn(
            command,
            File::create(root.join("stdout")).unwrap(),
            File::create(root.join("stderr")).unwrap(),
        )
        .unwrap()
    }
    fn wait_exit(owner: &mut Owned) {
        let deadline = Instant::now() + Duration::from_secs(3);
        while !owner.exited().unwrap() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    #[test]
    #[ignore = "actual managed process qualification; requires a separately granted CPU episode"]
    fn actual_terminal_and_cancel_close_owned_groups() {
        let root = tempfile::tempdir().unwrap();
        let mut terminal = child(root.path(), "exit 7");
        let pid = terminal.pid();
        wait_exit(&mut terminal);
        assert_eq!(terminal.close().unwrap(), 7);
        assert!(empty(pid).unwrap());
        let mut running = child(root.path(), "exec /bin/sleep 30");
        let pid = running.pid();
        assert_eq!(running.close().unwrap(), -libc::SIGKILL);
        assert!(empty(pid).unwrap());
    }
    #[test]
    #[ignore = "actual managed descendant/pipe qualification; requires a separately granted CPU episode"]
    fn actual_exited_bridge_cannot_leave_pipe_holding_descendant() {
        use std::{io::Read, os::fd::FromRawFd};
        let root = tempfile::tempdir().unwrap();
        let mut pipe = [-1; 2];
        // Fixed test pipe; no user or project descriptors are inherited.
        assert_eq!(unsafe { libc::pipe(pipe.as_mut_ptr()) }, 0);
        let mut reader = unsafe { File::from_raw_fd(pipe[0]) };
        let writer = unsafe { File::from_raw_fd(pipe[1]) };
        let mut command = Command::new("/bin/sh");
        command
            .args(["-c", "/bin/sleep 30 & echo $! > child; exit 19"])
            .current_dir(root.path())
            .env_clear();
        let mut bridge = Owned::spawn(
            command,
            writer,
            File::create(root.path().join("stderr")).unwrap(),
        )
        .unwrap();
        let pid = bridge.pid();
        wait_exit(&mut bridge);
        let descendant = std::fs::read_to_string(root.path().join("child"))
            .unwrap()
            .trim()
            .parse::<i32>()
            .unwrap();
        assert_eq!(unsafe { libc::getpgid(descendant) }, pid as i32);
        assert_eq!(bridge.close().unwrap(), 19);
        assert!(empty(pid).unwrap());
        // Nonblocking EOF proves the inherited writer closed rather than hanging a reader.
        use std::os::fd::AsRawFd;
        assert_eq!(
            unsafe { libc::fcntl(reader.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK) },
            0
        );
        assert_eq!(reader.read(&mut [0; 1]).unwrap(), 0);
    }
    #[test]
    #[ignore = "actual cleanup failure retention qualification; requires a separately granted CPU episode"]
    fn actual_observer_failure_retains_owner_without_blocking_wait() {
        let root = tempfile::tempdir().unwrap();
        let mut owner = child(root.path(), "exec /bin/sleep 30");
        let pid = owner.pid();
        owner.observer_error = Some("injected observer refusal".into());
        let start = Instant::now();
        assert!(matches!(owner.close(), Err(Error::Cleanup(_))));
        assert!(start.elapsed() < Duration::from_secs(1));
        assert!(!owner.closed);
        assert!(owner.child.is_some());
        // Test-only repair enables deterministic owned teardown; production quarantines instead.
        owner.observer_error = None;
        owner.close().unwrap();
        assert!(empty(pid).unwrap());
    }
    #[test]
    #[ignore = "actual crashed-bridge separate-child ownership; separately granted CPU qualification"]
    fn actual_crashed_bridge_still_requires_delegated_child_closure() {
        let bridge_root = tempfile::tempdir().unwrap();
        let child_root = tempfile::tempdir().unwrap();
        let mut bridge = child(bridge_root.path(), "exit 23");
        let mut delegated = child(child_root.path(), "exec /bin/sleep 30");
        let (bridge_pid, child_pid) = (bridge.pid(), delegated.pid());
        wait_exit(&mut bridge);
        assert_eq!(bridge.close().unwrap(), 23);
        assert!(empty(bridge_pid).unwrap());
        assert!(!empty(child_pid).unwrap());
        assert_eq!(delegated.close().unwrap(), -libc::SIGKILL);
        assert!(empty(child_pid).unwrap());
    }
}
// A cleanup failure must not drop the unreaped Child handle or its observer.
// Normally the backend retains Owned in its poisoned quarantine. This final
// reserve also covers unwinding/dropping that backend; no wait/re-signal runs on
// the editor event thread. Such a failure is never reported as closed.
struct Unclosed {
    _child: Child,
    _queue: RawFd,
}
static UNCLOSED: Mutex<Vec<Unclosed>> = Mutex::new(Vec::new());
impl Drop for Owned {
    fn drop(&mut self) {
        if !self.closed && !self.attempted {
            let _ = self.close();
        }
        if !self.closed
            && let Some(child) = self.child.take()
        {
            UNCLOSED
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .push(Unclosed {
                    _child: child,
                    _queue: self.queue,
                });
            self.queue = -1;
        }
    }
}

#[repr(C)]
struct TaskInfo {
    timing: [u64; 6],
    counters: [i32; 12],
}
unsafe extern "C" {
    fn proc_pidinfo(pid: i32, flavor: i32, arg: u64, buffer: *mut libc::c_void, size: i32) -> i32;
}
pub(crate) fn rss(group: u32) -> Result<Vec<(u32, u64)>> {
    let mut pids = [0i32; 256];
    unsafe {
        *libc::__error() = 0;
    }
    let count = unsafe {
        proc_listpgrppids(
            group,
            pids.as_mut_ptr().cast(),
            std::mem::size_of_val(&pids) as i32,
        )
    };
    let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
    if count < 0 || count as usize >= pids.len() || (count == 0 && errno != 0) {
        return Err(Error::Worker("owned RSS census unavailable/bound".into()));
    }
    let mut rows = Vec::new();
    for pid in pids.into_iter().take(count as usize).filter(|p| *p > 0) {
        let actual = unsafe { libc::getpgid(pid) };
        if actual < 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
            continue;
        }
        if actual != group as i32 {
            return Err(Error::Worker("owned RSS group identity changed".into()));
        }
        let mut info = TaskInfo {
            timing: [0; 6],
            counters: [0; 12],
        };
        unsafe {
            *libc::__error() = 0;
        }
        let size = unsafe {
            proc_pidinfo(
                pid,
                4,
                0,
                (&mut info as *mut TaskInfo).cast(),
                std::mem::size_of::<TaskInfo>() as i32,
            )
        };
        if size == 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
            continue;
        }
        if size != std::mem::size_of::<TaskInfo>() as i32 {
            return Err(Error::Worker("owned task RSS unavailable".into()));
        }
        rows.push((pid as u32, info.timing[1]));
    }
    Ok(rows)
}
pub(crate) fn disk_floor(path: &std::path::Path) -> Result<()> {
    use std::os::unix::ffi::OsStrExt;
    let path = std::ffi::CString::new(path.as_os_str().as_bytes())
        .map_err(|_| Error::Invalid("storage path"))?;
    let mut info: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(path.as_ptr(), &mut info) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if (info.f_bavail as u64).saturating_mul(info.f_frsize as u64) < 6 * 1024 * 1024 * 1024 {
        return Err(Error::Worker("free disk floor".into()));
    }
    Ok(())
}
