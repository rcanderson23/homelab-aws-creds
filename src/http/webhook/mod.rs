mod patch;
mod state;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::config::WebhookConfig;
use crate::http::mappings;
use anyhow::{anyhow, Error};
use axum_server::tls_rustls::RustlsConfig;
use futures::channel::mpsc::{channel, Receiver};
use futures::{SinkExt, StreamExt};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use state::{new_webhook_router, WebhookState};
use tokio::select;
use tokio::task::JoinHandle;
use tokio_rustls::rustls::pki_types::pem::PemObject;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls::rustls::{server::WebPkiClientVerifier, ServerConfig as RustlsServerConfig};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

pub(crate) async fn start_webhook(
    cancel: CancellationToken,
    cfg: Arc<WebhookConfig>,
) -> Result<(), Error> {
    let role_mappings = mappings::load_mappings(&cfg.common_config.role_mapping_path).await?;
    let router = new_webhook_router(WebhookState::new(
        Arc::new(role_mappings),
        cfg.agent_address.clone(),
        cfg.aws_region.clone(),
    ));
    let cert = cfg.cert.clone();
    let key = cfg.key.clone();
    let tls_config = create_tls_config(&cert, &key)?;

    tokio::spawn(start_tls_watch(tls_config.clone(), cert, key));

    let handle = axum_server::Handle::new();
    let shutdown_handle = handle.clone();
    let server_address = cfg.server_address.clone();
    let h: JoinHandle<Result<(), Error>> = tokio::spawn(async move {
        info!(
            "webhook configured to listen securely on {}",
            server_address
        );
        axum_server::tls_rustls::bind_rustls(server_address.parse()?, tls_config)
            .handle(handle)
            .serve(router.into_make_service())
            .await?;
        Ok(())
    });
    select! {
        h = h => {
                match h {
                    Ok(Err(e)) => return Err(e),
                    Ok(Ok(_)) => {}
                    Err(_) => return Err(anyhow!("join handle failure")),
                }
            },
        _  = cancel.cancelled() => {}
    }
    shutdown_handle.graceful_shutdown(None);
    Ok(())
}

fn create_watcher() -> Result<(RecommendedWatcher, Receiver<notify::Result<Event>>), Error> {
    let (mut tx, rx) = channel(1);
    let watcher = RecommendedWatcher::new(
        move |res| {
            futures::executor::block_on(async {
                if let Err(e) = tx.send(res).await {
                    error!("failed to tx event: {}", e)
                }
            })
        },
        Config::default(),
    )?;

    Ok((watcher, rx))
}

async fn start_tls_watch(
    tls_config: RustlsConfig,
    cert: PathBuf,
    key: PathBuf,
) -> Result<(), Error> {
    let (mut watcher, mut rx) = create_watcher()?;
    watcher.watch(&cert, RecursiveMode::NonRecursive)?;

    while let Some(res) = rx.next().await {
        match res {
            Ok(event) => {
                if event.kind.is_modify() {
                    reload_tls(&tls_config, &cert, &key).await
                }
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    }
    Ok(())
}

async fn reload_tls(config: &RustlsConfig, cert: impl AsRef<Path>, key: impl AsRef<Path>) {
    match config.reload_from_pem_file(cert, key).await {
        Ok(_) => info!("successfully reloaded tls config"),
        Err(e) => error!("failed to reload tls config: {}", e),
    }
}

fn create_tls_config(
    cert: impl AsRef<Path>,
    priv_key: impl AsRef<Path>,
) -> Result<RustlsConfig, Error> {
    let cert_chain = load_certs(cert)?;
    let key_der = load_private_key(priv_key)?;
    let mut tls_config = RustlsServerConfig::builder()
        .with_client_cert_verifier(WebPkiClientVerifier::no_client_auth())
        .with_single_cert(cert_chain, key_der)?;
    tls_config
        .alpn_protocols
        .append(&mut vec!["http/1.1".into()]);

    Ok(RustlsConfig::from_config(Arc::new(tls_config)))
}

fn load_private_key(priv_key: impl AsRef<Path>) -> Result<PrivateKeyDer<'static>, Error> {
    Ok(PrivateKeyDer::from_pem_file(priv_key)?)
}

fn load_certs(server_cert: impl AsRef<Path>) -> Result<Vec<CertificateDer<'static>>, Error> {
    let certs: Vec<_> = CertificateDer::pem_file_iter(server_cert)?.collect();

    let mut server_certs = vec![];
    for cert in certs {
        match cert {
            Ok(c) => server_certs.push(c),
            Err(e) => return Err(anyhow!("failed to parse server ca: {}", e)),
        }
    }
    Ok(server_certs)
}
