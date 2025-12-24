use crate::events::McpLog;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::StatusCode,
    response::Response,
    routing::get,
    Router,
};
use serde::Deserialize;
use serde_json;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::broadcast;
use tokio_stream::{wrappers::BroadcastStream, StreamExt};

#[derive(Deserialize)]
struct AuthQuery {
    token: Option<String>,
}

pub struct ServerState {
    pub tx: broadcast::Sender<McpLog>,
    pub auth_token: Option<String>,
}

pub async fn start_server(
    tx: broadcast::Sender<McpLog>,
    bind_addr: &str,
    auth_token: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(ServerState { tx, auth_token });

    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .with_state(state);

    let addr: SocketAddr = bind_addr.parse()?;
    
    if let Some(ref token) = auth_token {
        eprintln!("🔒 WebSocket server started with authentication on {}", addr);
        eprintln!("   Connect with: ws://{}?token={}", addr, token);
    } else {
        eprintln!("⚠️  WebSocket server started WITHOUT authentication on {}", addr);
        eprintln!("   For production, use --ws-token flag");
    }

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<AuthQuery>,
    State(state): State<Arc<ServerState>>,
) -> Result<Response, StatusCode> {
    // Validate authentication token if configured
    if let Some(ref expected_token) = state.auth_token {
        match params.token {
            Some(provided_token) if provided_token == *expected_token => {
                // Token matches - allow upgrade
            }
            Some(_) => {
                eprintln!("❌ WebSocket authentication failed: invalid token");
                return Err(StatusCode::UNAUTHORIZED);
            }
            None => {
                eprintln!("❌ WebSocket authentication failed: no token provided");
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
    }

    Ok(ws.on_upgrade(move |socket| websocket_loop(socket, state)))
}

async fn websocket_loop(mut socket: WebSocket, state: Arc<ServerState>) {
    let rx = state.tx.subscribe();
    let mut stream = BroadcastStream::new(rx);

    eprintln!("✅ WebSocket client connected");

    // We only send logs to the client; we ignore messages from the client for now.
    while let Some(Ok(log)) = stream.next().await {
        if let Ok(text) = serde_json::to_string(&log) {
            if socket.send(Message::Text(text)).await.is_err() {
                break;
            }
        }
    }

    eprintln!("❌ WebSocket client disconnected");
}
