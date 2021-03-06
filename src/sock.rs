use core::convert::TryInto;
use core::iter;
use core::marker::PhantomData;
use core::mem::{self, MaybeUninit};
use core::ops::Deref;
use core::option::IntoIter;
use core::ptr;
use core::ptr::NonNull;
use core::slice;
use core::time::Duration;

use std::ffi::CStr;
use std::io;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};

use bitflags::bitflags;
use failure::{err_msg, format_err, Error, Fail};

use crate::{
    addr::{Instance, Scope, ServiceAddr, ServiceRange, SocketAddr, Visibility, TIPC_ADDR_MCAST},
    ffi,
};

const TRUE: i32 = 1;
const FALSE: i32 = 0;

const MAX_MSG_SIZE: usize = 1024;

/// The bearer identity.
pub type BearerId = u32;

pub fn addr_not_available() -> io::Error {
    io::Error::new(
        io::ErrorKind::AddrNotAvailable,
        err_msg("address not available"),
    )
}

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
macro_rules! impl_raw_fd_traits {
    ($name:ident <$param:ident> ( $inner:ident )) => {
        impl<$param> AsRawFd for $name<$param> {
            fn as_raw_fd(&self) -> RawFd {
                self.0.as_raw_fd()
            }
        }

        impl<$param> FromRawFd for $name<$param> {
            unsafe fn from_raw_fd(fd: RawFd) -> Self {
                Self($inner::from_raw_fd(fd), PhantomData)
            }
        }

        impl<$param> IntoRawFd for $name<$param> {
            fn into_raw_fd(self) -> RawFd {
                self.0.into_raw_fd()
            }
        }

        impl<$param> Deref for $name<$param> {
            type Target = RawFd;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
    ($name:ident <$param:ident>) => {
        impl_raw_fd_traits! { $name<$param>(Socket) }
    };
    ( $name:ident ( $inner:ident ) ) => {
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
    ($name:ident) => {
        impl_raw_fd_traits! { $name(Socket) }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! impl_socket_wrapped {
    ($wrapped:ident) => {
        impl Wrapped for $wrapped {}

        impl AsRef<Socket> for $wrapped {
            fn as_ref(&self) -> &Socket {
                &self.0
            }
        }

        impl From<Socket> for $wrapped {
            fn from(sock: Socket) -> Self {
                Self(sock)
            }
        }

        impl From<$wrapped> for Socket {
            fn from(wrapped: $wrapped) -> Self {
                wrapped.0
            }
        }
    };
}

/// Binds this socket to the specified address.
pub fn bind<T, A>(addr: A) -> io::Result<Bound<T>>
where
    T: Bindable,
    A: ToServiceRanges,
{
    T::builder()?.bind(addr)
}

/// Opens a TIPC connection to a remote host.
pub fn connect<T, A>(addr: A) -> io::Result<Connected<T>>
where
    T: Connectable,
    A: ToServiceAddrs,
{
    T::builder()?.connect(addr)
}

/// Opens a TIPC connection to a remote host with a timeout.
pub fn connect_timeout<T, A>(addr: A, timeout: Duration) -> io::Result<Connected<T>>
where
    T: Connectable,
    A: ToServiceAddrs,
{
    T::builder()?.connect_timeout(timeout)?.connect(addr)
}

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

/// A wrapped TIPC socket.
pub trait Wrapped: AsRef<Socket> + From<Socket> + Into<Socket> {}

/// A bindable TIPC socket.
pub trait Bindable: Buildable + Wrapped {}

impl Bindable for Stream {}
impl Bindable for SeqPacket {}
impl Bindable for Datagram {}

/// A connectable TIPC socket.
pub trait Connectable: Buildable + Wrapped {}

impl Connectable for Stream {}
impl Connectable for SeqPacket {}

/// An "in progress" TIPC socket which has not yet been connected or listened.
///
/// Allows configuration of a socket before one of these operations is executed.
#[repr(transparent)]
#[derive(Debug)]
pub struct Builder<T>(Socket, PhantomData<T>);

impl_raw_fd_traits!(Builder<T>);

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
        A: ToServiceRanges,
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
        A: ToServiceAddrs,
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

    /// Sets the maximum socket receive buffer in bytes.
    pub fn recv_buf_size(self, size: i32) -> io::Result<Self> {
        self.0.set_recv_buf_size(size).map(|_| self)
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
    /// Binds this socket to the specified address.
    pub fn bind<A: ToServiceRanges>(&self, addr: A) -> io::Result<()> {
        self.0.as_ref().bind(addr)
    }

    /// Unbinds this socket from the specified address.
    pub fn unbind<A: ToServiceRanges>(self, addr: A) -> io::Result<()> {
        self.0.as_ref().unbind(addr)
    }

    /// Mark a socket as ready to accept incoming connection requests using accept()
    pub fn listen(self) -> io::Result<Listener<T>>
    where
        T: Connectable,
    {
        self.0
            .as_ref()
            .listen()
            .map(|_| Listener(self.0.into(), PhantomData))
    }
}

/// A TIPC socket server, listening for connections.
#[repr(transparent)]
#[derive(Debug)]
pub struct Listener<T>(Socket, PhantomData<T>);

impl_raw_fd_traits!(Listener<T>);

impl<T> From<T> for Listener<T>
where
    T: Connectable,
{
    fn from(sock: T) -> Self {
        Self(sock.into(), PhantomData)
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
        T: Connectable,
    {
        let mut sa = MaybeUninit::<ffi::sockaddr_tipc>::uninit();
        let mut len = mem::size_of::<ffi::sockaddr_tipc>() as u32;

        unsafe {
            libc::accept(self.0.as_raw_fd(), sa.as_mut_ptr() as *mut _, &mut len)
                .into_result()
                .map(|sd| {
                    (
                        Connected(Socket::from_raw_fd(sd).into()),
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
    T: Connectable,
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
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        let mut sa = MaybeUninit::<ffi::sockaddr_tipc>::uninit();
        let mut len = mem::size_of::<ffi::sockaddr_tipc>() as u32;

        unsafe {
            libc::getpeername(
                self.0.as_ref().as_raw_fd(),
                sa.as_mut_ptr() as *mut _,
                &mut len,
            )
            .into_result()
            .map(|_: ()| sa.assume_init().addr.id.into())
        }
    }

    /// Shut down the read and write halves of this connection.
    pub fn shutdown(&self) -> io::Result<()>
    where
        T: Connectable,
    {
        self.0.as_ref().shutdown()
    }
}

/// A TIPC stream between a local and a remote socket.
#[repr(transparent)]
#[derive(Debug)]
pub struct Stream(Socket);

impl_raw_fd_traits!(Stream);
impl_socket_wrapped!(Stream);

impl Stream {
    /// Opens a TIPC connection to a remote host.
    pub fn connect<A: ToServiceAddrs>(self, addr: A) -> io::Result<Connected<Self>> {
        self.0.connect(addr)?;
        Ok(Connected(self))
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

impl_raw_fd_traits!(SeqPacket);
impl_socket_wrapped!(SeqPacket);

impl SeqPacket {
    /// Opens a TIPC connection to a remote host.
    pub fn connect<A: ToServiceAddrs>(self, addr: A) -> io::Result<Connected<Self>> {
        self.0.connect(addr)?;
        Ok(Connected(self))
    }

    /// Into a implied connected socket.
    pub fn into_connected(self) -> Connected<Self> {
        Connected(self)
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

    /// Receives a single datagram message on the socket, without removing it from the queue.
    ///
    /// On success, returns the number of bytes read and the origin.
    pub fn peek_from<T: AsMut<[u8]>>(&self, buf: T) -> io::Result<(usize, SocketAddr)> {
        self.0.recv_from(buf, Recv::PEEK)
    }

    /// Sends data on the socket to the given address. On success, returns the number of bytes written.
    pub fn send_to<T, A>(&self, buf: T, dst: A) -> io::Result<usize>
    where
        T: AsRef<[u8]>,
        A: ToSocketAddrs,
    {
        self.0.send_to(buf, dst, Send::empty())
    }
}

/// A TIPC datagram socket.
#[repr(transparent)]
#[derive(Debug)]
pub struct Datagram(Socket);

impl_raw_fd_traits!(Datagram);
impl_socket_wrapped!(Datagram);

impl Datagram {
    /// Binds this socket to the specified address.
    pub fn bind<A>(self, addr: A) -> io::Result<Bound<Self>>
    where
        A: ToServiceRanges,
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

    /// Receives a single datagram message on the socket, without removing it from the queue.
    ///
    /// On success, returns the number of bytes read and the origin.
    pub fn peek_from<T: AsMut<[u8]>>(&self, buf: T) -> io::Result<(usize, SocketAddr)> {
        self.0.recv_from(buf, Recv::PEEK)
    }

    /// Like `recv_from`, except that it receives into a slice of buffers.
    ///
    /// Data is copied to fill each buffer in order, with the final buffer written to possibly being only partially filled.
    /// This method must behave as a single call to `recv_from` with the buffers concatenated would.
    pub fn recv_from_vectored(
        &self,
        bufs: &mut [io::IoSliceMut],
    ) -> io::Result<(usize, SocketAddr, Option<ServiceRange>)> {
        self.0.recv_from_vectored(bufs, Recv::empty())
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
    ///
    /// It is possible for addr to yield multiple addresses,
    /// but `send_to` will only send data to the first address yielded by addr.
    pub fn send_to<T, A>(&self, buf: T, dst: A) -> io::Result<usize>
    where
        T: AsRef<[u8]>,
        A: ToSocketAddrs,
    {
        self.0.send_to(buf, dst, Send::empty())
    }

    /// Like `send_to`, except that it sends from a slice of buffers.
    ///
    /// Data is copied to from each buffer in order, with the final buffer read from possibly being only partially consumed.
    /// This method must behave as a call to `send_to` with the buffers concatenated would.
    pub fn send_to_vectored<A>(&self, bufs: &[io::IoSlice], addr: A) -> io::Result<usize>
    where
        A: ToSocketAddrs,
    {
        self.0.send_to_vectored(bufs, addr, Send::empty())
    }

    /// Join a communication group.
    pub fn join<A: ToServiceAddrs>(self, addr: A, flags: Join) -> io::Result<Group<Self>> {
        self.0.join(addr, flags)?;

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
    /// Leave a communication group.
    pub fn leave(self) -> io::Result<T> {
        self.0.as_ref().leave().map(|_| self.0)
    }

    /// Sends a broadcast message to all matching sockets.
    pub fn broadcast<B: AsRef<[u8]>>(&self, buf: B) -> io::Result<usize> {
        self.0.as_ref().send(buf, Send::empty())
    }

    /// Sends a multicast message to all matching sockets.
    pub fn multicast<B, A>(&self, buf: B, addr: A) -> io::Result<usize>
    where
        B: AsRef<[u8]>,
        A: ToSocketAddrs,
    {
        self.0.as_ref().mcast(buf, addr, Send::empty())
    }

    /// Sends a anycast message to all matching sockets.
    ///
    /// The lookup algorithm applies the regular round-robin algorithm
    pub fn anycast<B, A>(&self, buf: B, dst: A) -> io::Result<usize>
    where
        B: AsRef<[u8]>,
        A: ToSocketAddrs,
    {
        self.0.as_ref().send_to(buf, dst, Send::empty())
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

    /// Gets the maximum socket receive buffer in bytes.
    pub fn recv_buf_size(&self) -> io::Result<i32> {
        self.get_sock_opt(libc::SOL_SOCKET, libc::SO_RCVBUF as u32)
    }

    /// Sets the maximum socket receive buffer in bytes.
    pub fn set_recv_buf_size(&self, size: i32) -> io::Result<()> {
        self.set_sock_opt(libc::SOL_SOCKET, libc::SO_RCVBUF as u32, size)
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
    pub fn bind<A: ToServiceRanges>(&self, addr: A) -> io::Result<()> {
        for addr in addr.to_service_ranges()? {
            let sa: ffi::sockaddr_tipc = addr.into();

            unsafe {
                libc::bind(
                    self.as_raw_fd(),
                    &sa as *const _ as *const _,
                    mem::size_of::<ffi::sockaddr_tipc>() as u32,
                )
            }
            .into_result()?;
        }

        Ok(())
    }

    /// Unbinds this socket from the specified address.
    pub fn unbind<A: ToServiceRanges>(&self, addr: A) -> io::Result<()> {
        for addr in addr.to_service_ranges()? {
            let mut sa: ffi::sockaddr_tipc = addr.into();

            sa.scope = -1;

            unsafe {
                libc::bind(
                    self.as_raw_fd(),
                    &sa as *const _ as *const _,
                    mem::size_of::<ffi::sockaddr_tipc>() as u32,
                )
            }
            .into_result()?;
        }

        Ok(())
    }

    /// Mark a socket as ready to accept incoming connection requests using accept()
    pub fn listen(&self) -> io::Result<()> {
        unsafe { libc::listen(self.as_raw_fd(), 0) }.into_result()
    }

    /// Initiate a connection on this socket to the specified address.
    ///
    /// Connects this TIPC socket to a remote address, allowing the `send` and `recv` syscalls to be used to send data
    /// and also applies filters to only receive data from the specified address.
    pub fn connect<A: ToServiceAddrs>(&self, addr: A) -> io::Result<()> {
        let mut res = Err(addr_not_available());

        for addr in addr.to_service_addrs()? {
            let sa: ffi::sockaddr_tipc = addr.into();

            res = unsafe {
                libc::connect(
                    self.as_raw_fd(),
                    &sa as *const _ as *const _,
                    mem::size_of::<ffi::sockaddr_tipc>() as u32,
                )
            }
            .into_result();

            if res.is_ok() {
                break;
            }
        }

        res
    }

    /// Shut down the read and write halves of this connection.
    ///
    /// The socket's peer is notified that the connection was gracefully terminated
    /// (by means of the TIPC_CONN_SHUTDOWN error code), rather than as the result of an error.
    pub fn shutdown(&self) -> io::Result<()> {
        unsafe { libc::shutdown(self.as_raw_fd(), libc::SHUT_RDWR) }.into_result()
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
    pub fn send_to<T: AsRef<[u8]>, A: ToSocketAddrs>(
        &self,
        buf: T,
        addr: A,
        flags: Send,
    ) -> io::Result<usize> {
        let buf = buf.as_ref();
        let sa: ffi::sockaddr_tipc = addr
            .to_socket_addrs()?
            .next()
            .ok_or_else(addr_not_available)?
            .into();

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
        A: ToSocketAddrs,
    {
        let buf = buf.as_ref();
        let mut sa: ffi::sockaddr_tipc = addr
            .to_socket_addrs()?
            .next()
            .ok_or_else(addr_not_available)?
            .into();

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
    pub fn send_to_vectored<A>(
        &self,
        bufs: &[io::IoSlice],
        addr: A,
        flags: Send,
    ) -> io::Result<usize>
    where
        A: ToSocketAddrs,
    {
        let addr = addr
            .to_socket_addrs()?
            .next()
            .ok_or_else(addr_not_available)?
            .into();
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
            (RecvMsg::Message { len, .. }, addr) => Ok((len, addr)),
            (RecvMsg::Rejected { err, .. }, _) => Err(io::Error::new(
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
    pub fn recv_from_vectored(
        &self,
        bufs: &mut [io::IoSliceMut],
        flags: Recv,
    ) -> io::Result<(usize, SocketAddr, Option<ServiceRange>)> {
        let mut sender = MaybeUninit::<ffi::sockaddr_tipc>::zeroed();
        let mut control = MaybeUninit::<[u8; MAX_MSG_SIZE]>::zeroed();
        let mut msg = libc::msghdr {
            msg_name: sender.as_mut_ptr() as *mut _ as *mut _,
            msg_namelen: mem::size_of::<ffi::sockaddr_tipc>() as u32,
            msg_iov: bufs.as_mut_ptr() as *mut _ as *mut _,
            msg_iovlen: bufs.len(),
            msg_control: control.as_mut_ptr() as *mut _ as *mut _,
            msg_controllen: MAX_MSG_SIZE,
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

        let len =
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
            if len != 0 {
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
            let mut err_len = None;
            let mut service = None;

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
                            err_len = chunks
                                .next()
                                .unwrap()
                                .try_into()
                                .map(u32::from_ne_bytes)
                                .map(|n| n as usize)
                                .ok();
                        }
                        ffi::TIPC_RETDATA if Some(data.len()) == err_len => {
                            let len = buf.len().min(err_len.unwrap());
                            let buf = &mut buf[..len];

                            buf.copy_from_slice(&data[..len]);
                        }
                        ffi::TIPC_DESTNAME
                            if data.len() == mem::size_of::<ffi::tipc_name_seq>() =>
                        {
                            service = NonNull::new(data.as_ptr() as *mut u8)
                                .map(|p| p.cast::<ffi::tipc_name_seq>())
                                .map(|p| unsafe { p.as_ptr().read() }.into());
                        }
                        _ => {}
                    }
                }
            }

            if let Some(err) = err {
                Ok((RecvMsg::Rejected { err, service }, self.local_addr()?))
            } else {
                Ok((RecvMsg::Message { len, service }, sock_id))
            }
        }
    }

    /// Join a communication group.
    pub fn join<A: ToServiceAddrs>(&self, addr: A, flags: Join) -> io::Result<()> {
        let (service, scope) = addr
            .to_service_addrs()?
            .next()
            .ok_or_else(addr_not_available)?;
        let req = ffi::tipc_group_req {
            type_: service.ty(),
            instance: service.instance(),
            scope: scope.visibility() as u32,
            flags: flags.bits(),
        };

        self.set_sock_opt(libc::SOL_TIPC, ffi::TIPC_GROUP_JOIN, req)
    }

    /// Leave a communication group.
    pub fn leave(&self) -> io::Result<()> {
        self.set_sock_opt(libc::SOL_TIPC, ffi::TIPC_GROUP_LEAVE, ())
    }

    /// Retrieve a node identity.
    pub fn node_id(&self, peer: Instance) -> io::Result<String> {
        let mut req = ffi::tipc_sioc_nodeid_req {
            peer,
            ..Default::default()
        };

        unsafe { libc::ioctl(self.as_raw_fd(), u64::from(ffi::SIOCGETNODEID), &mut req) }
            .into_result()?;

        Ok(unsafe { CStr::from_ptr(req.node_id.as_ptr() as *const _) }
            .to_str()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?
            .to_owned())
    }

    /// Retrieve a link name.
    pub fn link_name(&self, peer: Instance, bearer_id: BearerId) -> io::Result<String> {
        let mut req = ffi::tipc_sioc_ln_req {
            peer,
            bearer_id,
            ..Default::default()
        };

        unsafe { libc::ioctl(self.as_raw_fd(), u64::from(ffi::SIOCGETLINKNAME), &mut req) }
            .into_result()?;

        Ok(unsafe { CStr::from_ptr(req.linkname.as_ptr() as *const _) }
            .to_str()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?
            .to_owned())
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

/// A trait for objects which can be converted or resolved to one or more `ServiceRange` values.
pub trait ToServiceRanges {
    /// Returned iterator over socket ranges which this type may correspond to.
    type Iter: Iterator<Item = (ServiceRange, Visibility)>;

    /// Converts this object to an iterator of resolved `ServiceRange`s.
    ///
    /// The returned iterator may not actually yield any values depending on the outcome of any resolution performed.
    ///
    /// Note that this function may block the current thread while resolution is performed.
    fn to_service_ranges(&self) -> io::Result<Self::Iter>;
}

impl<T> ToServiceRanges for (T, Visibility)
where
    T: Into<ServiceRange> + Clone,
{
    type Iter = IntoIter<(ServiceRange, Visibility)>;

    fn to_service_ranges(&self) -> io::Result<Self::Iter> {
        let (service_range, visibility) = self.clone();

        Ok(Some((service_range.into(), visibility)).into_iter())
    }
}

impl<T> ToServiceRanges for T
where
    T: Into<ServiceRange> + Clone,
{
    type Iter = IntoIter<(ServiceRange, Visibility)>;

    fn to_service_ranges(&self) -> io::Result<Self::Iter> {
        Ok(Some((self.clone().into(), Visibility::Cluster)).into_iter())
    }
}

/// A trait for objects which can be converted or resolved to one or more `ServiceAddr` values.
pub trait ToServiceAddrs {
    /// Returned iterator over socket addresses which this type may correspond to.
    type Iter: Iterator<Item = (ServiceAddr, Scope)>;

    /// Converts this object to an iterator of resolved `ServiceAddr`s.
    ///
    /// The returned iterator may not actually yield any values depending on the outcome of any resolution performed.
    ///
    /// Note that this function may block the current thread while resolution is performed.
    fn to_service_addrs(&self) -> io::Result<Self::Iter>;
}

impl<T> ToServiceAddrs for T
where
    T: Into<ServiceAddr> + Clone,
{
    type Iter = IntoIter<(ServiceAddr, Scope)>;

    fn to_service_addrs(&self) -> io::Result<Self::Iter> {
        Ok(Some((self.clone().into(), Scope::Global)).into_iter())
    }
}

impl<T> ToServiceAddrs for (T, Scope)
where
    T: Into<ServiceAddr> + Clone,
{
    type Iter = IntoIter<(ServiceAddr, Scope)>;

    fn to_service_addrs(&self) -> io::Result<Self::Iter> {
        let (service_addr, scope) = self.clone();

        Ok(Some((service_addr.clone().into(), scope)).into_iter())
    }
}

/// A trait for objects which can be converted or resolved to one or more `ffi::sockaddr_tipc` values.
pub trait ToSocketAddrs {
    /// The socket addresses
    type Item: Into<ffi::sockaddr_tipc>;
    /// Returned iterator over socket addresses which this type may correspond to.
    type Iter: Iterator<Item = Self::Item>;

    /// Converts this object to an iterator of resolved `ffi::sockaddr_tipc`s.
    ///
    /// The returned iterator may not actually yield any values depending on the outcome of any resolution performed.
    ///
    /// Note that this function may block the current thread while resolution is performed.
    fn to_socket_addrs(&self) -> io::Result<Self::Iter>;
}

impl<T> ToSocketAddrs for T
where
    T: Into<ffi::sockaddr_tipc> + Clone,
{
    type Item = T;
    type Iter = IntoIter<T>;

    fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        Ok(Some(self.clone()).into_iter())
    }
}

/// Received message from a TIPC socket.
#[derive(Clone, Debug)]
pub enum RecvMsg {
    /// Reception of a membership event.
    MemberJoin(ServiceAddr),
    /// Reception of a membership event.
    MemberLeave(ServiceAddr),
    /// A returned undelivered data message.
    Message {
        len: usize,
        service: Option<ServiceRange>,
    },
    /// The message was rejected
    Rejected {
        err: u32,
        service: Option<ServiceRange>,
    },
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
