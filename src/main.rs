use clap::{Parser, Subcommand};
use crypto::keys::Keypair;
use log::info;
use rand_core::OsRng;
use std::path::PathBuf;

mod blockchain;
mod cli;
mod config;
mod consensus;
mod crypto;
mod network;
mod node;
mod runtime;
mod transaction;
mod accounts;
pub mod pubkey;

#[derive(Parser)]
#[command(name = "canggu-minimal")]
#[command(about = "Canggu L1 blockchain implementation")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Path to the configuration file
    #[arg(short, long, value_name = "FILE", global = true)]
    config: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a blockchain node
    Start {
        /// Port to listen on
        #[arg(short, long, default_value_t = 8899)]
        port: u16,

        /// Data directory
        #[arg(short, long, value_name = "DIR")]
        data_dir: Option<PathBuf>,
    },

    /// Create a new wallet
    CreateWallet {
        /// Wallet output file
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    /// Send a transaction
    SendTransaction {
        /// Path to the sender's wallet file
        #[arg(short, long, value_name = "FILE")]
        wallet: PathBuf,

        /// Recipient's public key
        #[arg(short, long)]
        recipient: String,

        /// Amount to send
        #[arg(short, long)]
        amount: u64,
    },
}

fn get_data_path() -> PathBuf {
    let home_dir = home::home_dir().expect("Failed to get home directory");
    let data_path = home_dir.join(".canggu");
    data_path
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command-line arguments
    let cli = Cli::parse();

    // Initialize logger
    let log_level = if cli.verbose { "debug" } else { "info" };
    unsafe {
        std::env::set_var("RUST_LOG", log_level);
    }
    env_logger::init();

    // Load configuration
    let config_path = cli.config.unwrap_or_else(|| PathBuf::from("config.json"));
    let config = config::Config::load_or_default(&config_path)?;

    // Process commands
    match cli.command {
        Commands::Start { port, data_dir } => {
            let data_dir = data_dir.unwrap_or_else(|| get_data_path());
            std::fs::create_dir_all(&data_dir)?;

            info!("Starting node on port {}", port);
            let mut node = node::Node::new(config, data_dir, port);
            node.start().await?;
        }
        Commands::CreateWallet { output } => {
            let wp = get_data_path().join("wallets").join("wallet.json");
            let output = output.unwrap_or_else(|| wp.clone());
            std::fs::create_dir_all(output.parent().unwrap())?;
            let keypair = Keypair::new();
            let _ = keypair.save_to_file(&wp).expect("Unable to save to file");

            println!("Public key: {}", keypair.public().to_base58_string());
            println!("Private key: {}", keypair.to_base58_string());
        }
        Commands::SendTransaction {
            wallet,
            recipient,
            amount,
        } => {
            // let keypair = crypto::keys::Keypair::load_from_file(&wallet)?;
            // let recipient = crypto::keys::PublicKey::from_string(&recipient)?;

            // let tx = transaction::tx::Transaction::new_transfer(&keypair, &recipient, amount);

            // // Connect to a node and submit transaction
            // let url = format!("http://localhost:{}", config.rpc_port);
            // let response = transaction::tx::submit_transaction(&url, &tx).await?;
            // println!("Transaction submitted: {}", response.signature);
        }
    }

    Ok(())

}