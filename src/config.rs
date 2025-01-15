use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Clone, Subcommand, Debug)]
pub enum Commands {
    Agent(AgentConfig),
    Webhook(WebhookConfig),
}

#[derive(Parser, Debug, Clone)]
pub struct AgentConfig {
    #[command(flatten)]
    pub common_config: CommonConfig,
}

#[derive(Parser, Debug, Clone)]
pub struct WebhookConfig {
    /// Server listener
    #[arg(long, default_value = "0.0.0.0:8080")]
    pub server_address: String,

    /// Path to server cert
    #[arg(long, default_value = "./certs")]
    pub cert: PathBuf,

    /// Path to private key
    #[arg(long)]
    pub key: PathBuf,

    #[command(flatten)]
    pub common_config: CommonConfig,
}

#[derive(Parser, Debug, Clone)]
pub struct CommonConfig {
    /// Metrics listener
    #[arg(long, default_value = "0.0.0.0:9090")]
    pub metrics_address: String,

    /// The amount of time the readyz endpoint will start failing prior to
    /// graceful shutdown signal to server in seconds
    #[arg(long, default_value = "0")]
    pub ready_grace_period: u64,

    // Path to the role mapping config
    #[arg(long, env)]
    pub role_mapping_path: String,
}
