// Contains fixtures values usable in tests, made to reduce boilerplate

use {
    adrena::state::custody::{BorrowRateParams, Fees, PricingParams},
    anchor_lang::prelude::Pubkey,
    pyth_solana_receiver_sdk::price_update::{PriceFeedMessage, PriceUpdateV2, VerificationLevel},
};

pub fn borrow_rate_regular() -> BorrowRateParams {
    BorrowRateParams {
        max_hourly_borrow_interest_rate: 100_000, // 0.01%
    }
}

pub fn get_pyth_price_update_v2_regular(
    pyth_price_feed_id: &Pubkey,
    price: i64,
    conf: u64,
    exponent: i32,
    publish_time: i64,
) -> PriceUpdateV2 {
    PriceUpdateV2 {
        write_authority: Pubkey::default(),
        verification_level: VerificationLevel::Full,
        price_message: PriceFeedMessage {
            feed_id: pyth_price_feed_id.to_bytes(),
            price,
            conf,
            exponent,
            prev_publish_time: publish_time - 1,
            publish_time,
            // Unused
            ema_conf: 0,
            ema_price: 0,
        },
        posted_slot: 0,
    }
}

pub fn fees_regular() -> Fees {
    Fees {
        swap_in: 10,
        swap_out: 10,
        stable_swap_in: 10,
        stable_swap_out: 10,
        add_liquidity: 10,
        remove_liquidity: 10,
        close_position: 16,
        liquidation: 16,
        fee_max: 200,
        ..Default::default()
    }
}

pub fn no_fees() -> Fees {
    Fees {
        swap_in: 0,
        swap_out: 0,
        stable_swap_in: 0,
        stable_swap_out: 0,
        add_liquidity: 0,
        remove_liquidity: 0,
        close_position: 0,
        liquidation: 0,
        fee_max: 0,
        ..Default::default()
    }
}

pub fn pricing_params_regular() -> PricingParams {
    PricingParams {
        max_initial_leverage: 1_200_000, // 120x
        max_leverage: 3_000_000,         // 300x
        max_position_locked_usd: 0,
        max_cumulative_short_position_size_usd: 0,
    }
}
