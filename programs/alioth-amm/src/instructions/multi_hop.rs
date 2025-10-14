use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::constants::*;
use crate::errors::AmmError;
use crate::state::Pool;
use crate::utils::{AmmMath, OracleHelper};

/// Multi-hop swap through up to 3 pools
/// Example: Token A -> Token B -> Token C -> Token D
#[derive(Accounts)]
#[instruction(hops: u8)]
pub struct MultiHopSwap<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    // Pool 1 (required)
    #[account(
        mut,
        seeds = [
            POOL_SEED,
            pool_1.token_a_mint.as_ref(),
            pool_1.token_b_mint.as_ref(),
        ],
        bump = pool_1.bump,
    )]
    pub pool_1: Account<'info, Pool>,

    // Pool 2 (optional, required if hops >= 2)
    #[account(
        mut,
        seeds = [
            POOL_SEED,
            pool_2.token_a_mint.as_ref(),
            pool_2.token_b_mint.as_ref(),
        ],
        bump = pool_2.bump,
    )]
    pub pool_2: Option<Account<'info, Pool>>,

    // Pool 3 (optional, required if hops == 3)
    #[account(
        mut,
        seeds = [
            POOL_SEED,
            pool_3.token_a_mint.as_ref(),
            pool_3.token_b_mint.as_ref(),
        ],
        bump = pool_3.bump,
    )]
    pub pool_3: Option<Account<'info, Pool>>,

    // User's initial input token account
    #[account(
        mut,
        constraint = user_token_in.owner == user.key() @ AmmError::InvalidAuthority,
    )]
    pub user_token_in: Account<'info, TokenAccount>,

    // User's final output token account
    #[account(
        mut,
        constraint = user_token_out.owner == user.key() @ AmmError::InvalidAuthority,
    )]
    pub user_token_out: Account<'info, TokenAccount>,

    // Intermediate token account 1 (for user, between hop 1 and 2)
    #[account(
        mut,
        constraint = intermediate_token_1.owner == user.key() @ AmmError::InvalidAuthority,
    )]
    pub intermediate_token_1: Option<Account<'info, TokenAccount>>,

    // Intermediate token account 2 (for user, between hop 2 and 3)
    #[account(
        mut,
        constraint = intermediate_token_2.owner == user.key() @ AmmError::InvalidAuthority,
    )]
    pub intermediate_token_2: Option<Account<'info, TokenAccount>>,

    // Pool 1 vaults
    #[account(mut)]
    pub pool_1_vault_in: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub pool_1_vault_out: Account<'info, TokenAccount>,

    // Pool 2 vaults (if applicable)
    #[account(mut)]
    pub pool_2_vault_in: Option<Account<'info, TokenAccount>>,
    
    #[account(mut)]
    pub pool_2_vault_out: Option<Account<'info, TokenAccount>>,

    // Pool 3 vaults (if applicable)
    #[account(mut)]
    pub pool_3_vault_in: Option<Account<'info, TokenAccount>>,
    
    #[account(mut)]
    pub pool_3_vault_out: Option<Account<'info, TokenAccount>>,

    // Oracle accounts for each pool
    /// CHECK: Validated in handler
    pub oracle_1_a: AccountInfo<'info>,
    
    /// CHECK: Validated in handler
    pub oracle_1_b: AccountInfo<'info>,
    
    /// CHECK: Validated in handler
    pub oracle_2_a: Option<AccountInfo<'info>>,
    
    /// CHECK: Validated in handler
    pub oracle_2_b: Option<AccountInfo<'info>>,
    
    /// CHECK: Validated in handler
    pub oracle_3_a: Option<AccountInfo<'info>>,
    
    /// CHECK: Validated in handler
    pub oracle_3_b: Option<AccountInfo<'info>>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(
    ctx: Context<MultiHopSwap>,
    amount_in: u64,
    minimum_amount_out: u64,
    hops: u8,
) -> Result<()> {
    // Validate hops
    require!(hops >= 1 && hops <= MAX_SWAP_HOPS, AmmError::MaxHopsExceeded);

    let clock = Clock::get()?;
    let mut current_amount = amount_in;

    // Validate initial amount
    require!(amount_in > 0, AmmError::ZeroAmount);

    // ========== HOP 1 ==========
    let pool_1 = &mut ctx.accounts.pool_1;
    require!(!pool_1.is_paused, AmmError::PoolPaused);

    // Determine swap direction for hop 1
    let is_a_to_b_1 = ctx.accounts.user_token_in.mint == pool_1.token_a_mint;
    require!(
        is_a_to_b_1 && ctx.accounts.pool_1_vault_in.mint == pool_1.token_a_mint ||
        !is_a_to_b_1 && ctx.accounts.pool_1_vault_in.mint == pool_1.token_b_mint,
        AmmError::InvalidSwapRoute
    );

    // Calculate output from hop 1
    let (reserve_in_1, reserve_out_1) = if is_a_to_b_1 {
        (pool_1.reserve_a, pool_1.reserve_b)
    } else {
        (pool_1.reserve_b, pool_1.reserve_a)
    };

    let amount_out_1 = AmmMath::get_amount_out(
        current_amount,
        reserve_in_1,
        reserve_out_1,
        pool_1.fee_numerator,
        pool_1.fee_denominator,
    )?;

    // Validate with oracle
    OracleHelper::validate_swap_price(
        current_amount,
        amount_out_1,
        &ctx.accounts.oracle_1_a,
        &ctx.accounts.oracle_1_b,
        pool_1.oracle_max_age,
        pool_1.oracle_max_deviation_bps,
        is_a_to_b_1,
    )?;

    // Execute hop 1
    // Transfer from user to pool 1
    let transfer_1_in_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.user_token_in.to_account_info(),
            to: ctx.accounts.pool_1_vault_in.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        },
    );
    token::transfer(transfer_1_in_ctx, current_amount)?;

    // Transfer from pool 1 to intermediate or final destination
    let seeds_1 = &[
        POOL_SEED,
        pool_1.token_a_mint.as_ref(),
        pool_1.token_b_mint.as_ref(),
        &[pool_1.bump],
    ];
    let signer_1 = &[&seeds_1[..]];

    let destination_1 = if hops > 1 {
        ctx.accounts.intermediate_token_1.as_ref()
            .ok_or(AmmError::InvalidSwapRoute)?
            .to_account_info()
    } else {
        ctx.accounts.user_token_out.to_account_info()
    };

    let transfer_1_out_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.pool_1_vault_out.to_account_info(),
            to: destination_1,
            authority: pool_1.to_account_info(),
        },
        signer_1,
    );
    token::transfer(transfer_1_out_ctx, amount_out_1)?;

    // Update pool 1 state
    let fee_1 = current_amount
        .checked_mul(pool_1.fee_numerator)
        .unwrap()
        .checked_div(pool_1.fee_denominator)
        .unwrap();

    if is_a_to_b_1 {
        pool_1.reserve_a = pool_1.reserve_a.checked_add(current_amount).unwrap();
        pool_1.reserve_b = pool_1.reserve_b.checked_sub(amount_out_1).unwrap();
        pool_1.total_volume_a = pool_1.total_volume_a.checked_add(current_amount).unwrap();
        pool_1.total_fees_a = pool_1.total_fees_a.checked_add(fee_1).unwrap();
    } else {
        pool_1.reserve_b = pool_1.reserve_b.checked_add(current_amount).unwrap();
        pool_1.reserve_a = pool_1.reserve_a.checked_sub(amount_out_1).unwrap();
        pool_1.total_volume_b = pool_1.total_volume_b.checked_add(current_amount).unwrap();
        pool_1.total_fees_b = pool_1.total_fees_b.checked_add(fee_1).unwrap();
    }
    pool_1.update_twap(clock.unix_timestamp)?;

    current_amount = amount_out_1;

    // ========== HOP 2 (if applicable) ==========
    if hops >= 2 {
        let pool_2 = ctx.accounts.pool_2.as_mut()
            .ok_or(AmmError::InvalidSwapRoute)?;
        require!(!pool_2.is_paused, AmmError::PoolPaused);

        let is_a_to_b_2 = ctx.accounts.intermediate_token_1.as_ref().unwrap().mint == pool_2.token_a_mint;
        
        let (reserve_in_2, reserve_out_2) = if is_a_to_b_2 {
            (pool_2.reserve_a, pool_2.reserve_b)
        } else {
            (pool_2.reserve_b, pool_2.reserve_a)
        };

        let amount_out_2 = AmmMath::get_amount_out(
            current_amount,
            reserve_in_2,
            reserve_out_2,
            pool_2.fee_numerator,
            pool_2.fee_denominator,
        )?;

        // Validate with oracle
        OracleHelper::validate_swap_price(
            current_amount,
            amount_out_2,
            ctx.accounts.oracle_2_a.as_ref().ok_or(AmmError::InvalidOracle)?,
            ctx.accounts.oracle_2_b.as_ref().ok_or(AmmError::InvalidOracle)?,
            pool_2.oracle_max_age,
            pool_2.oracle_max_deviation_bps,
            is_a_to_b_2,
        )?;

        // Execute hop 2
        let transfer_2_in_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.intermediate_token_1.as_ref().unwrap().to_account_info(),
                to: ctx.accounts.pool_2_vault_in.as_ref().ok_or(AmmError::InvalidSwapRoute)?.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        token::transfer(transfer_2_in_ctx, current_amount)?;

        let seeds_2 = &[
            POOL_SEED,
            pool_2.token_a_mint.as_ref(),
            pool_2.token_b_mint.as_ref(),
            &[pool_2.bump],
        ];
        let signer_2 = &[&seeds_2[..]];

        let destination_2 = if hops > 2 {
            ctx.accounts.intermediate_token_2.as_ref()
                .ok_or(AmmError::InvalidSwapRoute)?
                .to_account_info()
        } else {
            ctx.accounts.user_token_out.to_account_info()
        };

        let transfer_2_out_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.pool_2_vault_out.as_ref().ok_or(AmmError::InvalidSwapRoute)?.to_account_info(),
                to: destination_2,
                authority: pool_2.to_account_info(),
            },
            signer_2,
        );
        token::transfer(transfer_2_out_ctx, amount_out_2)?;

        // Update pool 2 state
        let fee_2 = current_amount
            .checked_mul(pool_2.fee_numerator)
            .unwrap()
            .checked_div(pool_2.fee_denominator)
            .unwrap();

        if is_a_to_b_2 {
            pool_2.reserve_a = pool_2.reserve_a.checked_add(current_amount).unwrap();
            pool_2.reserve_b = pool_2.reserve_b.checked_sub(amount_out_2).unwrap();
            pool_2.total_volume_a = pool_2.total_volume_a.checked_add(current_amount).unwrap();
            pool_2.total_fees_a = pool_2.total_fees_a.checked_add(fee_2).unwrap();
        } else {
            pool_2.reserve_b = pool_2.reserve_b.checked_add(current_amount).unwrap();
            pool_2.reserve_a = pool_2.reserve_a.checked_sub(amount_out_2).unwrap();
            pool_2.total_volume_b = pool_2.total_volume_b.checked_add(current_amount).unwrap();
            pool_2.total_fees_b = pool_2.total_fees_b.checked_add(fee_2).unwrap();
        }
        pool_2.update_twap(clock.unix_timestamp)?;

        current_amount = amount_out_2;
    }

    // ========== HOP 3 (if applicable) ==========
    if hops == 3 {
        let pool_3 = ctx.accounts.pool_3.as_mut()
            .ok_or(AmmError::InvalidSwapRoute)?;
        require!(!pool_3.is_paused, AmmError::PoolPaused);

        let is_a_to_b_3 = ctx.accounts.intermediate_token_2.as_ref().unwrap().mint == pool_3.token_a_mint;
        
        let (reserve_in_3, reserve_out_3) = if is_a_to_b_3 {
            (pool_3.reserve_a, pool_3.reserve_b)
        } else {
            (pool_3.reserve_b, pool_3.reserve_a)
        };

        let amount_out_3 = AmmMath::get_amount_out(
            current_amount,
            reserve_in_3,
            reserve_out_3,
            pool_3.fee_numerator,
            pool_3.fee_denominator,
        )?;

        // Validate with oracle
        OracleHelper::validate_swap_price(
            current_amount,
            amount_out_3,
            ctx.accounts.oracle_3_a.as_ref().ok_or(AmmError::InvalidOracle)?,
            ctx.accounts.oracle_3_b.as_ref().ok_or(AmmError::InvalidOracle)?,
            pool_3.oracle_max_age,
            pool_3.oracle_max_deviation_bps,
            is_a_to_b_3,
        )?;

        // Execute hop 3
        let transfer_3_in_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.intermediate_token_2.as_ref().unwrap().to_account_info(),
                to: ctx.accounts.pool_3_vault_in.as_ref().ok_or(AmmError::InvalidSwapRoute)?.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        token::transfer(transfer_3_in_ctx, current_amount)?;

        let seeds_3 = &[
            POOL_SEED,
            pool_3.token_a_mint.as_ref(),
            pool_3.token_b_mint.as_ref(),
            &[pool_3.bump],
        ];
        let signer_3 = &[&seeds_3[..]];

        let transfer_3_out_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.pool_3_vault_out.as_ref().ok_or(AmmError::InvalidSwapRoute)?.to_account_info(),
                to: ctx.accounts.user_token_out.to_account_info(),
                authority: pool_3.to_account_info(),
            },
            signer_3,
        );
        token::transfer(transfer_3_out_ctx, amount_out_3)?;

        // Update pool 3 state
        let fee_3 = current_amount
            .checked_mul(pool_3.fee_numerator)
            .unwrap()
            .checked_div(pool_3.fee_denominator)
            .unwrap();

        if is_a_to_b_3 {
            pool_3.reserve_a = pool_3.reserve_a.checked_add(current_amount).unwrap();
            pool_3.reserve_b = pool_3.reserve_b.checked_sub(amount_out_3).unwrap();
            pool_3.total_volume_a = pool_3.total_volume_a.checked_add(current_amount).unwrap();
            pool_3.total_fees_a = pool_3.total_fees_a.checked_add(fee_3).unwrap();
        } else {
            pool_3.reserve_b = pool_3.reserve_b.checked_add(current_amount).unwrap();
            pool_3.reserve_a = pool_3.reserve_a.checked_sub(amount_out_3).unwrap();
            pool_3.total_volume_b = pool_3.total_volume_b.checked_add(current_amount).unwrap();
            pool_3.total_fees_b = pool_3.total_fees_b.checked_add(fee_3).unwrap();
        }
        pool_3.update_twap(clock.unix_timestamp)?;

        current_amount = amount_out_3;
    }

    // Final slippage check
    require!(current_amount >= minimum_amount_out, AmmError::SlippageExceeded);

    msg!("Multi-hop swap completed successfully");
    msg!("Input amount: {}", amount_in);
    msg!("Output amount: {}", current_amount);
    msg!("Hops: {}", hops);

    Ok(())
}

