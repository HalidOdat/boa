use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::oneshot;

/// A simple HTTP server for serving WPT test files.
pub struct WptServer {
    addr: SocketAddr,
    shutdown_tx: Option<oneshot::Sender<()>>,
    _handle: std::thread::JoinHandle<()>,
}

impl WptServer {
    /// Start a new WPT test server on the given port.
    pub fn start(
        wpt_root: impl Into<PathBuf>,
        port: u16,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let wpt_root = Arc::new(wpt_root.into());
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();

        let handle = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            runtime.block_on(async move {
                let listener = match tokio::net::TcpListener::bind(addr).await {
                    Ok(l) => l,
                    Err(e) => {
                        ready_tx.send(Err(format!("Failed to bind: {}", e))).ok();
                        return;
                    }
                };

                eprintln!("WPT server listening on http://{}", addr);
                ready_tx.send(Ok(())).ok();

                let root = wpt_root.clone();
                let mut shutdown_rx = shutdown_rx;

                loop {
                    tokio::select! {
                        Ok((stream, _)) = listener.accept() => {
                            let root = root.clone();
                            tokio::task::spawn_local(async move {
                                if let Err(e) = handle_connection(stream, root).await {
                                    eprintln!("Error handling connection: {}", e);
                                }
                            });
                        }
                        _ = &mut shutdown_rx => {
                            eprintln!("WPT server shutting down");
                            break;
                        }
                    }
                }
            });
        });

        // Wait for the server to be ready
        if let Ok(Err(e)) = ready_rx.recv() {
            return Err(e.into());
        }

        Ok(Self {
            addr,
            shutdown_tx: Some(shutdown_tx),
            _handle: handle,
        })
    }

    /// Get the server host string (e.g., "127.0.0.1:8000").
    pub fn host(&self) -> String {
        format!("{}:{}", self.addr.ip(), self.addr.port())
    }
}

impl Drop for WptServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    wpt_root: Arc<PathBuf>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use http_body_util::Full;
    use hyper::body::Bytes;
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use hyper::{Request, Response, StatusCode};
    use hyper_util::rt::TokioIo;

    let io = TokioIo::new(stream);
    let wpt_root_clone = wpt_root.clone();

    let service = service_fn(move |req: Request<hyper::body::Incoming>| {
        let wpt_root = wpt_root_clone.clone();
        async move {
            let path = req.uri().path();
            let file_path = wpt_root.join(path.trim_start_matches('/'));

            // Security: ensure the path is still under wpt_root
            let canonical_file = match file_path.canonicalize() {
                Ok(p) => p,
                Err(_) => {
                    return Ok::<_, hyper::Error>(
                        Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .body(Full::new(Bytes::from("File not found")))
                            .unwrap(),
                    );
                }
            };

            let canonical_root = wpt_root.canonicalize().unwrap();
            if !canonical_file.starts_with(&canonical_root) {
                return Ok(Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .body(Full::new(Bytes::from("Forbidden")))
                    .unwrap());
            }

            // Read and serve the file
            match tokio::fs::read(&canonical_file).await {
                Ok(contents) => {
                    let content_type = get_content_type(&canonical_file);
                    Ok(Response::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type", content_type)
                        .header("Access-Control-Allow-Origin", "*")
                        .body(Full::new(Bytes::from(contents)))
                        .unwrap())
                }
                Err(_) => Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Full::new(Bytes::from("File not found")))
                    .unwrap()),
            }
        }
    });

    http1::Builder::new().serve_connection(io, service).await?;

    Ok(())
}

/// Get the MIME type for a file based on its extension.
fn get_content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|s| s.to_str()) {
        Some("js") => "application/javascript",
        Some("json") => "application/json",
        Some("html") => "text/html",
        Some("css") => "text/css",
        Some("txt") => "text/plain",
        Some("xml") => "application/xml",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        _ => "application/octet-stream",
    }
}
