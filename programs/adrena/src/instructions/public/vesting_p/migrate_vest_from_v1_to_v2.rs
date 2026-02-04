use {
    crate::{
        error::AdrenaError,
        state::{
            cortex::Cortex,
            vest::{legacy::VestV1, Vest, VestVersion},
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::Token,
};

#[derive(Accounts)]
pub struct MigrateVestFromV1ToV2<'info> {
    /// #1
    /// The caller of this instruction
    /// CHECK: Anyone can call this instruction
    pub caller: Signer<'info>,

    /// #2
    /// Wallet related to the vest
    /// CHECK: This is the owner of the vest
    pub owner: AccountInfo<'info>,

    /// #3
    /// Account paying for the reallocation
    #[account(mut)]
    pub payer: Signer<'info>,

    /// #4
    /// CHECK: Manual deserialization is used
    #[account(
        mut,
        seeds = [b"vest", owner.key().as_ref()],
        bump,
    )]
    pub vest: AccountInfo<'info>,

    /// #5
    pub system_program: Program<'info, System>,

    /// #6
    pub token_program: Program<'info, Token>,

    /// #7
    pub rent: Sysvar<'info, Rent>,
}

pub fn migrate_vest_from_v1_to_v2<'info>(
    ctx: Context<'_, '_, '_, 'info, MigrateVestFromV1ToV2<'info>>,
) -> Result<()> {
    let vest_account_info = &ctx.accounts.vest;

    // Manually deserialize the old VestV1
    let vest_v1_data: VestV1 = {
        let data = vest_account_info.try_borrow_data()?;
        let slice = &data[8..]; // Skip discriminator

        *bytemuck::try_from_bytes(slice)
            .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize)?
    };

    // Ensure the old version is correct
    require!(
        vest_v1_data.version == VestVersion::V1 as u8,
        AdrenaError::InvalidVestVersion
    );

    Cortex::realloc(
        ctx.accounts.payer.to_account_info(),
        vest_account_info.clone(),
        ctx.accounts.system_program.to_account_info(),
        Vest::LEN,
        false, // Extra size already allocated to 0
    )?;

    // Initialize the new Vest structure
    {
        let mut data = vest_account_info.try_borrow_mut_data()?;
        let new_vest: &mut Vest = bytemuck::from_bytes_mut(&mut data[8..]); // Skip discriminator

        // Change version
        new_vest.version = VestVersion::V2 as u8;

        // Initialize new fields
        new_vest.delegate = Pubkey::default();
        new_vest.has_delegate = 0;
    }

    Ok(())
}
