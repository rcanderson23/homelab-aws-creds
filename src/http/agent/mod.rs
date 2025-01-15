mod aws;
mod kubernetes;
mod state;

use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::Path;
use std::sync::Arc;

use crate::http::{mappings, shutdown_server};
use anyhow::{anyhow, Error};
use aws::AwsState;
use kubernetes::KubeState;
use state::{new_agent_router, AgentState};
use tokio::select;
use tokio_util::sync::CancellationToken;
use tracing::info;

// Container credentials expects this network addr over http
const IPV4_ADDR: Ipv4Addr = Ipv4Addr::new(169, 254, 170, 23);

// TODO: implement ipv6 listener
const IPV6_ADDR: Ipv6Addr = Ipv6Addr::new(0xfd00, 0xec2, 0, 0, 0, 0, 0, 0x23);

pub(crate) async fn start_agent(
    cancel: CancellationToken,
    role_mapping_path: impl AsRef<Path>,
) -> Result<(), Error> {
    let kube_state = KubeState::try_new().await?;
    let aws_state = AwsState::new().await;
    let role_mappings = mappings::load_mappings(role_mapping_path).await?;
    let router = new_agent_router(AgentState::new(
        aws_state,
        kube_state,
        Arc::new(role_mappings),
    ));

    let shutdown_cancel = cancel.clone();
    let h = tokio::spawn(async move {
        info!("agent listening on {}", "0.0.0.0:8080");
        let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
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
