use super::aws::{AwsState, TemporaryCredential};
use super::kubernetes::KubeState;
use super::mappings::Mappings;
use crate::error::Error;
use crate::http::middleware::add_default_middleware;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use axum_extra::TypedHeader;
use headers::authorization::Bearer;
use headers::Authorization;
use http::{HeaderMap, StatusCode};
use serde::Serialize;
use std::sync::Arc;

type NamespaceServiceAccount = (String, String);

#[derive(Clone)]
pub(crate) struct AgentState {
    aws_state: AwsState,
    kube_state: KubeState,
    role_mappings: Arc<Mappings>,
}

impl AgentState {
    pub(crate) fn new(
        aws_state: AwsState,
        kube_state: KubeState,
        role_mappings: Arc<Mappings>,
    ) -> Self {
        Self {
            aws_state,
            kube_state,
            role_mappings,
        }
    }

    async fn get_credentials(
        &self,
        role: String,
        session_name: String,
    ) -> Result<TemporaryCredential, Error> {
        self.aws_state.get_credentials(role, session_name).await
    }

    async fn check_token(&self, token: &str) -> Result<NamespaceServiceAccount, Error> {
        let status = self.kube_state.allowed_token(token.into()).await?;
        match (&status.error, &status.authenticated) {
            (Some(e), _) => return Err(Error::TokenError(e.to_string())),
            (_, Some(false)) => {
                return Err(Error::TokenError("token not authenticated".to_string()))
            }
            (_, _) => {}
        }
        let username = status
            .user
            .ok_or_else(|| Error::TokenError("user not found".to_string()))?
            .username
            .ok_or_else(|| Error::TokenError("username not found in status".to_string()))?;
        get_namespace_sa(username)
    }
}

#[derive(Clone, Serialize)]
pub(crate) struct CredentialError {
    pub code: String,
    pub message: String,
}

//TODO: implement proper status code, currently returns 200
impl IntoResponse for CredentialError {
    fn into_response(self) -> axum::response::Response {
        Json(self).into_response()
    }
}

pub(crate) fn new_agent_router(agent_state: AgentState) -> Router {
    let rt = Router::new()
        .route("/v1/container-credentials", get(container_credentials))
        .with_state(agent_state);
    add_default_middleware(rt)
}

async fn container_credentials(
    State(state): State<AgentState>,
    headers: HeaderMap,
) -> Result<Json<TemporaryCredential>, CredentialError> {
    let auth = headers
        .get("authorization")
        .ok_or_else(|| CredentialError {
            code: StatusCode::UNAUTHORIZED.to_string(),
            message: "No authorization token passed".to_string(),
        })?;
    let (namespace, sa) = state
        .check_token(auth.to_str().map_err(|e| CredentialError {
            code: StatusCode::UNAUTHORIZED.to_string(),
            message: e.to_string(),
        })?)
        .await
        .map_err(|e| CredentialError {
            code: StatusCode::UNAUTHORIZED.to_string(),
            message: e.to_string(),
        })?;
    let role = state
        .role_mappings
        .get_role(namespace.as_str(), sa.as_str())
        .ok_or(CredentialError {
            code: StatusCode::UNAUTHORIZED.to_string(),
            message: "role not found serviceaccount mappings".to_string(),
        })?;
    Ok(Json(
        state
            .get_credentials(role, format!("{namespace}-{sa}"))
            .await
            .map_err(|e| CredentialError {
                code: StatusCode::UNAUTHORIZED.to_string(),
                message: e.to_string(),
            })?,
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
