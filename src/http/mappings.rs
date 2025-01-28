use std::path::{Path, PathBuf};
use std::sync::Arc;

use arc_swap::ArcSwapAny;
use notify::{RecursiveMode, Watcher};
use serde::Deserialize;
use tracing::{error, info, trace};

use crate::error::Error;

use super::util::create_watcher;

#[derive(Clone)]
pub(crate) struct Mapping {
    pub mappings: Arc<ArcSwapAny<Arc<Mappings>>>,
}

impl Mapping {
    pub(crate) async fn try_new_from_file(path: PathBuf) -> Result<Self, Error> {
        let mappings = load_mappings(&path).await?;
        let mapping = Mapping {
            mappings: Arc::new(ArcSwapAny::new(Arc::new(mappings))),
        };

        tokio::spawn(start_mappings_watch(path, mapping.clone()));
        Ok(mapping)
    }
    pub(crate) fn get_role(&self, namespace: &str, service_account: &str) -> Option<String> {
        self.mappings
            .load()
            .mappings
            .iter()
            .find(|s| s.service_account == service_account && s.namespace == namespace)
            .map(|srm| srm.aws_role.to_owned())
    }
}

#[derive(Clone, Deserialize)]
pub(crate) struct Mappings {
    pub mappings: Vec<ServiceRoleMapping>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServiceRoleMapping {
    pub service_account: String,
    pub namespace: String,
    pub aws_role: String,
}

pub(crate) async fn load_mappings(path: impl AsRef<Path>) -> Result<Mappings, Error> {
    Ok(serde_yaml_ng::from_str(
        tokio::fs::read_to_string(path).await?.as_str(),
    )?)
}

async fn start_mappings_watch(path: PathBuf, mapping: Mapping) {
    loop {
        trace!("starting mapping watcher");
        let Ok((mut watcher, mut rx)) = create_watcher() else {
            error!("failed to create watcher");
            continue;
        };
        if watcher.watch(&path, RecursiveMode::Recursive).is_ok() {
            while let Some(res) = rx.recv().await {
                match res {
                    Ok(event) => {
                        match event.kind {
                            notify::EventKind::Modify(_) => match load_mappings(&path).await {
                                Ok(m) => {
                                    info!("reloading role mappings");
                                    mapping.mappings.store(Arc::new(m));
                                }
                                Err(e) => error!("failed to reload mappings config: {}", e),
                            },
                            notify::EventKind::Remove(_) => {
                                break;
                            }
                            _ => {}
                        }
                        if event.kind.is_modify() {}
                    }
                    Err(e) => error!("watcher error: {}", e),
                }
            }
        } else {
            error!("failed to watch path: {:?}", path);
        }
    }
}
