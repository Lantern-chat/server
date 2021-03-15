use std::borrow::Cow;

use futures::{future, ready, Future, Sink, Stream};
use std::pin::Pin;
use std::task::{Context, Poll};

use headers::{
    Connection, HeaderMapExt, SecWebsocketAccept, SecWebsocketKey, SecWebsocketVersion, Upgrade,
};
use http::{Method, StatusCode};
use hyper::{upgrade::OnUpgrade, Body};
use tokio_tungstenite::{
    tungstenite::{
        self,
        protocol::{self, WebSocketConfig},
    },
    WebSocketStream,
};

use super::{Reply, ReplyError, Response, Route};

pub struct Ws {
    config: WebSocketConfig,
    key: SecWebsocketKey,
    on_upgrade: Option<OnUpgrade>,
}

#[derive(Debug, thiserror::Error)]
pub enum WsError {
    #[error("Method Not Allowed")]
    MethodNotAllowed,

    #[error("Missing Upgrade Header")]
    MissingUpgrade,

    #[error("Incorrect Upgrade Header")]
    IncorrectUpgrade,

    #[error("Incorrect WebSocket Version")]
    IncorrectWebSocketVersion,

    #[error("Missing WebSocket Key")]
    MissingWebSocketKey,
}

impl Reply for WsError {
    fn into_response(self) -> Response {
        self.status().into_response()
    }
}

impl ReplyError for WsError {
    fn status(&self) -> StatusCode {
        match self {
            WsError::MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,
            _ => StatusCode::BAD_REQUEST,
        }
    }

    fn into_error_response(self) -> Response {
        self.into_response()
    }
}

impl Ws {
    /// Creates a Websocket response.
    ///
    /// The yielded `Ws` is used to finish the websocket upgrade.
    ///
    /// # Note
    ///
    /// This filter combines multiple filters internally, so you don't need them:
    ///
    /// - Method must be `GET`
    /// - Header `connection` must be `upgrade`
    /// - Header `upgrade` must be `websocket`
    /// - Header `sec-websocket-version` must be `13`
    /// - Header `sec-websocket-key` must be set.
    ///
    /// If the filters are met, yields a `Ws`. Calling `Ws::on_upgrade` will
    /// return a reply with:
    ///
    /// - Status of `101 Switching Protocols`
    /// - Header `connection: upgrade`
    /// - Header `upgrade: websocket`
    /// - Header `sec-websocket-accept` with the hash value of the received key.
    pub fn new(mut route: Route) -> Result<Ws, WsError> {
        if route.req.method() != &Method::GET {
            return Err(WsError::MethodNotAllowed);
        }

        let headers = route.req.headers();

        match headers.typed_get::<Connection>() {
            Some(header) if header.contains("upgrade") => {}
            _ => return Err(WsError::MissingUpgrade),
        }

        match headers.typed_get::<Upgrade>() {
            Some(upgrade) if upgrade == Upgrade::websocket() => {}
            _ => return Err(WsError::IncorrectUpgrade),
        }

        match headers.typed_get::<SecWebsocketVersion>() {
            Some(SecWebsocketVersion::V13) => {}
            _ => return Err(WsError::IncorrectWebSocketVersion),
        }

        let key: SecWebsocketKey = match headers.typed_get() {
            Some(key) => key,
            None => return Err(WsError::MissingWebSocketKey),
        };

        let on_upgrade = route.req.extensions_mut().remove::<OnUpgrade>();

        Ok(Ws {
            config: WebSocketConfig::default(),
            key,
            on_upgrade,
        })
    }

    /// Set the size of the internal message send queue.
    pub fn max_send_queue(mut self, max: usize) -> Self {
        self.config.max_send_queue = Some(max);
        self
    }

    /// Set the maximum message size (defaults to 64 megabytes)
    pub fn max_message_size(mut self, max: usize) -> Self {
        self.config.max_message_size = Some(max);
        self
    }

    /// Set the maximum frame size (defaults to 16 megabytes)
    pub fn max_frame_size(mut self, max: usize) -> Self {
        self.config.max_frame_size = Some(max);
        self
    }

    pub fn on_upgrade<F>(self, func: F) -> impl Reply
    where
        F: FnOnce(WebSocket) + Send + 'static,
    {
        WsReply {
            ws: self,
            on_upgrade: func,
        }
    }
}

struct WsReply<F> {
    ws: Ws,
    on_upgrade: F,
}

impl<F> Reply for WsReply<F>
where
    F: FnOnce(WebSocket) + Send + 'static,
{
    fn into_response(self) -> Response {
        if let Some(on_upgrade) = self.ws.on_upgrade {
            let on_upgrade_cb = self.on_upgrade;
            let config = self.ws.config;

            tokio::spawn(async move {
                match on_upgrade.await {
                    Err(e) => log::error!("ws upgrade error: {}", e),
                    Ok(upgraded) => {
                        log::trace!("websocket upgrade complete");
                        let socket =
                            WebSocket::from_raw_socket(upgraded, protocol::Role::Server, config)
                                .await;

                        on_upgrade_cb(socket);
                    }
                }
            });
        } else {
            log::warn!("ws couldn't be upgraded since no upgrade state was present");
        }

        StatusCode::SWITCHING_PROTOCOLS
            .with_header(Connection::upgrade())
            .with_header(Upgrade::websocket())
            .with_header(SecWebsocketAccept::from(self.ws.key))
            .into_response()
    }
}

pub struct WebSocket {
    inner: WebSocketStream<hyper::upgrade::Upgraded>,
}

