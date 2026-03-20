use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::{Response, StatusCode, Uri, header};
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
