use anyhow::Error;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug)]
pub(crate) struct StatusHandler {
    ready_token: CancellationToken,
    prom_handler: PrometheusHandle,
}

pub(crate) enum Readiness {
    Ready,
    NotReady,
}

impl IntoResponse for Readiness {
    fn into_response(self) -> axum::response::Response {
        match self {
            Readiness::Ready => Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/plain")
                .body(axum::body::Body::from("Ok"))
                .unwrap(),
            Readiness::NotReady => Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "text/plain")
                .body(axum::body::Body::from("NotReady"))
                .unwrap(),
        }
    }
}
impl StatusHandler {
    pub fn try_new(ready_token: CancellationToken) -> Result<StatusHandler, Error> {
        Ok(Self {
            ready_token,
            prom_handler: setup_metrics()?,
        })
    }
    pub fn ready(&self) -> Readiness {
        if self.ready_token.is_cancelled() {
            Readiness::NotReady
        } else {
            Readiness::Ready
        }
    }

    pub async fn metrics(&self) -> String {
        self.prom_handler.render()
    }
}

fn setup_metrics() -> Result<PrometheusHandle, Error> {
    Ok(PrometheusBuilder::new().install_recorder()?)
}

pub fn status_router(state: StatusHandler) -> Result<Router, Error> {
    Ok(Router::new()
        .route("/metrics", get(metrics))
        .route("/readyz", get(readyz))
        .with_state(Arc::new(state)))
}

async fn readyz(State(handler): State<Arc<StatusHandler>>) -> impl IntoResponse {
    handler.ready()
}

async fn metrics(State(handler): State<Arc<StatusHandler>>) -> String {
    handler.metrics().await
}
