use axum::{extract::Json, http::StatusCode, routing::post, Router};
use axum_server::bind;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::error::Error;
use std::net::SocketAddr;

// Define a structure for incoming webhook data
#[derive(Debug, Deserialize)]
struct WebhookPayload {
    transaction: Value,
    // You can add more fields as needed based on Helius webhook format
}

// Define a response type
#[derive(Serialize)]
struct WebhookResponse {
    status: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Load environment variables
    dotenv().ok();

    // Get port from environment variable or use default
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let port: u16 = port.parse()?;

    // Build our application
    let app = Router::new().route("/webhook", post(handle_webhook));

    // Create the server address
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Webhook server listening on http://{}", addr);

    // Start the server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

// Handler function for webhook endpoint
async fn handle_webhook(
    Json(payload): Json<WebhookPayload>,
) -> (StatusCode, Json<WebhookResponse>) {
    println!("Received SWAP transaction!");
    
    // Load the monitored wallet address from environment
    let monitored_wallet = std::env::var("WALLET_TO_MONITOR")
        .expect("WALLET_TO_MONITOR must be set in .env file");
    
    // Try to extract token transfer information
    if let Some(token_transfers) = payload.transaction.get("tokenTransfers").and_then(|v| v.as_array()) {
        println!("Token transfers found: {}", token_transfers.len());
        
        // Variables to store swap information
        let mut source_token = None;
        let mut source_amount = 0.0;
        let mut target_token = None;
        let mut target_amount = 0.0;
        
        // Loop through token transfers to find source and target
        for transfer in token_transfers {
            // Extract the from and to addresses
            let from_address = transfer.get("fromUserAccount").and_then(|a| a.as_str());
            let to_address = transfer.get("toUserAccount").and_then(|a| a.as_str());
            
            // Check if this is an outgoing transfer (source token)
            if from_address == Some(monitored_wallet.as_str()) {
                // This is a token being sent (sold)
                source_token = transfer.get("tokenAddress").and_then(|t| t.as_str()).map(String::from);
                source_amount = transfer.get("tokenAmount").and_then(|a| a.as_f64()).unwrap_or(0.0);
                println!("  Found source token: {}", source_token.as_ref().unwrap_or(&"unknown".to_string()));
            }
            
            // Check if this is an incoming transfer (target token)
            if to_address == Some(monitored_wallet.as_str()) {
                // This is a token being received (bought)
                target_token = transfer.get("tokenAddress").and_then(|t| t.as_str()).map(String::from);
                target_amount = transfer.get("tokenAmount").and_then(|a| a.as_f64()).unwrap_or(0.0);
                println!("  Found target token: {}", target_token.as_ref().unwrap_or(&"unknown".to_string()));
            }
        }
        
        // If we found both source and target tokens, we have a complete swap
        if let (Some(source), Some(target)) = (&source_token, &target_token) {
            println!("SWAP detected:");
            println!("  Sold: {} (token address: {})", source_amount, source);
            println!("  Bought: {} (token address: {})", target_amount, target);
            
            if target_amount > 0.0 && source_amount > 0.0 {
                let price = source_amount / target_amount;
                println!("  Price: {} {} per {}", price, source, target);
                
                // Here you would add code to execute the same swap on your bot's wallet
                // This is where your trade replication logic would go
            }
        } else {
            println!("Incomplete SWAP data - couldn't identify both source and target tokens");
        }
    } else {
        println!("No token transfers found in this SWAP transaction");
    }
    
    // Return a success response
    (
        StatusCode::OK,
        Json(WebhookResponse {
            status: "success".to_string(),
        }),
    )
}