use crate::blockchain::block::Block;
use crate::crypto::hash::{Hash, Hashable};
use crate::network::message::Message;
use crate::network::peer::{self, PeerManager};
use crate::transaction::tx::Transaction;
use libp2p::futures::StreamExt;
use libp2p::identity::ed25519::Keypair;
use libp2p::Multiaddr;
use libp2p::{
    core::upgrade,
    gossipsub::{
        error::GossipsubHandlerError, Gossipsub, GossipsubConfigBuilder, IdentTopic as Topic,
        MessageAuthenticity, ValidationMode,
    },
    identity,
    mdns::{Mdns, MdnsConfig},
    noise,
    swarm::{Swarm, SwarmBuilder},
    tcp::TcpConfig,
    yamux, NetworkBehaviour, PeerId, Transport,
};
use log::{debug, error, info, warn};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
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
#[behaviour(out_event = "NodeBehaviourEvent")]
struct NodeBehaviour {
    /// Gossip protocol for broadcasting messages
    gossipsub: Gossipsub,

    /// mDNS for peer discovery
    mdns: Mdns,
}

impl From<libp2p::gossipsub::GossipsubEvent> for NodeBehaviourEvent {
    fn from(event: libp2p::gossipsub::GossipsubEvent) -> Self {
        NodeBehaviourEvent::Gossipsub(event)
    }
}

impl From<libp2p::mdns::MdnsEvent> for NodeBehaviourEvent {
    fn from(event: libp2p::mdns::MdnsEvent) -> Self {
        NodeBehaviourEvent::Mdns(event)
    }
}

/// Events produced by the combined network behavior
#[derive(Debug)]
enum NodeBehaviourEvent {
    /// Events from the gossipsub protocol
    Gossipsub(libp2p::gossipsub::GossipsubEvent),
    /// Events from the mDNS protocol
    Mdns(libp2p::mdns::MdnsEvent),
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

    // Receiver is not needed in the struct since it's moved to the spawned task
    /// Channel for inbound messages from the network
    inbound_tx: mpsc::Sender<(PeerId, Message)>,
}

impl GossipService {
    /// Create a new gossip service
    pub async fn new(
        listen_addr: &str,
        max_peers: usize,
        inbound_tx: mpsc::Sender<(PeerId, Message)>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Create communication channel
        let (outbound_tx, outbound_rx) = mpsc::channel(100);

        // Generate keypair for this node
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        info!("Local peer id: {}", local_peer_id);

        // Create peer manager
        let peer_manager = Arc::new(PeerManager::new(local_peer_id, outbound_tx.clone()));

        // Build transport
        let transport = TcpConfig::new()
            .nodelay(true)
            .upgrade(upgrade::Version::V1)
            .authenticate(
                noise::NoiseConfig::xx(
                    noise::Keypair::<noise::X25519Spec>::new()
                        .into_authentic(&local_key)
                        .expect("Signing libp2p-noise static DH keypair failed."),
                )
                .into_authenticated(),
            )
            .multiplex(yamux::YamuxConfig::default())
            .boxed();

        // Create gossipsub configuration
        let gossipsub_config = GossipsubConfigBuilder::default()
            .validation_mode(ValidationMode::Strict)
            .heartbeat_interval(Duration::from_secs(10))
            .mesh_outbound_min(1)
            .mesh_n(2)
            .mesh_n_low(1)
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
            let topic_id = Topic::new(topic.as_str());
            gossipsub.subscribe(&topic_id)?;
        }

        // Create mDNS for local network discovery
        let mdns = Mdns::new(MdnsConfig::default()).await?;

        // Create swarm
        let mut swarm =
            SwarmBuilder::new(transport, NodeBehaviour { gossipsub, mdns }, local_peer_id)
                .executor(Box::new(|fut| {
                    tokio::spawn(fut);
                }))
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
        let seen_blocks_clone: Arc<Mutex<HashSet<Hash>>> = seen_blocks.clone();

