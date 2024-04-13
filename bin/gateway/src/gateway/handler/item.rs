use tokio::sync::broadcast::error::RecvError;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;

use sdk::models::gateway::message::ClientMsg;

use super::Event;

pub enum Item {
    Event(Result<Event, EventError>),
    Msg(Result<ClientMsg, MessageIncomingError>),
    Ping,
    MissedHeartbeat,
}

#[derive(Debug, thiserror::Error)]
pub enum EventError {
    #[error(transparent)]
    Broadcast(#[from] RecvError),

    #[error(transparent)]
    Oneshot(#[from] tokio::sync::oneshot::error::RecvError),
}

impl From<BroadcastStreamRecvError> for EventError {
    fn from(e: BroadcastStreamRecvError) -> Self {
        EventError::Broadcast(match e {
            BroadcastStreamRecvError::Lagged(l) => RecvError::Lagged(l),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MessageOutgoingError {
    #[error("Socket Closed")]
    SocketClosed,
}

#[derive(Debug, thiserror::Error)]
pub enum MessageIncomingError {
    #[error("Tungstentite Error: {0}")]
    TungsteniteError(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("Socket Closed")]
    SocketClosed,

    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON Parse Error: {0}")]
    JsonParseError(#[from] serde_json::Error),

    #[error("Cbor Parse Error: {0}")]
    CborParseError(#[from] ciborium::de::Error<std::io::Error>),

    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),
}

impl MessageIncomingError {
    /// Returns true if the error is due to the socket being closed
    #[rustfmt::skip]
    pub fn is_close(&self) -> bool {
        use tokio_tungstenite::tungstenite::error::{Error, ProtocolError};

        matches!(self,
            Self::SocketClosed |
            Self::TungsteniteError(
                | Error::AlreadyClosed
                | Error::ConnectionClosed
                | Error::Protocol(ProtocolError::ResetWithoutClosingHandshake)
            )
        )
    }
}
