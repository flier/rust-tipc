use core::convert::{TryFrom, TryInto};
use core::mem::{self, MaybeUninit};
use core::ptr::NonNull;
use core::slice;
use core::time::Duration;

use std::io;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};

use crate::{
    addr::{ServiceAddr, ServiceRange, SocketAddr, Visibility, TIPC_ADDR_MCAST},
    raw as ffi, Scope,
};

const TRUE: i32 = 1;
const FALSE: i32 = 0;

/// Message importance levels
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Importance {
    Low = ffi::TIPC_LOW_IMPORTANCE,
    Medium = ffi::TIPC_MEDIUM_IMPORTANCE,
    High = ffi::TIPC_HIGH_IMPORTANCE,
    Critical = ffi::TIPC_CRITICAL_IMPORTANCE,
}

/// An "in progress" TIPC socket which has not yet been connected or listened.
///
/// Allows configuration of a socket before one of these operations is executed.
#[repr(transparent)]
#[derive(Debug)]
pub struct Builder(pub(crate) Socket);

impl TryFrom<Builder> for Listener {
    type Error = io::Error;

    fn try_from(builder: Builder) -> Result<Self, Self::Error> {
        Ok(Self(builder.0))
    }
}

impl TryFrom<Builder> for Stream {
    type Error = io::Error;

    fn try_from(builder: Builder) -> Result<Self, Self::Error> {
        Ok(Self(builder.0))
    }
}

impl TryFrom<Builder> for Datagram {
    type Error = io::Error;

    fn try_from(builder: Builder) -> Result<Self, Self::Error> {
        Ok(Self(builder.0))
    }
}

impl TryFrom<Builder> for Socket {
    type Error = io::Error;

    fn try_from(builder: Builder) -> Result<Self, Self::Error> {
        Ok(builder.0)
    }
}

/// Constructs a new `Builder` with the `AF_TIPC` domain, the `SOCK_RDM` type, and with a protocol argument of 0.
pub fn rdm() -> io::Result<Builder> {
    new(libc::SOCK_RDM).map(Builder)
}

/// Constructs a new `Builder` with the `AF_TIPC` domain, the `SOCK_STREAM` type, and with a protocol argument of 0.
pub fn stream() -> io::Result<Builder> {
    new(libc::SOCK_STREAM).map(Builder)
}

/// Constructs a new `Builder` with the `AF_TIPC` domain, the `SOCK_DGRAM` type, and with a protocol argument of 0.
pub fn dgram() -> io::Result<Builder> {
    new(libc::SOCK_DGRAM).map(Builder)
}

/// Constructs a new `Builder` with the `AF_TIPC` domain, the `SOCK_SEQPACKET` type, and with a protocol argument of 0.
pub fn seq_packet() -> io::Result<Builder> {
    new(libc::SOCK_SEQPACKET).map(Builder)
}

impl Builder {
    /// Constructs a new `Builder` with the `AF_TIPC` domain, the `SOCK_RDM` type, and with a protocol argument of 0.
    pub fn new_rdm() -> io::Result<Self> {
        new(libc::SOCK_RDM).map(Builder)
    }

    /// Constructs a new `Builder` with the `AF_TIPC` domain, the `SOCK_STREAM` type, and with a protocol argument of 0.
    pub fn new_stream() -> io::Result<Self> {
        new(libc::SOCK_STREAM).map(Builder)
    }

    /// Constructs a new `Builder` with the `AF_TIPC` domain, the `SOCK_DGRAM` type, and with a protocol argument of 0.
    pub fn new_dgram() -> io::Result<Self> {
        new(libc::SOCK_DGRAM).map(Builder)
    }

    /// Constructs a new `Builder` with the `AF_TIPC` domain, the `SOCK_SEQPACKET` type, and with a protocol argument of 0.
    pub fn seq_packet() -> io::Result<Self> {
        new(libc::SOCK_SEQPACKET).map(Builder)
    }

    /// Binds this socket to the specified address.
    pub fn bind<T: Into<ServiceRange>>(
        &self,
        service_range: T,
        visibility: Visibility,
    ) -> io::Result<&Self> {
        let mut sa: ffi::sockaddr_tipc = service_range.into().into();

        sa.scope = visibility as i8;

        unsafe {
            libc::bind(
                self.0.as_raw_fd(),
                &sa as *const _ as *const _,
                mem::size_of::<ffi::sockaddr_tipc>() as u32,
            )
        }
        .into_result()
        .map(|_: ()| self)
    }

