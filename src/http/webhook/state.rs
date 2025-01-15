use super::mappings::Mappings;
use crate::http::middleware::add_default_middleware;
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use jsonptr::PointerBuf;
use k8s_openapi::api::core::v1::{EnvVar, Pod};
use kube::api::DynamicObject;
use kube::core::admission::{AdmissionRequest, AdmissionResponse, AdmissionReview};
use kube::ResourceExt;
use std::sync::Arc;
use tracing::error;

#[derive(Clone)]
pub(crate) struct WebhookState {
    role_mappings: Arc<Mappings>,
    server_address: String,
}

impl WebhookState {
    pub(crate) fn new(role_mappings: Arc<Mappings>, server_address: String) -> Self {
        Self {
            role_mappings,
            server_address,
        }
    }
    fn should_mutate(&self, service_account: &str, namespace: &str) -> bool {
        if service_account.is_empty() || namespace.is_empty() {
            return false;
        }
        self.role_mappings
            .get_role(namespace, service_account)
            .is_some()
    }
}

pub(crate) fn new_webhook_router(webhook_state: WebhookState) -> Router {
    let rt = Router::new()
        .route("/v1/mutate/pods", post(mutate_pod_handler))
        .with_state(webhook_state);
    add_default_middleware(rt)
}

async fn mutate_pod_handler(
    State(state): State<WebhookState>,
    Json(review): Json<AdmissionReview<Pod>>,
) -> Json<AdmissionReview<DynamicObject>> {
    let req: AdmissionRequest<_> = match review.try_into() {
        Ok(req) => req,
        Err(e) => {
            error!("invalid admission request: {}", e.to_string());
            return Json(AdmissionResponse::invalid(e.to_string()).into_review());
        }
    };
    let mut res = AdmissionResponse::from(&req);
    let og_res = res.clone();
    let mut patches = vec![];
    if let Some(pod) = req.object {
        let ns = pod.namespace().unwrap_or_default();
        let spec = pod.spec.unwrap_or_default();
        if state.should_mutate(
            spec.service_account_name.unwrap_or_default().as_str(),
            ns.as_str(),
        ) {
            for (idx, container) in spec.containers.iter().enumerate() {
                if container.env.is_none() {
                    patches.push(json_patch::PatchOperation::Add(json_patch::AddOperation {
                        path: PointerBuf::from_tokens([
                            "spec",
                            "containers",
                            idx.to_string().as_str(),
                            "env",
                        ]),
                        value: serde_json::to_value(vec![
                            EnvVar {
                                name: "AWS_CONTAINER_CREDENTIALS_FULL_URI".to_string(),
                                value: Some(format!(
                                    "http://{}/v1/container_credentials",
                                    state.server_address
                                )),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "AWS_CONTAINER_AUTHORIZATION_TOKEN_FILE".to_string(),
                                value: Some("/var/run/secrets/kubernetes.io/token".into()),
                                ..Default::default()
                            },
                        ])
                        .unwrap(),
                    }));
                }
                let env = container.clone().env.unwrap_or_default();
                if !contains_aws_cred_env(&env) {
                    patches.push(json_patch::PatchOperation::Add(json_patch::AddOperation {
                        path: PointerBuf::from_tokens([
                            "spec",
                            "containers",
                            idx.to_string().as_str(),
                            "env",
                            "-",
                        ]),
                        value: serde_json::to_value(vec![
                            EnvVar {
                                name: "AWS_CONTAINER_CREDENTIALS_FULL_URI".to_string(),
                                value: Some(format!(
                                    "http://{}/v1/container_credentials",
                                    state.server_address
                                )),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "AWS_CONTAINER_AUTHORIZATION_TOKEN_FILE".to_string(),
                                value: Some("/var/run/secrets/kubernetes.io/token".into()),
                                ..Default::default()
                            },
                        ])
                        .unwrap(),
                    }));
                }
            }
        }
        res = match res.with_patch(json_patch::Patch(patches)) {
            Ok(p) => p,
            Err(_) => return Json(og_res.into_review()),
        }
    };
    Json(res.into_review())
}

// checks if the environment variables contain and aws specific
fn contains_aws_cred_env(env: &[EnvVar]) -> bool {
    env.iter().any(|nv| {
        nv.name == "AWS_CONTAINER_CREDENTIALS_FULL_URI"
            || nv.name == "AWS_CONTAINER_CREDENTIALS_RELATIVE_URI"
            || nv.name == "AWS_CONTAINER_AUTHORIZATION_TOKEN"
            || nv.name == "AWS_CONTAINER_AUTHORIZATION_TOKEN_FILE"
    })
}
