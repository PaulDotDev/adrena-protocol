use {
    adrena::{adapters::spl_governance_program_adapter, state::position::Side},
    solana_sdk::pubkey::Pubkey,
};

pub fn get_transfer_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&["transfer_authority".as_ref()], &adrena::id())
}

pub fn get_oracle_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&["oracle".as_ref()], &adrena::id())
}

pub fn get_cortex_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&["cortex".as_ref()], &adrena::id())
}

pub fn get_vest_registry_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&["vest_registry".as_ref()], &adrena::id())
}

pub fn get_genesis_lock_pda(pool_pda: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&["genesis_lock".as_ref(), pool_pda.as_ref()], &adrena::id())
}

pub fn get_lm_token_mint_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&["lm_token_mint".as_ref()], &adrena::id())
}

pub fn get_metadata_pda(mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            "metadata".as_bytes(),
            mpl_token_metadata::ID.as_ref(),
            mint.as_ref(),
        ],
        &mpl_token_metadata::ID,
    )
}

pub fn get_governance_token_mint_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&["governance_token_mint".as_ref()], &adrena::id())
}

pub fn get_user_staking_pda(owner: &Pubkey, staking_pda: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            "user_staking".as_ref(),
            owner.as_ref(),
            staking_pda.as_ref(),
        ],
        &adrena::id(),
    )
}

pub fn get_staking_pda(staked_token_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &["staking".as_ref(), staked_token_mint.as_ref()],
        &adrena::id(),
    )
}

pub fn get_staking_staked_token_vault_pda(staking_pda: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &["staking_staked_token_vault".as_ref(), staking_pda.as_ref()],
        &adrena::id(),
    )
}

pub fn get_staking_reward_token_vault_pda(staking_pda: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &["staking_reward_token_vault".as_ref(), staking_pda.as_ref()],
        &adrena::id(),
    )
}

pub fn get_user_profile_nickname_pda(nickname: String) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &["nickname".as_ref(), nickname.into_bytes().as_slice()],
        &adrena::id(),
    )
}

pub fn get_user_profile_pda(user: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&["user_profile".as_ref(), user.as_ref()], &adrena::id())
}

pub fn get_staking_lm_reward_token_vault_pda(staking_pda: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            "staking_lm_reward_token_vault".as_ref(),
            staking_pda.as_ref(),
        ],
        &adrena::id(),
    )
}

pub fn get_program_data_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[adrena::id().as_ref()],
        &solana_program::bpf_loader_upgradeable::id(),
    )
}

pub fn get_pool_pda(name: &String) -> (Pubkey, u8) {
    Pubkey::find_program_address(&["pool".as_ref(), name.as_bytes()], &adrena::id())
}

pub fn get_vest_pda(owner: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&["vest".as_ref(), owner.as_ref()], &adrena::id())
}

pub fn get_lp_token_mint_pda(pool_pda: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &["lp_token_mint".as_ref(), pool_pda.as_ref()],
        &adrena::id(),
    )
}

pub fn get_referrer_reward_token_vault(fee_redistribution_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            "referrer_reward_token_vault".as_ref(),
            fee_redistribution_mint.as_ref(),
        ],
        &adrena::id(),
    )
}

pub fn get_custody_pda(pool_pda: &Pubkey, custody_token_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            "custody".as_ref(),
            pool_pda.as_ref(),
            custody_token_mint.as_ref(),
        ],
        &adrena::id(),
    )
}

pub fn get_collateral_escrow_pda(pool_pda: &Pubkey, owner: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            "escrow_account".as_ref(),
            owner.as_ref(),
            pool_pda.as_ref(),
            mint.as_ref(),
        ],
        &adrena::id(),
    )
}

pub fn get_limit_order_book_pda(pool_pda: &Pubkey, owner: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            "limit_order_book".as_ref(),
            owner.as_ref(),
            pool_pda.as_ref(),
        ],
        &adrena::id(),
    )
}

pub fn get_position_pda(
    owner: &Pubkey,
    pool_pda: &Pubkey,
    custody_pda: &Pubkey,
    side: Side,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            "position".as_ref(),
            owner.as_ref(),
            pool_pda.as_ref(),
            custody_pda.as_ref(),
            &[side as u8],
        ],
        &adrena::id(),
    )
}

pub fn get_custody_token_account_pda(
    pool_pda: &Pubkey,
    custody_token_mint: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            "custody_token_account".as_ref(),
            pool_pda.as_ref(),
            custody_token_mint.as_ref(),
        ],
        &adrena::id(),
    )
}

pub fn get_governance_realm_pda(name: String) -> Pubkey {
    spl_governance::state::realm::get_realm_address(&spl_governance_program_adapter::ID, &name)
}

pub fn get_governance_governing_token_holding_pda(
    governance_realm_pda: &Pubkey,
    governing_token_mint: &Pubkey,
) -> Pubkey {
    spl_governance::state::realm::get_governing_token_holding_address(
        &spl_governance_program_adapter::ID,
        governance_realm_pda,
        governing_token_mint,
    )
}

pub fn get_governance_realm_config_pda(governance_realm_pda: &Pubkey) -> Pubkey {
    spl_governance::state::realm_config::get_realm_config_address(
        &spl_governance_program_adapter::ID,
        governance_realm_pda,
    )
}

pub fn get_governance_governing_token_owner_record_pda(
    governance_realm_pda: &Pubkey,
    governing_token_mint: &Pubkey,
    governing_token_owner: &Pubkey,
) -> Pubkey {
    spl_governance::state::token_owner_record::get_token_owner_record_address(
        &spl_governance_program_adapter::ID,
        governance_realm_pda,
        governing_token_mint,
        governing_token_owner,
    )
}
