use alloy::primitives::U256;
use alloy::primitives::utils::format_units;
use anyhow::{Context, Result, bail};
use rust_decimal::Decimal;
use std::str::FromStr;

pub fn u256_to_decimal(value: U256, decimals: u8) -> Result<Decimal> {
    let s = format_units(value, decimals)?;
    let d = Decimal::from_str(&s)?;
    Ok(d)
}

pub fn decimal_to_u256(value: Decimal, decimals: u8) -> Result<U256> {
    let mantissa = value.mantissa();
    if mantissa < 0 {
        bail!("Negative value not supported");
    }

    let mut u256_val = U256::from(mantissa as u128);
    let scale = value.scale();
    let target_decimals = decimals as u32;

    if target_decimals >= scale {
        let diff = target_decimals - scale;
        let mul_factor = U256::from(10).pow(U256::from(diff));
        u256_val = u256_val
            .checked_mul(mul_factor)
            .context("Overflow during scaling")?;
    } else {
        let diff = scale - target_decimals;
        let div_factor = U256::from(10).pow(U256::from(diff));
        u256_val /= div_factor;
    }

    Ok(u256_val)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::U256;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn test_u256_to_decimal_eth() {
        // 1 ETH = 10^18 wei
        let one_eth = U256::from(1_000_000_000_000_000_000u64);
        let result = u256_to_decimal(one_eth, 18).unwrap();
        assert_eq!(result, Decimal::from_str("1.0").unwrap());
    }

    #[test]
    fn test_u256_to_decimal_usdc() {
        // 100 USDC = 100 * 10^6 (USDC has 6 decimals)
        let one_hundred_usdc = U256::from(100_000_000u64);
        let result = u256_to_decimal(one_hundred_usdc, 6).unwrap();
        assert_eq!(result, Decimal::from_str("100.0").unwrap());
    }

    #[test]
    fn test_u256_to_decimal_small_amount() {
        // 0.001 ETH = 10^15 wei
        let small_amount = U256::from(1_000_000_000_000_000u64);
        let result = u256_to_decimal(small_amount, 18).unwrap();
        assert_eq!(result, Decimal::from_str("0.001").unwrap());
    }

    #[test]
    fn test_decimal_to_u256_eth() {
        // 1.5 ETH should convert to 1.5 * 10^18 wei
        let decimal = Decimal::from_str("1.5").unwrap();
        let result = decimal_to_u256(decimal, 18).unwrap();
        let expected = U256::from(1_500_000_000_000_000_000u64);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_decimal_to_u256_usdc() {
        // 100.5 USDC should convert to 100.5 * 10^6
        let decimal = Decimal::from_str("100.5").unwrap();
        let result = decimal_to_u256(decimal, 6).unwrap();
        let expected = U256::from(100_500_000u64);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_decimal_to_u256_round_trip() {
        // Test round trip: U256 -> Decimal -> U256
        let original = U256::from(1_234_567_890_000_000_000u64);
        let decimal = u256_to_decimal(original, 18).unwrap();
        let back_to_u256 = decimal_to_u256(decimal, 18).unwrap();
        assert_eq!(original, back_to_u256);
    }

    #[test]
    fn test_decimal_to_u256_different_decimals() {
        // 1.0 with 18 decimals should convert correctly
        let decimal = Decimal::from_str("1.0").unwrap();
        let result = decimal_to_u256(decimal, 18).unwrap();
        let expected = U256::from(1_000_000_000_000_000_000u64);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_decimal_to_u256_negative_should_fail() {
        let decimal = Decimal::from_str("-1.0").unwrap();
        let result = decimal_to_u256(decimal, 18);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Negative"));
    }

    #[test]
    fn test_decimal_to_u256_large_number() {
        // Test with a large number
        let decimal = Decimal::from_str("1000000.0").unwrap();
        let result = decimal_to_u256(decimal, 18).unwrap();
        let expected = U256::from(1_000_000u64) * U256::from(10).pow(U256::from(18));
        assert_eq!(result, expected);
    }

    #[test]
    fn test_u256_to_decimal_zero() {
        let zero = U256::ZERO;
        let result = u256_to_decimal(zero, 18).unwrap();
        assert_eq!(result, Decimal::ZERO);
    }
}
