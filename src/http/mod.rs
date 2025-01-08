pub(crate) mod app;
pub(crate) mod mappings;
pub(crate) mod metrics;
pub(crate) mod middleware;
pub(crate) mod server;

use anyhow::anyhow;
use anyhow::Error;
use std::time::Duration;
use tokio::task::JoinError;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing::error;
use tracing::info;

use super::config::Config;

pub async fn serve(cfg: Config) -> Result<(), Error> {
    let crypto_provider = tokio_rustls::rustls::crypto::aws_lc_rs::default_provider();
    crypto_provider
        .install_default()
        .map_err(|_| anyhow!("failed to install crypto provider"))?;

    // setup cancellation for graceful shutdown
    let metrics_cancel = CancellationToken::new();
    let metrics_ready = CancellationToken::new();
    let server_cancel = CancellationToken::new();
    let metrics_addr = cfg.metrics_address.clone();

    // start metrics server
    let mut metrics_handle = tokio::spawn({
        let cancel = metrics_cancel.clone();
        let ready = metrics_ready.clone();
        async move { server::start_metrics_server(cancel, ready, &metrics_addr).await }
    });

    let ready_grace = cfg.ready_grace_period;

    // start application server
    let mut server_handle = tokio::spawn({
        let cancel = server_cancel.clone();
        async move { server::start_server(cancel, cfg).await }
    });

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
