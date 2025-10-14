use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod instructions;
pub mod state;
pub mod utils;

use instructions::*;

declare_id!("AMMorecL11111111111111111111111111111111111");

#[program]
pub mod alioth_amm {
    use super::*;

    /// Initialize a new liquidity pool
    pub fn initialize_pool(
        ctx: Context<InitializePool>,
        fee_numerator: u64,
        fee_denominator: u64,
        oracle_max_age: i64,
        oracle_max_deviation_bps: u64,
    ) -> Result<()> {
        instructions::initialize_pool::handler(
            ctx,
            fee_numerator,
            fee_denominator,
            oracle_max_age,
            oracle_max_deviation_bps,
        )
    }

    /// Add liquidity to a pool
    pub fn add_liquidity(
        ctx: Context<AddLiquidity>,
        amount_a: u64,
        amount_b: u64,
        min_liquidity: u64,
    ) -> Result<()> {
        instructions::add_liquidity::handler(ctx, amount_a, amount_b, min_liquidity)
    }

    /// Remove liquidity from a pool
    pub fn remove_liquidity(
        ctx: Context<RemoveLiquidity>,
        liquidity_amount: u64,
        min_amount_a: u64,
        min_amount_b: u64,
    ) -> Result<()> {
        instructions::remove_liquidity::handler(ctx, liquidity_amount, min_amount_a, min_amount_b)
    }

    /// Swap tokens with oracle price validation
    pub fn swap(
        ctx: Context<Swap>,
        amount_in: u64,
        minimum_amount_out: u64,
        is_a_to_b: bool,
    ) -> Result<()> {
        instructions::swap::handler(ctx, amount_in, minimum_amount_out, is_a_to_b)
    }

    /// Execute a flash loan
    pub fn flash_loan(ctx: Context<FlashLoan>, amount_a: u64, amount_b: u64) -> Result<()> {
        instructions::flash_loan::handler(ctx, amount_a, amount_b)
    }

    /// Repay a flash loan (must be called in the same transaction)
    pub fn flash_loan_repay(ctx: Context<FlashLoanRepay>) -> Result<()> {
        instructions::flash_loan::repay_handler(ctx)
    }

    /// Initialize farming pool for LP token staking
    pub fn initialize_farm(
        ctx: Context<InitializeFarm>,
        reward_per_slot: u64,
        start_slot: u64,
        end_slot: u64,
    ) -> Result<()> {
        instructions::farming::initialize_farm_handler(ctx, reward_per_slot, start_slot, end_slot)
    }

    /// Stake LP tokens to earn rewards
    pub fn stake(ctx: Context<Stake>, amount: u64) -> Result<()> {
        instructions::farming::stake_handler(ctx, amount)
    }

    /// Unstake LP tokens
    pub fn unstake(ctx: Context<Unstake>, amount: u64) -> Result<()> {
        instructions::farming::unstake_handler(ctx, amount)
    }

    /// Claim farming rewards
    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        instructions::farming::claim_rewards_handler(ctx)
    }

    /// Multi-hop swap through multiple pools
    pub fn multi_hop_swap(
        ctx: Context<MultiHopSwap>,
        amount_in: u64,
        minimum_amount_out: u64,
        hops: u8,
    ) -> Result<()> {
        instructions::multi_hop::handler(ctx, amount_in, minimum_amount_out, hops)
    }

    /// Pause pool (admin only)
    pub fn pause_pool(ctx: Context<PausePool>) -> Result<()> {
        instructions::admin::pause_pool_handler(ctx)
    }

    /// Unpause pool (admin only)
    pub fn unpause_pool(ctx: Context<UnpausePool>) -> Result<()> {
        instructions::admin::unpause_pool_handler(ctx)
    }

    /// Update pool fees (admin only)
    pub fn update_fees(
        ctx: Context<UpdateFees>,
        new_fee_numerator: u64,
        new_fee_denominator: u64,
    ) -> Result<()> {
        instructions::admin::update_fees_handler(ctx, new_fee_numerator, new_fee_denominator)
    }

    /// Transfer pool authority (admin only)
    pub fn transfer_authority(ctx: Context<TransferAuthority>) -> Result<()> {
        instructions::admin::transfer_authority_handler(ctx)
    }

    /// Update oracle configuration (admin only)
    pub fn update_oracle_config(
        ctx: Context<UpdateOracleConfig>,
        new_max_age: Option<i64>,
        new_max_deviation_bps: Option<u64>,
    ) -> Result<()> {
        instructions::admin::update_oracle_config_handler(ctx, new_max_age, new_max_deviation_bps)
    }
}