/// A websocket `Stream` and `Sink`, provided to `ws` filters.
///
/// Ping messages sent from the client will be handled internally by replying with a Pong message.
/// Close messages need to be handled explicitly: usually by closing the `Sink` end of the
/// `WebSocket`.
impl WebSocket {
    pub(crate) async fn from_raw_socket(
        upgraded: hyper::upgrade::Upgraded,
        role: protocol::Role,
        config: protocol::WebSocketConfig,
    ) -> Self {
        WebSocket {
            inner: WebSocketStream::from_raw_socket(upgraded, role, Some(config)).await,
        }
    }

    /// Gracefully close this websocket.
    pub async fn close(mut self) -> Result<(), tungstenite::Error> {
        future::poll_fn(|cx| Pin::new(&mut self).poll_close(cx)).await
    }
}

impl Stream for WebSocket {
    type Item = Result<Message, tungstenite::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        match ready!(Pin::new(&mut self.inner).poll_next(cx)) {
            Some(Ok(item)) => Poll::Ready(Some(Ok(Message { inner: item }))),
            Some(Err(e)) => {
                tracing::debug!("websocket poll error: {}", e);
                Poll::Ready(Some(Err(e)))
            }
            None => {
                tracing::trace!("websocket closed");
                Poll::Ready(None)
            }
        }
    }
}

impl Sink<Message> for WebSocket {
    type Error = tungstenite::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match ready!(Pin::new(&mut self.inner).poll_ready(cx)) {
            Ok(()) => Poll::Ready(Ok(())),
            Err(e) => Poll::Ready(Err(e)),
        }
    }

    fn start_send(mut self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        match Pin::new(&mut self.inner).start_send(item.inner) {
            Ok(()) => Ok(()),
            Err(e) => {
                tracing::debug!("websocket start_send error: {}", e);
                Err(e)
            }
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        match ready!(Pin::new(&mut self.inner).poll_flush(cx)) {
            Ok(()) => Poll::Ready(Ok(())),
            Err(e) => Poll::Ready(Err(e)),
        }
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        match ready!(Pin::new(&mut self.inner).poll_close(cx)) {
            Ok(()) => Poll::Ready(Ok(())),
            Err(err) => {
                tracing::debug!("websocket close error: {}", err);
                Poll::Ready(Err(err))
            }
        }
    }
}

/// A WebSocket message.
///
/// This will likely become a `non-exhaustive` enum in the future, once that
/// language feature has stabilized.
#[derive(Eq, PartialEq, Clone)]
pub struct Message {
    inner: protocol::Message,
}

impl Message {
    /// Construct a new Text `Message`.
    pub fn text<S: Into<String>>(s: S) -> Message {
        Message {
            inner: protocol::Message::text(s),
        }
    }

    /// Construct a new Binary `Message`.
    pub fn binary<V: Into<Vec<u8>>>(v: V) -> Message {
        Message {
            inner: protocol::Message::binary(v),
        }
    }

    /// Construct a new Ping `Message`.
    pub fn ping<V: Into<Vec<u8>>>(v: V) -> Message {
        Message {
            inner: protocol::Message::Ping(v.into()),
        }
    }

    /// Construct a new Pong `Message`.
    ///
    /// Note that one rarely needs to manually construct a Pong message because the underlying tungstenite socket
    /// automatically responds to the Ping messages it receives. Manual construction might still be useful in some cases
    /// like in tests or to send unidirectional heartbeats.
    pub fn pong<V: Into<Vec<u8>>>(v: V) -> Message {
        Message {
            inner: protocol::Message::Pong(v.into()),
        }
    }

    /// Construct the default Close `Message`.
    pub fn close() -> Message {
        Message {
            inner: protocol::Message::Close(None),
        }
    }

    /// Construct a Close `Message` with a code and reason.
    pub fn close_with(code: impl Into<u16>, reason: impl Into<Cow<'static, str>>) -> Message {
        Message {
            inner: protocol::Message::Close(Some(protocol::frame::CloseFrame {
                code: protocol::frame::coding::CloseCode::from(code.into()),
                reason: reason.into(),
            })),
        }
    }

    /// Returns true if this message is a Text message.
    pub fn is_text(&self) -> bool {
        self.inner.is_text()
    }

    /// Returns true if this message is a Binary message.
    pub fn is_binary(&self) -> bool {
        self.inner.is_binary()
    }

    /// Returns true if this message a is a Close message.
    pub fn is_close(&self) -> bool {
        self.inner.is_close()
    }

    /// Returns true if this message is a Ping message.
    pub fn is_ping(&self) -> bool {
        self.inner.is_ping()
    }

    /// Returns true if this message is a Pong message.
    pub fn is_pong(&self) -> bool {
        self.inner.is_pong()
    }

    /// Try to get the close frame (close code and reason)
    pub fn close_frame(&self) -> Option<(u16, &str)> {
        if let protocol::Message::Close(Some(ref close_frame)) = self.inner {
            Some((close_frame.code.into(), close_frame.reason.as_ref()))
        } else {
            None
        }
    }

    /// Try to get a reference to the string text, if this is a Text message.
    pub fn to_str(&self) -> Result<&str, ()> {
        match self.inner {
            protocol::Message::Text(ref s) => Ok(s),
            _ => Err(()),
        }
    }

    /// Return the bytes of this message, if the message can contain data.
    pub fn as_bytes(&self) -> &[u8] {
        match self.inner {
            protocol::Message::Text(ref s) => s.as_bytes(),
            protocol::Message::Binary(ref v) => v,
            protocol::Message::Ping(ref v) => v,
            protocol::Message::Pong(ref v) => v,
            protocol::Message::Close(_) => &[],
        }
    }

    /// Destructure this message into binary data.
    pub fn into_bytes(self) -> Vec<u8> {
        self.inner.into_data()
    }
}

use std::fmt;
impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl Into<Vec<u8>> for Message {
    fn into(self) -> Vec<u8> {
        self.into_bytes()
    }
}
