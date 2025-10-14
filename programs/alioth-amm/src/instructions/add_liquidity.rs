use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, MintTo, Transfer};
use crate::constants::*;
use crate::errors::AmmError;
use crate::state::{Pool, LiquidityProvider};
use crate::utils::AmmMath;

#[derive(Accounts)]
pub struct AddLiquidity<'info> {
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
        init_if_needed,
        payer = user,
        space = LiquidityProvider::LEN,
        seeds = [
            LP_PROVIDER_SEED,
            pool.key().as_ref(),
            user.key().as_ref(),
        ],
        bump
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
        init_if_needed,
        payer = user,
        associated_token::mint = lp_mint,
        associated_token::authority = user,
    )]
    pub user_lp_token: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, anchor_spl::associated_token::AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<AddLiquidity>,
    amount_a: u64,
    amount_b: u64,
    min_liquidity: u64,
) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    let clock = Clock::get()?;

    // Check if pool is paused
    require!(!pool.is_paused, AmmError::PoolPaused);

    // Validate amounts
    require!(amount_a > 0 && amount_b > 0, AmmError::ZeroAmount);

    // Calculate liquidity to mint
    let liquidity = if pool.total_lp_supply == 0 {
        // First liquidity provision
        let initial_liquidity = AmmMath::calculate_initial_liquidity(amount_a, amount_b)?;
        
        // Check minimum liquidity
        require!(
            initial_liquidity >= MINIMUM_LIQUIDITY,
            AmmError::MinimumLiquidityNotMet
        );

        initial_liquidity - MINIMUM_LIQUIDITY
    } else {
        // Subsequent liquidity provision
        AmmMath::calculate_liquidity(
            amount_a,
            amount_b,
            pool.reserve_a,
            pool.reserve_b,
            pool.total_lp_supply,
        )?
    };

    // Check slippage tolerance
    require!(liquidity >= min_liquidity, AmmError::SlippageExceeded);

    // Transfer tokens from user to pool
    let transfer_a_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.user_token_a.to_account_info(),
            to: ctx.accounts.token_a_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        },
    );
    token::transfer(transfer_a_ctx, amount_a)?;

    let transfer_b_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.user_token_b.to_account_info(),
            to: ctx.accounts.token_b_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        },
    );
    token::transfer(transfer_b_ctx, amount_b)?;

    // Mint LP tokens to user
    let seeds = &[
        POOL_SEED,
        pool.token_a_mint.as_ref(),
        pool.token_b_mint.as_ref(),
        &[pool.bump],
    ];
    let signer = &[&seeds[..]];

    let mint_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        MintTo {
            mint: ctx.accounts.lp_mint.to_account_info(),
            to: ctx.accounts.user_lp_token.to_account_info(),
            authority: pool.to_account_info(),
        },
        signer,
    );
    token::mint_to(mint_ctx, liquidity)?;

    // Update pool state
    pool.reserve_a = pool.reserve_a.checked_add(amount_a).unwrap();
    pool.reserve_b = pool.reserve_b.checked_add(amount_b).unwrap();
    pool.total_lp_supply = pool.total_lp_supply.checked_add(liquidity).unwrap();

    // Update TWAP
    pool.update_twap(clock.unix_timestamp)?;

    // Update or initialize LP provider state
    let lp_provider = &mut ctx.accounts.lp_provider;
    if lp_provider.lp_token_amount == 0 {
        lp_provider.owner = ctx.accounts.user.key();
        lp_provider.pool = pool.key();
        lp_provider.initial_deposit_a = amount_a;
        lp_provider.initial_deposit_b = amount_b;
        lp_provider.created_at = clock.unix_timestamp;
        lp_provider.bump = ctx.bumps.lp_provider;
    }
    lp_provider.lp_token_amount = lp_provider.lp_token_amount.checked_add(liquidity).unwrap();

    msg!("Liquidity added successfully");
    msg!("Amount A: {}, Amount B: {}", amount_a, amount_b);
    msg!("LP tokens minted: {}", liquidity);

    Ok(())
}

