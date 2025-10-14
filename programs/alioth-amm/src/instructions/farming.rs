use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use crate::constants::*;
use crate::errors::AmmError;
use crate::state::{Pool, FarmingPool, UserStake};

// ========== Initialize Farm ==========

#[derive(Accounts)]
pub struct InitializeFarm<'info> {
    #[account(
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
        payer = authority,
        space = FarmingPool::LEN,
        seeds = [
            FARMING_POOL_SEED,
            pool.key().as_ref(),
        ],
        bump
    )]
    pub farming_pool: Account<'info, FarmingPool>,

    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        constraint = lp_mint.key() == pool.lp_mint @ AmmError::InvalidPoolConfig,
    )]
    pub lp_mint: Account<'info, Mint>,

    pub reward_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = authority,
        seeds = [
            REWARD_VAULT_SEED,
            farming_pool.key().as_ref(),
        ],
        bump,
        token::mint = reward_mint,
        token::authority = farming_pool,
    )]
    pub reward_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn initialize_farm_handler(
    ctx: Context<InitializeFarm>,
    reward_per_slot: u64,
    start_slot: u64,
    end_slot: u64,
) -> Result<()> {
    let farming_pool = &mut ctx.accounts.farming_pool;
    let clock = Clock::get()?;

    // Validate farming parameters
    require!(reward_per_slot > 0, AmmError::InvalidPoolConfig);
    require!(start_slot >= clock.slot, AmmError::InvalidPoolConfig);
    require!(end_slot > start_slot, AmmError::InvalidPoolConfig);

    let duration = end_slot.checked_sub(start_slot).unwrap();
    require!(
        duration >= MIN_FARMING_DURATION && duration <= MAX_FARMING_DURATION,
        AmmError::InvalidPoolConfig
    );

    // Initialize farming pool state
    farming_pool.authority = ctx.accounts.authority.key();
    farming_pool.pool = ctx.accounts.pool.key();
    farming_pool.lp_mint = ctx.accounts.lp_mint.key();
    farming_pool.reward_mint = ctx.accounts.reward_mint.key();
    farming_pool.reward_vault = ctx.accounts.reward_vault.key();
    farming_pool.total_staked = 0;
    farming_pool.reward_per_slot = reward_per_slot;
    farming_pool.start_slot = start_slot;
    farming_pool.end_slot = end_slot;
    farming_pool.last_update_slot = start_slot;
    farming_pool.accumulated_reward_per_share = 0;
    farming_pool.total_rewards_distributed = 0;
    farming_pool.is_active = true;
    farming_pool.bump = ctx.bumps.farming_pool;

    msg!("Farming pool initialized successfully");
    msg!("Reward per slot: {}", reward_per_slot);
    msg!("Duration: {} slots", duration);

    Ok(())
}

