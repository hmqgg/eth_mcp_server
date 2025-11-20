#![allow(dead_code)]

use alloy::hex::FromHex;
use alloy::network::Ethereum;
use alloy::primitives::aliases::U24;
use alloy::primitives::{Address, Bytes, U256, Uint, address, keccak256};
use alloy::providers::Provider;
use alloy::rpc::types::eth::state::{AccountOverride, StateOverride};
use anyhow::{Context, Result, bail};
use rust_decimal::{Decimal, dec};
use std::str::FromStr;

use crate::tools::price::{FEE_TIERS, UNISWAP_V3_QUOTER_ADDRESS};
use crate::utils::contracts::IV3SwapRouter::ExactInputSingleParams;
use crate::utils::contracts::{IERC20, UniswapV3Quoter, UniswapV3Router};
use crate::utils::decimals::{decimal_to_u256, u256_to_decimal};
use crate::utils::provider::{get_wallet_address, make_provider};
use crate::utils::token_registry::resolve_token;

const UNISWAP_V3_ROUTER_ADDRESS: Address = address!("0x68b3465833fb72a70ecdf485e0e4c7bd8665fc45");
const USDT_ADDRESS: Address = address!("0xdAC17F958D2ee523a2206206994597C13D831ec7");
const USDC_ADDRESS: Address = address!("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
const MOCK_BYTECODE_HEX: &str = include_str!("../../sol/MockToken.hex");

#[derive(Clone, Copy)]
struct TokenSlotConfig {
    allowance_slot: u64,
    balance_slot: u64,
}

const DEFAULT_SLOT_CONFIG: TokenSlotConfig = TokenSlotConfig {
    allowance_slot: 1,
    balance_slot: 0,
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SwapRequest {
    #[schemars(description = "From token symbol (e.g., 'USDC') or address (e.g., '0x...')")]
    pub from_token: String,
    #[schemars(description = "To token symbol (e.g., 'WETH') or address (e.g., '0x...')")]
    pub to_token: String,
    #[schemars(description = "Amount to swap from in formatted string format (e.g., '100.5')")]
    // String is used to avoid precision loss.
    pub amount_from: String,
    #[schemars(description = "Slippage tolerance in percent as string format (e.g., '0.5')")]
    // String is used to avoid precision loss.
    pub slippage_percent: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SwapResponse {
    // Serialize as string to avoid precision loss.
    #[serde(with = "rust_decimal::serde::str")]
    pub amount_to: Decimal,
    pub gas_estimate: u64,
}

pub async fn swap_tokens(
    from_token: String,
    to_token: String,
    amount_from: String,
    slippage_percent: String,
) -> Result<SwapResponse> {
    tracing::trace!("Creating provider");
    let provider = make_provider()?;

    tracing::debug!("Resolving tokens: {} -> {}", from_token, to_token);
    let from_token_addr = resolve_token(&from_token).await?;
    let to_token_addr = resolve_token(&to_token).await?;
    tracing::trace!("From token address: {}, To token address: {}", from_token_addr, to_token_addr);

    let from_contract = IERC20::new(from_token_addr, &provider);
    let to_contract = IERC20::new(to_token_addr, &provider);

    tracing::trace!("Fetching token decimals");
    let (from_decimals, to_decimals) =
        tokio::try_join!(async { from_contract.decimals().call().await }, async {
            to_contract.decimals().call().await
        },)
        .context("Failed to fetch token decimals")?;
    tracing::trace!("From decimals: {}, To decimals: {}", from_decimals, to_decimals);

    // Convert amount_from (string) to Decimal, then to U256
    tracing::trace!("Parsing input amount: {}", amount_from);
    let amount_from_decimal =
        Decimal::from_str(&amount_from).context(format!("Invalid amount_from: {}", amount_from))?;

    // Convert to U256, using the helper function
    let amount_from_u256 = decimal_to_u256(amount_from_decimal, from_decimals)?;
    tracing::trace!("Input amount in U256: {}", amount_from_u256);

    // Move quoter outside of swap_tokens function to make it clear.
    // Use Quoter to find the best fee tier and estimate the output
    tracing::debug!("Finding best fee tier for swap {} -> {}", from_token, to_token);
    let (best_fee, best_amount_out) =
        get_best_fee_and_amount_out(from_token_addr, to_token_addr, amount_from_u256, &provider)
            .await?;
    tracing::debug!("Selected fee tier: {:?}, estimated output: {}", best_fee, best_amount_out);

    // Calculate amountOutMinimum (considering slippage)
    let slippage = Decimal::from_str(&slippage_percent)?;
    let slippage_multiplier = dec!(1.0) - slippage / dec!(100.0);
    let amount_out_decimal = u256_to_decimal(best_amount_out, to_decimals)?;
    let min_decimal = amount_out_decimal * slippage_multiplier;
    let amount_out_minimum = decimal_to_u256(min_decimal, to_decimals)?;
    tracing::trace!("Slippage: {}%, Min output: {}", slippage, amount_out_minimum);

    // Get wallet address for state override
    let wallet_addr = get_wallet_address()?;
    tracing::trace!("Wallet address for simulation: {}", wallet_addr);

    // Use Router to simulate swap
    let router = UniswapV3Router::new(UNISWAP_V3_ROUTER_ADDRESS, &provider);

    let params = ExactInputSingleParams {
        tokenIn: from_token_addr,
        tokenOut: to_token_addr,
        fee: best_fee,
        recipient: wallet_addr,
        amountIn: amount_from_u256,
        amountOutMinimum: amount_out_minimum,
        sqrtPriceLimitX96: Uint::ZERO,
    };

    tracing::trace!("Creating state override for token: {}", from_token_addr);
    let state_override = create_token_state_override(from_token_addr, wallet_addr);

    tracing::debug!("Simulating swap on Uniswap V3 Router");
    let gas_estimate = router
        .exactInputSingle(params.clone())
        .from(wallet_addr)
        .state(state_override.clone())
        .estimate_gas()
        .await?;
    tracing::trace!("Gas estimate: {}", gas_estimate);

    let swap_result = router
        .exactInputSingle(params)
        .from(wallet_addr)
        .state(state_override)
        .call()
        .await
        .map_err(|e| {
            tracing::error!("Swap simulation error: {:?}", e);
            anyhow::anyhow!("Failed to simulate swap: {:?}", e)
        })?;

    let amount_out = swap_result;
    tracing::debug!("Swap simulation successful, actual output: {}", amount_out);

    Ok(SwapResponse {
        amount_to: u256_to_decimal(amount_out, to_decimals)?,
        gas_estimate,
    })
}

async fn get_best_fee_and_amount_out(
    from_token_addr: Address,
    to_token_addr: Address,
    amount_from_u256: U256,
    provider: &impl Provider<Ethereum>,
) -> Result<(U24, U256)> {
    tracing::trace!("Querying quoter for best fee tier");
    let quoter = UniswapV3Quoter::new(UNISWAP_V3_QUOTER_ADDRESS, &provider);

    let mut best_fee = None;
    let mut best_amount_out = U256::ZERO;

    tracing::trace!("Testing fee tiers: {:?}", FEE_TIERS);
    for &fee in &FEE_TIERS {
        let fee_uint = Uint::<24, 1>::from_limbs([fee.into()]);

        let result = quoter
            .quoteExactInputSingle(
                from_token_addr,
                to_token_addr,
                fee_uint,
                amount_from_u256,
                Uint::ZERO,
            )
            .call()
            .await;

        if let Ok(quote) = result {
            let amount_out = quote;
            tracing::trace!("Fee tier {}: quote = {}", fee, amount_out);
            if amount_out > best_amount_out {
                best_amount_out = amount_out;
                best_fee = Some(fee);
                tracing::trace!("New best fee tier: {}", fee);
            }
        } else {
            tracing::trace!("Fee tier {}: no liquidity or error", fee);
        }
    }

    if best_fee.is_none() {
        tracing::warn!("No liquidity found for pair {}/{} in any V3 pool", from_token_addr, to_token_addr);
        bail!(
            "No liquidity found for pair {}/{} in V3 pools",
            from_token_addr,
            to_token_addr,
        );
    }

    let best_fee = best_fee.unwrap();
    let fee_uint = Uint::<24, 1>::from_limbs([best_fee.into()]);

    Ok((fee_uint, best_amount_out))
}

fn create_token_state_override(token_address: Address, signer_addr: Address) -> StateOverride {
    let balance_slot = keccak256(
        [
            &[0u8; 12],
            signer_addr.as_slice(), // Pad address to 32 bytes
            &[0u8; 32],             // Slot 0 (uint256 0 padded to 32 bytes)
        ]
        .concat(),
    );

    let mut storage = AccountOverride::default().state.unwrap_or_default();

    // Wealthy as much as possible.
    storage.insert(balance_slot, U256::MAX.into());

    let account_override = AccountOverride {
        // Use MockToken to skip the token allowance check.
        code: Some(Bytes::from_hex(MOCK_BYTECODE_HEX).unwrap()),
        balance: None,
        nonce: None,
        state: None,
        state_diff: Some(storage),
        move_precompile_to: None,
    };

    let mut state_override = StateOverride::default();
    state_override.insert(token_address, account_override);
    state_override
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use serde_json;
    use std::str::FromStr;

    #[test]
    fn swap_response_serde_uses_string_field() {
        let response = SwapResponse {
            amount_to: Decimal::from_str("42.5").unwrap(),
            gas_estimate: 99,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"amount_to\":\"42.5\""));
        assert!(json.contains("\"gas_estimate\":99"));

        let parsed: SwapResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.amount_to, response.amount_to);
        assert_eq!(parsed.gas_estimate, response.gas_estimate);
    }

    #[test]
    fn create_token_state_override_adds_account_override() {
        let token = address!("0x1000000000000000000000000000000000000000");
        let signer = address!("0x2000000000000000000000000000000000000000");

        let override_map = create_token_state_override(token, signer);
        let entry = override_map.get(&token).expect("token override entry");

        assert!(entry.state_diff.is_some());
        assert_eq!(
            entry.code.as_ref(),
            Some(&Bytes::from_hex(MOCK_BYTECODE_HEX).unwrap())
        );

        let storage = entry.state_diff.as_ref().unwrap();
        assert!(!storage.is_empty());
    }
}