    /// Unbinds this socket from the specified address.
    pub fn unbind<T: Into<ServiceRange>>(&self, service_range: T) -> io::Result<&Self> {
        let mut sa: ffi::sockaddr_tipc = service_range.into().into();

        sa.scope = -1;

        unsafe {
            libc::bind(
                self.0.as_raw_fd(),
                &sa as *const _ as *const _,
                mem::size_of::<ffi::sockaddr_tipc>() as u32,
            )
        }
        .into_result()
        .map(|_: ()| self)
    }

    /// Mark a socket as ready to accept incoming connection requests using accept()
    pub fn listen(self, backlog: usize) -> io::Result<Listener> {
        unsafe { libc::listen(self.0.as_raw_fd(), backlog as i32) }.into_result()?;

        self.try_into()
    }

    /// Initiate a connection on this socket to the specified address.
    ///
    /// Connects this TIPC socket to a remote address, allowing the `send` and `recv` syscalls to be used to send data
    /// and also applies filters to only receive data from the specified address.
    pub fn connect<T>(self, addr: ServiceAddr, scope: Scope) -> io::Result<T>
    where
        T: TryFrom<Self, Error = io::Error>,
    {
        self.0.connect(addr, scope)?;

        self.try_into()
    }

    /// Returns the address of the local half of this TIPC socket.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.local_addr()
    }

    /// Set the message importance levels
    pub fn importance(&self, importance: Importance) -> io::Result<&Self> {
        self.0.set_importance(importance).map(|_| self)
    }

    /// Sets the connect timeout to the timeout specified.
    pub fn connect_timeout(&self, timeout: Duration) -> io::Result<&Self> {
        self.0.set_connect_timeout(timeout).map(|_| self)
    }
}

/// A TIPC socket server, listening for connections.
#[repr(transparent)]
#[derive(Debug)]
pub struct Listener(Socket);

impl AsRawFd for Listener {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl FromRawFd for Listener {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self(Socket(fd))
    }
}

impl IntoRawFd for Listener {
    fn into_raw_fd(self) -> RawFd {
        self.0.into_raw_fd()
    }
}

impl Listener {
    /// Accept a new incoming connection from this listener.
    pub fn accept(&self) -> io::Result<(Stream, SocketAddr)> {
        let mut sa = MaybeUninit::<ffi::sockaddr_tipc>::uninit();
        let mut len = mem::size_of::<ffi::sockaddr_tipc>() as u32;

        unsafe {
            libc::accept(self.0.as_raw_fd(), sa.as_mut_ptr() as *mut _, &mut len)
                .into_result()
                .map(|sd| (Stream(Socket(sd)), sa.assume_init().addr.id.into()))
        }
    }

    /// Moves this listener into or out of nonblocking mode.
    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.0.set_nonblocking(nonblocking)
    }

    /// Returns the address of the local half of this TIPC socket.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.local_addr()
    }

    /// Creates a new independently owned handle to the underlying socket.
    pub fn try_clone(&self) -> io::Result<Self> {
        self.0.try_clone().map(Self)
    }
}

/// A TIPC stream between a local and a remote socket.
#[repr(transparent)]
#[derive(Debug)]
pub struct Stream(Socket);

impl AsRawFd for Stream {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl FromRawFd for Stream {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self(Socket(fd))
    }
}

impl IntoRawFd for Datagram {
    fn into_raw_fd(self) -> RawFd {
        self.0.into_raw_fd()
    }
}

impl Stream {
    /// Opens a TIPC connection to a remote host.
    pub fn connect(addr: ServiceAddr, scope: Scope) -> io::Result<Self> {
        Builder::new_stream()?.connect(addr, scope).map(Stream)
    }

    /// Opens a TIPC connection to a remote host with a timeout.
    pub fn connect_timeout(addr: ServiceAddr, scope: Scope, timeout: Duration) -> io::Result<Self> {
        let builder = Builder::new_stream()?;
        builder.connect_timeout(timeout)?;
        builder.connect(addr, scope).map(Stream)
    }

