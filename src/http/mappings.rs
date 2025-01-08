use std::path::Path;

use serde::Deserialize;

use crate::error::Error;

#[derive(Clone, Deserialize)]
pub(crate) struct Mappings {
    pub mappings: Vec<ServiceRoleMapping>,
}

impl Mappings {
    pub(crate) fn get_role(&self, namespace: &str, service_account: &str) -> Result<String, Error> {
        for mapping in &self.mappings {
            if mapping.service_account == service_account && mapping.namespace == namespace {
                return Ok(mapping.aws_role.to_owned());
            }
        }
        Err(Error::RoleMappingError(format!(
            "no role mapped to namespace {} and service account {}",
            namespace, service_account
        )))
    }
}

#[derive(Clone, Deserialize)]
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
