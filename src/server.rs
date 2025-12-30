use crate::events::McpLog;
use crate::frontend::FrontendAssets;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State, Path,
    },
    http::{StatusCode, HeaderMap},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use mime_guess::from_path;
use serde::Deserialize;
use serde_json;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::{broadcast, RwLock};
use tokio_stream::{wrappers::BroadcastStream, StreamExt};
use std::collections::VecDeque;

#[derive(Deserialize)]
struct AuthQuery {
    token: Option<String>,
}

pub struct ServerState {
    pub tx: broadcast::Sender<McpLog>,
    pub auth_token: Option<String>,
    pub history: RwLock<VecDeque<McpLog>>,
}

pub async fn start_server(
    state: Arc<ServerState>,
    bind_addr: &str,
) -> Result<(), Box<dyn std::error::Error>> {

    let app = Router::new()
        // WebSocket
        .route("/ws", get(websocket_handler))
        // Frontend (index.html + assets)
        .route("/", get(serve_index))
        .route("/*path", get(serve_static))
        .with_state(state.clone());

    let addr: SocketAddr = bind_addr.parse()?;

    if let Some(ref token) = state.auth_token {
        eprintln!("🔒 WebSocket server started with authentication on {}", addr);
        eprintln!("   Connect with: ws://{}?token={}", addr, token);
    } else {
        eprintln!("⚠️  WebSocket server started WITHOUT authentication on {}", addr);
        eprintln!("   For production, use --ws-token flag");
    }

    eprintln!("📊 Dashboard available at: http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

//
// ---------- Frontend handlers ----------
//

async fn serve_index() -> impl IntoResponse {
    serve_asset("index.html")
}

async fn serve_static(
    Path(path): Path<String>,
) -> impl IntoResponse {
    serve_asset(&path)
}

fn serve_asset(path: &str) -> Response {
    let path = path.trim_start_matches('/');

    match FrontendAssets::get(path) {
        Some(asset) => {
            let body = asset.data.into_owned();
            let mime = from_path(path).first_or_octet_stream();

            let mut headers = HeaderMap::new();
            headers.insert(
                axum::http::header::CONTENT_TYPE,
                mime.as_ref().parse().unwrap(),
            );

            (StatusCode::OK, headers, body).into_response()
        }
        None => {
            // SPA fallback → index.html
            if let Some(index) = FrontendAssets::get("index.html") {
                let body = index.data.into_owned();
                (
                    StatusCode::OK,
                    [(axum::http::header::CONTENT_TYPE, "text/html")],
                    body,
                )
                    .into_response()
            } else {
                StatusCode::NOT_FOUND.into_response()
            }
        }
    }
}

//
// ---------- WebSocket ----------
//

async fn websocket_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<AuthQuery>,
    State(state): State<Arc<ServerState>>,
) -> Result<Response, StatusCode> {
    if let Some(ref expected_token) = state.auth_token {
        match params.token {
            Some(provided) if provided == *expected_token => {}
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
    // Replay history
    {
        let hist = state.history.read().await;
        for log in hist.iter() {
            if let Ok(text) = serde_json::to_string(log) {
                if socket.send(Message::Text(text)).await.is_err() {
                    return;
                }
            }
        }
    }

    let rx = state.tx.subscribe();
    let mut stream = BroadcastStream::new(rx);

    eprintln!("✅ WebSocket client connected");

    while let Some(Ok(log)) = stream.next().await {
        if let Ok(text) = serde_json::to_string(&log) {
            if socket.send(Message::Text(text)).await.is_err() {
                break;
            }
        }
    }

    eprintln!("❌ WebSocket client disconnected");
}
