use solana_sdk::{
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
    signer::Signer,
};
use std::error::Error;
//use std::str::FromStr;

pub struct BotWallet {
    keypair: Keypair,
    pub pubkey: Pubkey,
}

impl BotWallet {
    pub fn generate_new() -> Self {
        let keypair = Keypair::new(); // Generates a random keypair
        let pubkey = keypair.pubkey();

        println!("Generated new wallet:");
        println!("  Public key: {}", pubkey);
        println!(
            "  Private key: {}",
            bs58::encode(keypair.to_bytes()).into_string()
        );

        Self { keypair, pubkey }
    }

    pub fn from_keypair_file(path: &str) -> Result<Self, Box<dyn Error>> {
        let keypair =
            read_keypair_file(path).map_err(|e| format!("Failed to read keypair file: {}", e))?;
        let pubkey = keypair.pubkey();

        Ok(Self { keypair, pubkey })
    }

    pub fn from_private_key(private_key: &str) -> Result<Self, Box<dyn Error>> {
        // Decode the private key from base58
        let decoded = bs58::decode(private_key)
            .into_vec()
            .map_err(|e| format!("Failed to decode private key: {}", e))?;

        let keypair =
            Keypair::from_bytes(&decoded).map_err(|e| format!("Invalid private key: {}", e))?;
        let pubkey = keypair.pubkey();

        Ok(Self { keypair, pubkey })
    }

    pub fn pubkey_string(&self) -> String {
        self.pubkey.to_string()
    }

    pub fn keypair(&self) -> &Keypair {
        &self.keypair
    }
}
