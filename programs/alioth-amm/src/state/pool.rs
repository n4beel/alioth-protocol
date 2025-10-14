use anchor_lang::prelude::*;

/// Main liquidity pool state account
#[account]
#[derive(Default)]
pub struct Pool {
    /// Authority that can manage the pool
    pub authority: Pubkey,
    
    /// Token A mint
    pub token_a_mint: Pubkey,
    
    /// Token B mint
    pub token_b_mint: Pubkey,
    
    /// Pool's token A account
    pub token_a_vault: Pubkey,
    
    /// Pool's token B account
    pub token_b_vault: Pubkey,
    
    /// LP token mint
    pub lp_mint: Pubkey,
    
    /// Current reserve of token A
    pub reserve_a: u64,
    
    /// Current reserve of token B
    pub reserve_b: u64,
    
    /// Total LP tokens in circulation
    pub total_lp_supply: u64,
    
    /// Fee numerator (e.g., 3 for 0.3%)
    pub fee_numerator: u64,
    
    /// Fee denominator (e.g., 1000 for 0.3%)
    pub fee_denominator: u64,
    
    /// Pyth oracle account for token A
    pub oracle_a: Pubkey,
    
    /// Pyth oracle account for token B
    pub oracle_b: Pubkey,
    
    /// Maximum age for oracle prices (in seconds)
    pub oracle_max_age: i64,
    
    /// Maximum allowed deviation from oracle price in basis points (1 bps = 0.01%)
    pub oracle_max_deviation_bps: u64,
    
    /// Whether the pool is paused
    pub is_paused: bool,
    
    /// Cumulative price A (for TWAP calculation)
    pub cumulative_price_a: u128,
    
    /// Cumulative price B (for TWAP calculation)
    pub cumulative_price_b: u128,
    
    /// Last update timestamp for TWAP
    pub last_update_timestamp: i64,
    
    /// Total volume in token A
    pub total_volume_a: u64,
    
    /// Total volume in token B
    pub total_volume_b: u64,
    
    /// Total fees collected in token A
    pub total_fees_a: u64,
    
    /// Total fees collected in token B
    pub total_fees_b: u64,
    
    /// Bump seed for PDA
    pub bump: u8,
}

impl Pool {
    pub const LEN: usize = 8 + // discriminator
        32 + // authority
        32 + // token_a_mint
        32 + // token_b_mint
        32 + // token_a_vault
        32 + // token_b_vault
        32 + // lp_mint
        8 + // reserve_a
        8 + // reserve_b
        8 + // total_lp_supply
        8 + // fee_numerator
        8 + // fee_denominator
        32 + // oracle_a
        32 + // oracle_b
        8 + // oracle_max_age
        8 + // oracle_max_deviation_bps
        1 + // is_paused
        16 + // cumulative_price_a
        16 + // cumulative_price_b
        8 + // last_update_timestamp
        8 + // total_volume_a
        8 + // total_volume_b
        8 + // total_fees_a
        8 + // total_fees_b
        1; // bump

    /// Calculate the current price of token A in terms of token B
    pub fn get_spot_price(&self) -> Result<u64> {
        require!(self.reserve_a > 0 && self.reserve_b > 0, crate::errors::AmmError::InsufficientLiquidity);
        
        // Price = reserve_b / reserve_a (scaled by 10^9 for precision)
        let price = (self.reserve_b as u128)
            .checked_mul(1_000_000_000u128)
            .unwrap()
            .checked_div(self.reserve_a as u128)
            .unwrap();
        
        Ok(price as u64)
    }

    /// Update TWAP accumulators
    pub fn update_twap(&mut self, current_timestamp: i64) -> Result<()> {
        if self.last_update_timestamp == 0 {
            self.last_update_timestamp = current_timestamp;
            return Ok(());
        }

        let time_elapsed = current_timestamp
            .checked_sub(self.last_update_timestamp)
            .unwrap_or(0);

        if time_elapsed > 0 && self.reserve_a > 0 && self.reserve_b > 0 {
            // Calculate price * time_elapsed
            let price_a = (self.reserve_b as u128)
                .checked_mul(time_elapsed as u128)
                .unwrap()
                .checked_div(self.reserve_a as u128)
                .unwrap();

            let price_b = (self.reserve_a as u128)
                .checked_mul(time_elapsed as u128)
                .unwrap()
                .checked_div(self.reserve_b as u128)
                .unwrap();

            self.cumulative_price_a = self.cumulative_price_a
                .checked_add(price_a)
                .unwrap();

            self.cumulative_price_b = self.cumulative_price_b
                .checked_add(price_b)
                .unwrap();

            self.last_update_timestamp = current_timestamp;
        }

        Ok(())
    }

    /// Get TWAP over a period
    pub fn get_twap(&self, from_timestamp: i64, to_timestamp: i64) -> Result<u64> {
        require!(
            to_timestamp > from_timestamp,
            crate::errors::AmmError::InvalidTimeRange
        );

        let time_delta = to_timestamp.checked_sub(from_timestamp).unwrap();
        let twap = self.cumulative_price_a
            .checked_div(time_delta as u128)
            .unwrap();

        Ok(twap as u64)
    }
}

/// Liquidity provider position
#[account]
#[derive(Default)]
pub struct LiquidityProvider {
    /// Owner of the position
    pub owner: Pubkey,
    
    /// Pool this position belongs to
    pub pool: Pubkey,
    
    /// Amount of LP tokens held
    pub lp_token_amount: u64,
    
    /// Initial deposit of token A
    pub initial_deposit_a: u64,
    
    /// Initial deposit of token B
    pub initial_deposit_b: u64,
    
    /// Timestamp of first deposit
    pub created_at: i64,
    
    /// Bump seed
    pub bump: u8,
}

impl LiquidityProvider {
    pub const LEN: usize = 8 + // discriminator
        32 + // owner
        32 + // pool
        8 + // lp_token_amount
        8 + // initial_deposit_a
        8 + // initial_deposit_b
        8 + // created_at
        1; // bump
}

