use core::convert::TryInto;
use core::iter;
use core::marker::PhantomData;
use core::mem::{self, MaybeUninit};
use core::ops::Deref;
use core::ops::Range;
use core::ptr;
use core::ptr::NonNull;
use core::slice;
use core::time::Duration;

use std::io;
use std::net::Shutdown;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};

use bitflags::bitflags;
use failure::{format_err, Error, Fail};

use crate::{
    addr::{Instance, ServiceAddr, ServiceRange, SocketAddr, Type, Visibility, TIPC_ADDR_MCAST},
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

#[doc(hidden)]
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

        impl<$tmpl> AsRef<$inner> for $name<$tmpl> {
            fn as_ref(&self) -> &$inner {
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

        impl AsRef<$inner> for $name {
            fn as_ref(&self) -> &$inner {
                &self.0
            }
        }
    };
}

/// Binds this socket to the specified address.
pub fn bind<T, A>(addr: A) -> io::Result<Bound<T>>
where
    T: Bindable,
    A: ToBindAddr,
{
    T::builder()?.bind(addr)
}

/// Opens a TIPC connection to a remote host.
pub fn connect<T, A>(addr: A) -> io::Result<Connected<T>>
where
    T: Connectable,
    A: ToConnectAddr,
{
    T::builder()?.connect(addr)
}

/// Opens a TIPC connection to a remote host with a timeout.
pub fn connect_timeout<T, A>(addr: A, timeout: Duration) -> io::Result<Connected<T>>
where
    T: Connectable,
    A: ToConnectAddr,
{
    T::builder()?.connect_timeout(timeout)?.connect(addr)
}

/// A bindable TIPC socket.
pub trait Bindable: Buildable + AsRef<Socket> {}

impl Bindable for Stream {}
impl Bindable for SeqPacket {}
impl Bindable for Datagram {}

/// A connectable TIPC socket.
pub trait Connectable: Buildable + AsRef<Socket> {}

impl Connectable for Stream {}
impl Connectable for SeqPacket {}

/// A buildable TIPC socket.
pub trait Buildable: From<Builder<Self>> + Sized {
    /// Constructs a new `Builder` with the `AF_TIPC` domain.
    fn builder() -> io::Result<Builder<Self>>;
}

impl Buildable for Stream {
    fn builder() -> io::Result<Builder<Self>> {
        Builder::stream()
    }
}

impl Buildable for SeqPacket {
    fn builder() -> io::Result<Builder<Self>> {
        Builder::seq_packet()
    }
}

impl Buildable for Datagram {
    fn builder() -> io::Result<Builder<Self>> {
        Builder::datagram()
    }
}

/// An "in progress" TIPC socket which has not yet been connected or listened.
///
/// Allows configuration of a socket before one of these operations is executed.
#[repr(transparent)]
#[derive(Debug)]
pub struct Builder<T>(Socket, PhantomData<T>);

forward_raw_fd_traits!(Builder<T> => Socket);

impl Builder<Datagram> {
    /// Constructs a new `Builder` with the `AF_TIPC` domain, the `SOCK_RDM` type.
    ///
    /// Provides a reliable datagram layer that does not guarantee ordering.
    pub fn rdm() -> io::Result<Self> {
        new(libc::SOCK_RDM).map(|sock| Builder(sock, PhantomData))
    }

    /// Constructs a new `Builder` with the `AF_TIPC` domain, the `SOCK_DGRAM` type.
    ///
    /// Supports datagrams (connectionless, unreliable messages of a fixed maximum length).
    pub fn datagram() -> io::Result<Self> {
        new(libc::SOCK_DGRAM).map(|sock| Builder(sock, PhantomData))
    }
}

impl Builder<Stream> {
    /// Constructs a new `Builder` with the `AF_TIPC` domain, the `SOCK_STREAM` type.
    ///
    /// Provides sequenced, reliable, two-way, connection-based byte streams.
    pub fn stream() -> io::Result<Self> {
        new(libc::SOCK_STREAM).map(|sock| Builder(sock, PhantomData))
    }
}

impl Builder<SeqPacket> {
    /// Constructs a new `Builder` with the `AF_TIPC` domain, the `SOCK_SEQPACKET` type.
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
    pub fn bind<A>(self, addr: A) -> io::Result<Bound<T>>
    where
        A: ToBindAddr,
        T: Bindable,
    {
        self.0.bind(addr).map(|_: ()| Bound(T::from(self)))
    }

    /// Initiate a connection on this socket to the specified address.
    ///
    /// Connects this TIPC socket to a remote address, allowing the `send` and `recv` syscalls to be used to send data
    /// and also applies filters to only receive data from the specified address.
    pub fn connect<A>(self, addr: A) -> io::Result<Connected<T>>
    where
        A: ToConnectAddr,
        T: Connectable,
    {
        self.0.connect(addr).map(|_: ()| Connected(T::from(self)))
    }

    /// Returns the address of the local half of this TIPC socket.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.local_addr()
    }

    /// Set the message importance levels
    pub fn importance(self, importance: Importance) -> io::Result<Self> {
        self.0.set_importance(importance).map(|_| self)
    }

    /// Sets the connect timeout to the timeout specified.
    pub fn connect_timeout(self, timeout: Duration) -> io::Result<Self> {
        self.0.set_connect_timeout(timeout).map(|_| self)
    }

    /// Moves this listener into or out of nonblocking mode.
    pub fn nonblocking(self, nonblocking: bool) -> io::Result<Self> {
        self.0.set_nonblocking(nonblocking).map(|_| self)
    }
}

