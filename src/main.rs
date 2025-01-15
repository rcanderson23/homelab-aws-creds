use anyhow::Error;
use clap::Parser;
use homelab_aws_creds::config::Cli;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "homelab_aws_creds=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();
    match cli.command {
        homelab_aws_creds::config::Commands::Agent(agent_config) => {
            homelab_aws_creds::http::serve_agent(agent_config).await
        }
        homelab_aws_creds::config::Commands::Webhook(webhook_config) => {
            homelab_aws_creds::http::serve_webhook(webhook_config).await
        }
    }
}
