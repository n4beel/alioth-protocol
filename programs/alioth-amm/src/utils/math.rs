use anchor_lang::prelude::*;
use crate::errors::AmmError;

/// AMM math utilities using constant product formula (x * y = k)
pub struct AmmMath;

impl AmmMath {
    /// Calculate output amount given input using constant product formula
    /// Formula: amountOut = (amountIn * reserveOut) / (reserveIn + amountIn)
    /// After fees: amountIn = amountIn * (fee_denominator - fee_numerator) / fee_denominator
    pub fn get_amount_out(
        amount_in: u64,
        reserve_in: u64,
        reserve_out: u64,
        fee_numerator: u64,
        fee_denominator: u64,
    ) -> Result<u64> {
        require!(amount_in > 0, AmmError::ZeroAmount);
        require!(reserve_in > 0 && reserve_out > 0, AmmError::InsufficientLiquidity);

        // Calculate amount after fees
        let amount_in_with_fee = (amount_in as u128)
            .checked_mul((fee_denominator - fee_numerator) as u128)
            .ok_or(AmmError::MathOverflow)?;

        let numerator = amount_in_with_fee
            .checked_mul(reserve_out as u128)
            .ok_or(AmmError::MathOverflow)?;

        let denominator = (reserve_in as u128)
            .checked_mul(fee_denominator as u128)
            .ok_or(AmmError::MathOverflow)?
            .checked_add(amount_in_with_fee)
            .ok_or(AmmError::MathOverflow)?;

        let amount_out = numerator
            .checked_div(denominator)
            .ok_or(AmmError::DivisionByZero)?;

        Ok(amount_out as u64)
    }

    /// Calculate input amount needed to get desired output
    /// Formula: amountIn = (reserveIn * amountOut) / ((reserveOut - amountOut) * (fee_denominator - fee_numerator))
    pub fn get_amount_in(
        amount_out: u64,
        reserve_in: u64,
        reserve_out: u64,
        fee_numerator: u64,
        fee_denominator: u64,
    ) -> Result<u64> {
        require!(amount_out > 0, AmmError::ZeroAmount);
        require!(reserve_in > 0 && reserve_out > amount_out, AmmError::InsufficientLiquidity);

        let numerator = (reserve_in as u128)
            .checked_mul(amount_out as u128)
            .ok_or(AmmError::MathOverflow)?
            .checked_mul(fee_denominator as u128)
            .ok_or(AmmError::MathOverflow)?;

        let denominator = (reserve_out as u128)
            .checked_sub(amount_out as u128)
            .ok_or(AmmError::MathOverflow)?
            .checked_mul((fee_denominator - fee_numerator) as u128)
            .ok_or(AmmError::MathOverflow)?;

        let amount_in = numerator
            .checked_div(denominator)
            .ok_or(AmmError::DivisionByZero)?
            .checked_add(1) // Add 1 to round up
            .ok_or(AmmError::MathOverflow)?;

        Ok(amount_in as u64)
    }

    /// Calculate liquidity tokens to mint for initial deposit
    /// Formula: sqrt(amount_a * amount_b)
    pub fn calculate_initial_liquidity(amount_a: u64, amount_b: u64) -> Result<u64> {
        let product = (amount_a as u128)
            .checked_mul(amount_b as u128)
            .ok_or(AmmError::MathOverflow)?;

        Ok(Self::sqrt(product) as u64)
    }

    /// Calculate liquidity tokens for subsequent deposits
    /// Formula: min(amount_a * total_supply / reserve_a, amount_b * total_supply / reserve_b)
    pub fn calculate_liquidity(
        amount_a: u64,
        amount_b: u64,
        reserve_a: u64,
        reserve_b: u64,
        total_supply: u64,
    ) -> Result<u64> {
        require!(reserve_a > 0 && reserve_b > 0, AmmError::InsufficientLiquidity);

        let liquidity_a = (amount_a as u128)
            .checked_mul(total_supply as u128)
            .ok_or(AmmError::MathOverflow)?
            .checked_div(reserve_a as u128)
            .ok_or(AmmError::DivisionByZero)?;

        let liquidity_b = (amount_b as u128)
            .checked_mul(total_supply as u128)
            .ok_or(AmmError::MathOverflow)?
            .checked_div(reserve_b as u128)
            .ok_or(AmmError::DivisionByZero)?;

        Ok(std::cmp::min(liquidity_a, liquidity_b) as u64)
    }

    /// Calculate amounts to withdraw given liquidity tokens
    pub fn calculate_withdraw_amounts(
        liquidity: u64,
        total_supply: u64,
        reserve_a: u64,
        reserve_b: u64,
    ) -> Result<(u64, u64)> {
        require!(total_supply > 0, AmmError::InsufficientLiquidity);

        let amount_a = (liquidity as u128)
            .checked_mul(reserve_a as u128)
            .ok_or(AmmError::MathOverflow)?
            .checked_div(total_supply as u128)
            .ok_or(AmmError::DivisionByZero)?;

        let amount_b = (liquidity as u128)
            .checked_mul(reserve_b as u128)
            .ok_or(AmmError::MathOverflow)?
            .checked_div(total_supply as u128)
            .ok_or(AmmError::DivisionByZero)?;

        Ok((amount_a as u64, amount_b as u64))
    }

    /// Integer square root using Newton's method
    pub fn sqrt(y: u128) -> u128 {
        if y == 0 {
            return 0;
        }

        let mut z = y;
        let mut x = y / 2 + 1;

        while x < z {
            z = x;
            x = (y / x + x) / 2;
        }

        z
    }

    /// Calculate percentage difference between two values in basis points
    pub fn calculate_deviation_bps(value1: u64, value2: u64) -> Result<u64> {
        if value1 == 0 || value2 == 0 {
            return Ok(10000); // 100% deviation if either is zero
        }

        let larger = std::cmp::max(value1, value2) as u128;
        let smaller = std::cmp::min(value1, value2) as u128;

        let deviation = larger
            .checked_sub(smaller)
            .ok_or(AmmError::MathOverflow)?
            .checked_mul(10000u128)
            .ok_or(AmmError::MathOverflow)?
            .checked_div(larger)
            .ok_or(AmmError::DivisionByZero)?;

        Ok(deviation as u64)
    }

    /// Apply basis points to an amount
    pub fn apply_bps(amount: u64, bps: u64) -> Result<u64> {
        let calculated = (amount as u128)
            .checked_mul(bps as u128)
            .ok_or(AmmError::MathOverflow)?
            .checked_div(10000u128)
            .ok_or(AmmError::DivisionByZero)?;

        Ok(calculated as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqrt() {
        assert_eq!(AmmMath::sqrt(0), 0);
        assert_eq!(AmmMath::sqrt(1), 1);
        assert_eq!(AmmMath::sqrt(4), 2);
        assert_eq!(AmmMath::sqrt(9), 3);
        assert_eq!(AmmMath::sqrt(16), 4);
        assert_eq!(AmmMath::sqrt(100), 10);
        assert_eq!(AmmMath::sqrt(10000), 100);
    }

    #[test]
    fn test_get_amount_out() {
        // With 0.3% fee (numerator=3, denominator=1000)
        // Input: 100, Reserve In: 1000, Reserve Out: 1000
        // After fee: 100 * 997 / 1000 = 99.7
        // Output: (99.7 * 1000) / (1000 + 99.7) ≈ 90.66
        let result = AmmMath::get_amount_out(100, 1000, 1000, 3, 1000);
        assert!(result.is_ok());
        assert!(result.unwrap() > 90 && result.unwrap() < 91);
    }
}

