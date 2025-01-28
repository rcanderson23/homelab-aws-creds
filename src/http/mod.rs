mod agent;
mod mappings;
mod metrics;
mod middleware;
mod util;
mod webhook;

use anyhow::anyhow;
use anyhow::Error;
use metrics::status_router;
use metrics::StatusHandler;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::task::JoinError;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing::error;
use tracing::info;

use crate::config::AgentConfig;
use crate::config::CommonConfig;
use crate::config::WebhookConfig;

pub async fn serve_agent(cfg: Arc<AgentConfig>) -> Result<(), Error> {
    install_crypto()?;
    let agent_cancel = CancellationToken::new();
    let agent_handle = tokio::spawn({
        let cfg = cfg.clone();
        let cancel = agent_cancel.clone();
        async move { agent::start_agent(cancel, cfg).await }
    });
    serve(&cfg.common_config, agent_handle, agent_cancel).await
}

pub async fn serve_webhook(cfg: Arc<WebhookConfig>) -> Result<(), Error> {
    install_crypto()?;
    let webhook_cancel = CancellationToken::new();
    let webhook_handle = tokio::spawn({
        let cfg = cfg.clone();
        let cancel = webhook_cancel.clone();
        async move { webhook::start_webhook(cancel, cfg).await }
    });
    serve(&cfg.common_config, webhook_handle, webhook_cancel).await
}

async fn serve(
    cfg: &CommonConfig,
    mut server_handle: JoinHandle<Result<(), Error>>,
    server_cancel: CancellationToken,
) -> Result<(), Error> {
    // setup cancellation for graceful shutdown
    let metrics_cancel = CancellationToken::new();
    let metrics_ready = CancellationToken::new();

    let metrics_addr = cfg.metrics_address.clone();

    // start metrics server
    let mut metrics_handle = tokio::spawn({
        let cancel = metrics_cancel.clone();
        let ready = metrics_ready.clone();
        async move { start_metrics_server(cancel, ready, &metrics_addr).await }
    });

    let ready_grace = cfg.ready_grace_period;

    // create shutdown signal handler
    let mut shutdown_handle = tokio::spawn(async move { shutdown_signal().await });

    // watch for shutdown and errors
    tokio::select! {
        h = &mut metrics_handle => exit("metrics", h),
        h = &mut server_handle => exit("app", h),
        _ = &mut shutdown_handle => {
                metrics_ready.cancel();
                sleep(Duration::new(ready_grace, 0)).await;
                metrics_cancel.cancel();
                server_cancel.cancel();
                let (metrics, server) = tokio::join!(metrics_handle, server_handle);
                if let Err(m) = metrics {
                    error!("metrics exited with error: {}", m.to_string());
                }
                if let Err(s) = server {
                    error!("server exited with error: {}", s.to_string());
                }
            },
    };

    Ok(())
}

async fn start_metrics_server(
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

fn exit(task: &str, out: Result<Result<(), Error>, JoinError>) {
    match out {
        Ok(Ok(_)) => {
            info!("{task} exited")
        }
        Ok(Err(e)) => {
            error!("{task} failed with error: {e}")
        }
        Err(e) => {
            error!("{task} task failed to complete: {e}")
        }
    }
}

async fn shutdown_server(cancel: CancellationToken) {
    tokio::select! {
        _ = cancel.cancelled() => {},
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
          info!("captured ctrl_c signal");
        },
        _ = terminate => {},
    }
}

fn install_crypto() -> Result<(), Error> {
    let crypto_provider = tokio_rustls::rustls::crypto::aws_lc_rs::default_provider();
    crypto_provider
        .install_default()
        .map_err(|_| anyhow!("failed to install crypto provider"))
}
