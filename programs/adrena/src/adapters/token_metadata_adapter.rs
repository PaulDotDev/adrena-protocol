use {
    anchor_lang::prelude::*,
    mpl_token_metadata::{
        instructions::{CreateMetadataAccountV3, CreateMetadataAccountV3InstructionArgs},
        types::DataV2,
    },
};

#[derive(Clone, Copy)]
pub struct MplTokenMetadataAdapter;

pub mod mpl_token_metadata_program_adapter {
    solana_program::declare_id!(mpl_token_metadata::programs::MPL_TOKEN_METADATA_ID);
}

impl anchor_lang::Id for MplTokenMetadataAdapter {
    fn id() -> Pubkey {
        mpl_token_metadata_program_adapter::ID
    }
}

fn assert_mpl_token_metadata_program_account(mpl_token_metadata_program: &Pubkey) -> Result<()> {
    require_eq!(
        mpl_token_metadata_program,
        &mpl_token_metadata_program_adapter::ID,
        ErrorCode::InvalidProgramId,
    );

    Ok(())
}

pub fn create_metadata_account_v3<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, CreateMetadataAccountV3Adapter<'info>>,
    name: String,
    symbol: String,
    uri: String,
    is_mutable: bool,
    authority_mutable: bool,
) -> Result<()> {
    assert_mpl_token_metadata_program_account(ctx.program.key)?;

    solana_program::program::invoke_signed(
        &CreateMetadataAccountV3 {
            metadata: ctx.accounts.metadata.key(),
            mint: ctx.accounts.mint.key(),
            mint_authority: ctx.accounts.mint_authority.key(),
            payer: ctx.accounts.payer.key(),
            update_authority: (ctx.accounts.update_authority.key(), authority_mutable),
            system_program: ctx.accounts.system_program.key(),
            rent: Some(ctx.accounts.rent.key()),
        }
        .instruction(CreateMetadataAccountV3InstructionArgs {
            data: DataV2 {
                name,
                symbol,
                uri,
                seller_fee_basis_points: 0,
                creators: None,
                collection: None,
                uses: None,
            },
            is_mutable,
            collection_details: None,
        }),
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct CreateMetadataAccountV3Adapter<'info> {
    /// Metadata key (pda of ['metadata', program id, mint id])
    /// CHECK: Checked by metaplex token metadata program
    pub metadata: AccountInfo<'info>,
    /// Mint of token asset
    /// CHECK: Checked by caller
    pub mint: AccountInfo<'info>,
    /// CHECK: Checked by caller
    pub mint_authority: AccountInfo<'info>,
    /// CHECK: Checked by caller
    pub payer: AccountInfo<'info>,
    /// CHECK: Checked by caller
    pub update_authority: AccountInfo<'info>,
    /// CHECK: Checked by caller
    pub system_program: AccountInfo<'info>,
    /// CHECK: Checked by caller
    pub rent: AccountInfo<'info>,
}
