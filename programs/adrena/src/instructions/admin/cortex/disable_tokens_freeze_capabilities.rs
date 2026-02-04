use {
    crate::{
        error::AdrenaError,
        state::{cortex::Cortex, pool::Pool},
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token},
};

#[derive(Accounts)]
pub struct DisableTokensFreezeCapabilities<'info> {
    /// #1
    pub admin: Signer<'info>,

    /// #2
    /// CHECK: Empty PDA, authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = cortex.load()?.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    /// #3
    #[account(
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        has_one = admin,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #4
    #[account(
        mut,
        seeds = [b"lm_token_mint"],
        bump = cortex.load()?.lm_token_bump
    )]
    pub lm_token_mint: Box<Account<'info, Mint>>,

    /// #5
    #[account(
        seeds = [b"pool",
            pool.load()?.name.to_bytes()],
        bump = pool.load()?.bump,
        constraint = pool.load()?.is_initialized() @AdrenaError::InvalidPoolState
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// #6
    #[account(
        mut,
        seeds = [b"lp_token_mint",
            pool.key().as_ref()],
        bump = pool.load()?.lp_token_bump
    )]
    pub lp_token_mint: Box<Account<'info, Mint>>,

    /// #7
    pub token_program: Program<'info, Token>,
}

pub fn disable_tokens_freeze_capabilities(
    ctx: Context<DisableTokensFreezeCapabilities>,
) -> Result<()> {
    let cortex = ctx.accounts.cortex.load()?;

    let authority_seeds: &[&[&[u8]]] =
        &[&[b"transfer_authority", &[cortex.transfer_authority_bump]]];

    // LM token mint
    {
        anchor_spl::token::set_authority(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::SetAuthority {
                    account_or_mint: ctx.accounts.lm_token_mint.to_account_info(),
                    current_authority: ctx.accounts.transfer_authority.to_account_info(),
                },
                authority_seeds,
            ),
            anchor_spl::token::spl_token::instruction::AuthorityType::FreezeAccount,
            None,
        )?;
    }

    // LP token mint
    {
        anchor_spl::token::set_authority(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::SetAuthority {
                    account_or_mint: ctx.accounts.lp_token_mint.to_account_info(),
                    current_authority: ctx.accounts.transfer_authority.to_account_info(),
                },
                authority_seeds,
            ),
            anchor_spl::token::spl_token::instruction::AuthorityType::FreezeAccount,
            None,
        )?;
    }

    Ok(())
}
