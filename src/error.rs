use std::time::SystemTimeError;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("aws sts: {0}")]
    StsError(#[from] aws_sdk_sts::error::BoxError),

    #[error("aws error: {0}")]
    AwsError(String),

    #[error("time conversion error: {0}")]
    TimeError(#[from] SystemTimeError),

    #[error("io error: {0}")]
    IoError(#[from] tokio::io::Error),

    #[error("error serializing/deserializing: {0}")]
    SerDeError(#[from] serde_yml::Error),

    #[error("{0}")]
    KubeError(#[from] kube::error::Error),

    #[error("kube error: {0}")]
    OtherError(String),

    #[error("error validating token: {0}")]
    TokenError(String),

    #[error("{0}")]
    RoleMappingError(String),

    #[error("rtnetlink error: {0}")]
    NetlinkError(#[from] rtnetlink::Error),
}
