use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::{Response, StatusCode, Uri, header};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use futures_util::StreamExt;
use include_dir::{Dir, include_dir};
use mime_guess::MimeGuess;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::{Mutex, broadcast, oneshot};
use tokio_tungstenite::connect_async;

const WEB: Dir = include_dir!("$OUT_DIR/web");

#[derive(Clone)]
struct AppState {
    shutdown_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    esp_tx: broadcast::Sender<shared::TestData>,
}

#[tokio::main]
async fn main() {
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let (esp_tx, _) = broadcast::channel::<shared::TestData>(16);

    let state = AppState {
        shutdown_tx: Arc::new(Mutex::new(Some(shutdown_tx))),
        esp_tx: esp_tx.clone(),
    };

    tokio::spawn(esp_ws_task(esp_tx));

    let app = Router::new()
        .route("/__shutdown", post(shutdown))
        .route(
            &format!("/{}", shared::WEB_SOCKET_DIR),
            get(websocket_handler),
        )
        .fallback(get(serve_embedded))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();

    let url = format!("http://{}/", addr);
    let _ = webbrowser::open(&url);

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = shutdown_rx.await;
        })
        .await
        .unwrap();
}

async fn esp_ws_task(tx: broadcast::Sender<shared::TestData>) {
    loop {
        eprintln!("Connecting to ESP...");
        match connect_async(shared::ESP_DATA_DIR).await {
            Ok((mut ws, _)) => {
                eprintln!("Connected to ESP");
                while let Some(msg) = ws.next().await {
                    if let Ok(tokio_tungstenite::tungstenite::Message::Binary(bytes)) = msg {
                        // We can just pass the bytes directly, but it is easier to follow the types this way
                        shared::TestData::from_bytes(&bytes).map(|data| tx.send(data));
                    }
                }
                eprintln!("ESP disconnected");
            }
            Err(e) => {
                eprintln!("ESP WebSocket connection failed: {e}, retrying in 2s");
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }
}

async fn shutdown(State(state): State<AppState>) -> &'static str {
    if let Some(tx) = state.shutdown_tx.lock().await.take() {
        let _ = tx.send(());
    }
    "shutting down"
}

async fn serve_embedded(uri: Uri) -> Response<Body> {
    let path = uri.path();

    let file_path = if path == "/" {
        "index.html"
    } else {
        &path[1..]
    };

    match WEB.get_file(file_path) {
        Some(file) => {
            let contents = file.contents();
            let mime = MimeGuess::from_path(file_path).first_or_octet_stream();
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(Body::from(contents.to_vec()))
                .unwrap()
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(Body::from("404"))
            .unwrap(),
    }
}

async fn handle_socket(mut socket: WebSocket, esp_tx: broadcast::Sender<shared::TestData>) {
    let mut rx = esp_tx.subscribe();
    loop {
        match rx.recv().await {
            Ok(data) => {
                if socket
                    .send(Message::Binary(data.to_bytes().into()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(_) => break,
        }
    }
}

async fn websocket_handler(
    web_socket: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    web_socket
        .on_failed_upgrade(|error| println!("Error upgrading websocket: {}", error))
        .on_upgrade(move |socket| handle_socket(socket, state.esp_tx))
}
