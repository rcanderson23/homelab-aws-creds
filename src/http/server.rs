use std::sync::Arc;

use crate::aws::AwsState;
use crate::config::Config;
use crate::http::app::{new_app_router, AppState};
use crate::http::metrics::{status_router, StatusHandler};
use crate::http::{mappings, shutdown_server};
use crate::kubernetes::KubeState;
use anyhow::{anyhow, Error};
use tokio::{net::TcpListener, select};
use tokio_util::sync::CancellationToken;
use tracing::info;

pub(crate) async fn start_server(cancel: CancellationToken, cfg: Config) -> Result<(), Error> {
    let kube_state = KubeState::try_new().await?;
    let aws_state = AwsState::new().await;
    let role_mappings = mappings::load_mappings(cfg.role_mapping_path).await?;
    let router = new_app_router(AppState {
        kube_state,
        aws_state,
        role_mappings: Arc::new(role_mappings),
    });

    let shutdown_cancel = cancel.clone();
    let h = tokio::spawn(async move {
        info!(
            "application listening insecurely on {}",
            &cfg.server_address
        );
        let listener = tokio::net::TcpListener::bind(cfg.server_address).await?;
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

pub(crate) async fn start_metrics_server(
    cancel: CancellationToken,
    ready: CancellationToken,
    addr: &str,
) -> Result<(), Error> {
    let listener = TcpListener::bind(addr).await?;
    info!("metrics listening on {}", addr);

    let app = status_router(StatusHandler::try_new(ready)?)?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_server(cancel))
        .await?;
    Ok(())
}
