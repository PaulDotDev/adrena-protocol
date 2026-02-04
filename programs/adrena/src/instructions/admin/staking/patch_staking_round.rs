use {
    crate::{
        error::AdrenaError,
        instructions::{BucketName, MintLmTokensFromBucketParams},
        program::Adrena,
        state::{cortex::Cortex, staking::Staking},
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token, TokenAccount},
};

#[derive(Accounts)]
pub struct PatchStakingRound<'info> {
    /// #1
    pub admin: Signer<'info>,

    /// #2
    #[account(mut)]
    pub payer: Signer<'info>,

    /// #3
    #[account(
        mut,
        constraint = funding_account.mint == fee_redistribution_mint.key(),
        constraint = funding_account.owner == admin.key() @AdrenaError::InvalidAccountData,
    )]
    pub funding_account: Box<Account<'info, TokenAccount>>,

    /// #4
    #[account(
        mut,
        token::mint = fee_redistribution_mint,
        seeds = [b"staking_reward_token_vault", staking.key().as_ref()],
        bump = staking.load()?.reward_token_vault_bump
    )]
    pub staking_reward_token_vault: Box<Account<'info, TokenAccount>>,

    /// #5
    #[account(
        mut,
        token::mint = lm_token_mint,
        seeds = [b"staking_lm_reward_token_vault", staking.key().as_ref()],
        bump = staking.load()?.lm_reward_token_vault_bump
    )]
    pub staking_lm_reward_token_vault: Box<Account<'info, TokenAccount>>,

    /// #6
    /// CHECK: Empty PDA, authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = cortex.load()?.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    /// #7
    #[account(
        mut,
        seeds = [b"staking", staking.load()?.staked_token_mint.as_ref()],
        bump = staking.load()?.bump,
        constraint = staking.load()?.is_initialized() @AdrenaError::InvalidStakingState
    )]
    pub staking: AccountLoader<'info, Staking>,

    /// #8
    #[account(
        mut,
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        has_one = fee_redistribution_mint,
        has_one = admin,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #9
    #[account(
        mut,
        seeds = [b"lm_token_mint"],
        bump = cortex.load()?.lm_token_bump
    )]
    pub lm_token_mint: Box<Account<'info, Mint>>,

    /// #10
    #[account()]
    pub fee_redistribution_mint: Box<Account<'info, Mint>>,

    /// #11
    pub adrena_program: Program<'info, Adrena>,

    /// #12
    pub system_program: Program<'info, System>,

    /// #13
    pub token_program: Program<'info, Token>,
}

pub fn patch_staking_round(ctx: Context<PatchStakingRound>) -> Result<()> {
    let mut staking = ctx.accounts.staking.load_mut()?;

    let cortex = ctx.accounts.cortex.load()?;
    let transfer_authority_bump = cortex.transfer_authority_bump;

    drop(cortex);

    let lm_token_amount = 2_000_000_000_000; // ADX: 6 decimals
    let reward_token_amount = 5_000_000_000; // USDC: 6 decimals

    // Mint LM tokens
    {
        let cpi_accounts = crate::cpi::accounts::MintLmTokensFromBucket {
            admin: ctx.accounts.transfer_authority.to_account_info(),
            receiving_account: ctx.accounts.staking_lm_reward_token_vault.to_account_info(),
            transfer_authority: ctx.accounts.transfer_authority.to_account_info(),
            cortex: ctx.accounts.cortex.to_account_info(),
            lm_token_mint: ctx.accounts.lm_token_mint.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
        };

        let cpi_program = ctx.accounts.adrena_program.to_account_info();
        crate::cpi::mint_lm_tokens_from_bucket(
            CpiContext::new_with_signer(
                cpi_program,
                cpi_accounts,
                &[&[b"transfer_authority", &[transfer_authority_bump]]],
            ),
            MintLmTokensFromBucketParams {
                bucket_name: BucketName::Ecosystem.into(),
                amount: lm_token_amount,
                reason: String::from("UserStaking rewards"),
            },
        )?;

        {
            ctx.accounts.staking_lm_reward_token_vault.reload()?;
            ctx.accounts.lm_token_mint.reload()?;
        }
    }

    // Transfer tokens
    {
        let cortex = ctx.accounts.cortex.load()?;

        cortex.transfer_tokens_from_user(
            ctx.accounts.funding_account.to_account_info(),
            ctx.accounts.staking_reward_token_vault.to_account_info(),
            ctx.accounts.admin.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            reward_token_amount,
        )?;
    }

    // NOTE: every token inserted here is not vowed to be distributed as rewards. They are just placeholders for the extra rewards that were distributed due to the bug.
    // Will re-equilibrate later once the bug is fixed
    {
        // Offset the number of tokens eligible for rewards in the stats, so we don't have overflow, will re-equilibrate later once the bug is fixed
        staking.resolved_lm_staked_token_amount += 50_000_000_000_000;

        // Add ADX tokens in the vault and mark as reserved so the extra distributed ADX rewards get taken from there
        staking.resolved_lm_reward_token_amount += lm_token_amount;

        // Offset the number of tokens eligible for rewards in the stats, so we don't have overflow, will re-equilibrate later once the bug is fixed
        staking.resolved_staked_token_amount += 50_000_000_000_000;

        // Add USDC tokens in the vault and mark as reserved so the extra distributed ADX rewards get taken from there
        staking.resolved_reward_token_amount += reward_token_amount;
    }

    Ok(())
}
