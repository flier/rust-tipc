use core::convert::TryInto;
use core::iter;
use core::marker::PhantomData;
use core::mem::{self, MaybeUninit};
use core::ops::Deref;
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

#[macro_export]
macro_rules! forward_raw_fd_traits {
    ($name:ident <$tmpl:ident> => $inner:ident) => {
        impl<$tmpl> AsRawFd for $name<$tmpl> {
            fn as_raw_fd(&self) -> RawFd {
                self.0.as_raw_fd()
            }
        }

        impl<$tmpl> FromRawFd for $name<$tmpl> {
            unsafe fn from_raw_fd(fd: RawFd) -> Self {
                Self($inner::from_raw_fd(fd), PhantomData)
            }
        }

        impl<$tmpl> IntoRawFd for $name<$tmpl> {
            fn into_raw_fd(self) -> RawFd {
                self.0.into_raw_fd()
            }
        }

        impl<$tmpl> Deref for $name<$tmpl> {
            type Target = RawFd;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
    ($name:ident => $inner:ident) => {
        impl AsRawFd for $name {
            fn as_raw_fd(&self) -> RawFd {
                self.0.as_raw_fd()
            }
        }

        impl FromRawFd for $name {
            unsafe fn from_raw_fd(fd: RawFd) -> Self {
                Self($inner::from_raw_fd(fd))
            }
        }

        impl IntoRawFd for $name {
            fn into_raw_fd(self) -> RawFd {
                self.0.into_raw_fd()
            }
        }

        impl Deref for $name {
            type Target = RawFd;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
}

/// Constructs a new `Datagram` with the `AF_TIPC` domain, the `SOCK_RDM` type, and with a protocol argument of 0.
///
/// Provides a reliable datagram layer that does not guarantee ordering.
pub fn rdm() -> io::Result<Datagram> {
    new(libc::SOCK_RDM).map(Datagram)
}

/// Constructs a new `Stream` with the `AF_TIPC` domain, the `SOCK_STREAM` type, and with a protocol argument of 0.
///
/// Provides sequenced, reliable, two-way, connection-based byte streams.
pub fn stream() -> io::Result<Stream> {
    new(libc::SOCK_STREAM).map(Stream)
}

/// Constructs a new `Datagram` with the `AF_TIPC` domain, the `SOCK_DGRAM` type, and with a protocol argument of 0.
///
/// Supports datagrams (connectionless, unreliable messages of a fixed maximum length).
pub fn datagram() -> io::Result<Datagram> {
    new(libc::SOCK_DGRAM).map(Datagram)
}

/// Constructs a new `SeqPacket` with the `AF_TIPC` domain, the `SOCK_SEQPACKET` type, and with a protocol argument of 0.
///
/// Provides a sequenced, reliable, two-way connection-based data transmission path for datagrams of fixed maximum length;
/// a consumer is required to read an entire packet with each input system call.
pub fn seq_packet() -> io::Result<SeqPacket> {
    new(libc::SOCK_SEQPACKET).map(SeqPacket)
}

/// An "in progress" TIPC socket which has not yet been connected or listened.
///
/// Allows configuration of a socket before one of these operations is executed.
#[repr(transparent)]
#[derive(Debug)]
pub struct Builder<T>(Socket, PhantomData<T>);

forward_raw_fd_traits!(Builder<T> => Socket);

impl Builder<Datagram> {
    /// Constructs a new `Builder` with the `AF_TIPC` domain, the `SOCK_RDM` type, and with a protocol argument of 0.
    ///
    /// Provides a reliable datagram layer that does not guarantee ordering.
    pub fn rdm() -> io::Result<Self> {
        new(libc::SOCK_RDM).map(|sock| Builder(sock, PhantomData))
    }

    /// Constructs a new `Builder` with the `AF_TIPC` domain, the `SOCK_DGRAM` type, and with a protocol argument of 0.
    ///
    /// Supports datagrams (connectionless, unreliable messages of a fixed maximum length).
    pub fn datagram() -> io::Result<Self> {
        new(libc::SOCK_DGRAM).map(|sock| Builder(sock, PhantomData))
    }
}

impl Builder<Stream> {
    /// Constructs a new `Builder` with the `AF_TIPC` domain, the `SOCK_STREAM` type, and with a protocol argument of 0.
    ///
    /// Provides sequenced, reliable, two-way, connection-based byte streams.
    pub fn stream() -> io::Result<Self> {
        new(libc::SOCK_STREAM).map(|sock| Builder(sock, PhantomData))
    }
}

impl Builder<SeqPacket> {
    /// Constructs a new `Builder` with the `AF_TIPC` domain, the `SOCK_SEQPACKET` type, and with a protocol argument of 0.
    ///
    /// Provides a sequenced, reliable, two-way connection-based data transmission path for datagrams of fixed maximum length;
    /// a consumer is required to read an entire packet with each input system call.
    pub fn seq_packet() -> io::Result<Self> {
        new(libc::SOCK_SEQPACKET).map(|sock| Builder(sock, PhantomData))
    }
}

impl From<Builder<Datagram>> for Datagram {
    fn from(builder: Builder<Datagram>) -> Self {
        Datagram(builder.0)
    }
}

impl From<Builder<Stream>> for Stream {
    fn from(builder: Builder<Stream>) -> Self {
        Stream(builder.0)
    }
}

impl From<Builder<SeqPacket>> for SeqPacket {
    fn from(builder: Builder<SeqPacket>) -> Self {
        SeqPacket(builder.0)
    }
}

impl<T> Builder<T> {
    /// Creates a new independently owned handle to the underlying socket.
    pub fn try_clone(&self) -> io::Result<Self> {
        self.0.try_clone().map(|sock| Builder(sock, PhantomData))
    }

    /// Binds this socket to the specified address.
    pub fn bind<A: Into<ServiceRange>>(
        &self,
        service_range: A,
        visibility: Visibility,
    ) -> io::Result<&Self> {
        let mut sa: ffi::sockaddr_tipc = service_range.into().into();

        sa.scope = visibility as i8;

        unsafe {
            libc::bind(
                self.as_raw_fd(),
                &sa as *const _ as *const _,
                mem::size_of::<ffi::sockaddr_tipc>() as u32,
            )
        }
        .into_result()
        .map(|_: ()| self)
    }

    /// Unbinds this socket from the specified address.
    pub fn unbind<A: Into<ServiceRange>>(&self, service_range: A) -> io::Result<&Self> {
        let mut sa: ffi::sockaddr_tipc = service_range.into().into();

        sa.scope = -1;

        unsafe {
            libc::bind(
                self.as_raw_fd(),
                &sa as *const _ as *const _,
                mem::size_of::<ffi::sockaddr_tipc>() as u32,
            )
        }
        .into_result()
        .map(|_: ()| self)
    }

    /// Mark a socket as ready to accept incoming connection requests using accept()
    pub fn listen(self, backlog: i32) -> io::Result<Listener<T>> {
        self.0
            .listen(backlog)
            .map(|_| Listener(self.0, PhantomData))
    }

    /// Initiate a connection on this socket to the specified address.
    ///
    /// Connects this TIPC socket to a remote address, allowing the `send` and `recv` syscalls to be used to send data
    /// and also applies filters to only receive data from the specified address.
    pub fn connect<A>(self, addr: A, scope: Scope) -> io::Result<T>
    where
        A: Into<ServiceAddr>,
        T: From<Builder<T>>,
    {
        self.0.connect(addr, scope).map(|_: ()| self.into())
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

    pub fn build(self) -> T
    where
        T: FromRawFd,
    {
        unsafe { T::from_raw_fd(self.0.into_raw_fd()) }
    }
}

/// A TIPC socket server, listening for connections.
#[repr(transparent)]
#[derive(Debug)]
pub struct Listener<T>(Socket, PhantomData<T>);

forward_raw_fd_traits!(Listener<T> => Socket);

impl<T> Listener<T> {
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
        self.0.try_clone().map(|sock| Self(sock, PhantomData))
    }
}

impl<T> Listener<T>
where
    T: From<Builder<T>>,
    Builder<T>: Default,
{
    /// Binds this socket to the specified address.
    pub fn bind<A: Into<ServiceRange>>(service_range: A, visibility: Visibility) -> io::Result<T> {
        let builder = Builder::<T>::default();
        builder.bind(service_range, visibility)?;
        Ok(builder.into())
    }
}

impl<T> Listener<T>
where
    T: FromRawFd,
{
    /// Accept a new incoming connection from this listener.
    pub fn accept(&self) -> io::Result<(T, SocketAddr)> {
        let mut sa = MaybeUninit::<ffi::sockaddr_tipc>::uninit();
        let mut len = mem::size_of::<ffi::sockaddr_tipc>() as u32;

        unsafe {
            libc::accept(self.0.as_raw_fd(), sa.as_mut_ptr() as *mut _, &mut len)
                .into_result()
                .map(|sd| (T::from_raw_fd(sd), sa.assume_init().addr.id.into()))
        }
    }
}

/// A TIPC stream between a local and a remote socket.
#[repr(transparent)]
#[derive(Debug)]
pub struct Stream(Socket);

forward_raw_fd_traits!(Stream => Socket);

impl Stream {
    /// Constructs a new `Builder` with the `AF_TIPC` domain, the `SOCK_STREAM` type, and with a protocol argument of 0.
    ///
    /// Provides sequenced, reliable, two-way, connection-based byte streams.
    pub fn builder() -> io::Result<Builder<Stream>> {
        Builder::stream()
    }

    /// Opens a TIPC connection to a remote host.
    pub fn connect<T>(addr: T, scope: Scope) -> io::Result<Self>
    where
        T: Into<ServiceAddr>,
    {
        Self::builder()?.connect(addr, scope)
    }

    /// Opens a TIPC connection to a remote host with a timeout.
    pub fn connect_timeout<T>(addr: T, scope: Scope, timeout: Duration) -> io::Result<Self>
    where
        T: Into<ServiceAddr>,
    {
        let builder = Self::builder()?;
        builder.connect_timeout(timeout)?;
        builder.connect(addr, scope)
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

/// A TIPC sequenced, reliable, two-way connection-based data transmission path
/// for datagrams between a local and a remote socket.
#[repr(transparent)]
#[derive(Debug)]
pub struct SeqPacket(Socket);

forward_raw_fd_traits!(SeqPacket => Socket);

impl SeqPacket {
    /// Constructs a new `Builder` with the `AF_TIPC` domain, the `SOCK_SEQPACKET` type, and with a protocol argument of 0.
    ///
    /// Provides a sequenced, reliable, two-way connection-based data transmission path for datagrams of fixed maximum length;
    /// a consumer is required to read an entire packet with each input system call.
    pub fn builder() -> io::Result<Builder<SeqPacket>> {
        Builder::seq_packet()
    }

    /// Opens a TIPC connection to a remote host.
    pub fn connect<T>(addr: T, scope: Scope) -> io::Result<Self>
    where
        T: Into<ServiceAddr>,
    {
        Self::builder()?.connect(addr, scope)
    }

    /// Opens a TIPC connection to a remote host with a timeout.
    pub fn connect_timeout<T>(addr: T, scope: Scope, timeout: Duration) -> io::Result<Self>
    where
        T: Into<ServiceAddr>,
    {
        let builder = Self::builder()?;
        builder.connect_timeout(timeout)?;
        builder.connect(addr, scope)
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
    pub fn recv<T: AsMut<[u8]>>(&self, buf: T) -> io::Result<usize> {
        self.0.recv(buf, false)
    }

    /// Sends data on the socket to the remote address to which it is connected.
    ///
    /// The `connect` method will connect this socket to a remote address.
    /// This method will fail if the socket is not connected.
    pub fn send<T: AsRef<[u8]>>(&self, buf: T) -> io::Result<usize> {
        self.0.send(buf)
    }
}

/// A TIPC datagram socket.
#[repr(transparent)]
#[derive(Debug)]
pub struct Datagram(Socket);

forward_raw_fd_traits!(Datagram => Socket);

impl Datagram {
    /// Constructs a new `Builder` with the `AF_TIPC` domain, the `SOCK_DGRAM` type, and with a protocol argument of 0.
    ///
    /// Supports datagrams (connectionless, unreliable messages of a fixed maximum length).
    pub fn datagram() -> io::Result<Builder<Datagram>> {
        Builder::datagram()
    }

    /// Initiate a connection on this socket to the specified address.
    ///
    /// Connects this TIPC socket to a remote address, allowing the `send` and `recv` syscalls to be used to send data
    /// and also applies filters to only receive data from the specified address.
    pub fn connect<T>(&self, addr: T, scope: Scope) -> io::Result<()>
    where
        T: Into<ServiceAddr>,
    {
        self.0.connect(addr, scope)
    }

    /// Moves this stream into or out of nonblocking mode.
    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.0.set_nonblocking(nonblocking)
    }

    pub fn set_rejectable(&self) -> io::Result<()> {
        self.0.set_rejectable()
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
    pub fn recv<T: AsMut<[u8]>>(&self, buf: T) -> io::Result<usize> {
        self.0.recv(buf, false)
    }

    /// Receives data from the socket.
    ///
    /// On success, returns the number of bytes read and the address from whence the data came.
    pub fn recv_from<T: AsMut<[u8]>>(&self, buf: T) -> io::Result<(Recv, SocketAddr)> {
        self.0.recv_from(buf)
    }

    /// Sends data on the socket to the remote address to which it is connected.
    ///
    /// The `connect` method will connect this socket to a remote address.
    /// This method will fail if the socket is not connected.
    pub fn send<T: AsRef<[u8]>>(&self, buf: T) -> io::Result<usize> {
        self.0.send(buf)
    }

    /// Sends data on the socket to the given address. On success, returns the number of bytes written.
    pub fn send_to<T: AsRef<[u8]>, A: SendToAddr>(&self, buf: T, dst: A) -> io::Result<usize> {
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

impl Deref for Socket {
    type Target = RawFd;

    fn deref(&self) -> &Self::Target {
        &self.0
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

    pub fn set_rejectable(&self) -> io::Result<()> {
        self.set_sock_opt(ffi::TIPC_DEST_DROPPABLE, FALSE)
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

    /// Mark a socket as ready to accept incoming connection requests using accept()
    fn listen(&self, backlog: i32) -> io::Result<()> {
        unsafe { libc::listen(self.as_raw_fd(), backlog) }.into_result()
    }

    /// Initiate a connection on this socket to the specified address.
    ///
    /// Connects this TIPC socket to a remote address, allowing the `send` and `recv` syscalls to be used to send data
    /// and also applies filters to only receive data from the specified address.
    pub fn connect<T: Into<ServiceAddr>>(&self, addr: T, scope: Scope) -> io::Result<()> {
        let mut sa: ffi::sockaddr_tipc = addr.into().into();

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
    fn send<T: AsRef<[u8]>>(&self, buf: T) -> io::Result<usize> {
        let buf = buf.as_ref();

        unsafe { libc::send(self.as_raw_fd(), buf.as_ptr() as *const _, buf.len(), 0) }
            .into_result()
    }

    /// Sends data on the socket to the given address. On success, returns the number of bytes written.
    fn send_to<T: AsRef<[u8]>, A: SendToAddr>(&self, buf: T, dst: A) -> io::Result<usize> {
        let buf = buf.as_ref();
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

    fn mcast<T, A>(&self, buf: T, dst: A, visibility: Visibility) -> io::Result<usize>
    where
        T: AsRef<[u8]>,
        A: Into<ServiceAddr>,
    {
        let buf = buf.as_ref();
        let mut sa: ffi::sockaddr_tipc = ServiceRange::from(dst.into()).into();

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
    fn recv<T: AsMut<[u8]>>(&self, mut buf: T, all: bool) -> io::Result<usize> {
        let buf = buf.as_mut();

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
    fn recv_from<T: AsMut<[u8]>>(&self, mut buf: T) -> io::Result<(Recv, SocketAddr)> {
        let buf = buf.as_mut();
        let mut addr = MaybeUninit::<[ffi::sockaddr_tipc; 2]>::zeroed();
        let addr_len = mem::size_of::<[ffi::sockaddr_tipc; 2]>() as u32;
        let iov = libc::iovec {
            iov_base: buf.as_mut_ptr() as *mut _,
            iov_len: buf.len(),
        };
        let mut msg = unsafe { MaybeUninit::<libc::msghdr>::zeroed().assume_init() };
        let anc_space_size =
            unsafe { libc::CMSG_SPACE(8) + libc::CMSG_SPACE(1024) + libc::CMSG_SPACE(16) };
        let mut anc_space = vec![0u8; anc_space_size as usize];

        msg.msg_name = addr.as_mut_ptr() as *mut _;
        msg.msg_namelen = addr_len;
        msg.msg_iov = &iov as *const _ as *mut _;
        msg.msg_iovlen = 1;

        msg.msg_control = anc_space.as_mut_ptr() as *mut _;
        msg.msg_controllen = anc_space.len();

        let rc = unsafe { libc::recvmsg(self.as_raw_fd(), &mut msg, 0) }.into_result()?;

        let addr = unsafe { addr.assume_init() };

        // Add source addresses
        let member_id = if msg.msg_namelen == addr_len {
            Some(unsafe { addr[1].addr.name.name.into() })
        } else {
            None
        };
        let sock_id = unsafe { addr[0].addr.id.into() };

        // Handle group member event
        if (msg.msg_flags & libc::MSG_OOB) == libc::MSG_OOB {
            if rc != 0 {
                Err(io::Error::new(io::ErrorKind::Other, "unexpected OOB data"))
            } else {
                let event = if (msg.msg_flags & libc::MSG_EOR) == libc::MSG_EOR {
                    Recv::GroupDown(member_id.unwrap())
                } else {
                    Recv::GroupUp(member_id.unwrap())
                };

                Ok((event, sock_id))
            }
        } else {
            let mut err = None;
            let mut len = None;

            for (ty, level, data) in unsafe { cmsgs(&msg) } {
                if level == libc::SOL_TIPC {
                    match ty as u32 {
                        ffi::TIPC_ERRINFO if data.len() == mem::size_of::<u32>() * 2 => {
                            let mut chunks = data.chunks_exact(mem::size_of::<u32>());

                            err = chunks
                                .next()
                                .unwrap()
                                .try_into()
                                .map(u32::from_ne_bytes)
                                .ok();
                            len = chunks
                                .next()
                                .unwrap()
                                .try_into()
                                .map(u32::from_ne_bytes)
                                .map(|n| n as usize)
                                .ok();
                        }
                        ffi::TIPC_RETDATA if Some(data.len()) == len => {
                            let len = buf.len().min(len.unwrap());
                            let buf = &mut buf[..len];

                            buf.copy_from_slice(&data[..len]);
                        }
                        ffi::TIPC_DESTNAME if data.len() == mem::size_of::<u32>() * 3 => {
                            let mut chunks = data.chunks_exact(mem::size_of::<u32>());

                            let ty = u32::from_ne_bytes(chunks.next().unwrap().try_into().unwrap());
                            let lower =
                                u32::from_ne_bytes(chunks.next().unwrap().try_into().unwrap());
                            let higher =
                                u32::from_ne_bytes(chunks.next().unwrap().try_into().unwrap());

                            let _dest_name = ServiceRange::new(ty, lower, higher);
                        }
                        _ => {}
                    }
                }
            }

            if let Some(err) = err {
                Ok((Recv::Rejected(err), self.local_addr()?))
            } else {
                Ok((Recv::Message(rc), sock_id))
            }
        }
    }
}

unsafe fn cmsgs(msg: &libc::msghdr) -> impl Iterator<Item = (libc::c_int, libc::c_int, &[u8])> {
    let mut hdr = NonNull::new(libc::CMSG_FIRSTHDR(msg as *const _));

    iter::from_fn(move || {
        if let Some(cmsg) = hdr {
            hdr = NonNull::new(libc::CMSG_NXTHDR(msg as *const _, cmsg.as_ptr()));

            let cmsg = cmsg.as_ref();

            Some((
                cmsg.cmsg_type,
                cmsg.cmsg_level,
                slice::from_raw_parts(
                    libc::CMSG_DATA(cmsg) as *const _,
                    cmsg.cmsg_len as usize - mem::size_of::<libc::cmsghdr>(),
                ),
            ))
        } else {
            None
        }
    })
}

pub trait SendToAddr: Into<ffi::sockaddr_tipc> {}

impl SendToAddr for SocketAddr {}
impl SendToAddr for ServiceAddr {}
impl SendToAddr for (ServiceAddr, Scope) {}

impl From<(ServiceAddr, Scope)> for ffi::sockaddr_tipc {
    fn from((addr, scope): (ServiceAddr, Scope)) -> Self {
        let mut sa: ffi::sockaddr_tipc = addr.into();
        sa.addr.name.domain = scope.into();
        sa
    }
}

#[derive(Clone, Debug)]
pub enum Recv {
    GroupUp(ServiceAddr),
    GroupDown(ServiceAddr),
    Message(usize),
    Rejected(u32),
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
