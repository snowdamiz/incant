//! Trusted qualification canary. Never compiled from project data.
//! Only owned IPv4 loopback endpoints supplied by the managed probe are used.
use std::{env, fs, io, os::raw::c_int, process::Command};

unsafe extern "C" {
    fn socket(domain: c_int, kind: c_int, protocol: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn connect(fd: c_int, address: *const Address, length: u32) -> c_int;
    fn bind(fd: c_int, address: *const Address, length: u32) -> c_int;
    fn sendto(
        fd: c_int,
        data: *const u8,
        length: usize,
        flags: c_int,
        address: *const Address,
        address_length: u32,
    ) -> isize;
}

#[repr(C)]
struct Address {
    length: u8,
    family: u8,
    port: u16,
    address: u32,
    zero: [u8; 8],
}
impl Address {
    fn loopback(port: u16) -> Self {
        Self {
            length: 16,
            family: 2,
            port: port.to_be(),
            address: u32::from_ne_bytes([127, 0, 0, 1]),
            zero: [0; 8],
        }
    }
}
struct Socket(c_int);
impl Socket {
    fn new(kind: c_int) -> Self {
        let fd = unsafe { socket(2, kind, 0) }; // AF_INET; no I/O yet.
        assert!(
            fd >= 0,
            "Socket creation refused; network operations were not exercised"
        );
        Self(fd)
    }
}
impl Drop for Socket {
    fn drop(&mut self) {
        assert_eq!(unsafe { close(self.0) }, 0, "Close owned socket");
    }
}
struct Outcome {
    result: isize,
    errno: Option<i32>,
}
impl Outcome {
    fn record(result: isize) -> Self {
        Self {
            result,
            errno: if result == -1 {
                io::Error::last_os_error().raw_os_error()
            } else {
                None
            },
        }
    }
    fn denied(&self) -> bool {
        self.result == -1 && matches!(self.errno, Some(1 | 13))
    }
    fn json(&self) -> String {
        let errno = self
            .errno
            .map_or_else(|| "null".to_owned(), |value| value.to_string());
        format!("{{\"return\":{},\"errno\":{errno}}}", self.result)
    }
}
fn port(value: &std::ffi::OsStr) -> u16 {
    let value = value
        .to_str()
        .expect("UTF-8 port")
        .parse::<u16>()
        .expect("Numeric loopback port");
    assert!(value > 0, "Owned endpoint port must be nonzero");
    value
}
fn main() {
    let args: Vec<_> = env::args_os().collect();
    assert_eq!(args.len(), 6);
    let private = &args[1];
    let escape = &args[2];
    let allowed = &args[3];
    let tcp = Address::loopback(port(&args[4]));
    let udp = Address::loopback(port(&args[5]));
    assert_eq!(std::mem::size_of::<Address>(), 16);
    fs::write(allowed, b"control executed").expect("admitted private write");
    let read_denied = fs::read(private).is_err();
    let write_denied = fs::write(escape, b"must not publish").is_err();
    let exec_denied = Command::new("/usr/bin/true").status().is_err();
    let tcp_connect = {
        let fd = Socket::new(1); // SOCK_STREAM; owned non-listening loopback reservation.
        Outcome::record(unsafe { connect(fd.0, &tcp, 16) } as isize)
    };
    let tcp_bind = {
        let fd = Socket::new(1);
        Outcome::record(unsafe { bind(fd.0, &Address::loopback(0), 16) } as isize)
    };
    let udp_bind = {
        let fd = Socket::new(2); // SOCK_DGRAM.
        Outcome::record(unsafe { bind(fd.0, &Address::loopback(0), 16) } as isize)
    };
    let udp_send_to = {
        let fd = Socket::new(2);
        let payload = b"INCANT_LOOPBACK_CANARY_V2";
        Outcome::record(unsafe { sendto(fd.0, payload.as_ptr(), payload.len(), 0, &udp, 16) })
    };
    let clean_environment = env::var_os("RUSTC_WRAPPER").is_none()
        && env::var_os("RUSTFLAGS").is_none()
        && env::var_os("AWS_SECRET_ACCESS_KEY").is_none();
    println!(
        "{{\"criterion\":\"network-io-v2\",\"read_denied\":{read_denied},\"write_denied\":{write_denied},\"exec_denied\":{exec_denied},\"clean_environment\":{clean_environment},\"sockets_created\":4,\"socket_creation_denied\":false,\"network\":{{\"tcp_connect\":{},\"tcp_bind\":{},\"udp_bind\":{},\"udp_send_to\":{}}}}}",
        tcp_connect.json(),
        tcp_bind.json(),
        udp_bind.json(),
        udp_send_to.json()
    );
    assert!(
        read_denied
            && write_denied
            && exec_denied
            && clean_environment
            && tcp_connect.denied()
            && tcp_bind.denied()
            && udp_bind.denied()
            && udp_send_to.denied()
    );
}
