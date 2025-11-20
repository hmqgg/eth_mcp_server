use alloy::network::Ethereum;
use alloy::primitives::Address;
use alloy::providers::{Provider, ProviderBuilder};
use alloy::signers::local::PrivateKeySigner;
use anyhow::Result;
use reqwest::Url;

const ETH_PRIVATE_KEY: &str = "ETH_PRIVATE_KEY";
const ETH_RPC_URL: &str = "ETH_RPC_URL";
pub const CHAIN_ID: u64 = 1;

fn make_wallet() -> Result<PrivateKeySigner> {
    let private_key_string = std::env::var(ETH_PRIVATE_KEY)?;
    let signer: PrivateKeySigner = private_key_string.parse()?;
    Ok(signer)
}

pub fn make_provider() -> Result<impl Provider<Ethereum>> {
    tracing::trace!("Creating provider with RPC_URL from environment");
    let rpc_url = std::env::var(ETH_RPC_URL)?;
    tracing::trace!("RPC URL: {}", rpc_url);
    let wallet = make_wallet()?;
    tracing::trace!("Wallet address: {}", wallet.address());
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .with_chain_id(CHAIN_ID)
        .connect_http(Url::parse(&rpc_url)?);
    tracing::trace!("Provider created successfully with chain_id: {}", CHAIN_ID);
    Ok(provider)
}

pub fn get_wallet_address() -> Result<Address> {
    let wallet = make_wallet()?;
    Ok(wallet.address())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_provider_missing_env() {
        // Clean up environment variables
        unsafe {
            std::env::remove_var(ETH_RPC_URL);
            std::env::remove_var(ETH_PRIVATE_KEY);
        }

        let result = make_provider();
        assert!(result.is_err());
    }

    #[test]
    fn test_make_provider_invalid_url() {
        unsafe {
            std::env::set_var(ETH_RPC_URL, "not-a-valid-url");
            std::env::set_var(
                ETH_PRIVATE_KEY,
                "0x0000000000000000000000000000000000000000000000000000000000000001",
            );
        }

        let result = make_provider();
        assert!(result.is_err());

        // Cleanup
        unsafe {
            std::env::remove_var(ETH_RPC_URL);
            std::env::remove_var(ETH_PRIVATE_KEY);
        }
    }

    #[test]
    fn test_make_provider_invalid_private_key() {
        unsafe {
            std::env::set_var(ETH_RPC_URL, "https://eth.llamarpc.com");
            std::env::set_var(ETH_PRIVATE_KEY, "invalid-key");
        }

        let result = make_provider();
        assert!(result.is_err());

        // Cleanup
        unsafe {
            std::env::remove_var(ETH_RPC_URL);
            std::env::remove_var(ETH_PRIVATE_KEY);
        }
    }
}
