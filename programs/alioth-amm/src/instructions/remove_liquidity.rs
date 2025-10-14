use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Burn, Transfer};
use crate::constants::*;
use crate::errors::AmmError;
use crate::state::{Pool, LiquidityProvider};
use crate::utils::AmmMath;

#[derive(Accounts)]
pub struct RemoveLiquidity<'info> {
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

    #[account(
        mut,
        seeds = [
            LP_PROVIDER_SEED,
            pool.key().as_ref(),
            user.key().as_ref(),
        ],
        bump = lp_provider.bump,
        constraint = lp_provider.owner == user.key() @ AmmError::InvalidAuthority,
    )]
    pub lp_provider: Account<'info, LiquidityProvider>,

    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        constraint = user_token_a.mint == pool.token_a_mint @ AmmError::TokenMintMismatch,
        constraint = user_token_a.owner == user.key() @ AmmError::InvalidAuthority,
    )]
    pub user_token_a: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = user_token_b.mint == pool.token_b_mint @ AmmError::TokenMintMismatch,
        constraint = user_token_b.owner == user.key() @ AmmError::InvalidAuthority,
    )]
    pub user_token_b: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = token_a_vault.key() == pool.token_a_vault @ AmmError::InvalidPoolConfig,
    )]
    pub token_a_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = token_b_vault.key() == pool.token_b_vault @ AmmError::InvalidPoolConfig,
    )]
    pub token_b_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = lp_mint.key() == pool.lp_mint @ AmmError::InvalidPoolConfig,
    )]
    pub lp_mint: Account<'info, Mint>,

    #[account(
        mut,
        constraint = user_lp_token.mint == lp_mint.key() @ AmmError::TokenMintMismatch,
        constraint = user_lp_token.owner == user.key() @ AmmError::InvalidAuthority,
    )]
    pub user_lp_token: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(
    ctx: Context<RemoveLiquidity>,
    liquidity_amount: u64,
    min_amount_a: u64,
    min_amount_b: u64,
) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    let clock = Clock::get()?;

    // Check if pool is paused
    require!(!pool.is_paused, AmmError::PoolPaused);

    // Validate liquidity amount
    require!(liquidity_amount > 0, AmmError::ZeroAmount);
    require!(
        ctx.accounts.lp_provider.lp_token_amount >= liquidity_amount,
        AmmError::InsufficientLiquidity
    );

    // Calculate amounts to withdraw
    let (amount_a, amount_b) = AmmMath::calculate_withdraw_amounts(
        liquidity_amount,
        pool.total_lp_supply,
        pool.reserve_a,
        pool.reserve_b,
    )?;

    // Check slippage tolerance
    require!(amount_a >= min_amount_a, AmmError::SlippageExceeded);
    require!(amount_b >= min_amount_b, AmmError::SlippageExceeded);

    // Burn LP tokens from user
    let burn_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Burn {
            mint: ctx.accounts.lp_mint.to_account_info(),
            from: ctx.accounts.user_lp_token.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        },
    );
    token::burn(burn_ctx, liquidity_amount)?;

    // Transfer tokens from pool to user
    let seeds = &[
        POOL_SEED,
        pool.token_a_mint.as_ref(),
        pool.token_b_mint.as_ref(),
        &[pool.bump],
    ];
    let signer = &[&seeds[..]];

    let transfer_a_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.token_a_vault.to_account_info(),
            to: ctx.accounts.user_token_a.to_account_info(),
            authority: pool.to_account_info(),
        },
        signer,
    );
    token::transfer(transfer_a_ctx, amount_a)?;

    let transfer_b_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.token_b_vault.to_account_info(),
            to: ctx.accounts.user_token_b.to_account_info(),
            authority: pool.to_account_info(),
        },
        signer,
    );
    token::transfer(transfer_b_ctx, amount_b)?;

    // Update pool state
    pool.reserve_a = pool.reserve_a.checked_sub(amount_a).unwrap();
    pool.reserve_b = pool.reserve_b.checked_sub(amount_b).unwrap();
    pool.total_lp_supply = pool.total_lp_supply.checked_sub(liquidity_amount).unwrap();

    // Update TWAP
    pool.update_twap(clock.unix_timestamp)?;

    // Update LP provider state
    let lp_provider = &mut ctx.accounts.lp_provider;
    lp_provider.lp_token_amount = lp_provider.lp_token_amount.checked_sub(liquidity_amount).unwrap();

    msg!("Liquidity removed successfully");
    msg!("LP tokens burned: {}", liquidity_amount);
    msg!("Amount A: {}, Amount B: {}", amount_a, amount_b);

    Ok(())
}

