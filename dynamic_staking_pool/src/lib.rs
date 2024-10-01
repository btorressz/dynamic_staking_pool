use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, TokenAccount, Token};

declare_id!("Cguyo4nag1xDbD9a8uroZF6AdVUhiNnrJd1M3fnNh7P4");

#[program]
pub mod dynamic_staking_pool {
    use super::*;

    /// Initializes the staking pool with a specified reward rate. This function creates a 
    /// `PoolAccount` to track the total staked tokens and reward rate for future staking activities.
    pub fn initialize(ctx: Context<Initialize>, reward_rate: u64) -> Result<()> {
        let pool_account = &mut ctx.accounts.pool_account;
        pool_account.reward_rate = reward_rate;
        pool_account.total_staked = 0;
        Ok(())
    }

    /// Allows a user to stake tokens into the pool. The amount staked is added to the user's 
    /// `UserStake` account and the total staked value in the `PoolAccount`. Tokens are transferred 
    /// from the user's token account to the pool's token account.
    pub fn stake(ctx: Context<Stake>, amount: u64) -> Result<()> {
        require!(amount > 0, StakingError::InvalidAmount);
        let clock = Clock::get()?;
        let user_stake = &mut ctx.accounts.user_stake;

        // If this is the first stake, initialize the user's stake account
        if user_stake.amount_staked == 0 {
            user_stake.start_time = clock.unix_timestamp;
            user_stake.pool_account = ctx.accounts.pool_account.key();
        }

        // Update the user's staked amount and the pool's total staked amount
        user_stake.amount_staked += amount;
        ctx.accounts.pool_account.total_staked += amount;

        // Transfer tokens from the user's token account to the pool's token account
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.user_token_account.to_account_info(),
                    to: ctx.accounts.pool_token_account.to_account_info(),
                    authority: ctx.accounts.staker.to_account_info(),
                },
            ),
            amount,
        )?;

        // Emit an event to log the staking action
        emit!(StakeEvent {
            user: ctx.accounts.staker.key(),
            amount,
            time: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Allows a user to claim rewards based on the amount staked and the duration of staking.
    /// Rewards are calculated using the `calculate_reward` function and minted to the user's 
    /// token account.
    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        let clock = Clock::get()?;
        let user_stake = &mut ctx.accounts.user_stake;
        let staking_duration = clock.unix_timestamp - user_stake.start_time;

        // Calculate the reward amount based on staking performance
        let reward_amount = calculate_reward(
            user_stake.amount_staked,
            staking_duration,
            ctx.accounts.pool_account.reward_rate,
            ctx.accounts.pool_account.total_staked,
        );

        // Update the last claim time for the user
        user_stake.last_claim_time = clock.unix_timestamp;

        // Mint the reward to the user's token account
        token::mint_to(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::MintTo {
                    mint: ctx.accounts.reward_mint.to_account_info(),
                    to: ctx.accounts.user_token_account.to_account_info(),
                    authority: ctx.accounts.pool_account.to_account_info(),
                },
            ),
            reward_amount,
        )?;

        Ok(())
    }

    /// Allows a user to unstake tokens from the pool. The amount is deducted from the user's staked 
    /// balance and the total pool balance. Tokens are transferred back to the user's token account.
    pub fn unstake(ctx: Context<Unstake>, amount: u64) -> Result<()> {
        require!(amount > 0, StakingError::InvalidAmount);
        let user_stake = &mut ctx.accounts.user_stake;
        require!(user_stake.amount_staked >= amount, StakingError::InsufficientBalance);

        // Update the user's staked amount and the pool's total staked amount
        user_stake.amount_staked -= amount;
        ctx.accounts.pool_account.total_staked -= amount;

        // Transfer the unstaked tokens back to the user's token account
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.pool_token_account.to_account_info(),
                    to: ctx.accounts.user_token_account.to_account_info(),
                    authority: ctx.accounts.staker.to_account_info(),
                },
            ),
            amount,
        )?;

        Ok(())
    }

    /// Allows the pool initializer to adjust the reward rate for the staking pool.
    pub fn adjust_reward_rate(ctx: Context<AdjustRewardRate>, new_rate: u64) -> Result<()> {
        let pool_account = &mut ctx.accounts.pool_account;
        pool_account.reward_rate = new_rate;
        Ok(())
    }
}

/// Calculates the reward for a user based on the amount of tokens they have staked,
/// how long they have been staking, and the total amount of tokens staked in the pool.
fn calculate_reward(
    amount_staked: u64,
    staking_duration: i64,
    reward_rate: u64,
    total_staked: u64,
) -> u64 {
    let base_reward = amount_staked * reward_rate * staking_duration as u64;
    let proportional_reward = base_reward / total_staked;
    proportional_reward
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = initializer, space = 8 + 40)]
    pub pool_account: Account<'info, PoolAccount>,
    #[account(mut)]
    pub initializer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(
        init_if_needed,
        seeds = [b"stake", staker.key().as_ref()],
        bump,
        payer = staker,
        space = 8 + 64
    )]
    pub user_stake: Account<'info, UserStake>,
    #[account(mut)]
    pub pool_account: Account<'info, PoolAccount>,
    #[account(mut)]
    pub staker: Signer<'info>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub pool_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(mut)]
    pub user_stake: Account<'info, UserStake>,
    #[account(mut)]
    pub pool_account: Account<'info, PoolAccount>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub reward_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut, has_one = staker)]  // Ensures the staker is the owner of the UserStake account
    pub user_stake: Account<'info, UserStake>,
    #[account(mut)]
    pub pool_account: Account<'info, PoolAccount>,
    #[account(mut)]
    pub staker: Signer<'info>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub pool_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct AdjustRewardRate<'info> {
    #[account(mut, has_one = initializer)]  // Ensures the initializer matches the one who created the PoolAccount
    pub pool_account: Account<'info, PoolAccount>,
    pub initializer: Signer<'info>,
}

#[account]
pub struct PoolAccount {
    pub reward_rate: u64,        // Reward rate for the pool
    pub total_staked: u64,       // Total amount of tokens staked in the pool
    pub initializer: Pubkey,     // The initializer's public key (the one who initialized the pool)
}

#[account]
pub struct UserStake {
    pub amount_staked: u64,      // User's staked amount
    pub start_time: i64,         // Timestamp when the user started staking
    pub last_claim_time: i64,    // Timestamp of the last reward claim
    pub pool_account: Pubkey,    // Reference to the pool account
    pub staker: Pubkey,          // The user's wallet public key
}

#[event]
pub struct StakeEvent {
    pub user: Pubkey,
    pub amount: u64,
    pub time: i64,
}

#[error_code]
pub enum StakingError {
    #[msg("Amount must be greater than zero")]
    InvalidAmount,

    #[msg("Insufficient balance for staking")]
    InsufficientBalance,
    
    #[msg("Rewards already claimed")]
    AlreadyClaimed,
}
