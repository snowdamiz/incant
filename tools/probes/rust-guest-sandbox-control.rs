//! Trusted qualification canary. Never compiled from project data.
//! Only creates a socket; never binds, listens, connects or sends network data.
use std::{env, fs, os::raw::c_int, process::Command};

unsafe extern "C" {
    fn socket(domain: c_int, kind: c_int, protocol: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

fn main() {
    let args: Vec<_> = env::args_os().collect();
    assert_eq!(args.len(), 4);
    let private = &args[1];
    let escape = &args[2];
    let allowed = &args[3];
    // Prove the control actually executed and the private write allowance works.
    fs::write(allowed, b"control executed").expect("admitted private write");
    let read_denied = fs::read(private).is_err();
    let write_denied = fs::write(escape, b"must not publish").is_err();
    let exec_denied = Command::new("/usr/bin/true").status().is_err();
    let fd = unsafe { socket(2, 1, 0) }; // AF_INET / SOCK_STREAM, creation only.
    let socket_denied = fd < 0;
    if fd >= 0 {
        unsafe { close(fd) };
    }
    let clean_environment = env::var_os("RUSTC_WRAPPER").is_none()
        && env::var_os("RUSTFLAGS").is_none()
        && env::var_os("AWS_SECRET_ACCESS_KEY").is_none();
    println!(
        "{{\"read_denied\":{read_denied},\"write_denied\":{write_denied},\"exec_denied\":{exec_denied},\"socket_denied\":{socket_denied},\"clean_environment\":{clean_environment}}}"
    );
    assert!(read_denied && write_denied && exec_denied && socket_denied && clean_environment);
}