/// A bound socket has a logical TIPC port name associated with it.
#[repr(transparent)]
#[derive(Debug)]
pub struct Bound<T>(T);

impl<T> Deref for Bound<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Bound<Datagram>> for Datagram {
    fn from(bound: Bound<Datagram>) -> Self {
        bound.0
    }
}

impl<T> Bound<T>
where
    T: AsRef<Socket>,
{
    /// Returns the address of the local half of this TIPC socket.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.as_ref().local_addr()
    }

    /// Binds this socket to the specified address.
    pub fn bind<A: ToBindAddr>(self, addr: A) -> io::Result<Self> {
        self.0.as_ref().bind(addr).map(|_: ()| self)
    }

    /// Unbinds this socket from the specified address.
    pub fn unbind<A: Into<ServiceRange>>(self, service_range: A) -> io::Result<Self> {
        self.0.as_ref().unbind(service_range).map(|_: ()| self)
    }

    /// Mark a socket as ready to accept incoming connection requests using accept()
    pub fn listen(self) -> io::Result<Listener<T>>
    where
        Listener<T>: From<T>,
    {
        self.0.as_ref().listen().map(|_| self.0.into())
    }
}

/// A TIPC socket server, listening for connections.
#[repr(transparent)]
#[derive(Debug)]
pub struct Listener<T>(Socket, PhantomData<T>);

forward_raw_fd_traits!(Listener<T> => Socket);

impl From<Stream> for Listener<Stream> {
    fn from(stream: Stream) -> Self {
        Self(stream.0, PhantomData)
    }
}

impl From<SeqPacket> for Listener<SeqPacket> {
    fn from(seq_packet: SeqPacket) -> Self {
        Self(seq_packet.0, PhantomData)
    }
}

impl<T> Listener<T> {
    /// Returns the address of the local half of this TIPC socket.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.local_addr()
    }

    /// Creates a new independently owned handle to the underlying socket.
    pub fn try_clone(&self) -> io::Result<Self> {
        self.0.try_clone().map(|sock| Self(sock, PhantomData))
    }

    /// Accept a new incoming connection from this listener.
    pub fn accept(&self) -> io::Result<(Connected<T>, SocketAddr)>
    where
        T: FromRawFd,
    {
        let mut sa = MaybeUninit::<ffi::sockaddr_tipc>::uninit();
        let mut len = mem::size_of::<ffi::sockaddr_tipc>() as u32;

        unsafe {
            libc::accept(self.0.as_raw_fd(), sa.as_mut_ptr() as *mut _, &mut len)
                .into_result()
                .map(|sd| {
                    (
                        Connected(T::from_raw_fd(sd)),
                        sa.assume_init().addr.id.into(),
                    )
                })
        }
    }

    /// Returns an iterator over the connections being received on this listener.
    ///
    /// The returned iterator will never return `None` and will also not yield the peer's `SocketAddr` structure.
    /// Iterating over it is equivalent to calling `accept` in a loop.
    pub fn incoming(&self) -> Incoming<T> {
        Incoming { listener: self }
    }
}

/// An iterator that infinitely accepts connections on a `Listener`.
///
/// This struct is created by the incoming method on `Listener`. See its documentation for more.
pub struct Incoming<'a, T> {
    listener: &'a Listener<T>,
}

impl<'a, T> Iterator for Incoming<'a, T>
where
    T: FromRawFd,
{
    type Item = io::Result<Connected<T>>;

    fn next(&mut self) -> Option<io::Result<Connected<T>>> {
        Some(self.listener.accept().map(|p| p.0))
    }
}

/// A connected socket is directly connected to another socket creating a point to point connection between TIPC sockets.
#[repr(transparent)]
#[derive(Debug)]
pub struct Connected<T>(T);

