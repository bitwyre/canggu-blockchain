use anyhow::Result;
use solana_minimal::cli::commands;
use solana_minimal::crypto::keys::Keypair;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    // URL of the blockchain node
    let url = "http://localhost:8899";

    // Load sender wallet
    let wallet_path = PathBuf::from("wallet.json");
    let sender_keypair = Keypair::load_from_file(&wallet_path)?;
    let sender_pubkey = sender_keypair.public_key().to_string();

    // Recipient address
    let recipient = "Abc123XYZ456..."; // Replace with actual recipient address

    // Amount to send
    let amount = 100;

    // Check sender balance
    let balance = commands::get_balance(url, &sender_pubkey).await?;
    println!("Sender balance: {}", balance);

    if balance < amount {
        println!("Insufficient balance!");
        return Ok(());
    }

    // Send transaction
    println!("Sending {} tokens to {}", amount, recipient);
    let signature = commands::transfer(url, &sender_keypair, recipient, amount).await?;
    println!("Transaction sent: {}", signature);

    // Check transaction status
    let status = commands::get_transaction_status(url, &signature).await?;
    println!("Transaction status: {}", status);

    // Check new balance
    let new_balance = commands::get_balance(url, &sender_pubkey).await?;
    println!("New balance: {}", new_balance);

    Ok(())
}
