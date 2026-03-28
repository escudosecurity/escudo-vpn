use anyhow::Result;
use tracing::warn;

/// Placeholder: update the escudo-gateway binary on a running server via SSH.
///
/// Future implementation will:
/// 1. SSH into the server using the provided key
/// 2. Upload the new binary
/// 3. Restart the escudo-gateway systemd service
pub async fn update_server_binary(
    public_ip: &str,
    ssh_key_path: &str,
    binary_path: &str,
) -> Result<()> {
    warn!(
        ip = %public_ip,
        key = %ssh_key_path,
        binary = %binary_path,
        "update_server_binary: SSH-based binary update is not yet implemented"
    );
    Ok(())
}
