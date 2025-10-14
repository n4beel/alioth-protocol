use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::constants::*;
use crate::errors::AmmError;
use crate::state::Pool;
use crate::utils::{AmmMath, OracleHelper};

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(
        mut,
        seeds = [
            POOL_SEED,
            pool.token_a_mint.as_ref(),
            pool.token_b_mint.as_ref(),
        ],
        bump = pool.bump,
    )]
    pub pool: Account<'info, Pool>,

    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        constraint = user_token_in.owner == user.key() @ AmmError::InvalidAuthority,
    )]
    pub user_token_in: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = user_token_out.owner == user.key() @ AmmError::InvalidAuthority,
    )]
    pub user_token_out: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = pool_token_in.key() == pool.token_a_vault || pool_token_in.key() == pool.token_b_vault @ AmmError::InvalidPoolConfig,
    )]
    pub pool_token_in: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = pool_token_out.key() == pool.token_a_vault || pool_token_out.key() == pool.token_b_vault @ AmmError::InvalidPoolConfig,
    )]
    pub pool_token_out: Account<'info, TokenAccount>,

    /// CHECK: Pyth oracle account for token A
    #[account(
        constraint = oracle_a.key() == pool.oracle_a @ AmmError::InvalidOracle,
    )]
    pub oracle_a: AccountInfo<'info>,

    /// CHECK: Pyth oracle account for token B
    #[account(
        constraint = oracle_b.key() == pool.oracle_b @ AmmError::InvalidOracle,
    )]
    pub oracle_b: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(
    ctx: Context<Swap>,
    amount_in: u64,
    minimum_amount_out: u64,
    is_a_to_b: bool,
) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    let clock = Clock::get()?;

    // Check if pool is paused
    require!(!pool.is_paused, AmmError::PoolPaused);

    // Validate swap amount
    require!(amount_in > 0, AmmError::ZeroAmount);

    // Verify token accounts match swap direction
    if is_a_to_b {
        require!(
            ctx.accounts.user_token_in.mint == pool.token_a_mint,
            AmmError::TokenMintMismatch
        );
        require!(
            ctx.accounts.user_token_out.mint == pool.token_b_mint,
            AmmError::TokenMintMismatch
        );
        require!(
            ctx.accounts.pool_token_in.key() == pool.token_a_vault,
            AmmError::InvalidPoolConfig
        );
        require!(
            ctx.accounts.pool_token_out.key() == pool.token_b_vault,
            AmmError::InvalidPoolConfig
        );
    } else {
        require!(
            ctx.accounts.user_token_in.mint == pool.token_b_mint,
            AmmError::TokenMintMismatch
        );
        require!(
            ctx.accounts.user_token_out.mint == pool.token_a_mint,
            AmmError::TokenMintMismatch
        );
        require!(
            ctx.accounts.pool_token_in.key() == pool.token_b_vault,
            AmmError::InvalidPoolConfig
        );
        require!(
            ctx.accounts.pool_token_out.key() == pool.token_a_vault,
            AmmError::InvalidPoolConfig
        );
    }

    // Calculate output amount using constant product formula
    let (reserve_in, reserve_out) = if is_a_to_b {
        (pool.reserve_a, pool.reserve_b)
    } else {
        (pool.reserve_b, pool.reserve_a)
    };

    let amount_out = AmmMath::get_amount_out(
        amount_in,
        reserve_in,
        reserve_out,
        pool.fee_numerator,
        pool.fee_denominator,
    )?;

    // Check slippage tolerance
    require!(amount_out >= minimum_amount_out, AmmError::SlippageExceeded);

    // Validate swap price against oracle
    OracleHelper::validate_swap_price(
        amount_in,
        amount_out,
        &ctx.accounts.oracle_a,
        &ctx.accounts.oracle_b,
        pool.oracle_max_age,
        pool.oracle_max_deviation_bps,
        is_a_to_b,
    )?;

    // Calculate fee
    let fee_amount = amount_in
        .checked_mul(pool.fee_numerator)
        .unwrap()
        .checked_div(pool.fee_denominator)
        .unwrap();

    // Transfer tokens from user to pool
    let transfer_in_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.user_token_in.to_account_info(),
            to: ctx.accounts.pool_token_in.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        },
    );
    token::transfer(transfer_in_ctx, amount_in)?;

    // Transfer tokens from pool to user
    let seeds = &[
        POOL_SEED,
        pool.token_a_mint.as_ref(),
        pool.token_b_mint.as_ref(),
        &[pool.bump],
    ];
    let signer = &[&seeds[..]];

    let transfer_out_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.pool_token_out.to_account_info(),
            to: ctx.accounts.user_token_out.to_account_info(),
            authority: pool.to_account_info(),
        },
        signer,
    );
    token::transfer(transfer_out_ctx, amount_out)?;

    // Update pool reserves
    if is_a_to_b {
        pool.reserve_a = pool.reserve_a.checked_add(amount_in).unwrap();
        pool.reserve_b = pool.reserve_b.checked_sub(amount_out).unwrap();
        pool.total_volume_a = pool.total_volume_a.checked_add(amount_in).unwrap();
        pool.total_fees_a = pool.total_fees_a.checked_add(fee_amount).unwrap();
    } else {
        pool.reserve_b = pool.reserve_b.checked_add(amount_in).unwrap();
        pool.reserve_a = pool.reserve_a.checked_sub(amount_out).unwrap();
        pool.total_volume_b = pool.total_volume_b.checked_add(amount_in).unwrap();
        pool.total_fees_b = pool.total_fees_b.checked_add(fee_amount).unwrap();
    }

    // Update TWAP
    pool.update_twap(clock.unix_timestamp)?;

    msg!("Swap executed successfully");
    msg!("Amount in: {}, Amount out: {}", amount_in, amount_out);
    msg!("Fee collected: {}", fee_amount);
    msg!("Direction: {}", if is_a_to_b { "A -> B" } else { "B -> A" });

    Ok(())
}

