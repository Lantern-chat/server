#![allow(unused)]

use async_stream::stream;
use core::task::{Context, Poll};
use futures::{future::TryFutureExt, stream::Stream};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use std::net::{SocketAddr, TcpListener as StdTcpListener};
use std::time::Duration;
use tokio::time::Sleep;
//use rustls::internal::pemfile;
use std::pin::Pin;
use std::vec::Vec;
use std::{env, fs, io, sync};
use tokio::net::{TcpListener, TcpStream};
//use tokio_rustls::server::TlsStream;
//use tokio_rustls::TlsAcceptor;

pub struct AddrIncoming {
    addr: SocketAddr,
    listener: TcpListener,
    sleep_on_errors: bool,
    tcp_keepalive_timeout: Option<Duration>,
    tcp_nodelay: bool,
    timeout: Option<Pin<Box<Sleep>>>,
}

impl AddrIncoming {
    fn from_std(std_listener: StdTcpListener) -> Self {
        unimplemented!()
    }
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
