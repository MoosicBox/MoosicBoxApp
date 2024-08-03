#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]

use std::future::Future;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use futures_channel::mpsc::UnboundedSender;
use futures_util::{future, pin_mut, StreamExt as _};
use thiserror::Error;
use tokio::select;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::time::sleep;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Error, Message},
};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Error)]
pub enum SendBytesError {
    #[error("Unknown {0:?}")]
    Unknown(String),
}

#[derive(Debug, Error)]
pub enum SendMessageError {
    #[error("Unknown {0:?}")]
    Unknown(String),
}

pub enum WsMessage {
    TextMessage(String),
    Message(Bytes),
    Ping,
}

#[derive(Debug, Error)]
pub enum WebsocketSendError {
    #[error("Unknown: {0}")]
    Unknown(String),
}

#[async_trait]
pub trait WebsocketSender: Send + Sync {
    async fn send(&self, data: &str) -> Result<(), WebsocketSendError>;
    async fn ping(&self) -> Result<(), WebsocketSendError>;
}

impl core::fmt::Debug for dyn WebsocketSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{WebsocketSender}}")
    }
}

#[derive(Debug, Error)]
pub enum CloseError {
    #[error("Unknown {0:?}")]
    Unknown(String),
}

#[derive(Clone)]
pub struct WsHandle {
    sender: Arc<RwLock<Option<UnboundedSender<WsMessage>>>>,
    cancellation_token: CancellationToken,
}

impl WsHandle {
    pub async fn close(&self) -> Result<(), CloseError> {
        self.cancellation_token.cancel();

        Ok(())
    }
}

#[async_trait]
impl WebsocketSender for WsHandle {
    async fn send(&self, data: &str) -> Result<(), WebsocketSendError> {
        if let Some(sender) = self.sender.read().unwrap().as_ref() {
            sender
                .unbounded_send(WsMessage::TextMessage(data.to_string()))
                .map_err(|e| WebsocketSendError::Unknown(e.to_string()))?;
        }
        Ok(())
    }

