use {
    super::{get_distribute_fees_ix, get_update_pool_ix},
    crate::utils::{self, pda},
    adrena::{
        instructions::{DistributeFeesParams, OpenPositionWithSwapParams},
        state::position::Side,
    },
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    tokio::sync::RwLock,
};

#[allow(clippy::too_many_arguments)]
pub async fn open_or_increase_position_with_swap_short(
    program_test_ctx: &RwLock<ProgramTestContext>,
    owner: &Keypair,
    payer: &Keypair,
    pool_pda: &Pubkey,
    // Token provided from the user as collateral for the position - A stable here for shorts
    user_collateral_token_mint: &Pubkey,
    // Token targeted for the position
    principal_token_mint: &Pubkey,
    params: OpenPositionWithSwapParams,
) -> std::result::Result<(Pubkey, u8), BanksClientError> {
    // ==== WHEN ==============================================================

    // Prepare PDA and addresses
    let oracle_pda = pda::get_oracle_pda().0;
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let cortex_pda = pda::get_cortex_pda().0;

    //
    // Infos about receiving custody (the token received by the ix from the user initially)
    //
    // i.e User short ETH with BTC. receiving custody is ETH.
    //
    let receiving_custody_pda = pda::get_custody_pda(pool_pda, user_collateral_token_mint).0;
    let receiving_custody_token_account_pda =
        pda::get_custody_token_account_pda(pool_pda, user_collateral_token_mint).0;

    //
    // Infos about principal custody (the token targeted by the position)
    //
    // i.e User short ETH with BTC. principal custody is BTC.
    //
    let principal_custody_pda = pda::get_custody_pda(pool_pda, principal_token_mint).0;
    let principal_custody_token_account_pda =
        pda::get_custody_token_account_pda(pool_pda, principal_token_mint).0;

    //
    // Infos about collateral custody (the token to swap receiving token for to provide as collateral for the position)
    //
    // i.e User short ETH with BTC. receiving custody is USDC (can only short with stable)
    //
    let (collateral_custody_pda, collateral_custody_token_account_pda, collateral_account_address) = {
        // Must be provided when short
        let stable_token_mint = user_collateral_token_mint;

        let collateral_custody_pda = pda::get_custody_pda(pool_pda, stable_token_mint).0;
        let collateral_custody_token_account_pda =
            pda::get_custody_token_account_pda(pool_pda, stable_token_mint).0;

        let collateral_account_address =
            utils::find_associated_token_account(&owner.pubkey(), stable_token_mint).0;

        (
            collateral_custody_pda,
            collateral_custody_token_account_pda,
            collateral_account_address,
        )
    };
    //
    //

    let (position_pda, position_bump) = pda::get_position_pda(
        &owner.pubkey(),
        pool_pda,
        &principal_custody_pda,
        Side::Short,
    );

    let funding_account_address =
        utils::find_associated_token_account(&owner.pubkey(), user_collateral_token_mint).0;

    let (ix_size, tx_used_cu) = utils::create_and_execute_adrena_ix(
        program_test_ctx,
        adrena::accounts::OpenOrIncreasePositionWithSwapShort {
            owner: owner.pubkey(),
            payer: payer.pubkey(),
            funding_account: funding_account_address,
            collateral_account: collateral_account_address,

            //
            receiving_custody: receiving_custody_pda,
            receiving_custody_token_account: receiving_custody_token_account_pda,
            //
            collateral_custody: collateral_custody_pda,
            collateral_custody_token_account: collateral_custody_token_account_pda,
            //
            principal_custody: principal_custody_pda,
            oracle: oracle_pda,
            principal_custody_token_account: principal_custody_token_account_pda,
            //
            transfer_authority: transfer_authority_pda,
            cortex: cortex_pda,
            pool: *pool_pda,
            position: position_pda,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
            adrena_program: adrena::ID,
        }
        .to_account_metas(None),
        adrena::instruction::OpenOrIncreasePositionWithSwapShort {
            params: params.clone(),
        },
        Some(&payer.pubkey()),
        &[owner, payer],
        Some(vec![
            get_update_pool_ix(
                program_test_ctx,
                payer,
                pool_pda,
                params.oracle_prices.clone(),
            )
            .await?,
        ]),
        Some(vec![
            get_distribute_fees_ix(
                program_test_ctx,
                payer,
                pool_pda,
                &DistributeFeesParams {
                    oracle_prices: params.oracle_prices,
                },
            )
            .await?,
        ]),
    )
    .await?;

    utils::log_instruction(
        "open_or_increase_position_with_swap_short",
        "public",
        ix_size,
        tx_used_cu,
    );

    // ==== THEN ==============================================================

    Ok((position_pda, position_bump))
}
