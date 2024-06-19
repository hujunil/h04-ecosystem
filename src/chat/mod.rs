use anyhow::Result;
use core::fmt;
use dashmap::DashMap;
use futures::{stream::SplitStream, SinkExt, StreamExt};
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    net::{TcpListener, TcpStream, ToSocketAddrs},
    sync::mpsc,
};
use tokio_util::codec::{Framed, LinesCodec};
use tracing::{info, warn};

const MAX_MESSAGES: usize = 128;

#[derive(Debug, Default)]
struct State {
    peers: DashMap<SocketAddr, mpsc::Sender<Arc<Message>>>,
}

struct Peer {
    username: String,
    stream: SplitStream<Framed<TcpStream, LinesCodec>>,
}

#[derive(Debug)]
enum Message {
    UserJoined(String),
    UserLeft(String),
    Chat { sender: String, content: String },
}

pub struct App;

impl App {
    pub async fn run(addr: impl ToSocketAddrs) -> Result<()> {
        let listener = TcpListener::bind(addr).await?;
        info!("Listening on: {:?}", listener.local_addr()?);

        let state = Arc::new(State::default());

        loop {
            let (stream, peer_addr) = listener.accept().await?;
            info!("Accepted connection from: {}", peer_addr);
            let state_cloned = state.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(stream, peer_addr, state_cloned).await {
                    info!("Error: {:?}", e);
                }
            });
        }
    }

    async fn handle_connection(
        stream: TcpStream,
        addr: SocketAddr,
        state: Arc<State>,
    ) -> Result<()> {
        let mut stream = Framed::new(stream, LinesCodec::new());
        // 当有新的连接时，向客户端发送消息，提示输入用户名
        stream.send("Enter your username:").await?;
        let username = match stream.next().await {
            Some(Ok(username)) => username,
            Some(Err(e)) => return Err(e.into()),
            None => return Ok(()),
        };

        let mut peer = state.add(addr, username, stream).await;
        let message = Arc::new(Message::user_joined(&peer.username));
        info!("{}", message);
        state.broadcast(addr, message).await;

        // 不停的接收客户端的消息，并广播给其他客户端
        while let Some(line) = peer.stream.next().await {
            let line = match line {
                Ok(line) => line,
                Err(e) => {
                    warn!("Failed to read line from {}: {}", addr, e);
                    break;
                }
            };
            let message = Arc::new(Message::chat(&peer.username, line));
            info!("{}", message);
            state.broadcast(addr, message).await;
        }

        // 当客户端断开连接或读取line失败时，从state中移除该客户端
        // User left
        state.peers.remove(&addr);

        // 广播用户离开的消息
        let message = Arc::new(Message::user_left(&peer.username));
        info!("{}", message);
        state.broadcast(addr, message).await;

        Ok(())
    }
}

impl State {
    async fn broadcast(&self, addr: SocketAddr, message: Arc<Message>) {
        for peer in self.peers.iter() {
            if peer.key() == &addr {
                continue;
            }
            if let Err(e) = peer.value().send(message.clone()).await {
                warn!("Failed to send message to {}: {}", peer.key(), e);
                // if send failed, peer might be gone, remove peer from state
                self.peers.remove(peer.key());
            }
        }
    }
    async fn add(
        &self,
        addr: SocketAddr,
        username: String,
        stream: Framed<TcpStream, LinesCodec>,
    ) -> Peer {
        let (tx, mut rx) = mpsc::channel(MAX_MESSAGES);
        self.peers.insert(addr, tx);
        let (mut stream_sender, stream_receiver) = stream.split();

        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if let Err(e) = stream_sender.send(message.to_string()).await {
                    warn!("Failed to send message to {}: {}", addr, e);
                    break;
                }
            }
        });

        Peer {
            username,
            stream: stream_receiver,
        }
    }
}

impl Message {
    fn user_joined(username: &str) -> Self {
        let content = format!("{} has joined the chat", username);
        Self::UserJoined(content)
    }

    fn user_left(username: &str) -> Self {
        let content = format!("{} has left the chat", username);
        Self::UserLeft(content)
    }

    fn chat(sender: impl Into<String>, content: impl Into<String>) -> Self {
        Self::Chat {
            sender: sender.into(),
            content: content.into(),
        }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::UserJoined(username) => write!(f, "{} joined the chat", username),
            Message::UserLeft(username) => write!(f, "{} left the chat", username),
            Message::Chat { sender, content } => write!(f, "{}: {}", sender, content),
        }
    }
}
