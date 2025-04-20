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
    println!("Received webhook notification!");

    // Print transaction details
    println!("Transaction data: {:#?}", payload.transaction);

    // Here you would add logic to:
    // 1. Parse the transaction to see if it's a trade
    // 2. If it is, execute the same trade on your account

    // Return a success response
    (
        StatusCode::OK,
        Json(WebhookResponse {
            status: "success".to_string(),
        }),
    )
}
