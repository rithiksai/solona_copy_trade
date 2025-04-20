use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::{error::Error, ffi::c_float, iter::Enumerate};

//This function fetches and returns the signature of each transaction and returns it as json
async fn get_recent_transactions(
    client: &reqwest::Client,
    rpc_url: &str,
    wallet_address: &str,
) -> Result<Value, Box<dyn Error>> {
    // Create a request to get transactions
    let request_payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getSignaturesForAddress",
        "params": [wallet_address, {"limit": 10}]
    });

    // Send the request
    let response = client.post(rpc_url).json(&request_payload).send().await?;

    // Parse the response
    let response_json = response.json::<Value>().await?;

    Ok(response_json)
}

//This function returns the transaction details as json, given the signature of the transaction
async fn get_transaction_details(
    client: &reqwest::Client,
    rpc_url: &str,
    signature: &str,
) -> Result<Value, Box<dyn Error>> {
    let request_payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTransaction",
        "params": [signature, {"encoding": "json", "maxSupportedTransactionVersion": 0}]
    });

    let response = client.post(rpc_url).json(&request_payload).send().await?;

    let response_json = response.json::<Value>().await?;

    Ok(response_json)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    // Get API key from environment
    let api_key = env::var("HELIUS_API_KEY").expect("HELIUS_API_KEY must be set in .env file");

    let wallet_address =
        env::var("WALLET_TO_MONITOR").expect("WALLET_TO_MONITOR must be set in .env file");

    // Helius RPC endpoint
    let rpc_url = format!("https://mainnet.helius-rpc.com/?api-key={}", api_key);

    // Create an HTTP client
    let client = reqwest::Client::new();

    // Get recent transactions
    println!("Getting recent transactions for {}...", wallet_address);
    let transactions = get_recent_transactions(&client, &rpc_url, &wallet_address).await?;

    // Extract transaction signatures
    if let Some(result) = transactions["result"].as_array() {
        println!("Found {} transactions", result.len());

        // Look at each transaction
        for (i, tx) in result.iter().enumerate() {
            if let Some(signature) = tx["signature"].as_str() {
                println!("\nTransaction #{}: {}", i + 1, signature);

                // Get details for this transaction
                let details = get_transaction_details(&client, &rpc_url, signature).await?;

                // Print some basic information
                if details["result"].is_null() {
                    println!("  No details available");
                } else {
                    if let Some(block_time) = details["result"]["blockTime"].as_i64() {
                        println!("  Block time: {}", block_time);
                    }

                    if let Some(slot) = details["result"]["slot"].as_u64() {
                        println!("  Slot: {}", slot);
                    }
                }
            }
        }
    }

    Ok(())
}
