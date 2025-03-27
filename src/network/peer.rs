use crate::crypto::hash::Hash;
use crate::network::message::Message;
use libp2p::{Multiaddr, PeerId};
use log::{debug, info, warn};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Maximum allowed latency for a peer
const MAX_PEER_LATENCY: Duration = Duration::from_millis(1000);

/// Information about a connected peer
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Peer ID
    pub id: PeerId,
    
    /// Time when the peer was connected
    pub connected_since: Instant,
    
    /// Peer's reported blockchain height
    pub height: Option<u64>,
    
    /// Peer's reported version
    pub version: Option<String>,
    
    /// Last time we received a message from this peer
    pub last_seen: Instant,
}

/// Manager for connected peers
pub struct PeerManager {
    /// Local node's peer ID
    local_id: PeerId,
    
    /// Connected peers
    peers: HashMap<PeerId, PeerInfo>,
    
    /// Channel for outbound messages
    outbound_tx: mpsc::Sender<(PeerId, Message)>,
}

impl PeerManager {
    /// Create a new peer manager
    pub fn new(local_id: PeerId, outbound_tx: mpsc::Sender<(PeerId, Message)>) -> Self {
        Self {
            local_id,
            peers: HashMap::new(),
            outbound_tx,
        }
    }
    
    /// Add a new peer
    pub fn add_peer(&mut self, peer_id: PeerId) {
        if peer_id == self.local_id {
            return; // Don't add ourselves
        }
        
        let now = Instant::now();
        
        let info = PeerInfo {
            id: peer_id,
            connected_since: now,
            height: None,
            version: None,
            last_seen: now,
        };
        
        self.peers.insert(peer_id, info);
        
        info!("Added peer: {}", peer_id);
    }
    
    /// Remove a peer
    pub fn remove_peer(&mut self, peer_id: &PeerId) {
        if self.peers.remove(peer_id).is_some() {
            info!("Removed peer: {}", peer_id);
        }
    }
    
    /// Update peer information
    pub fn update_peer(&mut self, peer_id: &PeerId, height: Option<u64>, version: Option<String>) {
        if let Some(info) = self.peers.get_mut(peer_id) {
            if let Some(h) = height {
                info.height = Some(h);
            }
            
            if let Some(v) = version {
                info.version = Some(v);
            }
            
            info.last_seen = Instant::now();
        }
    }
    
    /// Mark peer as seen
    pub fn mark_peer_seen(&mut self, peer_id: &PeerId) {
        if let Some(info) = self.peers.get_mut(peer_id) {
            info.last_seen = Instant::now();
        }
    }
    
    /// Get information about a peer
    pub fn get_peer(&self, peer_id: &PeerId) -> Option<&PeerInfo> {
        self.peers.get(peer_id)
    }
    
    /// Get all connected peers
    pub fn get_all_peers(&self) -> Vec<PeerInfo> {
        self.peers.values().cloned().collect()
    }
    
    /// Get the number of connected peers
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }
    
    /// Send a message to a specific peer
    pub async fn send_to_peer(&self, peer_id: PeerId, message: Message) {
        if !self.peers.contains_key(&peer_id) {
            warn!("Attempted to send message to unknown peer: {}", peer_id);
            return;
        }
        
        if let Err(e) = self.outbound_tx.send((peer_id, message)).await {
            warn!("Failed to send message to peer {}: {}", peer_id, e);
        }
    }
    
    /// Broadcast a message to all peers
    pub async fn broadcast(&self, message: Message) {
        for peer_id in self.peers.keys() {
            let _ = self.outbound_tx.send((*peer_id, message.clone())).await;
        }
    }
    
    /// Clean up inactive peers
    pub fn cleanup_inactive_peers(&mut self, timeout: Duration) {
        let now = Instant::now();
        let to_remove: Vec<PeerId> = self.peers
            .iter()
            .filter(|(_, info)| now.duration_since(info.last_seen) > timeout)
            .map(|(id, _)| *id)
            .collect();
        
        for peer_id in to_remove {
            self.remove_peer(&peer_id);
        }
    }
}
