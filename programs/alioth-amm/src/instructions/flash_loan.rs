use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::constants::*;
use crate::errors::AmmError;
use crate::state::{Pool, FlashLoanRecord};

#[derive(Accounts)]
pub struct FlashLoan<'info> {
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
        init,
        payer = borrower,
        space = FlashLoanRecord::LEN,
        seeds = [
            FLASH_LOAN_SEED,
            pool.key().as_ref(),
            borrower.key().as_ref(),
        ],
        bump
    )]
    pub flash_loan_record: Account<'info, FlashLoanRecord>,

    #[account(mut)]
    pub borrower: Signer<'info>,

    #[account(
        mut,
        constraint = borrower_token_a.mint == pool.token_a_mint @ AmmError::TokenMintMismatch,
        constraint = borrower_token_a.owner == borrower.key() @ AmmError::InvalidAuthority,
    )]
    pub borrower_token_a: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = borrower_token_b.mint == pool.token_b_mint @ AmmError::TokenMintMismatch,
        constraint = borrower_token_b.owner == borrower.key() @ AmmError::InvalidAuthority,
    )]
    pub borrower_token_b: Account<'info, TokenAccount>,

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

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct FlashLoanRepay<'info> {
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
            FLASH_LOAN_SEED,
            pool.key().as_ref(),
            borrower.key().as_ref(),
        ],
        bump = flash_loan_record.bump,
        constraint = flash_loan_record.borrower == borrower.key() @ AmmError::InvalidAuthority,
        constraint = !flash_loan_record.is_repaid @ AmmError::FlashLoanAlreadyRepaid,
        close = borrower
    )]
    pub flash_loan_record: Account<'info, FlashLoanRecord>,

    #[account(mut)]
    pub borrower: Signer<'info>,

    #[account(
        mut,
        constraint = borrower_token_a.mint == pool.token_a_mint @ AmmError::TokenMintMismatch,
        constraint = borrower_token_a.owner == borrower.key() @ AmmError::InvalidAuthority,
    )]
    pub borrower_token_a: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = borrower_token_b.mint == pool.token_b_mint @ AmmError::TokenMintMismatch,
        constraint = borrower_token_b.owner == borrower.key() @ AmmError::InvalidAuthority,
    )]
    pub borrower_token_b: Account<'info, TokenAccount>,

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

    pub token_program: Program<'info, Token>,
}

pub fn handler(
    ctx: Context<FlashLoan>,
    amount_a: u64,
    amount_b: u64,
) -> Result<()> {
    let pool = &ctx.accounts.pool;
    let clock = Clock::get()?;

    // Check if pool is paused
    require!(!pool.is_paused, AmmError::PoolPaused);

    // Validate at least one amount is requested
    require!(amount_a > 0 || amount_b > 0, AmmError::ZeroAmount);

    // Check pool has enough liquidity
    require!(
        pool.reserve_a >= amount_a && pool.reserve_b >= amount_b,
        AmmError::InsufficientLiquidity
    );

    // Calculate flash loan fees (0.09% = 9 basis points)
    let fee_a = if amount_a > 0 {
        amount_a
            .checked_mul(FLASH_LOAN_FEE_NUMERATOR)
            .unwrap()
            .checked_div(FLASH_LOAN_FEE_DENOMINATOR)
            .unwrap()
            .max(1) // Minimum fee of 1
    } else {
        0
    };

    let fee_b = if amount_b > 0 {
        amount_b
            .checked_mul(FLASH_LOAN_FEE_NUMERATOR)
            .unwrap()
            .checked_div(FLASH_LOAN_FEE_DENOMINATOR)
            .unwrap()
            .max(1) // Minimum fee of 1
    } else {
        0
    };

    // Transfer tokens from pool to borrower
    let seeds = &[
        POOL_SEED,
        pool.token_a_mint.as_ref(),
        pool.token_b_mint.as_ref(),
        &[pool.bump],
    ];
    let signer = &[&seeds[..]];

    if amount_a > 0 {
        let transfer_a_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.token_a_vault.to_account_info(),
                to: ctx.accounts.borrower_token_a.to_account_info(),
                authority: pool.to_account_info(),
            },
            signer,
        );
        token::transfer(transfer_a_ctx, amount_a)?;
    }

    if amount_b > 0 {
        let transfer_b_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.token_b_vault.to_account_info(),
                to: ctx.accounts.borrower_token_b.to_account_info(),
                authority: pool.to_account_info(),
            },
            signer,
        );
        token::transfer(transfer_b_ctx, amount_b)?;
    }

    // Initialize flash loan record
    let flash_loan_record = &mut ctx.accounts.flash_loan_record;
    flash_loan_record.pool = pool.key();
    flash_loan_record.borrower = ctx.accounts.borrower.key();
    flash_loan_record.amount_a_borrowed = amount_a;
    flash_loan_record.amount_b_borrowed = amount_b;
    flash_loan_record.fee_a = fee_a;
    flash_loan_record.fee_b = fee_b;
    flash_loan_record.initiated_slot = clock.slot;
    flash_loan_record.is_repaid = false;
    flash_loan_record.bump = ctx.bumps.flash_loan_record;

    msg!("Flash loan initiated");
    msg!("Borrowed Token A: {}, Token B: {}", amount_a, amount_b);
    msg!("Fee Token A: {}, Token B: {}", fee_a, fee_b);
    msg!("IMPORTANT: Must repay {} Token A and {} Token B in the same transaction", 
        flash_loan_record.total_repay_a(), 
        flash_loan_record.total_repay_b()
    );

    Ok(())
}

pub fn repay_handler(ctx: Context<FlashLoanRepay>) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    let flash_loan_record = &ctx.accounts.flash_loan_record;
    let clock = Clock::get()?;

    // Verify repayment is in the same slot (transaction)
    require!(
        clock.slot == flash_loan_record.initiated_slot,
        AmmError::FlashLoanNotRepaid
    );

    // Calculate total amounts to repay (principal + fee)
    let total_repay_a = flash_loan_record.total_repay_a();
    let total_repay_b = flash_loan_record.total_repay_b();

    // Transfer repayment from borrower to pool
    if total_repay_a > 0 {
        let transfer_a_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.borrower_token_a.to_account_info(),
                to: ctx.accounts.token_a_vault.to_account_info(),
                authority: ctx.accounts.borrower.to_account_info(),
            },
        );
        token::transfer(transfer_a_ctx, total_repay_a)?;
    }

    if total_repay_b > 0 {
        let transfer_b_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.borrower_token_b.to_account_info(),
                to: ctx.accounts.token_b_vault.to_account_info(),
                authority: ctx.accounts.borrower.to_account_info(),
            },
        );
        token::transfer(transfer_b_ctx, total_repay_b)?;
    }

    // Update pool with fees collected
    pool.total_fees_a = pool.total_fees_a.checked_add(flash_loan_record.fee_a).unwrap();
    pool.total_fees_b = pool.total_fees_b.checked_add(flash_loan_record.fee_b).unwrap();

    // Update reserves (should be original + fees)
    pool.reserve_a = pool.reserve_a.checked_add(flash_loan_record.fee_a).unwrap();
    pool.reserve_b = pool.reserve_b.checked_add(flash_loan_record.fee_b).unwrap();

    msg!("Flash loan repaid successfully");
    msg!("Repaid Token A: {}, Token B: {}", total_repay_a, total_repay_b);
    msg!("Fees collected Token A: {}, Token B: {}", 
        flash_loan_record.fee_a, 
        flash_loan_record.fee_b
    );

    // The flash loan record account will be closed automatically via the close constraint

    Ok(())
}