impl<T> Deref for Connected<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Connected<T>
where
    T: AsRef<Socket>,
{
    /// Receives data from the socket.
    ///
    /// On success, returns the number of bytes read.
    pub fn recv<B: AsMut<[u8]>>(&self, buf: B) -> io::Result<usize> {
        self.0.as_ref().recv(buf, Recv::empty())
    }

    /// Receives data on the socket from the remote address to which it is connected,
    /// without removing that data from the queue.
    ///
    /// On success, returns the number of bytes peeked. Successive calls return the same data.
    /// This is accomplished by passing `MSG_PEEK` as a flag to the underlying `recv` system call.
    pub fn peek<B: AsMut<[u8]>>(&self, buf: B) -> io::Result<usize> {
        self.0.as_ref().recv(buf, Recv::PEEK)
    }

    /// Sends data on the socket to the remote address to which it is connected.
    ///
    /// The `connect` method will connect this socket to a remote address.
    /// This method will fail if the socket is not connected.
    pub fn send<B: AsRef<[u8]>>(&self, buf: B) -> io::Result<usize> {
        self.0.as_ref().send(buf, Send::empty())
    }

    /// Get the socket address of the peer socket.
    pub fn peer_addr(&self) -> io::Result<SocketAddr>
    where
        T: AsRawFd,
    {
        let mut sa = MaybeUninit::<ffi::sockaddr_tipc>::uninit();
        let mut len = mem::size_of::<ffi::sockaddr_tipc>() as u32;

        unsafe {
            libc::getpeername(self.0.as_raw_fd(), sa.as_mut_ptr() as *mut _, &mut len)
                .into_result()
                .map(|_: ()| sa.assume_init().addr.id.into())
        }
    }
}

/// A TIPC stream between a local and a remote socket.
#[repr(transparent)]
#[derive(Debug)]
pub struct Stream(Socket);

forward_raw_fd_traits!(Stream => Socket);

impl Stream {
    /// Opens a TIPC connection to a remote host.
    pub fn connect<A: ToConnectAddr>(self, addr: A) -> io::Result<Connected<Self>> {
        self.0.connect(addr)?;
        Ok(Connected(self))
    }

    /// Shut down the read, write, or both halves of this connection.
    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.0.shutdown(how)
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

    /// Returns an error representing the last socket error which occurred.
    pub fn last_error(&self) -> io::Error {
        self.0.last_error()
    }
}

impl io::Read for Connected<Stream> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.as_ref().recv(buf, Recv::empty())
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        let expected = buf.len();

        match self.0.as_ref().recv(buf, Recv::WAIT_ALL) {
            Ok(size) => {
                if size == expected {
                    Ok(())
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        format_err!("incomplete read, {} of {}", size, expected),
                    ))
                }
            }
            Err(err) => Err(err),
        }
    }
}

impl io::Write for Connected<Stream> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.as_ref().send(buf, Send::empty())
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
    /// Opens a TIPC connection to a remote host.
    pub fn connect<A: ToConnectAddr>(self, addr: A) -> io::Result<Connected<Self>> {
        self.0.connect(addr)?;
        Ok(Connected(self))
    }

    /// Into a implied connected socket.
    pub fn into_connected(self) -> Connected<Self> {
        Connected(self)
    }

    /// Shut down the read, write, or both halves of this connection.
    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.0.shutdown(how)
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

    /// Returns an error representing the last socket error which occurred.
    pub fn last_error(&self) -> io::Error {
        self.0.last_error()
    }

    /// Receives data from the socket.
    ///
    /// On success, returns the number of bytes read and the address from whence the data came.
    pub fn recv_from<T: AsMut<[u8]>>(&self, buf: T) -> io::Result<(usize, SocketAddr)> {
        self.0.recv_from(buf, Recv::empty())
    }

    /// Sends data on the socket to the given address. On success, returns the number of bytes written.
    pub fn send_to<T: AsRef<[u8]>, A: ToSendToAddr>(&self, buf: T, dst: A) -> io::Result<usize> {
        self.0.send_to(buf, dst, Send::empty())
    }
}

/// A TIPC datagram socket.
#[repr(transparent)]
#[derive(Debug)]
pub struct Datagram(Socket);

forward_raw_fd_traits!(Datagram => Socket);

impl Datagram {
    /// Binds this socket to the specified address.
    pub fn bind<A>(self, addr: A) -> io::Result<Bound<Datagram>>
    where
        A: ToBindAddr,
    {
        self.0.bind(addr).map(|_: ()| Bound(self))
    }

    /// Into a implied connected socket.
    pub fn into_connected(self) -> Connected<Self> {
        Connected(self)
    }

