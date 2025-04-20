use crate::wallet::BotWallet;
use base64;
use serde_json::{json, Value};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, transaction::Transaction};
use std::error::Error;

pub struct SwapExecutor {
    rpc_client: RpcClient,
    helius_endpoint: String,
    jupiter_quote_api: String,
}

impl SwapExecutor {
    pub fn new(rpc_endpoint: &str, helius_api_key: &str) -> Self {
        let helius_endpoint = format!("https://mainnet.helius-rpc.com/?api-key={}", helius_api_key);
        let jupiter_quote_api = "https://quote-api.jup.ag/v6/quote".to_string();

        Self {
            rpc_client: RpcClient::new_with_commitment(
                rpc_endpoint.to_string(),
                CommitmentConfig::confirmed(),
            ),
            helius_endpoint,
            jupiter_quote_api,
        }
    }

    async fn get_token_decimals(&self, token_mint: &str) -> Result<u8, Box<dyn Error>> {
        // For simplicity, hardcode common token decimals
        let decimals = match token_mint {
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" => 6, // USDC
            "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB" => 6, // USDT
            "So11111111111111111111111111111111111111112" => 9,  // SOL
            _ => 9,                                              // Default to 9 for unknown tokens
        };

        Ok(decimals)
    }

    pub async fn replicate_swap(
        &self,
        wallet: &BotWallet,
        input_token: &str,
        output_token: &str,
        input_amount: f64,
        slippage_bps: u32,
    ) -> Result<String, Box<dyn Error>> {
        println!(
            "Replicating swap: {} {} -> {}",
            input_amount, input_token, output_token
        );

        let input_token = if input_token == "SOL" {
            "So11111111111111111111111111111111111111112"
        } else {
            input_token
        };

        // Step 1: Get a quote from Jupiter
        let input_decimals = self.get_token_decimals(input_token).await?;
        let raw_amount = (input_amount * 10f64.powi(input_decimals as i32)) as u64;

        // Build the quote URL
        let quote_url = format!(
            "{}?inputMint={}&outputMint={}&amount={}&slippageBps={}",
            self.jupiter_quote_api, input_token, output_token, raw_amount, slippage_bps
        );

        // Get the quote
        let response = reqwest::get(&quote_url).await?;
        let quote: Value = response.json().await?;

        println!(
            "Got quote from Jupiter:\n{}",
            serde_json::to_string_pretty(&quote)?
        );

        // Step 2: Get transaction data from Jupiter swap API
        let swap_url = "https://lite-api.jup.ag/swap/v1/swap"; // Updated API endpoint

        // Create the swap request according to Jupiter docs
        let swap_request = json!({
            "userPublicKey": wallet.pubkey.to_string(),
            "quoteResponse": {
                "inputMint": quote["inputMint"],
                "inAmount": quote["inAmount"],
                "outputMint": quote["outputMint"],
                "outAmount": quote["outAmount"],
                "otherAmountThreshold": quote["otherAmountThreshold"],
                "swapMode": quote["swapMode"],
                "slippageBps": slippage_bps,
                "platformFee": null,
                "priceImpactPct": quote["priceImpactPct"],
                "routePlan": quote["routePlan"]
            },
            "prioritizationFeeLamports": {
                "priorityLevelWithMaxLamports": {
                    "maxLamports": 10000000,
                    "priorityLevel": "veryHigh"
                }
            },
            "dynamicComputeUnitLimit": true
        });

        println!("Sending swap request to Jupiter");

        // Send the request to get the transaction
        let client = reqwest::Client::new();
        let response = client
            .post(swap_url)
            .header("Content-Type", "application/json")
            .json(&swap_request)
            .send()
            .await?;

        // Parse the response
        let swap_response: Value = response.json().await?;

        // Check for simulation errors
        if let Some(sim_error) = swap_response.get("simulationError") {
            println!(
                "Simulation error detected: {}",
                serde_json::to_string_pretty(sim_error)?
            );
            return Err(format!(
                "Transaction simulation failed: {}",
                sim_error
                    .get("error")
                    .and_then(|e| e.as_str())
                    .unwrap_or("Unknown error")
            )
            .into());
        }

        // Extract the transaction from the response
        let tx_data = match swap_response["swapTransaction"].as_str() {
            Some(data) => data,
            None => {
                println!("Transaction data not found in response!");
                println!(
                    "Full response: {}",
                    serde_json::to_string_pretty(&swap_response)?
                );
                return Err("Failed to get transaction data".into());
            }
        };

        // Jupiter returns a base64 encoded transaction
        let serialized_tx = match base64::decode(tx_data) {
            Ok(data) => data,
            Err(e) => {
                println!("Failed to decode base64 transaction: {}", e);
                return Err(format!("Base64 decode error: {}", e).into());
            }
        };

        // Deserialize the transaction
        let mut tx: Transaction = match bincode::deserialize(&serialized_tx) {
            Ok(tx) => tx,
            Err(e) => {
                println!("Failed to deserialize transaction: {}", e);
                return Err(format!("Transaction deserialize error: {}", e).into());
            }
        };

        // Sign the transaction
        tx.sign(&[wallet.keypair()], self.rpc_client.get_latest_blockhash()?);

        // Send the signed transaction
        println!("Sending signed transaction to the network...");
        let signature = self.rpc_client.send_and_confirm_transaction(&tx)?;

        println!("Swap executed successfully!");
        println!("Transaction signature: {}", signature);

        Ok(signature.to_string())
    }
}
