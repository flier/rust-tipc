# Examples

## api_demo

A client and a server progarm communicating via TIPC, but using an
adaptation layer written in Rust instead of working directly with socket
primitives. This simplifies usage of TIPC.

The demo shows four use cases:

1. Client sends a datagram message to server using its service address.
   The server responds with a datgram usng the client's socket address.
   Both endpoints use a `SOCK_RDM` socket.

2. Client sends a datagram message using an invalid service address.
   The system rejects the message back to sender with error code
   "No route to host"

3. Client sends a setup message containing data to server using a
   service address. The server accepts the setup message and responds
   back to client with another data message.  This is an example of the
   TIPC specific two-way "implicit" connection setup protocol.
   Both endpoints use a `SOCK_STREAM` socket in this case, but it could
   just as well have been a `SOCK_SEQPACKET`.

4. Client makes a traditional TCP style `connect()`, resulting in a
   TIPC internal two-way SYN/SYNACK signalling to establish the
   connection. Thereafter we have the same message exchange as above
   across the connection.
   Both endpoints use a `SOCK_SEQPACKET` socket in this case, but it could
   just as well have been a `SOCK_STREAM`.

## connection_demo

A client and a server program setting up a connection and sending a "Hello World" through i.

The connection is set up in two ways:

1. Traditional TCP-style `connect()` with implicit 2-way handshake.
2. TIPC specific-style 1-way "implicit" setup.

## groupcast_demo

A demo showing how unicast/anycast/multicast/broadcast can be sent
loss free from a client to multiple servers, demonstrating the group
communication feature's flow control and sequence guarantee.

## hello_world

A very simple connectionless demo showing how a client if necessary
waits for the server to come up before it sends a "Hello World"
message to it by using a service address.

## iov_control

A minimal program just showing how a server receives a message
by using `recvmsg()` while reading the source socket address
and the used destination service address.

## multicast_demo

A simple demo where a client sends a configurable number of messages
of configurable size to one or more servers.

Usage: mcast_client [lower [upper [kill]]]

If no arguments supplied, sends a predetermined set of multicast messages.
If optional "lower" and "upper" arguments are specified, client sends a
single message to the specified instance range; if "upper" is omitted,
it defaults to the same value as "lower".
If optional "kill" argument is specified (it can have any value), the client
sends the server termination message rather than the normal server data
message.

## stream_demo

Showing that a TCP style `SOCK_STREAM` connectiopn works when sent
message and and receive message buffer sizes don't match.

## topology_subscr_demo

A demo showing how the topology service can be used to keep track
of both logical (server program) and physical (cluster nodes) topology.