    async fn ping(&self) -> Result<(), WebsocketSendError> {
        if let Some(sender) = self.sender.read().unwrap().as_ref() {
            sender
                .unbounded_send(WsMessage::Ping)
                .map_err(|e| WebsocketSendError::Unknown(e.to_string()))?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct WsClient {
    url: String,
    sender: Arc<RwLock<Option<UnboundedSender<WsMessage>>>>,
    cancellation_token: CancellationToken,
}

impl WsClient {
    pub fn new(url: String) -> (Self, WsHandle) {
        let sender = Arc::new(RwLock::new(None));
        let cancellation_token = CancellationToken::new();
        let handle = WsHandle {
            sender: sender.clone(),
            cancellation_token: cancellation_token.clone(),
        };

        (
            Self {
                url,
                sender: sender.clone(),
                cancellation_token: cancellation_token.clone(),
            },
            handle,
        )
    }

    pub fn with_cancellation_token(mut self, token: CancellationToken) -> Self {
        self.cancellation_token = token;
        self
    }

    async fn message_handler(
        tx: Sender<WsMessage>,
        m: Message,
    ) -> Result<(), SendError<WsMessage>> {
        log::trace!("Message from ws server: {m:?}");
        tx.send(match m {
            Message::Text(m) => WsMessage::TextMessage(m),
            Message::Binary(m) => WsMessage::Message(Bytes::from(m)),
            Message::Ping(_m) => WsMessage::Ping,
            Message::Pong(_m) => {
                log::debug!("Received pong");
                return Ok(());
            }
            Message::Close(_m) => unimplemented!(),
            Message::Frame(_m) => unimplemented!(),
        })
        .await
    }

    pub fn start(
        &mut self,
        client_id: Option<String>,
        signature_token: Option<String>,
    ) -> Receiver<WsMessage> {
        self.start_handler(client_id, signature_token, Self::message_handler)
    }

    fn start_handler<T, O>(
        &mut self,
        client_id: Option<String>,
        signature_token: Option<String>,
        handler: fn(sender: Sender<T>, m: Message) -> O,
    ) -> Receiver<T>
    where
        T: Send + 'static,
        O: Future<Output = Result<(), SendError<T>>> + Send + 'static,
    {
        let (tx, rx) = channel(1024);

        let url = self.url.clone();
        let sender_arc = self.sender.clone();
        let cancellation_token = self.cancellation_token.clone();

        moosicbox_task::spawn("ws", async move {
            let mut just_retried = false;

            loop {
                let close_token = CancellationToken::new();

                let (txf, rxf) = futures_channel::mpsc::unbounded();

                sender_arc.write().unwrap().replace(txf.clone());

                let client_id_param = if let Some(id) = &client_id {
                    format!("?clientId={id}")
                } else {
                    "".to_string()
                };
                let signature_token_param = if let Some(token) = &signature_token {
                    format!("&signature={token}")
                } else {
                    "".to_string()
                };
                log::debug!("Connecting to websocket...");
                match select!(
                    resp = connect_async(format!("{url}{client_id_param}{signature_token_param}")) => resp,
                    _ = cancellation_token.cancelled() => {
                        log::debug!("Cancelling connect");
                        break;
                    }
                ) {
                    Ok((ws_stream, _)) => {
                        log::debug!("WebSocket handshake has been successfully completed");

                        if just_retried {
                            log::info!("WebSocket successfully reconnected");
                            just_retried = false;
                        }

                        let (write, read) = ws_stream.split();

                        let ws_writer = rxf
                            .map(|message| match message {
                                WsMessage::TextMessage(message) => {
                                    log::debug!("Sending text packet from request",);
                                    Ok(Message::Text(message))
                                }
                                WsMessage::Message(bytes) => {
                                    log::debug!("Sending packet from request",);
                                    Ok(Message::Binary(bytes.to_vec()))
                                }
                                WsMessage::Ping => {
                                    log::trace!("Sending ping");
                                    Ok(Message::Ping(vec![]))
                                }
                            })
                            .forward(write);

                        let ws_reader = read.for_each(|m| async {
                            let m = match m {
                                Ok(m) => m,
                                Err(e) => {
                                    log::error!("Send Loop error: {:?}", e);
                                    close_token.cancel();
                                    return;
                                }
                            };

                            moosicbox_task::spawn("ws: Process WS message", {
                                let tx = tx.clone();
                                let close_token = close_token.clone();

                                async move {
                                    if let Err(e) = handler(tx.clone(), m).await {
                                        log::error!("Handler Send Loop error: {e:?}");
                                        close_token.cancel();
                                    }
                                }
                            });
                        });

                        let pinger = moosicbox_task::spawn("ws: pinger", {
                            let txf = txf.clone();
                            let close_token = close_token.clone();
                            let cancellation_token = cancellation_token.clone();

                            async move {
                                loop {
                                    select!(
                                        _ = close_token.cancelled() => { break; }
                                        _ = cancellation_token.cancelled() => { break; }
                                        _ = tokio::time::sleep(std::time::Duration::from_millis(5000)) => {
                                            log::trace!("Sending ping to server");
                                            if let Err(e) = txf.unbounded_send(WsMessage::Ping) {
                                                log::error!("Pinger Send Loop error: {e:?}");
                                                close_token.cancel();
                                                break;
                                            }
                                        }
                                    );
                                }
                            }
                        });

                        pin_mut!(ws_writer, ws_reader);
                        select!(
                            _ = close_token.cancelled() => {}
                            _ = cancellation_token.cancelled() => {}
                            _ = future::select(ws_writer, ws_reader) => {}
                        );
                        if !close_token.is_cancelled() {
                            close_token.cancel();
                        }
                        log::debug!("start_handler: Waiting for pinger to finish...");
                        if let Err(e) = pinger.await {
                            log::warn!("start_handler: Pinger failed to finish: {e:?}");
                        }
                        log::info!("WebSocket connection closed");
                    }
                    Err(err) => match err {
                        Error::Http(response) => {
                            if let Ok(body) =
                                std::str::from_utf8(response.body().as_ref().unwrap_or(&vec![]))
                            {
                                log::error!("body: {}", body);
                            } else {
                                log::error!("body: (unable to get body)");
                            }
                        }
                        _ => log::error!("Failed to connect to websocket server: {err:?}"),
                    },
                }

                if just_retried {
                    select!(
                        _ = sleep(Duration::from_millis(5000)) => {}
                        _ = cancellation_token.cancelled() => {
                            log::debug!("Cancelling retry");
                            break;
                        }
                    );
                } else {
                    just_retried = true;
                }
            }

            log::debug!("Handler closed");
        });

        rx
    }
}
