use std::path::{Path, PathBuf};

use serde::Deserialize;
use tracing::info;

use crate::error::Error;

#[derive(Clone, Deserialize)]
pub(crate) struct Mappings {
    pub mappings: Vec<ServiceRoleMapping>,
}

impl Mappings {
    pub(crate) fn get_role(&self, namespace: &str, service_account: &str) -> Option<String> {
        self.mappings
            .iter()
            .find(|s| s.service_account == service_account && s.namespace == namespace)
            .map(|srm| srm.aws_role.to_owned())
    }
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServiceRoleMapping {
    pub service_account: String,
    pub namespace: String,
    pub aws_role: String,
}

pub(crate) async fn load_mappings(path: impl AsRef<Path>) -> Result<Mappings, Error> {
    Ok(serde_yml::from_str(
        tokio::fs::read_to_string(path).await?.as_str(),
    )?)
}
