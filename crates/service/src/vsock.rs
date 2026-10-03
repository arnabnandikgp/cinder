//! Owned, bounded Linux vsock streams. This is byte transport, not peer trust:
//! parent traffic still needs enclave-terminated TLS and authenticated egress.
use crate::{
    Error,
    transport::{Listener, Socket},
};
use std::{
    io::{self, Read, Write},
    net::Shutdown,
    time::Duration,
};

/// Fixed destination selected by the consumed runtime configuration. No wildcard
/// CID, hypervisor/local CID, zero port or implicit fallback to TCP is accepted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target {
    cid: u32,
    port: u32,
}
impl Target {
    /// Nitro parent CID is 3; enclave CIDs start at 4. Use a named unprivileged
    /// port below 65536 in this narrow application profile, not a dynamic proxy.
    pub fn new(cid: u32, port: u32) -> Result<Self, Error> {
        if !(3..u32::MAX).contains(&cid) || !(1024..=65535).contains(&port) {
            return Err(Error);
        }
        Ok(Self { cid, port })
    }
    /// Exact fixed endpoint encoding for the later consumed-runtime manifest.
    pub fn encode(&self) -> [u8; 8] {
        let mut out = [0; 8];
        out[..4].copy_from_slice(&self.cid.to_be_bytes());
        out[4..].copy_from_slice(&self.port.to_be_bytes());
        out
    }
    pub(crate) fn is_enclave(&self) -> bool {
        self.cid > 3
    }
}

/// Stream with one owned descriptor. Public construction uses AF_VSOCK only.
/// There is no raw-descriptor injection, socket selection flag or TCP fallback.
pub struct VsockStream {
    inner: socket2::Socket,
}
impl VsockStream {
    /// One bounded connect to one exact destination; never retries.
    pub fn connect(target: Target) -> Result<Self, Error> {
        #[cfg(target_os = "linux")]
        {
            use socket2::{Domain, SockAddr, Socket, Type};
            let inner = Socket::new(Domain::VSOCK, Type::STREAM, None)?;
            inner.connect_timeout(
                &SockAddr::vsock(target.cid, target.port),
                Duration::from_secs(5),
            )?;
            if inner.peer_addr()?.as_vsock_address() != Some((target.cid, target.port)) {
                return Err(Error);
            }
            let result = Self { inner };
            result.prepare()?;
            Ok(result)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = target;
            Err(Error)
        }
    }
}
impl Read for VsockStream {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.inner.read(bytes)
    }
}
impl Write for VsockStream {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.inner.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
impl Socket for VsockStream {
    fn idle(&self) -> io::Result<()> {
        self.inner.set_read_timeout(Some(Duration::from_secs(120)))
    }
    fn prepare(&self) -> io::Result<()> {
        self.inner.set_nonblocking(false)?;
        self.inner.set_read_timeout(Some(Duration::from_secs(5)))?;
        self.inner.set_write_timeout(Some(Duration::from_secs(5)))
    }
    fn try_clone(&self) -> io::Result<Self> {
        Ok(Self {
            inner: self.inner.try_clone()?,
        })
    }
    fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.inner.shutdown(how)
    }
}

/// AF_VSOCK listener restricted to one configured peer CID. The peer is routing,
/// NOT identity; TLS authenticates the session after acceptance.
pub struct VsockListener {
    inner: socket2::Socket,
    #[cfg(target_os = "linux")]
    peer: u32,
}
impl VsockListener {
    /// Bind the local CID's fixed port and allow only this expected remote CID.
    /// A parent ingress normally allows the configured enclave; enclave ingress
    /// allows parent CID 3. Backlog and service workers remain bounded to eight.
    pub fn bind(port: u32, peer: u32) -> Result<Self, Error> {
        Target::new(peer, port)?;
        #[cfg(target_os = "linux")]
        {
            use socket2::{Domain, SockAddr, Socket, Type};
            let inner = Socket::new(Domain::VSOCK, Type::STREAM, None)?;
            inner.bind(&SockAddr::vsock(u32::MAX, port))?;
            inner.listen(crate::transport::MAX_CONNECTIONS as i32)?;
            inner.set_nonblocking(true)?;
            Ok(Self { inner, peer })
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err(Error)
        }
    }
}
impl Listener for VsockListener {
    type Stream = VsockStream;
    fn nonblocking(&self) -> io::Result<()> {
        self.inner.set_nonblocking(true)
    }
    fn accept(&self) -> io::Result<Self::Stream> {
        #[cfg(target_os = "linux")]
        {
            let (inner, remote) = self.inner.accept()?;
            if remote.as_vsock_address().map(|(cid, _)| cid) != Some(self.peer) {
                let _ = inner.shutdown(Shutdown::Both);
                // One rejected peer consumes no worker and does not stop the
                // listener or fill an unbounded peer-filter retry loop.
                return Err(io::Error::from(io::ErrorKind::WouldBlock));
            }
            Ok(VsockStream { inner })
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err(io::Error::from(io::ErrorKind::Unsupported))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn targets_are_explicit_and_never_select_wildcard_local_or_privileged_routes() {
        for (cid, port) in [
            (0, 1024),
            (1, 1024),
            (2, 1024),
            (u32::MAX, 1024),
            (3, 0),
            (3, 1023),
            (3, 65536),
        ] {
            assert!(Target::new(cid, port).is_err());
            assert!(VsockListener::bind(port, cid).is_err());
        }
        assert_eq!(
            Target::new(3, 1024).unwrap().encode(),
            [0, 0, 0, 3, 0, 0, 4, 0]
        );
        assert_ne!(
            Target::new(16, 5000).unwrap().encode(),
            Target::new(16, 5001).unwrap().encode()
        );
    }
    #[cfg(not(target_os = "linux"))]
    #[test]
    fn missing_platform_refuses_instead_of_opening_a_tcp_fixture() {
        assert!(VsockStream::connect(Target::new(3, 5000).unwrap()).is_err());
        assert!(VsockListener::bind(5000, 3).is_err());
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn linux_address_and_owned_stream_io_do_not_need_a_raw_fd_conversion() {
        use socket2::{Domain, SockAddr, Socket as Owned, Type};
        assert_eq!(
            SockAddr::vsock(16, 5000).as_vsock_address(),
            Some((16, 5000))
        );
        // Unix pairs exercise the shared safe owned-socket operations only;
        // this is explicitly NOT an AF_VSOCK device or Nitro hardware test.
        let (first, second) = Owned::pair(Domain::UNIX, Type::STREAM, None).unwrap();
        let mut first = VsockStream { inner: first };
        let mut second = VsockStream { inner: second };
        first.prepare().unwrap();
        second.prepare().unwrap();
        assert_eq!(
            first.inner.read_timeout().unwrap(),
            Some(Duration::from_secs(5))
        );
        first.write_all(b"opaque").unwrap();
        let mut bytes = [0; 6];
        second.read_exact(&mut bytes).unwrap();
        assert_eq!(&bytes, b"opaque");
        let cloned = first.try_clone().unwrap();
        cloned.shutdown(Shutdown::Both).unwrap();
        assert_eq!(second.read(&mut bytes).unwrap(), 0);
    }
}