        tokio::spawn(async move {
            let mut outbound_rx_recv = outbound_rx;

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
                    Some((peer_id, message)) = outbound_rx_recv.recv() => {
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
            inbound_tx,
        };

        Ok(service)
    }

    /// Handle an event from the swarm
    async fn handle_swarm_event(
        event: libp2p::swarm::SwarmEvent<
            NodeBehaviourEvent,
            libp2p::core::either::EitherError<GossipsubHandlerError, void::Void>,
        >,
        swarm: &mut Swarm<NodeBehaviour>,
        peer_manager: &PeerManager,
        inbound_tx: &mpsc::Sender<(PeerId, Message)>,
        seen_txs: &Arc<Mutex<HashSet<Hash>>>,
        seen_blocks: &Arc<Mutex<HashSet<Hash>>>,
    ) {
        // Handle different event types
        match event {
            libp2p::swarm::SwarmEvent::Behaviour(behaviour_event) => {
                match behaviour_event {
                    NodeBehaviourEvent::Gossipsub(gossip_event) => {
                        // Handle gossipsub events
                        println!("Gossipsub event: {:?}", gossip_event);
                    }
                    NodeBehaviourEvent::Mdns(mdns_event) => {
                        // Handle mdns events like peer discovery
                        println!("MDNS event: {:?}", mdns_event);
                    }
                }
            }
            libp2p::swarm::SwarmEvent::ConnectionEstablished {
                peer_id,
                endpoint,
                num_established,
                concurrent_dial_errors,
            } => {
                // New peer connected
                println!("Connection established with peer: {}", peer_id);
                println!("  Endpoint: {:?}", endpoint);
                println!("  Total connections to this peer: {}", num_established);

                if let Some(errors) = concurrent_dial_errors {
                    println!("  Concurrent dial errors: {}", errors.len());
                    for (addr, err) in errors {
                        println!("    Address: {}, Error: {:?}", addr, err);
                    }
                }
            }
            libp2p::swarm::SwarmEvent::ConnectionClosed {
                peer_id,
                endpoint,
                num_established,
                cause,
            } => {
                // Peer disconnected
                println!("Connection closed with peer: {}", peer_id);
                println!("  Endpoint: {:?}", endpoint);
                println!("  Remaining connections: {}", num_established);
                if let Some(error) = cause {
                    println!("  Cause: {:?}", error);
                } else {
                    println!("  Cause: Clean disconnect");
                }
            }
            libp2p::swarm::SwarmEvent::IncomingConnection {
                local_addr,
                send_back_addr,
            } => {
                println!("Incoming connection:");
                println!("  Local address: {}", local_addr);
                println!("  Remote address: {}", send_back_addr);
            }
            libp2p::swarm::SwarmEvent::IncomingConnectionError {
                local_addr,
                send_back_addr,
                error,
            } => {
                println!("Incoming connection error:");
                println!("  Local address: {}", local_addr);
                println!("  Remote address: {}", send_back_addr);
                println!("  Error: {:?}", error);
            }
            libp2p::swarm::SwarmEvent::OutgoingConnectionError { peer_id, error } => {
                println!("Outgoing connection error:");
                if let Some(id) = peer_id {
                    println!("  Peer ID: {}", id);
                } else {
                    println!("  Peer ID: Unknown");
                }
                println!("  Error: {:?}", error);
            }
            libp2p::swarm::SwarmEvent::BannedPeer { peer_id, endpoint } => {
                println!("Banned peer connection rejected:");
                println!("  Peer ID: {}", peer_id);
                println!("  Endpoint: {:?}", endpoint);
            }
            libp2p::swarm::SwarmEvent::NewListenAddr {
                listener_id,
                address,
            } => {
                println!("New listen address:");
                println!("  Listener ID: {:?}", listener_id);
                println!("  Address: {}", address);
            }
            libp2p::swarm::SwarmEvent::ExpiredListenAddr {
                listener_id,
                address,
            } => {
                println!("Expired listen address:");
                println!("  Listener ID: {:?}", listener_id);
                println!("  Address: {}", address);
            }
            libp2p::swarm::SwarmEvent::ListenerClosed {
                listener_id,
                addresses,
                reason,
            } => {
                println!("Listener closed:");
                println!("  Listener ID: {:?}", listener_id);
                println!("  Addresses: {:?}", addresses);
                match reason {
                    Ok(_) => println!("  Reason: Graceful shutdown"),
                    Err(e) => println!("  Reason: Error - {:?}", e),
                }
            }
            libp2p::swarm::SwarmEvent::ListenerError { listener_id, error } => {
                println!("Listener error:");
                println!("  Listener ID: {:?}", listener_id);
                println!("  Error: {:?}", error);
            }
            libp2p::swarm::SwarmEvent::Dialing(peer_id) => {
                println!("Dialing peer: {}", peer_id);
            }
        }
    }

    /// Handle an outbound message
    async fn handle_outbound_message(
        swarm: &mut Swarm<NodeBehaviour>,
        peer_id: PeerId, // Unused for now
        message: Message,
        seen_txs: &Arc<Mutex<HashSet<Hash>>>,
        seen_blocks: &Arc<Mutex<HashSet<Hash>>>,
    ) {
        match &message {
            Message::Dial(x) => {
                swarm.dial(x.parse::<Multiaddr>().unwrap());
            }
            Message::Transaction(tx) => {
                let tx_hash = tx.hash();
                {
                    let mut seen = seen_txs.lock().unwrap();
                    seen.insert(tx_hash);
                }
                let topic = GossipTopic::Transactions.as_str();
                let topic_id = Topic::new(topic);
                let encoded = bincode::serialize(&message).unwrap_or_default();
                if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic_id, encoded) {
                    error!("Failed to publish transaction: {}", e);
                }
            }
            Message::Block(block) => {
                let block_hash = block.hash();
                {
                    let mut seen = seen_blocks.lock().unwrap();
                    seen.insert(block_hash);
                }
                let topic = GossipTopic::Blocks.as_str();
                let topic_id = Topic::new(topic);
                let encoded = bincode::serialize(&message).unwrap_or_default();

                println!(
                    "Is connected to Peer {} ---> {}",
                    peer_id,
                    swarm.is_connected(&peer_id)
                );

                for peer in swarm.connected_peers() {
                    println!("Connected to peer: {}", peer);
                }

                if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic_id, encoded) {
                    error!("Failed to publish block: {}", e);
                }
            }
            _ => {
                warn!("Unhandled message type for gossip");
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
        println!(
            "Broadcasting to peers: {:?}",
            self.peer_manager.get_all_peers()
        );
        self.peer_manager.broadcast(Message::Block(block)).await;
    }

    /// Request a block by hash from any peer that has it
    pub async fn request_block(&self, hash: Hash) {
        // Get all peers
        let peers = self.peer_manager.get_all_peers();

        // Try to request from a random peer
        if !peers.is_empty() {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let peer_index = rng.gen_range(0..peers.len());
            let peer_id = peers[peer_index].id;
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
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let peer_index = rng.gen_range(0..peers.len());
            let peer_id = peers[peer_index].id;
            let _ = self
                .outbound_tx
                .try_send((peer_id, Message::GetLatestBlock));
        }
    }

    /// Add a known peer to connect to
    pub async fn add_peer(
        &self,
        addr: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Parse address to Multiaddr
        let addr = addr.parse::<libp2p::Multiaddr>()?;

        // Add to known peers
        // In a real implementation, this would dial the peer
        println!("Connected to node: {}", addr);

        // Extract peer ID from the address if it contains one
        let peer_id = match addr.iter().find_map(|p| match p {
            libp2p::multiaddr::Protocol::Ip4(_) => {
                Some(libp2p::PeerId::try_from_multiaddr(&addr).expect("Valid hash"))
            }

            _ => None,
        }) {
            Some(peer_id) => {
                info!("Extracted peer ID from address: {}", peer_id);
                peer_id
            }
            None => {
                return Err("Address does not contain peer ID component (e.g., /p2p/Qm...)".into());
            }
        };

        // add peer here and broadcast it
        self.peer_manager.add_peer(peer_id, addr);

        Ok(())
    }
}
