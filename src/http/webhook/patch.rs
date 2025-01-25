use json_patch::{Patch, PatchOperation};
use jsonptr::PointerBuf;
use k8s_openapi::api::core::v1::{EnvVar, Pod};
use serde_json::json;

use crate::config::CONTAINER_IPV4_ADDR;

pub fn create_pod_patch(pod: &Pod) -> Patch {
    let Some(ref spec) = pod.spec else {
        return Patch(vec![]);
    };
    let mut patches = vec![];
    for (idx, container) in spec.containers.iter().enumerate() {
        let path = PointerBuf::from_tokens(["spec", "containers", idx.to_string().as_str(), "env"]);
        if let Some(ref env) = container.env {
        } else {
            patches.push(json_patch::PatchOperation::Add(json_patch::AddOperation {
                path: path.clone(),
                value: serde_json::to_value(vec![
                    EnvVar {
                        name: "AWS_CONTAINER_CREDENTIALS_FULL_URI".to_string(),
                        value: Some(format!(
                            "http://{}:8080/v1/container-credentials",
                            CONTAINER_IPV4_ADDR
                        )),
                        ..Default::default()
                    },
                    EnvVar {
                        name: "AWS_CONTAINER_AUTHORIZATION_TOKEN_FILE".to_string(),
                        value: Some("/var/run/secrets/kubernetes.io/serviceaccount/token".into()),
                        ..Default::default()
                    },
                ])
                .unwrap(),
            }));
        };
    }
    Patch(patches)
}

// checks if the environment variables contain aws credential env
fn contains_aws_cred_env(env: &[EnvVar]) -> bool {
    env.iter().any(|nv| {
        nv.name == "AWS_CONTAINER_CREDENTIALS_FULL_URI"
            || nv.name == "AWS_CONTAINER_CREDENTIALS_RELATIVE_URI"
            || nv.name == "AWS_CONTAINER_AUTHORIZATION_TOKEN"
            || nv.name == "AWS_CONTAINER_AUTHORIZATION_TOKEN_FILE"
    })
}

// checks if the environment variables contain aws region env
fn contains_aws_region_env(env: &[EnvVar]) -> bool {
    env.iter()
        .any(|nv| nv.name == "AWS_REGION" || nv.name == "AWS_DEFAULT_REGION")
}

#[cfg(test)]
mod tests {
    use k8s_openapi::api::core::v1::{Container, PodSpec};
    use serde_json::from_value;

    use super::*;

    #[test]
    fn test_create_pod_patch() {
        let pod = Pod {
            spec: Some(PodSpec {
                containers: vec![Container {
                    name: "test".into(),
                    env: None,
                    ..Default::default()
                }],
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(
            create_pod_patch(&pod),
            from_value::<Patch>(json!([
              { "op": "add", "path": "/spec/containers/0/env", "value": [{"name":"AWS_CONTAINER_CREDENTIALS_FULL_URI", "value":"http://169.254.170.23:8080/v1/container-credentials"},{"name":"AWS_CONTAINER_AUTHORIZATION_TOKEN_FILE", "value":"/var/run/secrets/kubernetes.io/serviceaccount/token"}] },
            ]))
            .unwrap()
        );
    }
}
