use std::time::Duration;

use axum::error_handling::HandleErrorLayer;
use axum::Router;
use http::StatusCode;
use tower::timeout::TimeoutLayer;
use tower::BoxError;
use tower_http::trace::{
    DefaultMakeSpan, DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, TraceLayer,
};
use tracing::Level;

use crate::http::middleware::metrics::MetricsLayer;

pub(crate) mod metrics;

pub(crate) fn add_default_middleware(router: Router) -> Router {
    let trace_layer = TraceLayer::new_for_http()
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        .make_span_with(
            DefaultMakeSpan::new()
                .level(Level::INFO)
                .include_headers(false),
        )
        .on_response(DefaultOnResponse::new().level(Level::INFO))
        .on_failure(DefaultOnFailure::new().level(Level::ERROR));

    let layer = tower::ServiceBuilder::new()
        .layer(MetricsLayer::new())
        .layer(HandleErrorLayer::new(|_: BoxError| async {
            StatusCode::REQUEST_TIMEOUT
        }))
        .layer(TimeoutLayer::new(Duration::from_secs(10)))
        .layer(trace_layer);
    router.layer(layer)
}
