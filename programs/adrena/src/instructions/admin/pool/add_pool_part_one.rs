use {
    crate::{
        adapters::{CreateMetadataAccountV3Adapter, MplTokenMetadataAdapter},
        error::AdrenaError,
        program::Adrena,
        state::{
            cortex::Cortex,
            pool::{Pool, PoolLiquidityState},
        },
        utils::limited_string::LimitedString,
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token},
    solana_program::program_error::ProgramError,
};

#[derive(Accounts)]
#[instruction(params: AddPoolPartOneParams)]
pub struct AddPoolPartOne<'info> {
    /// #1
    pub admin: Signer<'info>,

    /// #2
    #[account(mut)]
    pub payer: Signer<'info>,

    /// #3
    /// CHECK: Empty PDA, authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = cortex.load()?.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    /// #4
    #[account(
        mut,
        seeds = [b"cortex"],
        bump = cortex.load()?.bump,
        has_one = admin,
        constraint = cortex.load()?.is_initialized() @AdrenaError::InvalidCortexState
    )]
    pub cortex: AccountLoader<'info, Cortex>,

    /// #5
    #[account(
        init,
        payer = payer,
        space = Pool::LEN,
        seeds = [b"pool",
                 params.name.as_bytes()],
        bump
    )]
    pub pool: AccountLoader<'info, Pool>,

    /// #6
    #[account(
        init,
        payer = payer,
        mint::authority = transfer_authority,
        mint::freeze_authority = transfer_authority,
        mint::decimals = Cortex::LP_DECIMALS,
        seeds = [b"lp_token_mint",
            pool.key().as_ref()],
        bump
    )]
    pub lp_token_mint: Box<Account<'info, Mint>>,

    /// #7
    /// CHECK: checked by mpl token metadata program
    #[account(mut)]
    pub lp_token_mint_metadata: UncheckedAccount<'info>,

    /// #8
    system_program: Program<'info, System>,

    /// #9
    token_program: Program<'info, Token>,

    /// #10
    mpl_token_metadata_program: Program<'info, MplTokenMetadataAdapter>,

    /// #11
    adrena_program: Program<'info, Adrena>,

    /// #12
    rent: Sysvar<'info, Rent>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct AddPoolPartOneParams {
    pub name: String,
    pub aum_soft_cap_usd: u64,
    // Metadata
    pub lp_token_name: String,
    pub lp_token_symbol: String,
    pub lp_token_uri: String,
}

pub fn add_pool_part_one<'info>(
    ctx: Context<'_, '_, '_, 'info, AddPoolPartOne<'info>>,
    params: &AddPoolPartOneParams,
) -> Result<u8> {
    let mut pool = ctx.accounts.pool.load_init()?;

    // Preliminary checks
    {
        if params.name.is_empty() || params.name.len() > 64 {
            return Err(ProgramError::InvalidArgument.into());
        }

        if pool.inception_time != 0 {
            return Err(ProgramError::AccountAlreadyInitialized.into());
        }
    }

    msg!("Record pool: {}", params.name);

    {
        pool.inception_time = ctx.accounts.cortex.load()?.get_time()?;

        pool.name = LimitedString::new(&params.name);

        pool.bump = ctx.bumps.pool;
        pool.lp_token_bump = ctx.bumps.lp_token_mint;

        pool.initialized = false as u8;

        pool.nb_stable_custody = 0;

        pool.aum_soft_cap_usd = params.aum_soft_cap_usd;

        pool.liquidity_state = PoolLiquidityState::GenesisLiquidity.into();

        // Needs to be manually turned on after genesis lock campaign is over
        pool.allow_swap = false as u8;
        pool.allow_trade = false as u8;

        pool.registered_custody_count = 0;

        pool.whitelisted_swapper = Pubkey::default();
    }

    // Create token metadata for LP Token
    {
        let authority_seeds: &[&[&[u8]]] = &[&[
            b"transfer_authority",
            &[ctx.accounts.cortex.load()?.transfer_authority_bump],
        ]];
        let cpi_accounts = CreateMetadataAccountV3Adapter {
            metadata: ctx.accounts.lp_token_mint_metadata.to_account_info(),
            mint: ctx.accounts.lp_token_mint.to_account_info(),
            mint_authority: ctx.accounts.transfer_authority.to_account_info(),
            payer: ctx.accounts.payer.to_account_info(),
            update_authority: ctx.accounts.transfer_authority.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        };

        let cpi_program = ctx.accounts.mpl_token_metadata_program.to_account_info();

        crate::adapters::create_metadata_account_v3(
            CpiContext::new(cpi_program, cpi_accounts).with_signer(authority_seeds),
            params.lp_token_name.clone(),
            params.lp_token_symbol.clone(),
            params.lp_token_uri.clone(),
            true,
            false,
        )?;
    }

    require!(pool.validate(), AdrenaError::InvalidPoolConfig);

    let mut cortex = ctx.accounts.cortex.load_mut()?;

    // Add the pool to the cortex
    cortex.add_pool(&ctx.accounts.pool.key())?;

    Ok(0)
}
