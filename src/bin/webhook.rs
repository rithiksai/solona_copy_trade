mod swapper;
mod wallet;

use std::sync::Arc;
use swapper::SwapExecutor;
use wallet::BotWallet;

use axum::{extract::Json, http::StatusCode, routing::post, Router};
//use axum_server::bind;
use axum::extract::State;
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

// Define application state to be shared with handlers
#[derive(Clone)]
struct AppState {
    wallet: Arc<BotWallet>,
    executor: Arc<SwapExecutor>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Load environment variables
    dotenv().ok();

    // Initialize the bot wallet
    println!("Initializing bot wallet...");
    let bot_wallet = match std::env::var("BOT_WALLET_PRIVATE_KEY") {
        Ok(key) => BotWallet::from_private_key(&key)?,
        Err(_) => {
            println!("No wallet key found, generating new wallet...");
            BotWallet::generate_new()
        }
    };
    println!("Bot wallet address: {}", bot_wallet.pubkey_string());

    // Create a swap executor
    let rpc_endpoint = std::env::var("RPC_ENDPOINT")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
    let helius_api_key =
        std::env::var("HELIUS_API_KEY").expect("HELIUS_API_KEY must be set in .env file");

    let swap_executor = SwapExecutor::new(&rpc_endpoint, &helius_api_key);

    // Wrap in Arc for thread-safe sharing
    let wallet = Arc::new(bot_wallet);
    let executor = Arc::new(swap_executor);

    // Get port from environment variable or use default
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let port: u16 = port.parse()?;

    // Build our application with state
    let app = Router::new()
        .route("/webhook", post(handle_webhook))
        .with_state(AppState {
            wallet: wallet.clone(),
            executor: executor.clone(),
        });

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
    State(state): State<AppState>,
    Json(payload): Json<WebhookPayload>,
) -> (StatusCode, Json<WebhookResponse>) {
    println!("Received SWAP transaction!");

    // Get the monitored wallet address
    let monitored_wallet =
        std::env::var("WALLET_TO_MONITOR").expect("WALLET_TO_MONITOR must be set in .env file");

    println!("Monitoring wallet: {}", monitored_wallet);

    // Extract the swap information from events.swap
    if let Some(events) = payload.transaction.get("events") {
        if let Some(swap) = events.get("swap") {
            // Check for tokens bought (outputs)
            let mut bought_tokens = Vec::new();
            if let Some(outputs) = swap.get("tokenOutputs").and_then(|o| o.as_array()) {
                for output in outputs {
                    let to_user = output.get("userAccount").and_then(|u| u.as_str());

                    // If this output goes to our monitored wallet, it's a token we bought
                    if to_user == Some(monitored_wallet.as_str()) {
                        let mint = output
                            .get("mint")
                            .and_then(|m| m.as_str())
                            .unwrap_or("unknown");
                        let amount = output
                            .get("rawTokenAmount")
                            .and_then(|a| a.get("tokenAmount").and_then(|t| t.as_str()))
                            .unwrap_or("0");
                        let decimals = output
                            .get("rawTokenAmount")
                            .and_then(|a| a.get("decimals").and_then(|d| d.as_u64()))
                            .unwrap_or(0);

                        // Convert string amount and decimals to actual float amount
                        let parsed_amount =
                            amount.parse::<f64>().unwrap_or(0.0) / 10f64.powi(decimals as i32);

                        println!("BOUGHT: {} of token {}", parsed_amount, mint);
                        bought_tokens.push((mint, parsed_amount));
                    }
                }
            }

            // Check for tokens sold (inputs)
            let mut sold_tokens = Vec::new();
            if let Some(inputs) = swap.get("tokenInputs").and_then(|i| i.as_array()) {
                for input in inputs {
                    let from_user = input.get("userAccount").and_then(|u| u.as_str());

                    // If this input comes from our monitored wallet, it's a token we sold
                    if from_user == Some(monitored_wallet.as_str()) {
                        let mint = input
                            .get("mint")
                            .and_then(|m| m.as_str())
                            .unwrap_or("unknown");
                        let amount = input
                            .get("rawTokenAmount")
                            .and_then(|a| a.get("tokenAmount").and_then(|t| t.as_str()))
                            .unwrap_or("0");
                        let decimals = input
                            .get("rawTokenAmount")
                            .and_then(|a| a.get("decimals").and_then(|d| d.as_u64()))
                            .unwrap_or(0);

                        // Convert string amount and decimals to actual float amount
                        let parsed_amount =
                            amount.parse::<f64>().unwrap_or(0.0) / 10f64.powi(decimals as i32);

                        println!("SOLD: {} of token {}", parsed_amount, mint);
                        sold_tokens.push((mint, parsed_amount));
                    }
                }
            }

            // Check for native SOL input
            if let Some(native_input) = swap.get("nativeInput") {
                let account = native_input.get("account").and_then(|a| a.as_str());

                if account == Some(monitored_wallet.as_str()) {
                    let amount = native_input
                        .get("amount")
                        .and_then(|a| a.as_str())
                        .and_then(|s| s.parse::<f64>().ok())
                        .map(|lamports| lamports / 1_000_000_000.0); // Convert lamports to SOL

                    if let Some(sol_amount) = amount {
                        println!("SOLD: {} SOL", sol_amount);
                        sold_tokens.push(("SOL", sol_amount));
                    }
                }
            }

            // Print a summary of the swap
            if !bought_tokens.is_empty() && !sold_tokens.is_empty() {
                println!("SWAP SUMMARY:");
                for (token, amount) in &sold_tokens {
                    println!("  SOLD: {} {}", amount, token);
                }
                for (token, amount) in &bought_tokens {
                    println!("  BOUGHT: {} {}", amount, token);
                }

                // This is where you would implement logic to replicate the trade
                // Replicate the trade with our bot wallet
                if !bought_tokens.is_empty() && !sold_tokens.is_empty() {
                    // Call replicate_trade with access to the state
                    match replicate_trade(
                        &state.wallet,
                        &state.executor,
                        &sold_tokens,
                        &bought_tokens,
                    )
                    .await
                    {
                        Ok(_) => println!("Trade replication successful!"),
                        Err(e) => println!("Error replicating trade: {}", e),
                    }
                }
            }
        }
    }

    // Return a success response
    (
        StatusCode::OK,
        Json(WebhookResponse {
            status: "success".to_string(),
        }),
    )
}

