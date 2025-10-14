use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::constants::*;
use crate::errors::AmmError;
use crate::state::Pool;

#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(
        init,
        payer = authority,
        space = Pool::LEN,
        seeds = [
            POOL_SEED,
            token_a_mint.key().as_ref(),
            token_b_mint.key().as_ref(),
        ],
        bump
    )]
    pub pool: Account<'info, Pool>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub token_a_mint: Account<'info, Mint>,
    pub token_b_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = authority,
        seeds = [
            LP_MINT_SEED,
            pool.key().as_ref(),
        ],
        bump,
        mint::decimals = 9,
        mint::authority = pool,
    )]
    pub lp_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = authority,
        seeds = [
            TOKEN_A_VAULT_SEED,
            pool.key().as_ref(),
        ],
        bump,
        token::mint = token_a_mint,
        token::authority = pool,
    )]
    pub token_a_vault: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = authority,
        seeds = [
            TOKEN_B_VAULT_SEED,
            pool.key().as_ref(),
        ],
        bump,
        token::mint = token_b_mint,
        token::authority = pool,
    )]
    pub token_b_vault: Account<'info, TokenAccount>,

    /// CHECK: Pyth oracle account for token A - validated in handler
    pub oracle_a: AccountInfo<'info>,

    /// CHECK: Pyth oracle account for token B - validated in handler
    pub oracle_b: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(
    ctx: Context<InitializePool>,
    fee_numerator: u64,
    fee_denominator: u64,
    oracle_max_age: i64,
    oracle_max_deviation_bps: u64,
) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    let clock = Clock::get()?;

    // Validate fee parameters
    require!(
        fee_denominator > 0 && fee_numerator < fee_denominator,
        AmmError::InvalidFeeParameters
    );

    // Validate fee is reasonable (max 10%)
    require!(
        fee_numerator * 10 <= fee_denominator,
        AmmError::InvalidFeeParameters
    );

    // Validate oracle parameters
    require!(oracle_max_age > 0, AmmError::InvalidOracle);
    require!(
        oracle_max_deviation_bps <= MAX_BPS,
        AmmError::InvalidOracle
    );

    // Ensure token mints are different
    require!(
        ctx.accounts.token_a_mint.key() != ctx.accounts.token_b_mint.key(),
        AmmError::InvalidPoolConfig
    );

    // Initialize pool state
    pool.authority = ctx.accounts.authority.key();
    pool.token_a_mint = ctx.accounts.token_a_mint.key();
    pool.token_b_mint = ctx.accounts.token_b_mint.key();
    pool.token_a_vault = ctx.accounts.token_a_vault.key();
    pool.token_b_vault = ctx.accounts.token_b_vault.key();
    pool.lp_mint = ctx.accounts.lp_mint.key();
    pool.reserve_a = 0;
    pool.reserve_b = 0;
    pool.total_lp_supply = 0;
    pool.fee_numerator = fee_numerator;
    pool.fee_denominator = fee_denominator;
    pool.oracle_a = ctx.accounts.oracle_a.key();
    pool.oracle_b = ctx.accounts.oracle_b.key();
    pool.oracle_max_age = oracle_max_age;
    pool.oracle_max_deviation_bps = oracle_max_deviation_bps;
    pool.is_paused = false;
    pool.cumulative_price_a = 0;
    pool.cumulative_price_b = 0;
    pool.last_update_timestamp = clock.unix_timestamp;
    pool.total_volume_a = 0;
    pool.total_volume_b = 0;
    pool.total_fees_a = 0;
    pool.total_fees_b = 0;
    pool.bump = ctx.bumps.pool;

    msg!("Pool initialized successfully");
    msg!("Token A: {}", pool.token_a_mint);
    msg!("Token B: {}", pool.token_b_mint);
    msg!("Fee: {}%", (fee_numerator as f64 / fee_denominator as f64) * 100.0);

    Ok(())
}

