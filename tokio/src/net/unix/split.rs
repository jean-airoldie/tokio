//! `UnixStream` and `UnixDatagram` split support.
//!
//! ## UnixStream
//!
//! A `UnixStream` can be split into a read half and a write half with
//! `UnixStream::split`. The read half implements `AsyncRead` while the write
//! half implements `AsyncWrite`.
//!
//! Compared to the generic split of `AsyncRead + AsyncWrite`, this specialized
//! split has no associated overhead and enforces all invariants at the type
//! level.
//!
//! ## UnixDatagram
//!
//! A `UnixDatagram` can be split into a receive half and a send half with
//! `UnixDatagram::split`. The send half implements `send` and `send_to` and
//! the receiving one implements `recv` and `recv_from`.
//!
//! This split method has no overhead and enforces all invariants at the type
//! level.

use crate::io::{AsyncRead, AsyncWrite};
use crate::net::{UnixDatagram, UnixStream};

use bytes::{Buf, BufMut};
use futures_util::future::poll_fn;
use std::io;
use std::net::Shutdown;
use std::os::unix::net::SocketAddr;
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Receiving half of a `UnixDatagram`.
#[derive(Debug)]
pub struct RecvHalf<'a>(&'a UnixDatagram);

/// Sending half of a `UnixDatagram`.
#[derive(Debug)]
pub struct SendHalf<'a>(&'a UnixDatagram);

/// Read half of a `UnixStream`.
#[derive(Debug)]
pub struct ReadHalf<'a>(&'a UnixStream);

/// Write half of a `UnixStream`.
#[derive(Debug)]
pub struct WriteHalf<'a>(&'a UnixStream);

pub(crate) fn split(stream: &mut UnixStream) -> (ReadHalf<'_>, WriteHalf<'_>) {
    (ReadHalf(stream), WriteHalf(stream))
}

pub(crate) fn split_dgram(dgram: &mut UnixDatagram) -> (RecvHalf<'_>, SendHalf<'_>) {
    (RecvHalf(dgram), SendHalf(dgram))
}

impl RecvHalf<'_> {
    /// Receives a datagram from the socket.
    pub async fn recv(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        poll_fn(|cx| self.0.poll_recv_priv(cx, buf)).await
    }

    /// Receives a datagram with the source address from the socket.
    pub async fn recv_from(&mut self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        poll_fn(|cx| self.0.poll_recv_from_priv(cx, buf)).await
    }
}

impl SendHalf<'_> {
    /// Sends a datagram to the socket's peer.
    pub async fn send(&mut self, buf: &[u8]) -> io::Result<usize> {
        poll_fn(|cx| self.0.poll_send_priv(cx, buf)).await
    }

    /// Sends a datagram to the specified address.
    pub async fn send_to<P>(&mut self, buf: &[u8], target: P) -> io::Result<usize>
    where
        P: AsRef<Path> + Unpin,
    {
        poll_fn(|cx| self.0.poll_send_to_priv(cx, buf, target.as_ref())).await
    }
}

impl AsyncRead for ReadHalf<'_> {
    unsafe fn prepare_uninitialized_buffer(&self, _: &mut [u8]) -> bool {
        false
    }

    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        self.0.poll_read_priv(cx, buf)
    }

    fn poll_read_buf<B: BufMut>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut B,
    ) -> Poll<io::Result<usize>> {
        self.0.poll_read_buf_priv(cx, buf)
    }
}

impl AsyncWrite for WriteHalf<'_> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.0.poll_write_priv(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.0.shutdown(Shutdown::Write).into()
    }

    fn poll_write_buf<B: Buf>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut B,
    ) -> Poll<io::Result<usize>> {
        self.0.poll_write_buf_priv(cx, buf)
    }
}

impl AsRef<UnixStream> for ReadHalf<'_> {
    fn as_ref(&self) -> &UnixStream {
        self.0
    }
}

impl AsRef<UnixStream> for WriteHalf<'_> {
    fn as_ref(&self) -> &UnixStream {
        self.0
    }
}
