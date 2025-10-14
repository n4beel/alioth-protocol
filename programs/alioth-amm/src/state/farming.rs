use anchor_lang::prelude::*;

/// Farming pool for LP token staking
#[account]
#[derive(Default)]
pub struct FarmingPool {
    /// Authority that can manage the farm
    pub authority: Pubkey,
    
    /// The liquidity pool this farm is for
    pub pool: Pubkey,
    
    /// LP token mint (same as pool's LP mint)
    pub lp_mint: Pubkey,
    
    /// Reward token mint
    pub reward_mint: Pubkey,
    
    /// Vault holding reward tokens
    pub reward_vault: Pubkey,
    
    /// Total LP tokens staked
    pub total_staked: u64,
    
    /// Reward tokens distributed per slot
    pub reward_per_slot: u64,
    
    /// Start slot for farming
    pub start_slot: u64,
    
    /// End slot for farming
    pub end_slot: u64,
    
    /// Last slot rewards were calculated
    pub last_update_slot: u64,
    
    /// Accumulated rewards per LP token (scaled by 10^12 for precision)
    pub accumulated_reward_per_share: u128,
    
    /// Total rewards distributed
    pub total_rewards_distributed: u64,
    
    /// Whether the farm is active
    pub is_active: bool,
    
    /// Bump seed
    pub bump: u8,
}

impl FarmingPool {
    pub const LEN: usize = 8 + // discriminator
        32 + // authority
        32 + // pool
        32 + // lp_mint
        32 + // reward_mint
        32 + // reward_vault
        8 + // total_staked
        8 + // reward_per_slot
        8 + // start_slot
        8 + // end_slot
        8 + // last_update_slot
        16 + // accumulated_reward_per_share
        8 + // total_rewards_distributed
        1 + // is_active
        1; // bump

    /// Update reward calculations up to current slot
    pub fn update_rewards(&mut self, current_slot: u64) -> Result<()> {
        if self.total_staked == 0 {
            self.last_update_slot = current_slot;
            return Ok(());
        }

        let slots_elapsed = if current_slot > self.last_update_slot {
            let end_slot = std::cmp::min(current_slot, self.end_slot);
            if end_slot > self.last_update_slot {
                end_slot.checked_sub(self.last_update_slot).unwrap()
            } else {
                0
            }
        } else {
            0
        };

        if slots_elapsed > 0 {
            let rewards = (self.reward_per_slot as u128)
                .checked_mul(slots_elapsed as u128)
                .unwrap();

            let reward_per_share = rewards
                .checked_mul(1_000_000_000_000u128) // Scale by 10^12
                .unwrap()
                .checked_div(self.total_staked as u128)
                .unwrap();

            self.accumulated_reward_per_share = self.accumulated_reward_per_share
                .checked_add(reward_per_share)
                .unwrap();

            self.total_rewards_distributed = self.total_rewards_distributed
                .checked_add((rewards as u64).min(u64::MAX))
                .unwrap();

            self.last_update_slot = current_slot;
        }

        Ok(())
    }

    /// Calculate pending rewards for a given stake amount and reward debt
    pub fn calculate_pending_rewards(
        &self,
        staked_amount: u64,
        reward_debt: u128,
    ) -> Result<u64> {
        let total_accumulated = (staked_amount as u128)
            .checked_mul(self.accumulated_reward_per_share)
            .unwrap()
            .checked_div(1_000_000_000_000u128) // Unscale
            .unwrap();

        let pending = total_accumulated
            .checked_sub(reward_debt)
            .unwrap_or(0);

        Ok(pending as u64)
    }
}

/// User's stake position in a farming pool
#[account]
#[derive(Default)]
pub struct UserStake {
    /// Owner of the stake
    pub owner: Pubkey,
    
    /// Farming pool this stake belongs to
    pub farming_pool: Pubkey,
    
    /// Amount of LP tokens staked
    pub staked_amount: u64,
    
    /// Reward debt (for reward calculation)
    pub reward_debt: u128,
    
    /// Timestamp when stake was created
    pub created_at: i64,
    
    /// Last time rewards were claimed
    pub last_claim_slot: u64,
    
    /// Total rewards claimed by user
    pub total_rewards_claimed: u64,
    
    /// Bump seed
    pub bump: u8,
}

impl UserStake {
    pub const LEN: usize = 8 + // discriminator
        32 + // owner
        32 + // farming_pool
        8 + // staked_amount
        16 + // reward_debt
        8 + // created_at
        8 + // last_claim_slot
        8 + // total_rewards_claimed
        1; // bump

    /// Update reward debt after stake changes
    pub fn update_reward_debt(&mut self, accumulated_reward_per_share: u128) {
        self.reward_debt = (self.staked_amount as u128)
            .checked_mul(accumulated_reward_per_share)
            .unwrap()
            .checked_div(1_000_000_000_000u128)
            .unwrap();
    }
}

