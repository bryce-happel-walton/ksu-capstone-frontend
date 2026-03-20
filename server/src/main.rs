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

type JPEGVec = Vec<u8>;

#[derive(Clone)]
struct AppState {
    shutdown_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    esp_tx: broadcast::Sender<shared::TestData>,
    image_tx: broadcast::Sender<JPEGVec>,
}

#[tokio::main]
async fn main() {
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let (esp_tx, _) = broadcast::channel::<shared::TestData>(16);
    let (image_tx, _) = broadcast::channel::<JPEGVec>(16);

    let state = AppState {
        shutdown_tx: Arc::new(Mutex::new(Some(shutdown_tx))),
        esp_tx: esp_tx.clone(),
        image_tx: image_tx.clone(),
    };

    tokio::spawn(esp_ws_test_task(esp_tx));
    tokio::spawn(esp_ws_camera_stream_task(image_tx));

    let app = Router::new()
        .route("/__shutdown", post(shutdown))
        .route(
            &format!("/{}", shared::SERVER_WS_TEST_DATA_DIR),
            get(ws_test_data_handler),
        )
        .route(
            &format!("/{}", shared::SERVER_WS_IMAGE_STREAM_DIR),
            get(ws_image_stream_handler),
        )
        .fallback(get(serve_embedded))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&format!("{}:0", shared::SERVER_IP))
        .await
        .unwrap();
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

async fn esp_ws_test_task(tx: broadcast::Sender<shared::TestData>) {
    loop {
        eprintln!("Connecting to ESP test data socket...");
        match connect_async(&format!(
            "ws://{}/{}",
            shared::ESP_IP,
            shared::cstr_to_str(shared::bindings::TEST_DATA_URI)
        ))
        .await
        {
            Ok((mut ws, _)) => {
                eprintln!("Connected to ESP test data socket");

                while let Some(msg) = ws.next().await {
                    if let Ok(tokio_tungstenite::tungstenite::Message::Binary(bytes)) = msg {
                        // We can just pass the bytes directly, but it is easier to follow the types this way
                        shared::TestData::from_bytes(&bytes).map(|data| tx.send(data));
                    }
                }
            }
            Err(e) => {
                eprintln!("ESP test data socket connection failed: {e}, retrying in 2s");
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }
}

async fn esp_ws_camera_stream_task(tx: broadcast::Sender<JPEGVec>) {
    loop {
        eprintln!("Connecting to ESP image stream socket...");
        match connect_async(&format!(
            "ws://{}/{}",
            shared::ESP_IP,
            shared::cstr_to_str(shared::bindings::IMAGE_STREAM_URI)
        ))
        .await
        {
            Ok((mut ws, _)) => {
                eprintln!("Connected to ESP image stream socket");
                while let Some(msg) = ws.next().await {
                    if let Ok(tokio_tungstenite::tungstenite::Message::Binary(bytes)) = msg {
                        let _ = tx.send(bytes.to_vec());
                    }
                }
            }
            Err(e) => {
                eprintln!("ESP image stream socket connection failed: {e}, retrying in 2s");
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

async fn handle_test_data_socket(
    mut socket: WebSocket,
    esp_tx: broadcast::Sender<shared::TestData>,
) {
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

async fn handle_image_stream_socket(mut socket: WebSocket, image_tx: broadcast::Sender<JPEGVec>) {
    let mut rx = image_tx.subscribe();
    loop {
        match rx.recv().await {
            Ok(data) => {
                if socket.send(Message::Binary(data.into())).await.is_err() {
                    break;
                }
            }
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(_) => break,
        }
    }
}

async fn ws_test_data_handler(
    web_socket: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    web_socket
        .on_failed_upgrade(|error| println!("Error upgrading test_data websocket: {}", error))
        .on_upgrade(move |socket| handle_test_data_socket(socket, state.esp_tx))
}

async fn ws_image_stream_handler(
    web_socket: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    web_socket
        .on_failed_upgrade(|error| println!("Error upgrading image stream websocket: {}", error))
        .on_upgrade(move |socket| handle_image_stream_socket(socket, state.image_tx))
}
