use crate::crypto::hash::Hash;
use crate::network::message::Message;
use libp2p::{Multiaddr, PeerId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Maximum allowed latency for a peer
const MAX_PEER_LATENCY: Duration = Duration::from_millis(1000);

/// Information about a peer
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Peer ID
    pub id: PeerId,

    /// Peer address
    pub addr: Multiaddr,

    /// When the peer was first seen
    pub first_seen: Instant,

    /// When the peer was last seen
    pub last_seen: Instant,

    /// Peer's reported version
    pub version: Option<String>,

    /// Peer's reported blockchain height
    pub height: Option<u64>,

    /// Peer's reported genesis hash
    pub genesis_hash: Option<Hash>,

    /// Average latency to this peer
    pub latency: Duration,

    /// Number of failed ping attempts
    pub failed_pings: u32,
}

/// Manager for peer connections
pub struct PeerManager {
    /// Our node ID
    node_id: PeerId,

    /// Connected peers
    peers: Arc<Mutex<HashMap<PeerId, PeerInfo>>>,

    /// Outbound message channel
    outbound_tx: mpsc::Sender<(PeerId, Message)>,
}

impl PeerManager {
    /// Create a new peer manager
    pub fn new(node_id: PeerId, outbound_tx: mpsc::Sender<(PeerId, Message)>) -> Self {
        Self {
            node_id,
            peers: Arc::new(Mutex::new(HashMap::new())),
            outbound_tx,
        }
    }

    /// Add a new peer
    pub fn add_peer(&self, id: PeerId, addr: Multiaddr) {
        let now = Instant::now();
        let peer_info = PeerInfo {
            id,
            addr,
            first_seen: now,
            last_seen: now,
            version: None,
            height: None,
            genesis_hash: None,
            latency: Duration::from_secs(0),
            failed_pings: 0,
        };

        let mut peers = self.peers.lock().unwrap();
        peers.insert(id, peer_info);

        // Request peer information
        let _ = self.outbound_tx.try_send((id, Message::GetPeerInfo));
    }

    /// Remove a peer
    pub fn remove_peer(&self, id: &PeerId) {
        let mut peers = self.peers.lock().unwrap();
        peers.remove(id);
    }

    /// Update peer information when a message is received
    pub fn update_peer(&self, id: &PeerId, message: &Message) {
        let mut peers = self.peers.lock().unwrap();

        if let Some(peer) = peers.get_mut(id) {
            peer.last_seen = Instant::now();

            match message {
                Message::PeerInfo {
                    version,
                    height,
                    genesis_hash,
                } => {
                    peer.version = Some(version.clone());
                    peer.height = Some(*height);
                    peer.genesis_hash = Some(*genesis_hash);
                }
                Message::Pong { nonce: _ } => {
                    // In a real implementation, we would measure latency
                    // by comparing the current time with when the ping was sent
                    peer.latency = Duration::from_millis(100); // Example value
                    peer.failed_pings = 0;
                }
                _ => {}
            }
        }
    }

    /// Check if a peer exists
    pub fn has_peer(&self, id: &PeerId) -> bool {
        let peers = self.peers.lock().unwrap();
        peers.contains_key(id)
    }

    /// Get information about a peer
    pub fn get_peer(&self, id: &PeerId) -> Option<PeerInfo> {
        let peers = self.peers.lock().unwrap();
        peers.get(id).cloned()
    }

    /// Get all peers
    pub fn get_all_peers(&self) -> Vec<PeerInfo> {
        let peers = self.peers.lock().unwrap();
        peers.values().cloned().collect()
    }

    /// Get the number of connected peers
    pub fn peer_count(&self) -> usize {
        let peers = self.peers.lock().unwrap();
        peers.len()
    }

    /// Send a message to a specific peer
    pub async fn send_to_peer(
        &self,
        id: &PeerId,
        message: Message,
    ) -> Result<(), mpsc::error::TrySendError<(PeerId, Message)>> {
        self.outbound_tx.try_send((*id, message))
    }

    /// Broadcast a message to all peers
    pub async fn broadcast(&self, message: Message) {
        let peers = self.peers.lock().unwrap();

        for id in peers.keys() {
            let _ = self.outbound_tx.try_send((*id, message.clone()));
        }
    }

    /// Ping all peers to check connection
    pub async fn ping_all_peers(&self) {
        let peers = {
            let peers = self.peers.lock().unwrap();
            peers.keys().cloned().collect::<Vec<_>>()
        };

        for peer_id in peers {
            let nonce = rand::random::<u64>();
            let _ = self
                .outbound_tx
                .try_send((peer_id, Message::Ping { nonce }));
        }
    }

    /// Prune inactive peers
    pub fn prune_inactive_peers(&self, max_inactivity: Duration) {
        let now = Instant::now();
        let mut peers = self.peers.lock().unwrap();

        // Collect peers to remove
        let to_remove: Vec<PeerId> = peers
            .iter()
            .filter(|(_, info)| now.duration_since(info.last_seen) > max_inactivity)
            .map(|(id, _)| *id)
            .collect();

        // Remove inactive peers
        for id in to_remove {
            peers.remove(&id);
        }
    }
}
