use anchor_lang::prelude::*;
use crate::constants::*;
use crate::errors::AmmError;
use crate::state::Pool;

// ========== Pause Pool ==========

#[derive(Accounts)]
pub struct PausePool<'info> {
    #[account(
        mut,
        seeds = [
            POOL_SEED,
            pool.token_a_mint.as_ref(),
            pool.token_b_mint.as_ref(),
        ],
        bump = pool.bump,
        constraint = pool.authority == authority.key() @ AmmError::Unauthorized,
    )]
    pub pool: Account<'info, Pool>,

    pub authority: Signer<'info>,
}

pub fn pause_pool_handler(ctx: Context<PausePool>) -> Result<()> {
    let pool = &mut ctx.accounts.pool;

    require!(!pool.is_paused, AmmError::PoolPaused);

    pool.is_paused = true;

    msg!("Pool paused successfully");
    msg!("Pool: {}", pool.key());

    Ok(())
}

// ========== Unpause Pool ==========

#[derive(Accounts)]
pub struct UnpausePool<'info> {
    #[account(
        mut,
        seeds = [
            POOL_SEED,
            pool.token_a_mint.as_ref(),
            pool.token_b_mint.as_ref(),
        ],
        bump = pool.bump,
        constraint = pool.authority == authority.key() @ AmmError::Unauthorized,
    )]
    pub pool: Account<'info, Pool>,

    pub authority: Signer<'info>,
}

pub fn unpause_pool_handler(ctx: Context<UnpausePool>) -> Result<()> {
    let pool = &mut ctx.accounts.pool;

    require!(pool.is_paused, AmmError::InvalidPoolConfig);

    pool.is_paused = false;

    msg!("Pool unpaused successfully");
    msg!("Pool: {}", pool.key());

    Ok(())
}

// ========== Update Fees ==========

#[derive(Accounts)]
pub struct UpdateFees<'info> {
    #[account(
        mut,
        seeds = [
            POOL_SEED,
            pool.token_a_mint.as_ref(),
            pool.token_b_mint.as_ref(),
        ],
        bump = pool.bump,
        constraint = pool.authority == authority.key() @ AmmError::Unauthorized,
    )]
    pub pool: Account<'info, Pool>,

    pub authority: Signer<'info>,
}

pub fn update_fees_handler(
    ctx: Context<UpdateFees>,
    new_fee_numerator: u64,
    new_fee_denominator: u64,
) -> Result<()> {
    let pool = &mut ctx.accounts.pool;

    // Validate new fee parameters
    require!(
        new_fee_denominator > 0 && new_fee_numerator < new_fee_denominator,
        AmmError::InvalidFeeParameters
    );

    // Validate fee is reasonable (max 10%)
    require!(
        new_fee_numerator * 10 <= new_fee_denominator,
        AmmError::InvalidFeeParameters
    );

    let old_fee_numerator = pool.fee_numerator;
    let old_fee_denominator = pool.fee_denominator;

    pool.fee_numerator = new_fee_numerator;
    pool.fee_denominator = new_fee_denominator;

    msg!("Pool fees updated successfully");
    msg!("Old fee: {}%", (old_fee_numerator as f64 / old_fee_denominator as f64) * 100.0);
    msg!("New fee: {}%", (new_fee_numerator as f64 / new_fee_denominator as f64) * 100.0);

    Ok(())
}

// ========== Transfer Authority ==========

#[derive(Accounts)]
pub struct TransferAuthority<'info> {
    #[account(
        mut,
        seeds = [
            POOL_SEED,
            pool.token_a_mint.as_ref(),
            pool.token_b_mint.as_ref(),
        ],
        bump = pool.bump,
        constraint = pool.authority == authority.key() @ AmmError::Unauthorized,
    )]
    pub pool: Account<'info, Pool>,

    pub authority: Signer<'info>,

    /// CHECK: New authority can be any valid pubkey
    pub new_authority: AccountInfo<'info>,
}

pub fn transfer_authority_handler(ctx: Context<TransferAuthority>) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    let old_authority = pool.authority;

    pool.authority = ctx.accounts.new_authority.key();

    msg!("Pool authority transferred successfully");
    msg!("Old authority: {}", old_authority);
    msg!("New authority: {}", pool.authority);

    Ok(())
}

// ========== Update Oracle Config ==========

#[derive(Accounts)]
pub struct UpdateOracleConfig<'info> {
    #[account(
        mut,
        seeds = [
            POOL_SEED,
            pool.token_a_mint.as_ref(),
            pool.token_b_mint.as_ref(),
        ],
        bump = pool.bump,
        constraint = pool.authority == authority.key() @ AmmError::Unauthorized,
    )]
    pub pool: Account<'info, Pool>,

    pub authority: Signer<'info>,
}

pub fn update_oracle_config_handler(
    ctx: Context<UpdateOracleConfig>,
    new_max_age: Option<i64>,
    new_max_deviation_bps: Option<u64>,
) -> Result<()> {
    let pool = &mut ctx.accounts.pool;

    if let Some(max_age) = new_max_age {
        require!(max_age > 0, AmmError::InvalidOracle);
        pool.oracle_max_age = max_age;
        msg!("Oracle max age updated to: {}", max_age);
    }

    if let Some(max_deviation) = new_max_deviation_bps {
        require!(max_deviation <= MAX_BPS, AmmError::InvalidOracle);
        pool.oracle_max_deviation_bps = max_deviation;
        msg!("Oracle max deviation updated to: {} bps", max_deviation);
    }

    msg!("Oracle configuration updated successfully");

    Ok(())
}

