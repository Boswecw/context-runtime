//! context-runtime HTTP service entrypoint.

use context_runtime::config::Config;
use context_runtime::http::{AppState, router};
use context_runtime::store::BundleStore;

#[tokio::main]
async fn main() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let cfg = Config::from_env();
    let state = AppState {
        cfg: cfg.clone(),
        store: BundleStore::new(),
    };
    let app = router(state);

    let listener = tokio::net::TcpListener::bind(&cfg.bind_addr)
        .await
        .unwrap_or_else(|e| panic!("failed to bind {}: {e}", cfg.bind_addr));
    tracing::info!(addr = %cfg.bind_addr, "context-runtime listening");

    axum::serve(listener, app)
        .await
        .expect("context-runtime server error");
}
