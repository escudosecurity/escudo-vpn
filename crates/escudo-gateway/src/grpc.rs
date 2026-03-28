use std::sync::Arc;
use std::time::Instant;
use tonic::{Request, Response, Status};

use crate::proxy::{ProxyCredential, ProxyManager, ProxyTargetKind};
use crate::stats::Metrics;
use crate::wg::MultiWgManager;

pub mod gateway {
    tonic::include_proto!("gateway");
}

use gateway::gateway_service_server::GatewayService;
use gateway::{
    AddExitPeerRequest, AddExitPeerResponse, AddMultihopPeerRequest, AddMultihopPeerResponse,
    AddPeerRequest, AddPeerResponse, GetStatsRequest, GetStatsResponse, ListPeersRequest,
    ListPeersResponse, PeerInfo, RemovePeerRequest, RemovePeerResponse,
    UpdateProxyCredentialsRequest, UpdateProxyCredentialsResponse,
};

pub struct GatewayServiceImpl {
    pub wg: Arc<MultiWgManager>,
    pub start_time: Instant,
    pub metrics: Metrics,
    pub proxy: Option<Arc<ProxyManager>>,
}

#[tonic::async_trait]
impl GatewayService for GatewayServiceImpl {
    async fn add_peer(
        &self,
        request: Request<AddPeerRequest>,
    ) -> Result<Response<AddPeerResponse>, Status> {
        let req = request.into_inner();
        let iface = self.wg.for_tier(req.tier);

        iface
            .add_peer(&req.public_key, &req.allowed_ip, &req.preshared_key)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        self.metrics.connections_total.inc();

        Ok(Response::new(AddPeerResponse {
            success: true,
            message: "Peer added".to_string(),
        }))
    }

    async fn remove_peer(
        &self,
        request: Request<RemovePeerRequest>,
    ) -> Result<Response<RemovePeerResponse>, Status> {
        let req = request.into_inner();

        // Try removing from all three interfaces — we don't know which one the peer is on
        let r0 = self.wg.wg0.remove_peer(&req.public_key).await;
        let r1 = self.wg.wg1.remove_peer(&req.public_key).await;
        let r2 = self.wg.wg2.remove_peer(&req.public_key).await;

        // Succeed if at least one interface accepted the removal
        if r0.is_err() && r1.is_err() && r2.is_err() {
            return Err(Status::internal(format!(
                "Failed to remove peer from any interface: wg0={} wg1={} wg2={}",
                r0.unwrap_err(),
                r1.unwrap_err(),
                r2.unwrap_err(),
            )));
        }

        Ok(Response::new(RemovePeerResponse {
            success: true,
            message: "Peer removed".to_string(),
        }))
    }

    async fn list_peers(
        &self,
        _request: Request<ListPeersRequest>,
    ) -> Result<Response<ListPeersResponse>, Status> {
        let peers0 = self
            .wg
            .wg0
            .list_peers()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        let peers1 = self
            .wg
            .wg1
            .list_peers()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        let peers2 = self
            .wg
            .wg2
            .list_peers()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let peer_infos: Vec<PeerInfo> = peers0
            .into_iter()
            .chain(peers1)
            .chain(peers2)
            .map(|p| PeerInfo {
                public_key: p.public_key,
                allowed_ip: p.allowed_ip,
                last_handshake: p.last_handshake,
                rx_bytes: p.rx_bytes,
                tx_bytes: p.tx_bytes,
            })
            .collect();

        Ok(Response::new(ListPeersResponse { peers: peer_infos }))
    }

    async fn get_stats(
        &self,
        _request: Request<GetStatsRequest>,
    ) -> Result<Response<GetStatsResponse>, Status> {
        let (total_peers, total_rx, total_tx) = self
            .wg
            .get_aggregate_stats()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        // get_aggregate_stats sums across all three interfaces (wg0 + wg1 + wg2)

        Ok(Response::new(GetStatsResponse {
            total_peers,
            total_rx_bytes: total_rx,
            total_tx_bytes: total_tx,
            uptime_seconds: self.start_time.elapsed().as_secs_f64(),
        }))
    }

    async fn add_multihop_peer(
        &self,
        request: Request<AddMultihopPeerRequest>,
    ) -> Result<Response<AddMultihopPeerResponse>, Status> {
        let req = request.into_inner();

        // Multihop uses wg0 (default interface)
        // Add local peer
        self.wg
            .wg0
            .add_peer(&req.public_key, &req.allowed_ip, &req.preshared_key)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        self.wg
            .wg0
            .add_multihop_source_route(&req.allowed_ip)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Configure forwarding to exit server
        self.wg
            .wg0
            .add_forwarding_peer(&req.exit_server_public_key, &req.exit_server_endpoint)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        self.metrics.connections_total.inc();

        Ok(Response::new(AddMultihopPeerResponse {
            success: true,
            message: "Multi-hop peer added".to_string(),
        }))
    }

    async fn add_exit_peer(
        &self,
        request: Request<AddExitPeerRequest>,
    ) -> Result<Response<AddExitPeerResponse>, Status> {
        let req = request.into_inner();

        self.wg
            .wg0
            .add_exit_peer(&req.entry_server_public_key, &req.allowed_ip)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(AddExitPeerResponse {
            success: true,
            message: "Exit peer added".to_string(),
        }))
    }

    async fn update_proxy_credentials(
        &self,
        request: Request<UpdateProxyCredentialsRequest>,
    ) -> Result<Response<UpdateProxyCredentialsResponse>, Status> {
        let req = request.into_inner();
        let manager = self
            .proxy
            .as_ref()
            .ok_or_else(|| Status::failed_precondition("proxy manager is disabled"))?;
        let credential = ProxyCredential {
            host: req.socks5_host,
            port: u16::try_from(req.socks5_port)
                .map_err(|_| Status::invalid_argument("proxy port out of range"))?,
            username: req.socks5_username,
            password: req.socks5_password,
        };
        manager
            .update_credentials(ProxyTargetKind::from_proto(req.target), credential)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(UpdateProxyCredentialsResponse {
            success: true,
            error: String::new(),
        }))
    }
}
