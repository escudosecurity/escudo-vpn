use std::io::Write;
use std::process::{Command, Stdio};

pub struct KillSwitch {
    active: bool,
}

impl KillSwitch {
    pub fn new() -> Self {
        Self { active: false }
    }

    pub fn enable(
        &mut self,
        tunnel_ip: &str,
        endpoint_ip: &str,
        dns_server: &str,
    ) -> Result<(), String> {
        let ruleset = format!(
            r#"table inet escudo_killswitch {{
    chain output {{
        type filter hook output priority 0; policy drop;

        # Allow loopback
        oifname "lo" accept

        # Allow traffic to VPN endpoint (WireGuard handshake)
        ip daddr {endpoint_ip} udp accept
        ip daddr {endpoint_ip} tcp accept

        # Allow DNS only to tunnel DNS server (prevent DNS leaks)
        ip daddr {dns_server} udp dport 53 accept
        ip daddr {dns_server} tcp dport 53 accept

        # Allow all traffic from tunnel IP (VPN is up)
        ip saddr {tunnel_ip} accept

        # Block all IPv6 OUTPUT except loopback (prevent IPv6 leaks)
        ip6 daddr ::1 accept
        ip6 daddr != ::1 drop

        # Everything else is dropped by policy
    }}
}}"#
        );

        run_nft_ruleset(&ruleset)?;
        self.active = true;
        Ok(())
    }

    pub fn disable(&mut self) -> Result<(), String> {
        if self.active {
            run_nft(&["delete", "table", "inet", "escudo_killswitch"])?;
            self.active = false;
        }
        Ok(())
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}

impl Drop for KillSwitch {
    fn drop(&mut self) {
        let _ = self.disable();
    }
}

fn run_nft_ruleset(ruleset: &str) -> Result<(), String> {
    // Atomic ruleset load via stdin
    let mut child = Command::new("nft")
        .args(["-f", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn nft: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(ruleset.as_bytes())
            .map_err(|e| format!("Failed to write ruleset: {e}"))?;
    }

    let output = child.wait_with_output().map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}

fn run_nft(args: &[&str]) -> Result<(), String> {
    let output = Command::new("nft")
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}
