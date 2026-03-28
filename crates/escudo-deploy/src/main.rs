mod cloudinit;
mod config;
mod provider;
mod providers;
mod reconciler;
mod ssh;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use reconciler::Reconciler;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, warn};

#[derive(Parser)]
#[command(
    name = "escudo-deploy",
    about = "Declarative VPN server provisioning for Vultr and Hetzner",
    version
)]
struct Cli {
    /// Path to the deployment config file
    #[arg(short, long, default_value = "config/deploy.toml")]
    config: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate API credentials for all configured providers
    Validate,

    /// Show what changes would be made (dry run)
    Plan,

    /// Apply changes: create missing servers, destroy extra servers
    Apply,

    /// Show current status of all managed servers
    Status,

    /// Destroy ALL managed servers (use with caution!)
    Destroy,
}

fn require_env(var: &str) -> Result<String> {
    std::env::var(var).with_context(|| format!("Missing required environment variable: {}", var))
}

fn optional_env(var: &str) -> Option<String> {
    std::env::var(var).ok()
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    // Load config
    let cfg = config::DeployConfig::load(&cli.config)
        .with_context(|| format!("Failed to load config from '{}'", cli.config))?;
    info!(path = %cli.config, servers = cfg.servers.len(), "Loaded deployment config");

    // Read environment variables
    let vultr_api_key = optional_env("VULTR_API_KEY");
    let hetzner_api_token = optional_env("HETZNER_API_TOKEN");
    let iproyal_api_token = optional_env("IPROYAL_API_TOKEN");
    let deploy_secret = optional_env("DEPLOY_SECRET").unwrap_or_else(|| "changeme".to_string());
    let home_url =
        optional_env("HOME_URL").unwrap_or_else(|| "https://api.escudovpn.com".to_string());

    // Build provider map
    let mut providers: HashMap<String, Arc<dyn provider::ServerProvider>> = HashMap::new();

    if let Some(key) = vultr_api_key {
        let p = providers::vultr::VultrProvider::new(key)?;
        providers.insert("vultr".to_string(), Arc::new(p));
        info!("Vultr provider registered");
    }

    if let Some(token) = hetzner_api_token {
        let p = providers::hetzner::HetznerProvider::new(token)?;
        providers.insert("hetzner".to_string(), Arc::new(p));
        info!("Hetzner provider registered");
    }

    match cli.command {
        Commands::Validate => {
            info!("Validating all provider credentials...");
            let mut all_ok = true;

            // Validate Vultr
            if let Some(p) = providers.get("vultr") {
                match p.validate().await {
                    Ok(()) => info!("Vultr: OK"),
                    Err(e) => {
                        error!("Vultr: FAILED - {}", e);
                        all_ok = false;
                    }
                }
            } else {
                warn!("Vultr: SKIPPED (VULTR_API_KEY not set)");
            }

            // Validate Hetzner
            if let Some(p) = providers.get("hetzner") {
                match p.validate().await {
                    Ok(()) => info!("Hetzner: OK"),
                    Err(e) => {
                        error!("Hetzner: FAILED - {}", e);
                        all_ok = false;
                    }
                }
            } else {
                warn!("Hetzner: SKIPPED (HETZNER_API_TOKEN not set)");
            }

            // Validate IPRoyal
            if let Some(token) = iproyal_api_token {
                match validate_iproyal(&token).await {
                    Ok(()) => info!("IPRoyal: OK"),
                    Err(e) => {
                        error!("IPRoyal: FAILED - {}", e);
                        all_ok = false;
                    }
                }
            } else {
                warn!("IPRoyal: SKIPPED (IPROYAL_API_TOKEN not set)");
            }

            if all_ok {
                info!("All provider validations passed.");
            } else {
                anyhow::bail!("One or more provider validations failed.");
            }
        }

        Commands::Plan => {
            let reconciler = Reconciler::new(providers, cfg, deploy_secret, home_url);
            let plan = reconciler.plan().await?;
            plan.print();
        }

        Commands::Apply => {
            let reconciler = Reconciler::new(providers, cfg, deploy_secret, home_url);
            let result = reconciler.apply().await?;

            println!("\nApply complete:");
            println!("  Created:   {}", result.created.len());
            println!("  Destroyed: {}", result.destroyed.len());
            println!("  Errors:    {}", result.errors.len());

            for err in &result.errors {
                error!("  ERROR: {}", err);
            }

            if !result.errors.is_empty() {
                anyhow::bail!("{} error(s) occurred during apply", result.errors.len());
            }
        }

        Commands::Status => {
            let reconciler = Reconciler::new(providers, cfg, deploy_secret, home_url);
            reconciler.status().await?;
        }

        Commands::Destroy => {
            warn!("╔══════════════════════════════════════════════════════════╗");
            warn!("║  DESTROY: This will delete ALL managed servers!          ║");
            warn!("║  This operation is not yet implemented for safety.       ║");
            warn!("║  To destroy a specific server, use your cloud console.   ║");
            warn!("╚══════════════════════════════════════════════════════════╝");
            anyhow::bail!(
                "Destroy command is disabled. \
                 To remove servers, delete them from the config and run 'apply'."
            );
        }
    }

    Ok(())
}

/// Validate IPRoyal residential proxy API token.
async fn validate_iproyal(api_token: &str) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    // IPRoyal dashboard API - check account info
    let resp = client
        .get("https://resi-api.iproyal.com/v1/me")
        .header("Authorization", format!("Bearer {}", api_token))
        .send()
        .await
        .context("Failed to reach IPRoyal API")?;

    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(anyhow::anyhow!(
            "IPRoyal API token is invalid (401 Unauthorized)"
        ));
    }

    resp.error_for_status()
        .context("IPRoyal validation request failed")?;

    info!("IPRoyal residential proxy API validated successfully");
    Ok(())
}
