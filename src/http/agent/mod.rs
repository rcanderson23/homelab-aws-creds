mod aws;
mod kubernetes;
mod state;

use std::sync::Arc;

use crate::config::AgentConfig;
use crate::http::{mappings, shutdown_server};
use anyhow::{anyhow, Error};
use aws::AwsState;
use kubernetes::KubeState;
use state::{new_agent_router, AgentState};
use tokio::select;
use tokio_util::sync::CancellationToken;
use tracing::info;

pub(crate) async fn start_agent(
    cancel: CancellationToken,
    cfg: Arc<AgentConfig>,
) -> Result<(), Error> {
    let kube_state = KubeState::try_new().await?;
    let aws_state = AwsState::new().await;
    let role_mappings = mappings::load_mappings(&cfg.common_config.role_mapping_path).await?;

    info!("creating agent router");
    let router = new_agent_router(AgentState::new(
        aws_state,
        kube_state,
        Arc::new(role_mappings),
    ));

    let shutdown_cancel = cancel.clone();
    let h = tokio::spawn(async move {
        info!("agent listening on {}", cfg.server_address);
        let listener = tokio::net::TcpListener::bind(&cfg.server_address).await?;
        axum::serve(listener, router.into_make_service())
            .with_graceful_shutdown(shutdown_server(shutdown_cancel))
            .await
    });
    select! {
        h = h => {
                match h {
                    Ok(Err(e)) => return Err(e.into()),
                    Ok(Ok(_)) => {}
                    Err(_) => return Err(anyhow!("join handle failure")),
                }
            },
        _  = cancel.cancelled() => {}
    }
    Ok(())
}
