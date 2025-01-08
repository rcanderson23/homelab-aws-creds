use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {}

#[derive(Parser, Clone)]
pub struct Config {
    /// Server listener
    #[arg(long, default_value = "0.0.0.0:8080")]
    pub server_address: String,

    /// Metrics listener
    #[arg(long, default_value = "0.0.0.0:9090")]
    pub metrics_address: String,

    /// Path to kube CA
    #[arg(long)]
    pub kube_ca: Option<PathBuf>,

    /// The amount of time the readyz endpoint will start failing prior to
    /// graceful shutdown signal to server in seconds
    #[arg(long, default_value = "0")]
    pub ready_grace_period: u64,

    // Path to the kube ca
    #[arg(
        long,
        env,
        default_value = "/var/run/secrets/kubernetes.io/serviceaccount/ca.crt"
    )]
    pub kube_ca_path: String,

    // Path to the role mapping config
    #[arg(long, env)]
    pub role_mapping_path: String,
}