// Helper function to replicate a trade
async fn replicate_trade(
    wallet: &BotWallet,
    executor: &SwapExecutor,
    sold_tokens: &Vec<(&str, f64)>,
    bought_tokens: &Vec<(&str, f64)>,
) -> Result<(), Box<dyn Error>> {
    // Load our trading bot's wallet
    /*let wallet_private_key = std::env::var("BOT_WALLET_PRIVATE_KEY")
        .expect("BOT_WALLET_PRIVATE_KEY must be set in .env file");

    let bot_wallet = BotWallet::from_private_key(&wallet_private_key)?;
    println!("Bot wallet address: {}", bot_wallet.pubkey_string());*/

    // Create the swap executor
    let rpc_endpoint =
        std::env::var("RPC_ENDPOINT").expect("RPC_ENDPOINT must be set in .env file");
    let helius_api_key =
        std::env::var("HELIUS_API_KEY").expect("HELIUS_API_KEY must be set in .env file");

    let _swap_executor = SwapExecutor::new(&rpc_endpoint, &helius_api_key);

    // Assume we're doing a simple swap (first sold token -> first bought token)
    if sold_tokens.len() > 0 && bought_tokens.len() > 0 {
        let (sold_token, sold_amount) = sold_tokens[0];
        let (bought_token, _) = bought_tokens[0];

        // Calculate the amount to swap (e.g., 90% of original amount)
        let percentage = 0.9; // 90%
        let swap_amount = sold_amount * percentage;

        // Execute the swap with 1% slippage
        executor
            .replicate_swap(
                wallet,
                sold_token,
                bought_token,
                swap_amount,
                100, // 1% slippage
            )
            .await?;
    }

    Ok(())
}