    /// Moves this stream into or out of nonblocking mode.
    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.0.set_nonblocking(nonblocking)
    }

    pub fn set_rejectable(&self, rejectable: bool) -> io::Result<()> {
        self.0.set_rejectable(rejectable)
    }

    /// Returns the address of the local half of this TIPC socket.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.local_addr()
    }

    /// Creates a new independently owned handle to the underlying socket.
    pub fn try_clone(&self) -> io::Result<Self> {
        self.0.try_clone().map(Self)
    }

    /// Returns an error representing the last socket error which occurred.
    pub fn last_error(&self) -> io::Error {
        self.0.last_error()
    }

    /// Receives data from the socket.
    ///
    /// On success, returns the number of bytes read and the address from whence the data came.
    pub fn recv_from<T: AsMut<[u8]>>(&self, buf: T) -> io::Result<(usize, SocketAddr)> {
        self.0.recv_from(buf, Recv::empty())
    }

    /// Like `recv_from`, except that it receives into a slice of buffers.
    ///
    /// Data is copied to fill each buffer in order, with the final buffer written to possibly being only partially filled.
    /// This method must behave as a single call to `recv_from` with the buffers concatenated would.
    pub fn recv_vectored(
        &self,
        bufs: &mut [io::IoSliceMut],
    ) -> io::Result<(usize, SocketAddr, Option<ServiceRange>)> {
        self.0.recv_vectored(bufs, Recv::empty())
    }

    /// Attempt to send a message from the socket to the specified destination.
    ///
    /// There are three cases:
    ///
    /// - If the destination is a socket address the message is unicast to that specific socket.
    /// - If the destination is a service address, it is an anycast to any matching destination.
    /// - If the destination is a service range, the message is a multicast to all matching sockets.
    ///
    /// Note however that the rules for what is a match differ between datagram and group messaging.
    pub fn send_to<T: AsRef<[u8]>, A: ToSendToAddr>(&self, buf: T, dst: A) -> io::Result<usize> {
        self.0.send_to(buf, dst, Send::empty())
    }

    /// Like `send_to`, except that it sends from a slice of buffers.
    ///
    /// Data is copied to from each buffer in order, with the final buffer read from possibly being only partially consumed.
    /// This method must behave as a call to `send_to` with the buffers concatenated would.
    pub fn send_vectored<A>(&self, bufs: &[io::IoSlice], addr: A) -> io::Result<usize>
    where
        A: ToSendToAddr,
    {
        self.0.send_vectored(bufs, addr, Send::empty())
    }

    /// Join a communication group.
    pub fn join(
        self,
        service: ServiceAddr,
        visibility: Visibility,
        flags: Join,
    ) -> io::Result<Group<Self>> {
        self.0.join(service, visibility, flags)?;

        Ok(Group(self))
    }
}

/// A communication group.
#[repr(transparent)]
#[derive(Debug)]
pub struct Group<T>(T);

impl<T> Deref for Group<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Group<T>
where
    T: AsRef<Socket>,
{
    /// Join a communication group.
    pub fn join(
        &self,
        service: ServiceAddr,
        visibility: Visibility,
        flags: Join,
    ) -> io::Result<()> {
        self.0.as_ref().join(service, visibility, flags)
    }

    /// Leave a communication group.
    pub fn leave(&self) -> io::Result<()> {
        self.0.as_ref().leave()
    }

    /// Sends a broadcast message to all matching sockets.
    pub fn broadcast<B: AsRef<[u8]>>(&self, buf: B) -> io::Result<usize> {
        self.0.as_ref().send(buf, Send::empty())
    }

    /// Sends a multicast message to all matching sockets.
    pub fn multicast<B, A>(&self, buf: B, addr: A) -> io::Result<usize>
    where
        B: AsRef<[u8]>,
        A: ToSendToAddr,
    {
        self.0.as_ref().mcast(buf, addr, Send::empty())
    }

    /// Sends a anycast message to all matching sockets.
    ///
    /// The lookup algorithm applies the regular round-robin algorithm
    pub fn anycast<B: AsRef<[u8]>, A: ToSendToAddr>(&self, buf: B, dst: A) -> io::Result<usize> {
        self.0.as_ref().send_to(buf, dst, Send::empty())
    }

    /// Consumes the `Group`, returning the underlying value.
    pub fn into_inner(self) -> T {
        self.0
    }
}

/// A message was rejected.
#[derive(Debug, Fail)]
#[fail(display = "message rejected, {}", _0)]
pub struct Rejected(u32);

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

/// Constructs a new `Datagram` with the `AF_TIPC` domain, the `SOCK_RDM` type.
///
/// Provides a reliable datagram layer that does not guarantee ordering.
pub fn rdm() -> io::Result<Datagram> {
    new(libc::SOCK_RDM).map(Datagram)
}

/// Constructs a new `Stream` with the `AF_TIPC` domain, the `SOCK_STREAM` type.
///
/// Provides sequenced, reliable, two-way, connection-based byte streams.
pub fn stream() -> io::Result<Stream> {
    new(libc::SOCK_STREAM).map(Stream)
}

/// Constructs a new `Datagram` with the `AF_TIPC` domain, the `SOCK_DGRAM` type.
///
/// Supports datagrams (connectionless, unreliable messages of a fixed maximum length).
pub fn datagram() -> io::Result<Datagram> {
    new(libc::SOCK_DGRAM).map(Datagram)
}

/// Constructs a new `SeqPacket` with the `AF_TIPC` domain, the `SOCK_SEQPACKET` type.
///
/// Provides a sequenced, reliable, two-way connection-based data transmission path for datagrams of fixed maximum length;
/// a consumer is required to read an entire packet with each input system call.
pub fn seq_packet() -> io::Result<SeqPacket> {
    new(libc::SOCK_SEQPACKET).map(SeqPacket)
}

