use crate::blockchain::block::Block;
use crate::crypto::hash::Hash;
use crate::network::message::Message;
use crate::network::peer::PeerManager;
use crate::transaction::tx::Transaction;
use libp2p::{
    PeerId, Transport,
    core::upgrade,
    gossipsub::{
        Gossipsub, GossipsubConfig, GossipsubConfigBuilder, MessageAuthenticity, MessageId,
        ValidationMode,
    },
    identity,
    mdns::{Mdns, MdnsConfig, MdnsEvent},
    noise,
    swarm::{NetworkBehaviour, Swarm, SwarmBuilder},
    tcp::{Config as TcpConfig, GenTcpConfig},
    yamux,
};
use log::{debug, error, info, warn};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;

/// Topics for gossip protocol
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum GossipTopic {
    /// Transactions topic
    Transactions,

    /// Blocks topic
    Blocks,

    /// Consensus messages
    Consensus,
}

impl GossipTopic {
    /// Convert to a string representation
    fn as_str(&self) -> &'static str {
        match self {
            GossipTopic::Transactions => "solana-mini/transactions/1",
            GossipTopic::Blocks => "solana-mini/blocks/1",
            GossipTopic::Consensus => "solana-mini/consensus/1",
        }
    }

    /// Get all topics
    fn all() -> Vec<Self> {
        vec![
            GossipTopic::Transactions,
            GossipTopic::Blocks,
            GossipTopic::Consensus,
        ]
    }
}

/// Network behavior that combines gossipsub and mDNS
#[derive(NetworkBehaviour)]
struct NodeBehaviour {
    /// Gossip protocol for broadcasting messages
    gossipsub: Gossipsub,

    /// mDNS for peer discovery
    mdns: Mdns,
}

/// Network service for gossiping messages
pub struct GossipService {
    /// Peer manager
    peer_manager: Arc<PeerManager>,

    /// Set of seen transactions to avoid re-broadcasting
    seen_txs: Arc<Mutex<HashSet<Hash>>>,

    /// Set of seen blocks to avoid re-broadcasting
    seen_blocks: Arc<Mutex<HashSet<Hash>>>,

    /// Channel for outbound messages
    outbound_tx: mpsc::Sender<(PeerId, Message)>,

    /// Channel for outbound messages (receiver end)
    outbound_rx: Arc<Mutex<Option<mpsc::Receiver<(PeerId, Message)>>>>,

    /// Channel for inbound messages from the network
    inbound_tx: mpsc::Sender<(PeerId, Message)>,
}

impl GossipService {
    /// Create a new gossip service
    pub async fn new(
        listen_addr: &str,
        max_peers: usize,
        inbound_tx: mpsc::Sender<(PeerId, Message)>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Create communication channel
        let (outbound_tx, outbound_rx) = mpsc::channel(100);

        // Generate keypair for this node
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        info!("Local peer id: {}", local_peer_id);

        // Create peer manager
        let peer_manager = Arc::new(PeerManager::new(local_peer_id, outbound_tx.clone()));

        // Build transport
        let transport = libp2p::tcp::tokio::Transport::new(TcpConfig::default().nodelay(true))
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::NoiseAuthenticated::xx(&local_key).unwrap())
            .multiplex(yamux::YamuxConfig::default())
            .boxed();

        // Create gossipsub configuration
        let gossipsub_config = GossipsubConfigBuilder::default()
            .validation_mode(ValidationMode::Strict)
            .heartbeat_interval(Duration::from_secs(10))
            .mesh_n(6)
            .mesh_n_low(4)
            .mesh_n_high(12)
            .gossip_lazy(3)
            .history_length(5)
            .max_transmit_size(1024 * 1024) // 1 MB
            .build()
            .expect("Valid config");

        // Build gossipsub
        let message_authenticity = MessageAuthenticity::Signed(local_key.clone());
        let mut gossipsub =
            Gossipsub::new(message_authenticity, gossipsub_config).expect("Correct configuration");

        // Subscribe to topics
        for topic in GossipTopic::all() {
            let topic_id = libp2p::gossipsub::IdentTopic::new(topic.as_str());
            gossipsub.subscribe(&topic_id)?;
        }

        // Create mDNS for local network discovery
        let mdns = Mdns::new(MdnsConfig::default()).await?;

        // Create swarm
        let mut swarm = SwarmBuilder::with_tokio_executor(
            transport,
            NodeBehaviour { gossipsub, mdns },
            local_peer_id,
        )
        .build();

        // Listen on the given address
        swarm.listen_on(listen_addr.parse()?)?;

        // Spawn network task
        let peer_manager_clone = peer_manager.clone();
        let outbound_tx_clone = outbound_tx.clone();
        let inbound_tx_clone = inbound_tx.clone();
        let seen_txs = Arc::new(Mutex::new(HashSet::new()));
        let seen_blocks = Arc::new(Mutex::new(HashSet::new()));

        let seen_txs_clone = seen_txs.clone();
        let seen_blocks_clone = seen_blocks.clone();

        tokio::spawn(async move {
            let mut outbound_rx = outbound_rx;

            loop {
                tokio::select! {
                    // Process network events
                    event = swarm.select_next_some() => {
                        Self::handle_swarm_event(
                            event,
                            &mut swarm,
                            &peer_manager_clone,
                            &inbound_tx_clone,
                            &seen_txs_clone,
                            &seen_blocks_clone,
                        ).await;
                    }

                    // Process outbound messages
                    Some((peer_id, message)) = outbound_rx.lock().unwrap().as_mut().unwrap().recv() => {
                        Self::handle_outbound_message(
                            &mut swarm,
                            peer_id,
                            message,
                            &seen_txs_clone,
                            &seen_blocks_clone,
                        ).await;
                    }
                }
            }
        });

