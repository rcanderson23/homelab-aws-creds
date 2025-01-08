use crate::error::Error;
use k8s_openapi::api::authentication::v1::{TokenReview, TokenReviewSpec, TokenReviewStatus};
use k8s_openapi::apimachinery::pkg::apis::meta;
use kube::api::PostParams;
use kube::{Api, Client as KubeClient};

#[derive(Clone)]
pub(crate) struct KubeState {
    kube_client: KubeClient,
}

impl KubeState {
    pub(crate) async fn try_new() -> Result<Self, Error> {
        let kube_client = KubeClient::try_default().await?;
        Ok(Self { kube_client })
    }

    // Verifies if the token is allowed by make a TokenReview request to kubernetes API
    pub(crate) async fn allowed_token(&self, token: String) -> Result<TokenReviewStatus, Error> {
        let api: Api<TokenReview> = Api::all(self.kube_client.clone());
        let response = api
            .create(
                &PostParams::default(),
                &TokenReview {
                    metadata: meta::v1::ObjectMeta {
                        generate_name: Some("aws-credentials-token-review-".into()),
                        ..Default::default()
                    },
                    spec: TokenReviewSpec {
                        token: Some(token),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .await?;
        if let Some(status) = response.status {
            Ok(status)
        } else {
            Err(Error::OtherError(
                "TokenReview request did not return TokenReviewStatus".into(),
            ))
        }
    }
}