    /// Moves this stream into or out of nonblocking mode.
    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.0.set_nonblocking(nonblocking)
    }

    /// Returns the address of the local half of this TIPC socket.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.local_addr()
    }

    /// Creates a new independently owned handle to the underlying socket.
    pub fn try_clone(&self) -> io::Result<Self> {
        self.0.try_clone().map(Self)
    }
}

impl io::Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.recv(buf, false)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.0.recv(buf, true).map(|_| ())
    }
}

impl io::Write for Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.send(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A TIPC datagram socket.
#[repr(transparent)]
#[derive(Debug)]
pub struct Datagram(Socket);

impl AsRawFd for Datagram {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl FromRawFd for Datagram {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self(Socket(fd))
    }
}

impl IntoRawFd for Stream {
    fn into_raw_fd(self) -> RawFd {
        self.0.into_raw_fd()
    }
}

impl Datagram {
    /// Initiate a connection on this socket to the specified address.
    ///
    /// Connects this TIPC socket to a remote address, allowing the `send` and `recv` syscalls to be used to send data
    /// and also applies filters to only receive data from the specified address.
    pub fn connect(&self, addr: ServiceAddr, scope: Scope) -> io::Result<()> {
        self.0.connect(addr, scope)
    }

    /// Moves this stream into or out of nonblocking mode.
    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.0.set_nonblocking(nonblocking)
    }

    /// Returns the address of the local half of this TIPC socket.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.local_addr()
    }

    /// Creates a new independently owned handle to the underlying socket.
    pub fn try_clone(&self) -> io::Result<Self> {
        self.0.try_clone().map(Self)
    }

    /// Receives data from the socket.
    ///
    /// On success, returns the number of bytes read.
    pub fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.recv(buf, false)
    }

    /// Receives data from the socket.
    ///
    /// On success, returns the number of bytes read and the address from whence the data came.
    pub fn recv_from(&self, buf: &mut [u8]) -> io::Result<(Recv, SocketAddr, Option<ServiceAddr>)> {
        self.0.recv_from(buf)
    }

    /// Sends data on the socket to the remote address to which it is connected.
    ///
    /// The `connect` method will connect this socket to a remote address.
    /// This method will fail if the socket is not connected.
    pub fn send(&self, buf: &[u8]) -> io::Result<usize> {
        self.0.send(buf)
    }

    /// Sends data on the socket to the given address. On success, returns the number of bytes written.
    pub fn send_to(&self, buf: &[u8], dst: SocketAddr) -> io::Result<usize> {
        self.0.send_to(buf, dst)
    }
}

/// A TIPC socket.
#[repr(transparent)]
#[derive(Debug)]
pub struct Socket(RawFd);

impl Drop for Socket {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.as_raw_fd());
        }
    }
}

impl AsRawFd for Socket {
    fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}

impl FromRawFd for Socket {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Socket(fd)
    }
}

impl IntoRawFd for Socket {
    fn into_raw_fd(self) -> RawFd {
        let sd = self.0;
        mem::forget(self);
        sd
    }
}

pub fn new(sock_type: i32) -> io::Result<Socket> {
    unsafe { libc::socket(libc::AF_TIPC, sock_type, 0) }
        .into_result()
        .map(Socket)
}

