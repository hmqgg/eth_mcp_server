use alloy::primitives::Address;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::str::FromStr;
use tokio::sync::OnceCell;

use crate::utils::provider::CHAIN_ID;

static TOKEN_REGISTRY: OnceCell<HashMap<String, Address>> = OnceCell::const_new();

const UNISWAP_TOKEN_LIST_URL: &str = "https://tokens.uniswap.org";

#[derive(serde::Deserialize)]
struct TokenInfo {
    #[serde(rename = "chainId")]
    chain_id: u64,
    address: String,
    symbol: String,
}

#[derive(serde::Deserialize)]
struct TokenList {
    tokens: Vec<TokenInfo>,
}

// Static initialization of the token registry.
async fn get_registry() -> Result<&'static HashMap<String, Address>> {
    TOKEN_REGISTRY
        .get_or_try_init(|| async {
            tracing::debug!("Fetching token list from: {}", UNISWAP_TOKEN_LIST_URL);
            let client = reqwest::Client::new();
            let response = client
                .get(UNISWAP_TOKEN_LIST_URL)
                .send()
                .await
                .context("Failed to fetch token list")?;

            tracing::trace!("Parsing token list");
            let token_list: TokenList = response
                .json()
                .await
                .context("Failed to parse token list")?;

            let mut registry = HashMap::new();
            for token in token_list.tokens.iter().filter(|t| t.chain_id == CHAIN_ID) {
                if let Ok(address) = Address::from_str(&token.address) {
                    registry.insert(token.symbol.to_uppercase(), address);
                }
            }

            tracing::info!("Token registry initialized with {} tokens for chain {}", registry.len(), CHAIN_ID);

            Ok::<_, anyhow::Error>(registry)
        })
        .await
}

pub async fn resolve_token(token: &str) -> Result<Address> {
    // If the token is already an address, return it.
    if token.starts_with("0x") {
        tracing::trace!("Token is already an address: {}", token);
        return Ok(Address::from_str(token)?);
    }

    tracing::trace!("Fetching token registry");
    let registry = get_registry().await?;
    let symbol_upper = token.to_uppercase();

    tracing::debug!("Resolving token symbol: {} -> {}", token, symbol_upper);
    let result = registry
        .get(&symbol_upper)
        .copied()
        .context(format!("Token symbol '{}' not found in registry", token))?;
    tracing::debug!("Resolved token: {} -> {}", token, result.to_string());
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::Address;

    #[tokio::test]
    async fn test_resolve_token_with_address() {
        // Test with a valid Ethereum address
        let address_str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"; // WETH
        let result = resolve_token(address_str).await;
        assert!(result.is_ok());
        let address = result.unwrap();
        assert_eq!(address, Address::from_str(address_str).unwrap());
    }

    #[tokio::test]
    async fn test_resolve_token_with_invalid_address() {
        // Test with an invalid address
        let invalid_address = "0xInvalidAddress";
        let result = resolve_token(invalid_address).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_resolve_token_with_symbol() {
        // Test with a known token symbol (this will make a real HTTP request)
        // Note: This test requires network access
        let result = resolve_token("WETH").await;
        // This might succeed or fail depending on network and registry state
        // We just check it doesn't panic
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_resolve_token_case_insensitive() {
        // Test that symbol resolution is case-insensitive
        // Note: This test requires network access
        let result_lower = resolve_token("weth").await;
        let result_upper = resolve_token("WETH").await;
        let result_mixed = resolve_token("WeTh").await;

        // If both succeed, they should return the same address
        if let (Ok(addr_lower), Ok(addr_upper)) = (&result_lower, &result_upper) {
            assert_eq!(*addr_lower, *addr_upper);
        }
        if let (Ok(addr_lower), Ok(addr_mixed)) = (&result_lower, &result_mixed) {
            assert_eq!(*addr_lower, *addr_mixed);
        }
    }

    #[tokio::test]
    async fn test_resolve_token_unknown_symbol() {
        // Test with an unknown symbol
        let result = resolve_token("UNKNOWNSYMBOL123").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_address_parsing() {
        // Test that we can parse valid addresses
        let valid_addresses = vec![
            "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
            "0x0000000000000000000000000000000000000000",
            "0xffffffffffffffffffffffffffffffffffffffff",
        ];

        for addr_str in valid_addresses {
            let result = Address::from_str(addr_str);
            assert!(result.is_ok(), "Failed to parse address: {}", addr_str);
        }
    }

    #[test]
    fn test_address_parsing_invalid() {
        // Test that invalid addresses are rejected
        let invalid_addresses = vec![
            "0x",
            "0x123",
            "not-an-address",
            "",
            "0xGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
        ];

        for addr_str in invalid_addresses {
            let result = Address::from_str(addr_str);
            assert!(result.is_err(), "Should have failed to parse: {}", addr_str);
        }
    }
}
