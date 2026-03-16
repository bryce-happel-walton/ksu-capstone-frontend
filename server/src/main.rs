use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::{Response, StatusCode, Uri, header};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use include_dir::{Dir, include_dir};
use mime_guess::MimeGuess;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::{Mutex, oneshot};

const WEB: Dir = include_dir!("$OUT_DIR/web");

#[derive(Clone)]
struct AppState {
    shutdown_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

#[tokio::main]
async fn main() {
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let state = AppState {
        shutdown_tx: Arc::new(Mutex::new(Some(shutdown_tx))),
    };

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

async fn shutdown(State(state): State<AppState>) -> &'static str {
    if let Some(tx) = state.shutdown_tx.lock().await.take() {
        let _ = tx.send(());
    }
    "shutting down"
}

async fn serve_embedded(uri: Uri) -> Response<Body> {
    let path = uri.path();

    // default route
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

async fn handle_socket(mut socket: WebSocket) {
    let mut increment = 0;
    let mut boolcrement = false;
    loop {
        let mut data = shared::EspData::default();
        data.hello = "world!".to_owned();
        data.beep = increment;
        data.boop = boolcrement;

        increment += 1;
        boolcrement = !boolcrement;

        let json = serde_json::to_string(&data).unwrap();
        if socket.send(Message::Text(json.into())).await.is_err() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

async fn websocket_handler(web_socket: WebSocketUpgrade) -> impl IntoResponse {
    web_socket
        .on_failed_upgrade(|error| println!("Error upgrading websocket: {}", error))
        .on_upgrade(handle_socket)
}

async fn get_raw_esp_data() -> axum::http::Response<String> {
    match reqwest::get(shared::ESP_DATA_DIR).await {
        Ok(res) => match res.text().await {
            Ok(body) => Response::builder()
                .status(StatusCode::OK)
                .body(body)
                .unwrap(),
            Err(_) => Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body("Failed to read response".to_string())
                .unwrap(),
        },
        Err(_) => Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .body("Failed to reach network".to_string())
            .unwrap(),
    }
}