// ========== Stake LP Tokens ==========

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(
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
            FARMING_POOL_SEED,
            pool.key().as_ref(),
        ],
        bump = farming_pool.bump,
        constraint = farming_pool.is_active @ AmmError::FarmingNotActive,
    )]
    pub farming_pool: Account<'info, FarmingPool>,

    #[account(
        init_if_needed,
        payer = user,
        space = UserStake::LEN,
        seeds = [
            USER_STAKE_SEED,
            farming_pool.key().as_ref(),
            user.key().as_ref(),
        ],
        bump
    )]
    pub user_stake: Account<'info, UserStake>,

    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        constraint = user_lp_token.mint == farming_pool.lp_mint @ AmmError::TokenMintMismatch,
        constraint = user_lp_token.owner == user.key() @ AmmError::InvalidAuthority,
    )]
    pub user_lp_token: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = lp_token_vault.mint == farming_pool.lp_mint @ AmmError::TokenMintMismatch,
    )]
    pub lp_token_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn stake_handler(ctx: Context<Stake>, amount: u64) -> Result<()> {
    let farming_pool = &mut ctx.accounts.farming_pool;
    let user_stake = &mut ctx.accounts.user_stake;
    let clock = Clock::get()?;

    // Check farming period
    require!(
        clock.slot >= farming_pool.start_slot,
        AmmError::FarmingNotStarted
    );
    require!(
        clock.slot < farming_pool.end_slot,
        AmmError::FarmingEnded
    );

    // Validate amount
    require!(amount > 0, AmmError::ZeroAmount);

    // Update farming pool rewards
    farming_pool.update_rewards(clock.slot)?;

    // If user has existing stake, claim pending rewards first
    if user_stake.staked_amount > 0 {
        let pending_rewards = farming_pool.calculate_pending_rewards(
            user_stake.staked_amount,
            user_stake.reward_debt,
        )?;

        if pending_rewards > 0 {
            // Transfer rewards to user
            let seeds = &[
                FARMING_POOL_SEED,
                farming_pool.pool.as_ref(),
                &[farming_pool.bump],
            ];
            let signer = &[&seeds[..]];

            let transfer_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.lp_token_vault.to_account_info(),
                    to: ctx.accounts.user_lp_token.to_account_info(),
                    authority: farming_pool.to_account_info(),
                },
                signer,
            );
            token::transfer(transfer_ctx, pending_rewards)?;

            user_stake.total_rewards_claimed = user_stake.total_rewards_claimed
                .checked_add(pending_rewards)
                .unwrap();
        }
    } else {
        // Initialize user stake
        user_stake.owner = ctx.accounts.user.key();
        user_stake.farming_pool = farming_pool.key();
        user_stake.created_at = clock.unix_timestamp;
        user_stake.last_claim_slot = clock.slot;
        user_stake.total_rewards_claimed = 0;
        user_stake.bump = ctx.bumps.user_stake;
    }

    // Transfer LP tokens from user to vault
    let transfer_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.user_lp_token.to_account_info(),
            to: ctx.accounts.lp_token_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        },
    );
    token::transfer(transfer_ctx, amount)?;

    // Update user stake
    user_stake.staked_amount = user_stake.staked_amount.checked_add(amount).unwrap();
    user_stake.update_reward_debt(farming_pool.accumulated_reward_per_share);

    // Update farming pool
    farming_pool.total_staked = farming_pool.total_staked.checked_add(amount).unwrap();

    msg!("LP tokens staked successfully");
    msg!("Amount staked: {}", amount);
    msg!("Total user stake: {}", user_stake.staked_amount);

    Ok(())
}

// ========== Unstake LP Tokens ==========

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(
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
            FARMING_POOL_SEED,
            pool.key().as_ref(),
        ],
        bump = farming_pool.bump,
    )]
    pub farming_pool: Account<'info, FarmingPool>,

    #[account(
        mut,
        seeds = [
            USER_STAKE_SEED,
            farming_pool.key().as_ref(),
            user.key().as_ref(),
        ],
        bump = user_stake.bump,
        constraint = user_stake.owner == user.key() @ AmmError::InvalidAuthority,
    )]
    pub user_stake: Account<'info, UserStake>,

    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        constraint = user_lp_token.mint == farming_pool.lp_mint @ AmmError::TokenMintMismatch,
        constraint = user_lp_token.owner == user.key() @ AmmError::InvalidAuthority,
    )]
    pub user_lp_token: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = lp_token_vault.mint == farming_pool.lp_mint @ AmmError::TokenMintMismatch,
    )]
    pub lp_token_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = reward_vault.key() == farming_pool.reward_vault @ AmmError::InvalidPoolConfig,
    )]
    pub reward_vault: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = farming_pool.reward_mint,
        associated_token::authority = user,
    )]
    pub user_reward_token: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, anchor_spl::associated_token::AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn unstake_handler(ctx: Context<Unstake>, amount: u64) -> Result<()> {
    let farming_pool = &mut ctx.accounts.farming_pool;
    let user_stake = &mut ctx.accounts.user_stake;
    let clock = Clock::get()?;

    // Validate amount
    require!(amount > 0, AmmError::ZeroAmount);
    require!(
        user_stake.staked_amount >= amount,
        AmmError::InsufficientStake
    );

    // Update farming pool rewards
    farming_pool.update_rewards(clock.slot)?;

    // Calculate and claim pending rewards
    let pending_rewards = farming_pool.calculate_pending_rewards(
        user_stake.staked_amount,
        user_stake.reward_debt,
    )?;

    if pending_rewards > 0 {
        // Transfer rewards to user
        let seeds = &[
            FARMING_POOL_SEED,
            farming_pool.pool.as_ref(),
            &[farming_pool.bump],
        ];
        let signer = &[&seeds[..]];

        let transfer_reward_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.reward_vault.to_account_info(),
                to: ctx.accounts.user_reward_token.to_account_info(),
                authority: farming_pool.to_account_info(),
            },
            signer,
        );
        token::transfer(transfer_reward_ctx, pending_rewards)?;

        user_stake.total_rewards_claimed = user_stake.total_rewards_claimed
            .checked_add(pending_rewards)
            .unwrap();
        user_stake.last_claim_slot = clock.slot;
    }

    // Transfer LP tokens back to user
    let seeds = &[
        FARMING_POOL_SEED,
        farming_pool.pool.as_ref(),
        &[farming_pool.bump],
    ];
    let signer = &[&seeds[..]];

    let transfer_lp_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.lp_token_vault.to_account_info(),
            to: ctx.accounts.user_lp_token.to_account_info(),
            authority: farming_pool.to_account_info(),
        },
        signer,
    );
    token::transfer(transfer_lp_ctx, amount)?;

    // Update user stake
    user_stake.staked_amount = user_stake.staked_amount.checked_sub(amount).unwrap();
    user_stake.update_reward_debt(farming_pool.accumulated_reward_per_share);

    // Update farming pool
    farming_pool.total_staked = farming_pool.total_staked.checked_sub(amount).unwrap();

    msg!("LP tokens unstaked successfully");
    msg!("Amount unstaked: {}", amount);
    msg!("Rewards claimed: {}", pending_rewards);

    Ok(())
}

