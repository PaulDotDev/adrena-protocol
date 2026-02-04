use {
    crate::{
        adapters::{CreateMetadataAccountV3Adapter, MplTokenMetadataAdapter},
        error::AdrenaError,
        state::cortex::{Cortex, CortexInitializationStep},
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token},
};

#[derive(Accounts)]
pub struct InitTwoLmTokenMetadata<'info> {
    /// #1
    pub admin: Signer<'info>,

    /// #2
    #[account(mut)]
    pub payer: Signer<'info>,

    /// #3
    /// CHECK: Empty PDA, will be set as authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = cortex.load()?.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    /// #5
    #[account(
        mut,
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        has_one = admin,
        constraint = cortex.load()?.get_initialized() == CortexInitializationStep::Step1 @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #6
    #[account(
        mut,
        seeds = [b"lm_token_mint"],
        bump = cortex.load()?.lm_token_bump
    )]
    pub lm_token_mint: Box<Account<'info, Mint>>,

    /// #7
    /// CHECK: checked by mpl token metadata program
    #[account(mut)]
    pub lm_token_mint_metadata: UncheckedAccount<'info>,

    /// #8
    system_program: Program<'info, System>,

    /// #9
    token_program: Program<'info, Token>,

    /// #10
    mpl_token_metadata_program: Program<'info, MplTokenMetadataAdapter>,

    /// #11
    rent: Sysvar<'info, Rent>,
}

pub fn init_two_lm_token_metadata<'info>(
    ctx: Context<'_, '_, '_, 'info, InitTwoLmTokenMetadata<'info>>,
) -> Result<()> {
    let mut cortex = ctx.accounts.cortex.load_mut()?;

    cortex.inception_time = cortex.get_time()?;
    cortex.initialized = CortexInitializationStep::Step2.into();

    // Create token metadata for LM Token
    {
        let authority_seeds: &[&[&[u8]]] =
            &[&[b"transfer_authority", &[cortex.transfer_authority_bump]]];

        let cpi_accounts = CreateMetadataAccountV3Adapter {
            metadata: ctx.accounts.lm_token_mint_metadata.to_account_info(),
            mint: ctx.accounts.lm_token_mint.to_account_info(),
            mint_authority: ctx.accounts.transfer_authority.to_account_info(),
            payer: ctx.accounts.payer.to_account_info(),
            update_authority: ctx.accounts.transfer_authority.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        };

        let cpi_program = ctx.accounts.mpl_token_metadata_program.to_account_info();

        crate::adapters::create_metadata_account_v3(
            CpiContext::new(cpi_program, cpi_accounts).with_signer(authority_seeds),
            "Adrena Governance Token".to_string(),
            "ADX".to_string(),
            "https://arweave.net/8HohRjBW7P5uMUR0Cz9eHxLfOO2PcnLzciyvhhgNkck".to_string(), // ADX red logo. PNG
            true,
            false,
        )?;
    }

    Ok(())
}