        // Create service
        let service = Self {
            peer_manager,
            seen_txs,
            seen_blocks,
            outbound_tx,
            outbound_rx: Arc::new(Mutex::new(Some(outbound_rx))),
            inbound_tx,
        };

        Ok(service)
    }

    /// Handle an event from the swarm
    async fn handle_swarm_event(
        event: libp2p::swarm::SwarmEvent<
            <NodeBehaviour as NetworkBehaviour>::ToSwarm,
            <NodeBehaviour as NetworkBehaviour>::ConnectionHandler,
        >,
        swarm: &mut Swarm<NodeBehaviour>,
        peer_manager: &PeerManager,
        inbound_tx: &mpsc::Sender<(PeerId, Message)>,
        seen_txs: &Arc<Mutex<HashSet<Hash>>>,
        seen_blocks: &Arc<Mutex<HashSet<Hash>>>,
    ) {
        // Handle different event types
        // This would be a full implementation in a real system
    }

    /// Handle an outbound message
    async fn handle_outbound_message(
        swarm: &mut Swarm<NodeBehaviour>,
        peer_id: PeerId,
        message: Message,
        seen_txs: &Arc<Mutex<HashSet<Hash>>>,
        seen_blocks: &Arc<Mutex<HashSet<Hash>>>,
    ) {
        // Handle different message types for direct or gossip communication
        match &message {
            Message::Transaction(tx) => {
                // Add to seen transactions
                let tx_hash = tx.hash();
                {
                    let mut seen = seen_txs.lock().unwrap();
                    seen.insert(tx_hash);
                }

                // Publish to gossipsub
                let topic = GossipTopic::Transactions.as_str();
                let topic_id = libp2p::gossipsub::IdentTopic::new(topic);

                let encoded = bincode::serialize(&message).unwrap_or_default();
                if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic_id, encoded) {
                    error!("Failed to publish transaction: {}", e);
                }
            }
            Message::Block(block) => {
                // Add to seen blocks
                let block_hash = block.hash();
                {
                    let mut seen = seen_blocks.lock().unwrap();
                    seen.insert(block_hash);
                }

                // Publish to gossipsub
                let topic = GossipTopic::Blocks.as_str();
                let topic_id = libp2p::gossipsub::IdentTopic::new(topic);

                let encoded = bincode::serialize(&message).unwrap_or_default();
                if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic_id, encoded) {
                    error!("Failed to publish block: {}", e);
                }
            }
            _ => {
                // Direct message to peer
                let encoded = bincode::serialize(&message).unwrap_or_default();
                if let Err(e) = swarm
                    .behaviour_mut()
                    .gossipsub
                    .publish_direct(peer_id, encoded)
                {
                    error!("Failed to send direct message: {}", e);
                }
            }
        }
    }

    /// Get the peer manager
    pub fn peer_manager(&self) -> Arc<PeerManager> {
        self.peer_manager.clone()
    }

    /// Broadcast a transaction to the network
    pub async fn broadcast_transaction(&self, transaction: Transaction) {
        let tx_hash = transaction.hash();

        // Check if already seen
        {
            let seen = self.seen_txs.lock().unwrap();
            if seen.contains(&tx_hash) {
                return;
            }
        }

        // Add to seen transactions
        {
            let mut seen = self.seen_txs.lock().unwrap();
            seen.insert(tx_hash);
        }

        // Broadcast to all peers
        self.peer_manager
            .broadcast(Message::Transaction(transaction))
            .await;
    }

    /// Broadcast a block to the network
    pub async fn broadcast_block(&self, block: Block) {
        let block_hash = block.hash();

        // Check if already seen
        {
            let seen = self.seen_blocks.lock().unwrap();
            if seen.contains(&block_hash) {
                return;
            }
        }

        // Add to seen blocks
        {
            let mut seen = self.seen_blocks.lock().unwrap();
            seen.insert(block_hash);
        }

        // Broadcast to all peers
        self.peer_manager.broadcast(Message::Block(block)).await;
    }

    /// Request a block by hash from any peer that has it
    pub async fn request_block(&self, hash: Hash) {
        // Get all peers
        let peers = self.peer_manager.get_all_peers();

        // Try to request from a random peer
        if !peers.is_empty() {
            let peer_id = peers[rand::random::<usize>() % peers.len()].id;
            let _ = self
                .outbound_tx
                .try_send((peer_id, Message::GetBlock { hash }));
        }
    }

    /// Request the latest block from any peer
    pub async fn request_latest_block(&self) {
        // Get all peers
        let peers = self.peer_manager.get_all_peers();

        // Try to request from a random peer
        if !peers.is_empty() {
            let peer_id = peers[rand::random::<usize>() % peers.len()].id;
            let _ = self
                .outbound_tx
                .try_send((peer_id, Message::GetLatestBlock));
        }
    }

    /// Add a known peer to connect to
    pub async fn add_peer(&self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Parse address
        let addr = addr.parse()?;

        // Add to known peers
        // In a real implementation, this would dial the peer

        Ok(())
    }
}
