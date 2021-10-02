#![allow(unused)]

use async_stream::stream;
use futures::{future::TryFutureExt, ready, stream::Stream};
use hyper::server::accept::Accept;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use std::future::Future;
use std::net::{SocketAddr, TcpListener as StdTcpListener};
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::time::Sleep;
//use rustls::internal::pemfile;
use std::pin::Pin;
use std::vec::Vec;
use std::{env, fs, io, sync};
use tokio::net::{TcpListener, TcpStream};
//use tokio_rustls::server::TlsStream;
//use tokio_rustls::TlsAcceptor;

pub struct FilteredAddrIncoming<F = ()> {
    addr: SocketAddr,
    listener: TcpListener,
    sleep_on_errors: bool,
    tcp_keepalive_timeout: Option<Duration>,
    tcp_nodelay: bool,
    timeout: Option<Pin<Box<Sleep>>>,
    filter: F,
}

use super::addr_stream::AddrStream;
use super::ip_filter::AddrFilter;

impl<F: AddrFilter> FilteredAddrIncoming<F> {
    pub fn from_listener(listener: TcpListener, filter: F) -> io::Result<Self> {
        let addr = listener.local_addr()?;

        Ok(FilteredAddrIncoming {
            listener,
            addr,
            filter,
            sleep_on_errors: true,
            tcp_keepalive_timeout: None,
            tcp_nodelay: true,
            timeout: None,
        })
    }

    fn from_std(std_listener: StdTcpListener, filter: F) -> io::Result<Self> {
        std_listener.set_nonblocking(true)?;

        Self::from_listener(TcpListener::from_std(std_listener)?, filter)
    }

    /// Get the local address bound to this listener.
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    /// Set whether TCP keepalive messages are enabled on accepted connections.
    ///
    /// If `None` is specified, keepalive is disabled, otherwise the duration
    /// specified will be the time to remain idle before sending TCP keepalive
    /// probes.
    pub fn set_keepalive(&mut self, keepalive: Option<Duration>) -> &mut Self {
        self.tcp_keepalive_timeout = keepalive;
        self
    }

    /// Set the value of `TCP_NODELAY` option for accepted connections.
    pub fn set_nodelay(&mut self, enabled: bool) -> &mut Self {
        self.tcp_nodelay = enabled;
        self
    }

    /// Set whether to sleep on accept errors.
    ///
    /// A possible scenario is that the process has hit the max open files
    /// allowed, and so trying to accept a new connection will fail with
    /// `EMFILE`. In some cases, it's preferable to just wait for some time, if
    /// the application will likely close some files (or connections), and try
    /// to accept the connection again. If this option is `true`, the error
    /// will be logged at the `error` level, since it is still a big deal,
    /// and then the listener will sleep for 1 second.
    ///
    /// In other cases, hitting the max open files should be treat similarly
    /// to being out-of-memory, and simply error (and shutdown). Setting
    /// this option to `false` will allow that.
    ///
    /// Default is `true`.
    pub fn set_sleep_on_errors(&mut self, val: bool) {
        self.sleep_on_errors = val;
    }

    fn poll_next_(&mut self, cx: &mut Context<'_>) -> Poll<Result<AddrStream, io::Error>> {
        // Check if a previous timeout is active that was set by IO errors.
        if let Some(ref mut to) = self.timeout {
            ready!(Pin::new(to).poll(cx));
        }
        self.timeout = None;

        loop {
            match ready!(self.listener.poll_accept(cx)) {
                Ok((socket, addr)) => {
                    if !self.filter.allow(&addr.ip()) {
                        if cfg!(debug_assertions) {
                            log::trace!("Dropping IP: {}", addr.ip());
                        }

                        continue;
                    }

                    if let Some(dur) = self.tcp_keepalive_timeout {
                        let socket = socket2::SockRef::from(&socket);
                        let conf = socket2::TcpKeepalive::new().with_time(dur);

                        if let Err(e) = socket.set_tcp_keepalive(&conf) {
                            log::trace!("error trying to set TCP keepalive: {}", e);
                        }
                    }

                    if let Err(e) = socket.set_nodelay(self.tcp_nodelay) {
                        log::trace!("error trying to set TCP nodelay: {}", e);
                    }

                    return Poll::Ready(Ok(AddrStream::new(socket, addr)));
                }
                Err(e) => {
                    // Connection errors can be ignored directly, continue by
                    // accepting the next request.
                    if is_connection_error(&e) {
                        log::debug!("accepted connection already errored: {}", e);
                        continue;
                    }

                    if self.sleep_on_errors {
                        log::error!("accept error: {}", e);

                        // Sleep 1s.
                        let mut timeout = Box::pin(tokio::time::sleep(Duration::from_secs(1)));

                        match timeout.as_mut().poll(cx) {
                            Poll::Ready(()) => {
                                // Wow, it's been a second already? Ok then...
                                continue;
                            }
                            Poll::Pending => {
                                self.timeout = Some(timeout);
                                return Poll::Pending;
                            }
                        }
                    } else {
                        return Poll::Ready(Err(e.into()));
                    }
                }
            }
        }
    }
}

impl<F: AddrFilter> Accept for FilteredAddrIncoming<F>
where
    F: Unpin,
{
    type Conn = AddrStream;
    type Error = io::Error;

    #[inline]
    fn poll_accept(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
        Poll::Ready(Some(ready!(self.poll_next_(cx))))
    }
}

/// This function defines errors that are per-connection. Which basically
/// means that if we get this error from `accept()` system call it means
/// next connection might be ready to be accepted.
///
/// All other errors will incur a timeout before next `accept()` is performed.
/// The timeout is useful to handle resource exhaustion errors like ENFILE
/// and EMFILE. Otherwise, could enter into tight loop.
fn is_connection_error(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::ConnectionRefused | io::ErrorKind::ConnectionAborted | io::ErrorKind::ConnectionReset
    )
}

/*
struct HyperAcceptor<'a> {
    acceptor: Pin<Box<dyn Stream<Item = Result<TlsStream<TcpStream>, io::Error>> + 'a>>,
}

impl hyper::server::accept::Accept for HyperAcceptor<'_> {
    type Conn = TlsStream<TcpStream>;
    type Error = io::Error;

    fn poll_accept(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
        Pin::new(&mut self.acceptor).poll_next(cx)
    }
}
*/