// ========== Claim Rewards ==========

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(
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
            FARMING_POOL_SEED,
            pool.key().as_ref(),
        ],
        bump = farming_pool.bump,
    )]
    pub farming_pool: Account<'info, FarmingPool>,

    #[account(
        mut,
        seeds = [
            USER_STAKE_SEED,
            farming_pool.key().as_ref(),
            user.key().as_ref(),
        ],
        bump = user_stake.bump,
        constraint = user_stake.owner == user.key() @ AmmError::InvalidAuthority,
    )]
    pub user_stake: Account<'info, UserStake>,

    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        constraint = reward_vault.key() == farming_pool.reward_vault @ AmmError::InvalidPoolConfig,
    )]
    pub reward_vault: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = farming_pool.reward_mint,
        associated_token::authority = user,
    )]
    pub user_reward_token: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, anchor_spl::associated_token::AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn claim_rewards_handler(ctx: Context<ClaimRewards>) -> Result<()> {
    let farming_pool = &mut ctx.accounts.farming_pool;
    let user_stake = &mut ctx.accounts.user_stake;
    let clock = Clock::get()?;

    // Update farming pool rewards
    farming_pool.update_rewards(clock.slot)?;

    // Calculate pending rewards
    let pending_rewards = farming_pool.calculate_pending_rewards(
        user_stake.staked_amount,
        user_stake.reward_debt,
    )?;

    require!(pending_rewards > 0, AmmError::NoRewards);

    // Transfer rewards to user
    let seeds = &[
        FARMING_POOL_SEED,
        farming_pool.pool.as_ref(),
        &[farming_pool.bump],
    ];
    let signer = &[&seeds[..]];

    let transfer_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.reward_vault.to_account_info(),
            to: ctx.accounts.user_reward_token.to_account_info(),
            authority: farming_pool.to_account_info(),
        },
        signer,
    );
    token::transfer(transfer_ctx, pending_rewards)?;

    // Update user stake
    user_stake.total_rewards_claimed = user_stake.total_rewards_claimed
        .checked_add(pending_rewards)
        .unwrap();
    user_stake.last_claim_slot = clock.slot;
    user_stake.update_reward_debt(farming_pool.accumulated_reward_per_share);

    msg!("Rewards claimed successfully");
    msg!("Amount: {}", pending_rewards);

    Ok(())
}

