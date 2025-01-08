use std::sync::Arc;
use std::time::Duration;

use axum::error_handling::HandleErrorLayer;
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use axum_extra::TypedHeader;
use headers::authorization::Bearer;
use headers::Authorization;
use http::StatusCode;
use tower::timeout::TimeoutLayer;
use tower::BoxError;
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;

use crate::aws::{AwsState, TemporaryCredential};
use crate::error::Error;
use crate::kubernetes::KubeState;

use super::mappings::Mappings;
use super::middleware::metrics::MetricsLayer;

type NamespaceServiceAccount = (String, String);

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) aws_state: AwsState,
    pub(crate) kube_state: KubeState,
    pub(crate) role_mappings: Arc<Mappings>,
}

impl AppState {
    async fn get_credentials(
        &self,
        role: String,
        session_name: String,
    ) -> Result<TemporaryCredential, Error> {
        self.aws_state.get_credentials(role, session_name).await
    }

    async fn check_token(&self, token: &str) -> Result<NamespaceServiceAccount, Error> {
        let status = self.kube_state.allowed_token(token.into()).await?;
        if let Some(error) = &status.error {
            return Err(Error::TokenError(error.to_string()));
        }
        if let Some(false) = &status.authenticated {
            return Err(Error::TokenError("token not authenticated".to_string()));
        }
        let username = status
            .user
            .ok_or_else(|| Error::TokenError("user not found".to_string()))?
            .username
            .ok_or_else(|| Error::TokenError("username not found in status".to_string()))?;
        get_namespace_sa(username)
    }
}
pub(crate) fn new_app_router(app_state: AppState) -> Router {
    let trace_layer = TraceLayer::new_for_http()
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        .make_span_with(
            DefaultMakeSpan::new()
                .level(Level::INFO)
                .include_headers(false),
        )
        .on_response(DefaultOnResponse::new().level(Level::INFO))
        .on_failure(());

    let layer = tower::ServiceBuilder::new()
        .layer(MetricsLayer::new())
        .layer(HandleErrorLayer::new(|_: BoxError| async {
            StatusCode::REQUEST_TIMEOUT
        }))
        .layer(TimeoutLayer::new(Duration::from_secs(10)))
        .layer(trace_layer);
    Router::new()
        .route("/v1/container_credentials", get(container_credentials))
        .layer(layer)
        .with_state(app_state)
}

#[axum::debug_handler]
async fn container_credentials(
    State(state): State<AppState>,
    TypedHeader(token): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<TemporaryCredential>, (StatusCode, String)> {
    let (namespace, sa) = state
        .check_token(token.token())
        .await
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;
    let role = state
        .role_mappings
        .get_role(namespace.as_str(), sa.as_str())
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;
    Ok(Json(
        state
            .get_credentials(role, format!("{namespace}-{sa}"))
            .await
            .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?,
    ))
}

fn get_namespace_sa(username: String) -> Result<NamespaceServiceAccount, Error> {
    let usplit: Vec<&str> = username.split(':').collect();
    if usplit.len() != 4 {
        Err(Error::TokenError("username does not conform to expected 'system:serviceaccount:namespace:serviceaccount' format".to_string()))
    } else {
        Ok((usplit[2].into(), usplit[3].into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn username_parse() {
        assert_eq!(
            get_namespace_sa("system:serviceaccount:default:test".into()).unwrap(),
            ("default".into(), "test".into())
        );
    }
}
