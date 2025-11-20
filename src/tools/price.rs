#![allow(dead_code)]

use alloy::primitives::{Address, U256, Uint, address};
use anyhow::{Context, Result, bail};
use rust_decimal::Decimal;

use crate::utils::contracts::{IERC20, UniswapV3Quoter};
use crate::utils::decimals::u256_to_decimal;
use crate::utils::provider::make_provider;
use crate::utils::token_registry::resolve_token;

pub const UNISWAP_V3_QUOTER_ADDRESS: Address =
    address!("0xb27308f9F90D607463bb33ea1BeBb41C27CE5AB6");
pub const FEE_TIERS: [u32; 4] = [100, 500, 3000, 10000];

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PriceRequest {
    #[schemars(description = "Token symbol (e.g., 'UNI') or address (e.g., '0x...')")]
    pub token: String,
    #[schemars(
        description = "Currency symbol (e.g., 'USDC', 'USDT', 'WETH') or address (e.g., '0x...')"
    )]
    pub currency: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PriceResponse {
    // Serialize as string to avoid precision loss.
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
}

pub async fn get_token_price(token: String, currency: String) -> Result<PriceResponse> {
    let provider = make_provider()?;

    let token_addr = resolve_token(&token).await?;
    let currency_addr = resolve_token(&currency).await?;

    let token_contract = IERC20::new(token_addr, &provider);
    let currency_contract = IERC20::new(currency_addr, &provider);

    let (token_decimals, currency_decimals) =
        tokio::try_join!(async { token_contract.decimals().call().await }, async {
            currency_contract.decimals().call().await
        },)
        .context("Failed to fetch token/currency decimals")?;

    // IMPORTANT: Set the input amount to 1 token (10^token_decimals).
    // So we do not need to divide by the token amount (and with decimals) in the final calculation.
    let amount_in_u256 = U256::from(10).pow(U256::from(token_decimals));

    let quoter = UniswapV3Quoter::new(UNISWAP_V3_QUOTER_ADDRESS, &provider);

    // Try all fee tiers and find the best price.
    let mut best_out = U256::ZERO;

    for &fee in &FEE_TIERS {
        let fee_uint = Uint::<24, 1>::from_limbs([fee.into()]);

        let result = quoter
            .quoteExactInputSingle(
                token_addr,
                currency_addr,
                fee_uint,
                amount_in_u256,
                Uint::ZERO, // sqrtPriceLimitX96 = 0
            )
            .call()
            .await;

        if let Ok(quote) = result {
            let amount_out = quote;
            if amount_out > best_out {
                best_out = amount_out;
            }
        }
    }

    if best_out == U256::ZERO {
        bail!(
            "No liquidity found for pair {}/{} in V3 pools",
            token,
            currency
        );
    }

    Ok(PriceResponse {
        price: u256_to_decimal(best_out, currency_decimals)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use serde_json;
    use std::str::FromStr;

    #[test]
    fn price_response_serde_uses_string_field() {
        let decimal = Decimal::from_str("1.2345").unwrap();
        let response = PriceResponse { price: decimal };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"price\":\"1.2345\""));

        let parsed: PriceResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.price, response.price);
    }
}
