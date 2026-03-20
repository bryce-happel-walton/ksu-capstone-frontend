use axum::Router;
use axum::body::Body;
use axum::http::{Response, StatusCode, Uri, header};
use axum::routing::get;
use include_dir::{Dir, include_dir};
use mime_guess::MimeGuess;
use std::net::SocketAddr;

const WEB: Dir = include_dir!("$OUT_DIR/web");

#[tokio::main]
async fn main() {
    let app = Router::new().fallback(get(serve_embedded));

    let listener = tokio::net::TcpListener::bind(&format!("{}:0", shared::SERVER_IP))
        .await
        .unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();

    let url = format!("http://{}/", addr);
    let _ = webbrowser::open(&url);

    axum::serve(listener, app).await.unwrap();
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
