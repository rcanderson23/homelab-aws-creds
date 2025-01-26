use super::mappings::Mappings;
use crate::config::CONTAINER_IPV4_ADDR;
use crate::http::middleware::add_default_middleware;
use crate::http::webhook::patch::create_pod_patch;
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use json_patch::Patch;
use jsonptr::PointerBuf;
use k8s_openapi::api::core::v1::{EnvVar, Pod};
use kube::api::DynamicObject;
use kube::core::admission::{AdmissionRequest, AdmissionResponse, AdmissionReview};
use kube::ResourceExt;
use std::sync::Arc;
use tracing::{error, info, trace};

#[derive(Clone)]
pub(crate) struct WebhookState {
    role_mappings: Arc<Mappings>,
    agent_address: String,
    aws_region: String,
}

impl WebhookState {
    pub(crate) fn new(
        role_mappings: Arc<Mappings>,
        agent_address: String,
        aws_region: String,
    ) -> Self {
        Self {
            role_mappings,
            agent_address,
            aws_region,
        }
    }
    fn should_mutate(&self, service_account: Option<String>, namespace: Option<String>) -> bool {
        if let (Some(sa), Some(ns)) = (service_account, namespace) {
            self.role_mappings
                .get_role(ns.as_str(), sa.as_str())
                .is_some()
        } else {
            false
        }
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
    println!("request: {}", serde_json::to_string(&req).unwrap());
    let mut res = AdmissionResponse::from(&req);
    let og_res = res.clone();
    let mut patch = Patch(vec![]);
    if let Some(ref pod) = req.object {
        if state.should_mutate(
            pod.spec
                .to_owned()
                .unwrap_or_default()
                .service_account
                .to_owned(),
            pod.namespace(),
        ) {
            patch = create_pod_patch(pod, &state.agent_address, &state.aws_region);
        }
        trace!("{}", &patch);
        res = match res.with_patch(patch) {
            Ok(p) => p,
            Err(_) => return Json(og_res.into_review()),
        }
    };
    trace!("{}", serde_json::to_string(&res).unwrap());
    Json(res.into_review())
}
