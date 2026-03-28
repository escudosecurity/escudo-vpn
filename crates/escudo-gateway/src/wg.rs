use tokio::process::Command;
use tracing::{error, info};

pub struct WgManager {
    interface: String,
}

#[derive(Debug, Clone)]
pub struct PeerStats {
    pub public_key: String,
    pub allowed_ip: String,
    pub last_handshake: i64,
    pub rx_bytes: i64,
    pub tx_bytes: i64,
}

impl WgManager {
    pub fn new(interface: &str) -> Self {
        Self {
            interface: interface.to_string(),
        }
    }

    pub async fn add_peer(
        &self,
        public_key: &str,
        allowed_ip: &str,
        preshared_key: &str,
    ) -> anyhow::Result<()> {
        // Use wg directly with separate args to avoid command injection
        let allowed_ip_cidr = format!("{}/32", allowed_ip);
        let mut child = Command::new("wg")
            .args([
                "set",
                &self.interface,
                "peer",
                public_key,
                "preshared-key",
                "/dev/stdin",
                "allowed-ips",
                &allowed_ip_cidr,
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        // Write PSK to stdin
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(preshared_key.as_bytes()).await?;
            // Drop stdin to close it, signaling EOF
        }

        let output = child.wait_with_output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("wg set failed: {stderr}");
            anyhow::bail!("Failed to add peer: {stderr}");
        }

        info!("Added peer {public_key} with IP {allowed_ip}");
        Ok(())
    }

    pub async fn remove_peer(&self, public_key: &str) -> anyhow::Result<()> {
        let output = Command::new("wg")
            .args(["set", &self.interface, "peer", public_key, "remove"])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("wg set remove failed: {stderr}");
            anyhow::bail!("Failed to remove peer: {stderr}");
        }