/// Constructs a new `Socket` with the `AF_TIPC` domain.
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
        self.get_sock_opt(libc::SOL_TIPC, ffi::TIPC_IMPORTANCE)
    }

    /// Set the message importance levels.
    pub fn set_importance(&self, importance: Importance) -> io::Result<()> {
        self.set_sock_opt(libc::SOL_TIPC, ffi::TIPC_IMPORTANCE, importance as u32)
    }

    /// Get the connect timeout.
    pub fn connect_timeout(&self) -> io::Result<Duration> {
        self.get_sock_opt(libc::SOL_TIPC, ffi::TIPC_CONN_TIMEOUT)
            .map(|ms: u32| Duration::from_millis(u64::from(ms)))
    }

    /// Sets the connect timeout to the timeout specified.
    pub fn set_connect_timeout(&self, timeout: Duration) -> io::Result<()> {
        self.set_sock_opt(
            libc::SOL_TIPC,
            ffi::TIPC_CONN_TIMEOUT,
            timeout.as_millis() as u32,
        )
    }

    pub fn set_rejectable(&self, rejectable: bool) -> io::Result<()> {
        self.set_sock_opt(
            libc::SOL_TIPC,
            ffi::TIPC_DEST_DROPPABLE,
            if rejectable { TRUE } else { FALSE },
        )
    }

    /// Returns an error representing the last socket error which occurred.
    pub fn last_error(&self) -> io::Error {
        match self.get_sock_opt::<libc::socklen_t>(libc::SOL_SOCKET, libc::SO_ERROR as u32) {
            Ok(err) => io::Error::from_raw_os_error(err as i32),
            Err(err) => err,
        }
    }

    /// Get the current value of a socket option.
    pub fn get_sock_opt<T>(&self, level: i32, opt: u32) -> io::Result<T> {
        let mut buf = MaybeUninit::<T>::zeroed();
        let mut len = mem::size_of::<T>() as u32;

        unsafe {
            libc::getsockopt(
                self.as_raw_fd(),
                level,
                opt as i32,
                buf.as_mut_ptr() as *mut _,
                &mut len,
            )
        }
        .into_result()
        .map(|_: ()| unsafe { buf.assume_init() })
    }

    /// Set a socket option.
    pub fn set_sock_opt<T>(&self, level: i32, opt: u32, val: T) -> io::Result<()> {
        unsafe {
            libc::setsockopt(
                self.as_raw_fd(),
                level,
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
    pub fn try_clone(&self) -> io::Result<Self> {
        unsafe { libc::dup(self.as_raw_fd()) }
            .into_result()
            .map(Self)
    }

    /// Binds this socket to the specified address.
    pub fn bind<A: ToBindAddr>(&self, addr: A) -> io::Result<()> {
        let sa: ffi::sockaddr_tipc = addr.to_bind_addr();

        unsafe {
            libc::bind(
                self.as_raw_fd(),
                &sa as *const _ as *const _,
                mem::size_of::<ffi::sockaddr_tipc>() as u32,
            )
        }
        .into_result()
    }

    /// Unbinds this socket from the specified address.
    pub fn unbind<A: Into<ServiceRange>>(&self, service_range: A) -> io::Result<()> {
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
    }

    /// Mark a socket as ready to accept incoming connection requests using accept()
    pub fn listen(&self) -> io::Result<()> {
        unsafe { libc::listen(self.as_raw_fd(), 0) }.into_result()
    }

    /// Initiate a connection on this socket to the specified address.
    ///
    /// Connects this TIPC socket to a remote address, allowing the `send` and `recv` syscalls to be used to send data
    /// and also applies filters to only receive data from the specified address.
    pub fn connect<A: ToConnectAddr>(&self, addr: A) -> io::Result<()> {
        let sa: ffi::sockaddr_tipc = addr.to_connect_addr();

        unsafe {
            libc::connect(
                self.as_raw_fd(),
                &sa as *const _ as *const _,
                mem::size_of::<ffi::sockaddr_tipc>() as u32,
            )
        }
        .into_result()
    }

    /// Shut down the read, write, or both halves of this connection.
    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        unsafe {
            libc::shutdown(
                self.as_raw_fd(),
                match how {
                    Shutdown::Read => libc::SHUT_RD,
                    Shutdown::Write => libc::SHUT_WR,
                    Shutdown::Both => libc::SHUT_RDWR,
                },
            )
        }
        .into_result()
    }

    /// Sends data on the socket to the remote address to which it is connected.
    ///
    /// The `connect` method will connect this socket to a remote address.
    /// This method will fail if the socket is not connected.
    pub fn send<T: AsRef<[u8]>>(&self, buf: T, flags: Send) -> io::Result<usize> {
        let buf = buf.as_ref();

        unsafe {
            libc::send(
                self.as_raw_fd(),
                buf.as_ptr() as *const _,
                buf.len(),
                flags.bits(),
            )
        }
        .into_result()
    }

    /// Sends data on the socket to the given address. On success, returns the number of bytes written.
    pub fn send_to<T: AsRef<[u8]>, A: ToSendToAddr>(
        &self,
        buf: T,
        addr: A,
        flags: Send,
    ) -> io::Result<usize> {
        let buf = buf.as_ref();
        let sa: ffi::sockaddr_tipc = addr.to_send_to_addr();

        unsafe {
            libc::sendto(
                self.as_raw_fd(),
                buf.as_ptr() as *const _,
                buf.len(),
                flags.bits(),
                &sa as *const _ as *const _,
                mem::size_of::<ffi::sockaddr_tipc>() as u32,
            )
        }
        .into_result()
    }

    /// Sends a multicast message to all matching sockets.
    pub fn mcast<T, A>(&self, buf: T, addr: A, flags: Send) -> io::Result<usize>
    where
        T: AsRef<[u8]>,
        A: ToSendToAddr,
    {
        let buf = buf.as_ref();
        let mut sa: ffi::sockaddr_tipc = addr.to_send_to_addr();

        sa.addrtype = TIPC_ADDR_MCAST as u8;

        unsafe {
            libc::sendto(
                self.as_raw_fd(),
                buf.as_ptr() as *const _,
                buf.len(),
                flags.bits(),
                &sa as *const _ as *const _,
                mem::size_of::<ffi::sockaddr_tipc>() as u32,
            )
        }
        .into_result()
    }

    /// Like `send_to`, except that it sends from a slice of buffers.
    ///
    /// Data is copied to from each buffer in order, with the final buffer read from possibly being only partially consumed.
    /// This method must behave as a call to `send_to` with the buffers concatenated would.
    pub fn send_vectored<A>(&self, bufs: &[io::IoSlice], addr: A, flags: Send) -> io::Result<usize>
    where
        A: ToSendToAddr,
    {
        let addr = addr.to_send_to_addr();
        let msg = libc::msghdr {
            msg_name: &addr as *const _ as *mut _,
            msg_namelen: mem::size_of::<ffi::sockaddr_tipc>() as u32,
            msg_iov: bufs.as_ptr() as *const _ as *mut _,
            msg_iovlen: bufs.len(),
            msg_control: ptr::null_mut(),
            msg_controllen: 0,
            msg_flags: 0,
        };

        unsafe { libc::sendmsg(self.as_raw_fd(), &msg, flags.bits()) }.into_result()
    }

    /// Receives data from the socket.
    ///
    /// On success, returns the number of bytes read.
    pub fn recv<T: AsMut<[u8]>>(&self, mut buf: T, flags: Recv) -> io::Result<usize> {
        let buf = buf.as_mut();

        unsafe {
            libc::recv(
                self.as_raw_fd(),
                buf.as_mut_ptr() as *mut _,
                buf.len(),
                flags.bits(),
            )
        }
        .into_result()
    }

    /// Receives data from the socket.
    ///
    /// On success, returns the number of bytes read and the address from whence the data came.
    pub fn recv_from<T: AsMut<[u8]>>(
        &self,
        buf: T,
        flags: Recv,
    ) -> io::Result<(usize, SocketAddr)> {
        match self.recv_msg(buf, flags)? {
            (RecvMsg::Message(len, _), addr) => Ok((len, addr)),
            (RecvMsg::Rejected(err, _), _) => Err(io::Error::new(
                io::ErrorKind::Other,
                Error::from(Rejected(err)),
            )),
            (msg, _) => Err(io::Error::new(
                io::ErrorKind::Other,
                format_err!("unexpected group event: {:?}", msg),
            )),
        }
    }

    /// Like `recv_from`, except that it receives into a slice of buffers.
    ///
    /// Data is copied to fill each buffer in order, with the final buffer written to possibly being only partially filled.
    /// This method must behave as a single call to `recv_from` with the buffers concatenated would.
    pub fn recv_vectored(
        &self,
        bufs: &mut [io::IoSliceMut],
        flags: Recv,
    ) -> io::Result<(usize, SocketAddr, Option<ServiceRange>)> {
        let mut sender = MaybeUninit::<ffi::sockaddr_tipc>::zeroed();
        let mut control = MaybeUninit::<[u8; 256]>::zeroed();
        let mut msg = libc::msghdr {
            msg_name: sender.as_mut_ptr() as *mut _ as *mut _,
            msg_namelen: mem::size_of::<ffi::sockaddr_tipc>() as u32,
            msg_iov: bufs.as_mut_ptr() as *mut _ as *mut _,
            msg_iovlen: bufs.len(),
            msg_control: control.as_mut_ptr() as *mut _ as *mut _,
            msg_controllen: 256,
            msg_flags: 0,
        };

        let len =
            unsafe { libc::recvmsg(self.as_raw_fd(), &mut msg, flags.bits()) }.into_result()?;

        let sender = unsafe { sender.assume_init().addr.id.into() };
        let dest_name = unsafe { cmsgs(&msg) }
            .find(|&(ty, level, data)| {
                level == libc::SOL_TIPC
                    && ty == ffi::TIPC_DESTNAME as i32
                    && data.len() == mem::size_of::<ffi::tipc_name_seq>()
            })
            .and_then(|(_, _, data)| NonNull::new(data.as_ptr() as *mut u8))
            .map(|p| p.cast::<ffi::tipc_name_seq>())
            .map(|p| unsafe { p.as_ptr().read() }.into());

        Ok((len, sender, dest_name))
    }

    pub fn recv_msg<T: AsMut<[u8]>>(
        &self,
        mut buf: T,
        flags: Recv,
    ) -> io::Result<(RecvMsg, SocketAddr)> {
        let buf = buf.as_mut();
        let mut addr = MaybeUninit::<[ffi::sockaddr_tipc; 2]>::zeroed();
        let addr_len = mem::size_of_val(&addr) as u32;
        let iov = libc::iovec {
            iov_base: buf.as_mut_ptr() as *mut _,
            iov_len: buf.len(),
        };
        let anc_space_size =
            unsafe { libc::CMSG_SPACE(8) + libc::CMSG_SPACE(1024) + libc::CMSG_SPACE(16) };
        let mut anc_space = vec![0u8; anc_space_size as usize];
        let mut msg = libc::msghdr {
            msg_name: addr.as_mut_ptr() as *mut _,
            msg_namelen: addr_len,
            msg_iov: &iov as *const _ as *mut _,
            msg_iovlen: 1,
            msg_control: anc_space.as_mut_ptr() as *mut _,
            msg_controllen: anc_space.len(),
            msg_flags: 0,
        };

        let rc =
            unsafe { libc::recvmsg(self.as_raw_fd(), &mut msg, flags.bits()) }.into_result()?;

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
                    RecvMsg::MemberLeave(member_id.unwrap())
                } else {
                    RecvMsg::MemberJoin(member_id.unwrap())
                };

                Ok((event, sock_id))
            }
        } else {
            let mut err = None;
            let mut len = None;
            let mut dest_name = None;

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
                        ffi::TIPC_DESTNAME
                            if data.len() == mem::size_of::<ffi::tipc_name_seq>() =>
                        {
                            dest_name = NonNull::new(data.as_ptr() as *mut u8)
                                .map(|p| p.cast::<ffi::tipc_name_seq>())
                                .map(|p| unsafe { p.as_ptr().read() }.into());
                        }
                        _ => {}
                    }
                }
            }

            if let Some(err) = err {
                Ok((RecvMsg::Rejected(err, dest_name), self.local_addr()?))
            } else {
                Ok((RecvMsg::Message(rc, dest_name), sock_id))
            }
        }
    }

    /// Join a communication group.
    pub fn join(
        &self,
        service: ServiceAddr,
        visibility: Visibility,
        flags: Join,
    ) -> io::Result<()> {
        let req = ffi::tipc_group_req {
            type_: service.ty(),
            instance: service.instance(),
            scope: visibility as u32,
            flags: flags.bits(),
        };

        self.set_sock_opt(libc::SOL_TIPC, ffi::TIPC_GROUP_JOIN, req)
    }

    /// Leave a communication group.
    pub fn leave(&self) -> io::Result<()> {
        self.set_sock_opt(libc::SOL_TIPC, ffi::TIPC_GROUP_LEAVE, ())
    }
}

