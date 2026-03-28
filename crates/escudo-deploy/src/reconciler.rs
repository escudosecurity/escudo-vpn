use crate::cloudinit::generate_cloudinit;
use crate::config::{DeployConfig, ServerEntry};
use crate::provider::{ProvisionedServer, ServerProvider};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, warn};

pub struct Reconciler {
    pub providers: HashMap<String, Arc<dyn ServerProvider>>,
    pub config: DeployConfig,
    pub deploy_secret: String,
    pub home_url: String,
}

pub struct PlanResult {
    pub to_create: Vec<ServerEntry>,
    pub to_destroy: Vec<ProvisionedServer>,
    pub up_to_date: Vec<ProvisionedServer>,
}

impl PlanResult {
    pub fn print(&self) {
        if self.to_create.is_empty() && self.to_destroy.is_empty() {
            println!("Plan: No changes required. Infrastructure is up to date.");
            return;
        }

        println!(
            "Plan: {} to create, {} to destroy, {} unchanged",
            self.to_create.len(),
            self.to_destroy.len(),
            self.up_to_date.len()
        );

        for entry in &self.to_create {
            println!(
                "  + CREATE  {} ({}/{}) on {}",
                entry.label, entry.provider, entry.plan, entry.region
            );
        }

        for server in &self.to_destroy {
            println!(
                "  - DESTROY {} ({}/{}) [id: {}]",
                server.label, server.provider, server.plan, server.provider_instance_id
            );
        }

        for server in &self.up_to_date {
            println!(
                "  = OK      {} ({}/{}) ip={}",
                server.label,
                server.provider,
                server.plan,
                server.public_ip.as_deref().unwrap_or("pending")
            );
        }
    }
}

pub struct ApplyResult {
    pub created: Vec<ProvisionedServer>,
    pub destroyed: Vec<String>,
    pub errors: Vec<String>,
}

impl Reconciler {
    pub fn new(
        providers: HashMap<String, Arc<dyn ServerProvider>>,
        config: DeployConfig,
        deploy_secret: String,
        home_url: String,
    ) -> Self {
        Self {
            providers,
            config,
            deploy_secret,
            home_url,
        }
    }

    /// Query all providers and return current state keyed by server label.
    pub async fn get_actual_state(&self) -> Result<HashMap<String, ProvisionedServer>> {
        let mut state: HashMap<String, ProvisionedServer> = HashMap::new();

        for (name, provider) in &self.providers {
            match provider.list_servers().await {
                Ok(servers) => {
                    info!(provider = %name, count = servers.len(), "Listed servers");
                    for server in servers {
                        state.insert(server.label.clone(), server);
                    }
                }
                Err(e) => {
                    warn!(provider = %name, error = %e, "Failed to list servers from provider");
                }
            }
        }

        Ok(state)
    }

    /// Compute the diff between desired and actual state.
    pub async fn plan(&self) -> Result<PlanResult> {
        let actual = self.get_actual_state().await?;

        let desired_labels: HashMap<&str, &ServerEntry> = self
            .config
            .servers
            .iter()
            .map(|s| (s.label.as_str(), s))
            .collect();

        let mut to_create: Vec<ServerEntry> = Vec::new();
        let mut up_to_date: Vec<ProvisionedServer> = Vec::new();

        for entry in &self.config.servers {
            if let Some(server) = actual.get(&entry.label) {
                up_to_date.push(server.clone());
            } else {
                to_create.push(entry.clone());
            }
        }

        let to_destroy: Vec<ProvisionedServer> = actual
            .values()
            .filter(|s| !desired_labels.contains_key(s.label.as_str()))
            .cloned()
            .collect();

        Ok(PlanResult {
            to_create,
            to_destroy,
            up_to_date,
        })
    }

    /// Apply changes: create missing servers, destroy extras.
    pub async fn apply(&self) -> Result<ApplyResult> {
        let plan = self.plan().await?;
        plan.print();

        let mut created: Vec<ProvisionedServer> = Vec::new();
        let mut destroyed: Vec<String> = Vec::new();
        let mut errors: Vec<String> = Vec::new();

        // Create missing servers
        for entry in &plan.to_create {
            let provider = match self.providers.get(&entry.provider) {
                Some(p) => p,
                None => {
                    let msg = format!(
                        "Unknown provider '{}' for server '{}'",
                        entry.provider, entry.label
                    );
                    error!("{}", msg);
                    errors.push(msg);
                    continue;
                }
            };

            let user_data = generate_cloudinit(&entry.label, &self.deploy_secret, &self.home_url);
            let tags = self.config.defaults.tags.clone();

            info!(label = %entry.label, provider = %entry.provider, "Creating server");

            match provider
                .create_server(&entry.label, &entry.region, &entry.plan, &user_data, &tags)
                .await
            {
                Ok(server) => {
                    info!(
                        label = %server.label,
                        id = %server.provider_instance_id,
                        "Server created successfully"
                    );
                    created.push(server);
                }
                Err(e) => {
                    let msg = format!("Failed to create server '{}': {}", entry.label, e);
                    error!("{}", msg);
                    errors.push(msg);
                }
            }
        }

        // Destroy extra servers
        for server in &plan.to_destroy {
            let provider = match self.providers.get(&server.provider) {
                Some(p) => p,
                None => {
                    let msg = format!(
                        "Unknown provider '{}' for server '{}' (id: {})",
                        server.provider, server.label, server.provider_instance_id
                    );
                    error!("{}", msg);
                    errors.push(msg);
                    continue;
                }
            };

            warn!(
                label = %server.label,
                id = %server.provider_instance_id,
                "Destroying server not in desired config"
            );

            match provider.destroy_server(&server.provider_instance_id).await {
                Ok(()) => {
                    destroyed.push(server.label.clone());
                }
                Err(e) => {
                    let msg = format!("Failed to destroy server '{}': {}", server.label, e);
                    error!("{}", msg);
                    errors.push(msg);
                }
            }
        }

        Ok(ApplyResult {
            created,
            destroyed,
            errors,
        })
    }

    /// Print current status of all managed servers.
    pub async fn status(&self) -> Result<()> {
        let actual = self.get_actual_state().await?;

        if actual.is_empty() {
            println!("No servers found across all providers.");
            return Ok(());
        }

        println!(
            "{:<20} {:<10} {:<12} {:<15} {:<10} {:<8}",
            "LABEL", "PROVIDER", "REGION", "IP", "STATUS", "$/MO"
        );
        println!("{}", "-".repeat(80));

        let mut servers: Vec<&ProvisionedServer> = actual.values().collect();
        servers.sort_by_key(|s| &s.label);

        for server in servers {
            println!(
                "{:<20} {:<10} {:<12} {:<15} {:<10} {:.2}",
                server.label,
                server.provider,
                server.region,
                server.public_ip.as_deref().unwrap_or("pending"),
                server.status,
                server.monthly_cost_usd
            );
        }

        let total_cost: f64 = actual.values().map(|s| s.monthly_cost_usd).sum();
        println!("{}", "-".repeat(80));
        println!("Total: {} servers, ${:.2}/month", actual.len(), total_cost);

        Ok(())
    }
}
