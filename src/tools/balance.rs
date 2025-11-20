#![allow(dead_code)]

use crate::utils::contracts::IERC20;
use crate::utils::decimals::u256_to_decimal;
use crate::utils::provider::make_provider;
use crate::utils::token_registry::resolve_token;
use alloy::primitives::Address;
use alloy::providers::Provider;
use anyhow::{Context, Result};
use rust_decimal::Decimal;
use std::str::FromStr;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BalanceRequest {
    #[schemars(description = "Wallet address (e.g., '0x...')")]
    pub wallet_address: String,
    #[schemars(
        description = "Token symbol (e.g., 'UNI') or address (e.g., '0x...'); If not provided, the balance of the native asset will be returned"
    )]
    pub token: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BalanceResponse {
    // Serialize as string to avoid precision loss.
    #[serde(with = "rust_decimal::serde::str")]
    pub balance: Decimal,
}

pub async fn get_balance(wallet_address: String, token: Option<String>) -> Result<BalanceResponse> {
    let wallet_address = Address::from_str(&wallet_address)
        .context(format!("Invalid wallet address: {}", wallet_address))?;
    let provider = make_provider()?;

    match token {
        None => {
            let balance = provider
                .get_balance(wallet_address)
                .await
                .context("Failed to get ETH balance")?;
            Ok(BalanceResponse {
                balance: u256_to_decimal(balance, 18)?,
            })
        }
        Some(token_str) => {
            let token_address = resolve_token(&token_str).await?;
            let contract = IERC20::new(token_address, &provider);

            let decimals = contract
                .decimals()
                .call()
                .await
                .context("Failed to call decimals")?;
            let balance = contract
                .balanceOf(wallet_address)
                .call()
                .await
                .context("Failed to call balanceOf")?;

            Ok(BalanceResponse {
                balance: u256_to_decimal(balance, decimals)?,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use serde_json;
    use std::str::FromStr;

    #[tokio::test]
    async fn get_balance_invalid_wallet_returns_error() {
        let result = get_balance("not-a-valid-address".to_string(), None).await;
        assert!(result.is_err());
    }

    #[test]
    fn balance_response_serde_uses_string_field() {
        let balance = Decimal::from_str("1234.5678").unwrap();
        let response = BalanceResponse { balance };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"balance\":\"1234.5678\""));

        let parsed: BalanceResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.balance, response.balance);
    }
}