        info!("Removed peer {public_key}");
        Ok(())
    }

    async fn peer_allowed_ips(&self, public_key: &str) -> anyhow::Result<Vec<String>> {
        let output = Command::new("wg")
            .args(["show", &self.interface, "allowed-ips"])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to inspect peer allowed IPs: {stderr}");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let mut parts = line.split_whitespace();
            let Some(peer_key) = parts.next() else {
                continue;
            };
            if peer_key != public_key {
                continue;
            }
            let ips = parts
                .flat_map(|part| part.split(','))
                .map(str::trim)
                .filter(|ip| !ip.is_empty())
                .map(ToOwned::to_owned)
                .collect();
            return Ok(ips);
        }

        Ok(Vec::new())
    }

    pub async fn list_peers(&self) -> anyhow::Result<Vec<PeerStats>> {
        let output = Command::new("wg")
            .args(["show", &self.interface, "dump"])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to list peers: {stderr}");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut peers = Vec::new();

        for line in stdout.lines().skip(1) {
            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() >= 8 {
                peers.push(PeerStats {
                    public_key: fields[0].to_string(),
                    allowed_ip: fields[3].trim_end_matches("/32").to_string(),
                    last_handshake: fields[4].parse().unwrap_or(0),
                    rx_bytes: fields[5].parse().unwrap_or(0),
                    tx_bytes: fields[6].parse().unwrap_or(0),
                });
            }
        }

        Ok(peers)
    }

    pub async fn add_forwarding_peer(
        &self,
        exit_public_key: &str,
        exit_endpoint: &str,
    ) -> anyhow::Result<()> {
        let output = Command::new("wg")
            .args([
                "set",
                &self.interface,
                "peer",
                exit_public_key,
                "endpoint",
                exit_endpoint,
                "allowed-ips",
                "0.0.0.0/0",
            ])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("wg set forwarding peer failed: {stderr}");
            anyhow::bail!("Failed to add forwarding peer: {stderr}");
        }

        info!("Added forwarding peer to exit server {exit_endpoint}");
        Ok(())
    }

    pub async fn add_exit_peer(
        &self,
        entry_public_key: &str,
        allowed_ip: &str,
    ) -> anyhow::Result<()> {
        let allowed_ip_cidr = format!("{allowed_ip}/32");
        let mut allowed_ips = self.peer_allowed_ips(entry_public_key).await?;
        if !allowed_ips.iter().any(|ip| ip == &allowed_ip_cidr) {
            allowed_ips.push(allowed_ip_cidr);
        }
        let allowed_ips_csv = allowed_ips.join(",");

        let output = Command::new("wg")
            .args([
                "set",
                &self.interface,
                "peer",
                entry_public_key,
                "allowed-ips",
                &allowed_ips_csv,
            ])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("wg set exit peer failed: {stderr}");
            anyhow::bail!("Failed to add exit peer: {stderr}");
        }

        info!("Added exit peer {entry_public_key} for {allowed_ip}");
        Ok(())
    }

    pub async fn add_multihop_source_route(&self, allowed_ip: &str) -> anyhow::Result<()> {
        let allowed_ip_cidr = format!("{allowed_ip}/32");

        let _ = Command::new("iptables")
            .args([
                "-C",
                "FORWARD",
                "-i",
                &self.interface,
                "-o",
                &self.interface,
                "-j",
                "ACCEPT",
            ])
            .output()
            .await;

        let output = Command::new("iptables")
            .args([
                "-C",
                "FORWARD",
                "-i",
                &self.interface,
                "-o",
                &self.interface,
                "-j",
                "ACCEPT",
            ])
            .output()
            .await?;
        if !output.status.success() {
            let add = Command::new("iptables")
                .args([
                    "-I",
                    "FORWARD",
                    "1",
                    "-i",
                    &self.interface,
                    "-o",
                    &self.interface,
                    "-j",
                    "ACCEPT",
                ])
                .output()
                .await?;
            if !add.status.success() {
                let stderr = String::from_utf8_lossy(&add.stderr);
                anyhow::bail!("Failed to allow multihop forwarding: {stderr}");
            }
        }

        let route = Command::new("ip")
            .args([
                "route",
                "replace",
                "default",
                "dev",
                &self.interface,
                "table",
                "200",
            ])
            .output()
            .await?;
        if !route.status.success() {
            let stderr = String::from_utf8_lossy(&route.stderr);
            anyhow::bail!("Failed to install multihop route table: {stderr}");
        }

        let rule = Command::new("ip")
            .args([
                "rule",
                "add",
                "from",
                &allowed_ip_cidr,
                "table",
                "200",
                "priority",
                "10000",
            ])
            .output()
            .await?;
        if !rule.status.success() {
            let stderr = String::from_utf8_lossy(&rule.stderr);
            if !stderr.contains("File exists") {
                anyhow::bail!("Failed to install multihop source rule: {stderr}");
            }
        }

        info!("Installed multihop source route for {allowed_ip}");
        Ok(())
    }

    pub async fn get_aggregate_stats(&self) -> anyhow::Result<(i32, i64, i64)> {
        let peers = self.list_peers().await?;
        let total = peers.len() as i32;
        let rx: i64 = peers.iter().map(|p| p.rx_bytes).sum();
        let tx: i64 = peers.iter().map(|p| p.tx_bytes).sum();
        Ok((total, rx, tx))
    }
}

pub struct MultiWgManager {
    pub wg0: WgManager, // Free/Escudo
    pub wg1: WgManager, // Pro
    pub wg2: WgManager, // Dedicated
}

impl MultiWgManager {
    pub fn new(wg0_iface: &str, wg1_iface: &str, wg2_iface: &str) -> Self {
        Self {
            wg0: WgManager::new(wg0_iface),
            wg1: WgManager::new(wg1_iface),
            wg2: WgManager::new(wg2_iface),
        }
    }

    /// Select WireGuard interface by tier
    /// FREE (0) and ESCUDO (1) → wg0
    /// PRO (2) → wg1
    /// DEDICATED (3) → wg2
    pub fn for_tier(&self, tier: i32) -> &WgManager {
        match tier {
            2 => &self.wg1,
            3 => &self.wg2,
            _ => &self.wg0,
        }
    }

    pub async fn get_aggregate_stats(&self) -> anyhow::Result<(i32, i64, i64)> {
        let (p0, rx0, tx0) = self.wg0.get_aggregate_stats().await?;
        let (p1, rx1, tx1) = self.wg1.get_aggregate_stats().await?;
        let (p2, rx2, tx2) = self.wg2.get_aggregate_stats().await?;
        Ok((p0 + p1 + p2, rx0 + rx1 + rx2, tx0 + tx1 + tx2))
    }
}
