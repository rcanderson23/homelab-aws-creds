use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::PathBuf;

use clap::{Parser, Subcommand};

// Container credentials expects this network addr over http
pub const CONTAINER_IPV4_ADDR: Ipv4Addr = Ipv4Addr::new(169, 254, 170, 23);

#[allow(dead_code)]
// TODO: implement ipv6 listener
pub const CONTAINER_IPV6_ADDR: Ipv6Addr = Ipv6Addr::new(0xfd00, 0xec2, 0, 0, 0, 0, 0, 0x23);

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
    #[cfg(target_os = "linux")]
    Netlink,
}

#[derive(Parser, Debug, Clone)]
pub struct AgentConfig {
    #[command(flatten)]
    pub common_config: CommonConfig,

    /// Server listener for agent
    #[arg(long, default_value = "169.254.170.23:8080")]
    pub server_address: String,
}

#[derive(Parser, Debug, Clone)]
pub struct WebhookConfig {
    /// Server listener webhook
    #[arg(long, default_value = "0.0.0.0:8080")]
    pub server_address: String,

    /// Path to server cert
    #[arg(long)]
    pub cert: PathBuf,

    /// Path to private key
    #[arg(long)]
    pub key: PathBuf,

    /// AWS region
    #[arg(long)]
    pub aws_region: String,

    /// Server listener for agent
    #[arg(long, default_value = "169.254.170.23:8080")]
    pub agent_address: String,

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
