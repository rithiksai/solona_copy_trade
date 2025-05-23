# Solana Copy Trading Bot

A real-time automated copy trading system for the Solana blockchain that monitors successful traders and replicates their transactions.

# Solana Copy Trading Bot

A real-time automated copy trading system for the Solana blockchain that monitors successful traders and replicates their transactions.

## Features

- Real-time monitoring of trader accounts via Helius webhooks
- Automatic detection and parsing of swap transactions
- Transaction replication using Jupiter aggregator
- Configurable slippage protection
- Secure wallet management

## Prerequisites

- Rust (stable version)
- Solana CLI tools
- Helius API key (for transaction monitoring)
- Solana RPC endpoint

## Installation

1. Clone the repository:
   git clone https://github.com/rithiksai/solona_copy_trade
   cd solana-copy-trade

2. Install dependencies:
   cargo build

3. Create a `.env` file with the following variables:
   HELIUS_API_KEY=your_helius_api_key
   WALLET_TO_MONITOR=wallet_address_to_monitor
   BOT_WALLET_PRIVATE_KEY=your_bot_private_key
   RPC_ENDPOINT=https://api.mainnet-beta.solana.com
   PORT=3000

## Usage

Start the webhook server:
cargo run --bin webhook

The server will:

1. Listen for swap transactions from the monitored wallet
2. Parse transaction details to identify the tokens and amounts
3. Replicate the same trade using your bot wallet

## How It Works

### Architecture

The bot consists of three main components:

1. **Webhook Server**: Receives real-time notifications from Helius when the monitored wallet executes trades
2. **Wallet Management**: Securely manages the bot's wallet for executing trades
3. **Swap Execution**: Interacts with Jupiter API to replicate trades with similar parameters

### Transaction Flow

1. Wallet being monitored executes a swap on any Solana DEX
2. Helius detects the transaction and sends a webhook notification
3. Bot parses the transaction details (tokens, amounts, etc.)
4. Bot creates a similar swap transaction using Jupiter
5. Transaction is signed with the bot's private key and sent to the Solana network

## Future Improvements

- [ ] Token symbol mapping for better readability
- [ ] Portfolio balance tracking
- [ ] Performance metrics dashboard
- [ ] Risk management features
- [ ] Support for more transaction types (limit orders, etc.)
- [ ] Web interface for configuration

## License

This project is licensed under the MIT License - see the LICENSE file for details.