bitflags! {
    /// Flags for `join`.
    pub struct Join: u32 {
        /// Receive copy of sent msg when match
        const LOOPBACK = ffi::TIPC_GROUP_LOOPBACK;
        /// Receive membership events in socket
        const MEMBER_EVTS = ffi::TIPC_GROUP_MEMBER_EVTS;
    }
}

bitflags! {
    /// Flags for `recv`.
    pub struct Recv: i32 {
        /// This flag causes the receive operation to return data from the beginning of the receive queue
        /// without removing that data from the queue.
        ///
        /// Thus, a subsequent receive call will return the same data.
        const PEEK = libc::MSG_PEEK;
        /// This flag requests that the operation block until the full request is satisfied.
        ///
        /// However, the call may still return less data than requested if a signal is caught,
        /// an error or disconnect occurs, or the next data to be received is of a different type than that returned.
        /// This flag has no effect for datagram sockets.
        const WAIT_ALL = libc::MSG_WAITALL;
        /// Enables nonblocking operation;
        /// if the operation would block, the call fails with the error `EAGAIN` or `EWOULDBLOCK`.
        const DONT_WAIT = libc::MSG_DONTWAIT;
    }
}

bitflags! {
    /// Flags for `send`.
    pub struct Send: i32 {
        /// Enables nonblocking operation;
        /// if the operation would block, `EAGAIN` or `EWOULDBLOCK` is returned.
        const DONT_WAIT = libc::MSG_DONTWAIT;
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

/// Conversion into a local address to bind.
pub trait ToBindAddr {
    /// Creates a `ffi::sockaddr_tipc` from the address.
    fn to_bind_addr(&self) -> ffi::sockaddr_tipc;
}

impl ToBindAddr for ServiceAddr {
    fn to_bind_addr(&self) -> ffi::sockaddr_tipc {
        (ServiceRange::from(*self), Visibility::Cluster).to_bind_addr()
    }
}
impl ToBindAddr for ServiceRange {
    fn to_bind_addr(&self) -> ffi::sockaddr_tipc {
        (*self, Visibility::Cluster).to_bind_addr()
    }
}
impl ToBindAddr for (ServiceRange, Visibility) {
    fn to_bind_addr(&self) -> ffi::sockaddr_tipc {
        let (service_range, visibility) = *self;
        let mut sa: ffi::sockaddr_tipc = service_range.into();
        sa.scope = visibility as i8;
        sa
    }
}
impl ToBindAddr for (Type, Instance, Visibility) {
    fn to_bind_addr(&self) -> ffi::sockaddr_tipc {
        let (ty, instance, visibility) = *self;
        (ServiceRange::from((ty, instance)), visibility).to_bind_addr()
    }
}
impl ToBindAddr for (Type, Instance) {
    fn to_bind_addr(&self) -> ffi::sockaddr_tipc {
        ServiceRange::from(*self).into()
    }
}
impl ToBindAddr for (Type, Range<Instance>, Visibility) {
    fn to_bind_addr(&self) -> ffi::sockaddr_tipc {
        let (ty, service_range, visibility) = self;
        (
            ServiceRange::from((*ty, service_range.clone())),
            *visibility,
        )
            .to_bind_addr()
    }
}
impl ToBindAddr for (Type, Range<Instance>) {
    fn to_bind_addr(&self) -> ffi::sockaddr_tipc {
        ServiceRange::from(self.clone()).into()
    }
}
impl ToBindAddr for (Type, Visibility) {
    fn to_bind_addr(&self) -> ffi::sockaddr_tipc {
        let (ty, visibility) = *self;
        (ServiceRange::with_range(ty, ..), visibility).to_bind_addr()
    }
}
impl ToBindAddr for Type {
    fn to_bind_addr(&self) -> ffi::sockaddr_tipc {
        ServiceRange::with_range(*self, ..).into()
    }
}

/// Conversion into a remote address to connect.
pub trait ToConnectAddr {
    /// Creates a `ffi::sockaddr_tipc` from the address.
    fn to_connect_addr(&self) -> ffi::sockaddr_tipc;
}

impl ToConnectAddr for ServiceAddr {
    fn to_connect_addr(&self) -> ffi::sockaddr_tipc {
        (*self, Scope::Global).to_connect_addr()
    }
}
impl ToConnectAddr for (ServiceAddr, Scope) {
    fn to_connect_addr(&self) -> ffi::sockaddr_tipc {
        let (addr, scope) = *self;
        let mut sa: ffi::sockaddr_tipc = addr.into();
        sa.addr.name.domain = scope.into();
        sa
    }
}
impl ToConnectAddr for (Type, Instance) {
    fn to_connect_addr(&self) -> ffi::sockaddr_tipc {
        ServiceAddr::from(*self).into()
    }
}

/// Conversion into a remote address to sends data to.
///
/// * If the destination is a socket address the message is unicast to that specific socket.
/// * If the destination is a service address, it is an anycast to any matching destination.
/// * If the destination is a service range, the message is a multicast to all matching sockets.
pub trait ToSendToAddr {
    /// Creates a `ffi::sockaddr_tipc` from the address.
    fn to_send_to_addr(&self) -> ffi::sockaddr_tipc;
}

impl ToSendToAddr for SocketAddr {
    fn to_send_to_addr(&self) -> ffi::sockaddr_tipc {
        let mut sa: ffi::sockaddr_tipc = (*self).into();
        sa.addr.name.domain = Scope::Global.into();
        sa
    }
}
impl ToSendToAddr for ServiceAddr {
    fn to_send_to_addr(&self) -> ffi::sockaddr_tipc {
        (*self, Scope::Global).to_send_to_addr()
    }
}
impl ToSendToAddr for ServiceRange {
    fn to_send_to_addr(&self) -> ffi::sockaddr_tipc {
        (*self, Scope::Global).to_send_to_addr()
    }
}
impl<T> ToSendToAddr for (T, Scope)
where
    T: Into<ffi::sockaddr_tipc> + Copy,
{
    fn to_send_to_addr(&self) -> ffi::sockaddr_tipc {
        let (addr, scope) = *self;
        let mut sa: ffi::sockaddr_tipc = addr.into();
        sa.addr.name.domain = scope.into();
        sa
    }
}

#[derive(Clone, Debug)]
pub enum RecvMsg {
    MemberJoin(ServiceAddr),
    MemberLeave(ServiceAddr),
    Message(usize, Option<ServiceRange>),
    Rejected(u32, Option<ServiceRange>),
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
