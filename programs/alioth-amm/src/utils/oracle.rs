use anchor_lang::prelude::*;
use crate::errors::AmmError;

/// Oracle utilities for Pyth Network integration
pub struct OracleHelper;

impl OracleHelper {
    /// Get current price from Pyth oracle and validate
    /// Note: In production, integrate with real Pyth SDK
    /// This is a simplified version for demonstration
    pub fn get_price(
        _oracle_account: &AccountInfo,
        _max_age: i64,
    ) -> Result<(i64, u64, i32)> {
        // TODO: Integrate with actual Pyth SDK
        // For now, return mock data to allow compilation
        // In production, use: pyth_sdk_solana::Price::get_price_from_account(oracle_account)

        
        // Mock oracle data for testing
        // In production, parse the Pyth price account properly
        Ok((100_000_000, 1_000_000, -8))
    }

    /// Convert Pyth price to a standardized format (scaled by 10^9)
    pub fn normalize_price(price: i64, expo: i32, target_decimals: u32) -> Result<u64> {
        // Pyth prices come with an exponent (usually negative)
        // We need to normalize to our target decimals (e.g., 9 for SOL)
        
        let abs_expo = expo.abs() as u32;
        
        // Convert price to u128 for calculations
        let price_u128 = if price >= 0 {
            price as u128
        } else {
            return err!(AmmError::InvalidOracle);
        };

        let normalized = if expo < 0 {
            // Price is in format: price * 10^expo
            // We want: price * 10^target_decimals
            if abs_expo > target_decimals {
                // Need to divide
                let divisor = 10u128.pow(abs_expo - target_decimals);
                price_u128
                    .checked_div(divisor)
                    .ok_or(AmmError::DivisionByZero)?
            } else {
                // Need to multiply
                let multiplier = 10u128.pow(target_decimals - abs_expo);
                price_u128
                    .checked_mul(multiplier)
                    .ok_or(AmmError::MathOverflow)?
            }
        } else {
            // Positive exponent (rare for Pyth)
            let multiplier = 10u128.pow(expo as u32 + target_decimals);
            price_u128
                .checked_mul(multiplier)
                .ok_or(AmmError::MathOverflow)?
        };

        Ok(normalized as u64)
    }

    /// Validate swap against oracle price with maximum deviation
    pub fn validate_swap_price(
        amount_in: u64,
        amount_out: u64,
        oracle_a: &AccountInfo,
        oracle_b: &AccountInfo,
        max_age: i64,
        max_deviation_bps: u64,
        _is_a_to_b: bool,
    ) -> Result<()> {
        // Get prices from oracles
        let (price_a, _conf_a, expo_a) = Self::get_price(oracle_a, max_age)?;
        let (price_b, _conf_b, expo_b) = Self::get_price(oracle_b, max_age)?;

        // Normalize prices to same scale (9 decimals)
        let normalized_price_a = Self::normalize_price(price_a, expo_a as i32, 9)?;
        let normalized_price_b = Self::normalize_price(price_b, expo_b as i32, 9)?;

        // Calculate actual exchange rate from the swap
        let actual_rate = (amount_out as u128)
            .checked_mul(1_000_000_000u128)
            .ok_or(AmmError::MathOverflow)?
            .checked_div(amount_in as u128)
            .ok_or(AmmError::DivisionByZero)?;

        // Calculate oracle exchange rate (price_b / price_a)
        let oracle_rate = (normalized_price_b as u128)
            .checked_mul(1_000_000_000u128)
            .ok_or(AmmError::MathOverflow)?
            .checked_div(normalized_price_a as u128)
            .ok_or(AmmError::DivisionByZero)?;

        // Calculate deviation
        let larger = std::cmp::max(actual_rate, oracle_rate);
        let smaller = std::cmp::min(actual_rate, oracle_rate);
        
        if larger > 0 {
            let deviation_bps = larger
                .checked_sub(smaller)
                .ok_or(AmmError::MathOverflow)?
                .checked_mul(10000u128)
                .ok_or(AmmError::MathOverflow)?
                .checked_div(larger)
                .ok_or(AmmError::DivisionByZero)?;

            require!(
                deviation_bps <= max_deviation_bps as u128,
                AmmError::OraclePriceDeviation
            );
        }

        Ok(())
    }

    /// Get confidence interval as percentage of price
    pub fn get_confidence_percentage(price: i64, confidence: u64) -> Result<u64> {
        if price == 0 {
            return Ok(10000); // 100% if price is zero
        }

        let confidence_pct = (confidence as u128)
            .checked_mul(10000u128)
            .ok_or(AmmError::MathOverflow)?
            .checked_div(price.abs() as u128)
            .ok_or(AmmError::DivisionByZero)?;

        Ok(confidence_pct as u64)
    }
}