impl Socket {
    /// Moves this TIPC stream into or out of nonblocking mode.
    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        unsafe {
            let mut flags: i32 = libc::fcntl(self.as_raw_fd(), libc::F_GETFL, 0).into_result()?;

            if nonblocking {
                flags |= libc::O_NONBLOCK;
            } else {
                flags &= !libc::O_NONBLOCK;
            }

            libc::fcntl(self.as_raw_fd(), libc::F_SETFL, flags).into_result()
        }
    }

    /// Get the message importance levels.
    pub fn importance(&self) -> io::Result<Importance> {
        self.get_sock_opt(ffi::TIPC_IMPORTANCE)
    }

    /// Set the message importance levels.
    pub fn set_importance(&self, importance: Importance) -> io::Result<()> {
        self.set_sock_opt(ffi::TIPC_IMPORTANCE, importance as u32)
    }

    /// Get the connect timeout.
    pub fn connect_timeout(&self) -> io::Result<Duration> {
        self.get_sock_opt(ffi::TIPC_CONN_TIMEOUT)
            .map(|ms: u32| Duration::from_millis(u64::from(ms)))
    }

    /// Sets the connect timeout to the timeout specified.
    pub fn set_connect_timeout(&self, timeout: Duration) -> io::Result<()> {
        self.set_sock_opt(ffi::TIPC_CONN_TIMEOUT, timeout.as_millis() as u32)
    }

    pub fn set_rejectable(&self, rejectable: bool) -> io::Result<()> {
        self.set_sock_opt(
            ffi::TIPC_DEST_DROPPABLE,
            if rejectable { TRUE } else { FALSE },
        )
    }

    fn get_sock_opt<T>(&self, opt: u32) -> io::Result<T> {
        let mut buf = MaybeUninit::<T>::zeroed();
        let mut len = mem::size_of::<T>() as u32;

        unsafe {
            libc::getsockopt(
                self.as_raw_fd(),
                libc::SOL_TIPC,
                opt as i32,
                buf.as_mut_ptr() as *mut _,
                &mut len,
            )
        }
        .into_result()
        .map(|_: ()| unsafe { buf.assume_init() })
    }

    fn set_sock_opt<T>(&self, opt: u32, val: T) -> io::Result<()> {
        unsafe {
            libc::setsockopt(
                self.as_raw_fd(),
                libc::SOL_TIPC,
                opt as i32,
                &val as *const _ as *const _,
                mem::size_of::<T>() as u32,
            )
        }
        .into_result()
    }

    /// Returns the address of the local half of this TIPC socket.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        let mut sa = MaybeUninit::<ffi::sockaddr_tipc>::uninit();
        let mut len = mem::size_of::<ffi::sockaddr_tipc>() as u32;

        unsafe {
            libc::getsockname(self.as_raw_fd(), sa.as_mut_ptr() as *mut _, &mut len)
                .into_result()
                .map(|_: ()| sa.assume_init().addr.id.into())
        }
    }

    /// Creates a new independently owned handle to the underlying socket.
    fn try_clone(&self) -> io::Result<Self> {
        unsafe { libc::dup(self.as_raw_fd()) }
            .into_result()
            .map(Self)
    }

    /// Initiate a connection on this socket to the specified address.
    ///
    /// Connects this TIPC socket to a remote address, allowing the `send` and `recv` syscalls to be used to send data
    /// and also applies filters to only receive data from the specified address.
    pub fn connect(&self, addr: ServiceAddr, scope: Scope) -> io::Result<()> {
        let mut sa: ffi::sockaddr_tipc = addr.into();

        sa.addr.name.domain = scope.into();

        unsafe {
            libc::connect(
                self.as_raw_fd(),
                &sa as *const _ as *const _,
                mem::size_of::<ffi::sockaddr_tipc>() as u32,
            )
        }
        .into_result()
    }

    /// Sends data on the socket to the remote address to which it is connected.
    ///
    /// The `connect` method will connect this socket to a remote address.
    /// This method will fail if the socket is not connected.
    fn send(&self, buf: &[u8]) -> io::Result<usize> {
        unsafe { libc::send(self.as_raw_fd(), buf.as_ptr() as *const _, buf.len(), 0) }
            .into_result()
    }

    /// Sends data on the socket to the given address. On success, returns the number of bytes written.
    fn send_to<T: Into<ffi::sockaddr_tipc>>(&self, buf: &[u8], dst: T) -> io::Result<usize> {
        let sa: ffi::sockaddr_tipc = dst.into();

        unsafe {
            libc::sendto(
                self.as_raw_fd(),
                buf.as_ptr() as *const _,
                buf.len(),
                0,
                &sa as *const _ as *const _,
                mem::size_of::<ffi::sockaddr_tipc>() as u32,
            )
        }
        .into_result()
    }

    fn mcast(&self, buf: &[u8], dst: ServiceRange, visibility: Visibility) -> io::Result<usize> {
        let mut sa: ffi::sockaddr_tipc = dst.into();

        sa.addrtype = TIPC_ADDR_MCAST as u8;
        sa.scope = visibility as i8;

        unsafe {
            libc::sendto(
                self.as_raw_fd(),
                buf.as_ptr() as *const _,
                buf.len(),
                0,
                &sa as *const _ as *const _,
                mem::size_of::<ffi::sockaddr_tipc>() as u32,
            )
        }
        .into_result()
    }

    /// Receives data from the socket.
    ///
    /// On success, returns the number of bytes read.
    fn recv(&self, buf: &mut [u8], all: bool) -> io::Result<usize> {
        unsafe {
            libc::recv(
                self.as_raw_fd(),
                buf.as_mut_ptr() as *mut _,
                buf.len(),
                if all { libc::MSG_WAITALL } else { 0 },
            )
        }
        .into_result()
    }

    /// Receives data from the socket.
    ///
    /// On success, returns the number of bytes read and the address from whence the data came.
    fn recv_from(&self, buf: &mut [u8]) -> io::Result<(Recv, SocketAddr, Option<ServiceAddr>)> {
        let mut addr = MaybeUninit::<[ffi::sockaddr_tipc; 2]>::zeroed();
        let addr_len = mem::size_of::<[ffi::sockaddr_tipc; 2]>() as u32;
        let iov = libc::iovec {
            iov_base: buf.as_mut_ptr() as *mut _,
            iov_len: buf.len(),
        };
        let msg = MaybeUninit::<libc::msghdr>::zeroed();
        let mut msg = unsafe { msg.assume_init() };
        let mut anc_space = vec![
            0;
            unsafe { libc::CMSG_SPACE(8) + libc::CMSG_SPACE(1024) + libc::CMSG_SPACE(16) }
                as usize
        ];

        msg.msg_name = addr.as_mut_ptr() as *mut _;
        msg.msg_namelen = addr_len;
        msg.msg_iov = &iov as *const _ as *mut _;
        msg.msg_iovlen = 1;

        msg.msg_control = anc_space.as_mut_ptr() as *mut _;
        msg.msg_controllen = anc_space.len();

        let rc = unsafe { libc::recvmsg(self.as_raw_fd(), &mut msg, 0) }.into_result()?;

        let addr = unsafe { addr.assume_init() };

        let member_id = if msg.msg_namelen == addr_len {
            unsafe { Some(addr[1].addr.name.name.into()) }
        } else {
            None
        };
        let sock_id = unsafe { addr[0].addr.id.into() };

        // Handle group member event
        if (msg.msg_flags & libc::MSG_OOB) == libc::MSG_OOB {
            if rc > 0 {
                Err(io::Error::new(io::ErrorKind::Other, "unexpected OOB data"))
            } else {
                let event = if (msg.msg_flags & libc::MSG_EOR) == libc::MSG_EOR {
                    Recv::GroupDown
                } else {
                    Recv::GroupUp
                };

                Ok((event, sock_id, member_id))
            }
        } else if let Some(anc) = NonNull::new(unsafe { libc::CMSG_FIRSTHDR(&msg) }) {
            let anc = unsafe { anc.as_ref() };

            if anc.cmsg_type == ffi::TIPC_ERRINFO as i32 {
                unsafe {
                    let data = libc::CMSG_DATA(anc) as *const u32;
                    let err = data.read();
                    let len = buf.len().min(data.add(1).read() as usize);

                    let anc = libc::CMSG_NXTHDR(&msg, anc);
                    let data = slice::from_raw_parts(libc::CMSG_DATA(anc), len);

                    buf[..len].copy_from_slice(data);

                    let sock_id = self.local_addr()?;

                    Ok((Recv::Message(len), sock_id, member_id))
                }
            } else {
                Err(io::Error::new(io::ErrorKind::Other, "unexpected message"))
            }
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "invalid message"))
        }
    }
}

pub enum Recv {
    GroupUp,
    GroupDown,
    Message(usize),
}

pub trait IntoResult<T> {
    type Error;

    fn into_result(self) -> Result<T, Self::Error>;
}

impl IntoResult<i32> for i32 {
    type Error = io::Error;

    fn into_result(self) -> Result<i32, Self::Error> {
        if self < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(self)
        }
    }
}

impl IntoResult<usize> for isize {
    type Error = io::Error;

    fn into_result(self) -> Result<usize, Self::Error> {
        if self < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(self as usize)
        }
    }
}

impl<T> IntoResult<()> for T
where
    T: IntoResult<T>,
{
    type Error = T::Error;

    fn into_result(self) -> Result<(), Self::Error> {
        self.into_result().map(|_: T| ())
    }
}
