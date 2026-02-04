use {
    crate::utils::limited_string::LimitedString,
    anchor_lang::{prelude::*, Discriminator},
    borsh::{BorshDeserialize, BorshSerialize},
};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Title {
    Zero = 0,
    GoldenHands = 1,
    DiamondHands = 2,
    AwakeningRank1 = 3,
    AwakeningChallenger = 4,
    AwakeningContender = 5,
    ExpanseRank1 = 6,
    ExpanseChallenger = 7,
    ExpanseContender = 8,
    Trader = 9,
    EmergingTrader = 10,
    TopTier = 11,
    VolumeKing = 12,
    FutureMcDonaldsEmployee = 13,
    HighlyUnprofitableTrader = 14,
    SeverelyWounded = 15,
    DaddysMoney = 16,
    AllInAllGone = 17,
    HighlyProfitableTrader = 18,
    CertifiedMoneyPrinter = 19,
    WhaleAmongMen = 20,
    ApexTrader = 21,
    Unstoppable = 22,
    FreeKebab = 23,
    PassiveIncome = 24,
    AdrenaStakeholder = 25,
    BoardMember = 26,
    LiquidityKing = 27,
    BadLuckBrian = 28,
    LeCramer = 29,
    TheChameleon = 30,
    SoldierS2 = 31,
    SergeantS2 = 32,
    LieutenantS2 = 33,
    GeneralS2 = 34,
    BonkOperative = 35,
    JitoJuggernaut = 36,
    Season2Champion = 37,
    Season2Destroyer = 38,
    CrownSniffer = 39,
    CertifiedMenace = 40,
    TombRaider = 41,
    Saboteur = 42,
    Traitor = 43,
    Opportunist = 44,
    Relentless = 45,
    WetAndLosing = 46,
    Underwater = 47,
    BossMuncher = 48,
    Scratcher = 49,
    PaperBeatRock = 50,
    ThePainmaker = 51,
    ChickenWings = 52,
    NiceGuy = 53,
}

impl TryFrom<u8> for Title {
    type Error = &'static str;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(Title::Zero),
            1 => Ok(Title::GoldenHands),
            2 => Ok(Title::DiamondHands),
            3 => Ok(Title::AwakeningRank1),
            4 => Ok(Title::AwakeningChallenger),
            5 => Ok(Title::AwakeningContender),
            6 => Ok(Title::ExpanseRank1),
            7 => Ok(Title::ExpanseChallenger),
            8 => Ok(Title::ExpanseContender),
            9 => Ok(Title::Trader),
            10 => Ok(Title::EmergingTrader),
            11 => Ok(Title::TopTier),
            12 => Ok(Title::VolumeKing),
            13 => Ok(Title::FutureMcDonaldsEmployee),
            14 => Ok(Title::HighlyUnprofitableTrader),
            15 => Ok(Title::SeverelyWounded),
            16 => Ok(Title::DaddysMoney),
            17 => Ok(Title::AllInAllGone),
            18 => Ok(Title::HighlyProfitableTrader),
            19 => Ok(Title::CertifiedMoneyPrinter),
            20 => Ok(Title::WhaleAmongMen),
            21 => Ok(Title::ApexTrader),
            22 => Ok(Title::Unstoppable),
            23 => Ok(Title::FreeKebab),
            24 => Ok(Title::PassiveIncome),
            25 => Ok(Title::AdrenaStakeholder),
            26 => Ok(Title::BoardMember),
            27 => Ok(Title::LiquidityKing),
            28 => Ok(Title::BadLuckBrian),
            29 => Ok(Title::LeCramer),
            30 => Ok(Title::TheChameleon),
            31 => Ok(Title::SoldierS2),
            32 => Ok(Title::SergeantS2),
            33 => Ok(Title::LieutenantS2),
            34 => Ok(Title::GeneralS2),
            35 => Ok(Title::BonkOperative),
            36 => Ok(Title::JitoJuggernaut),
            37 => Ok(Title::Season2Champion),
            38 => Ok(Title::Season2Destroyer),
            39 => Ok(Title::CrownSniffer),
            40 => Ok(Title::CertifiedMenace),
            41 => Ok(Title::TombRaider),
            42 => Ok(Title::Saboteur),
            43 => Ok(Title::Traitor),
            44 => Ok(Title::Opportunist),
            45 => Ok(Title::Relentless),
            46 => Ok(Title::WetAndLosing),
            47 => Ok(Title::Underwater),
            48 => Ok(Title::BossMuncher),
            49 => Ok(Title::Scratcher),
            50 => Ok(Title::PaperBeatRock),
            51 => Ok(Title::ThePainmaker),
            52 => Ok(Title::ChickenWings),
            53 => Ok(Title::NiceGuy),
            _ => Err("Invalid value for Title enum"),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum Wallpaper {
    Zero = 0,
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    VolumeKing = 5,
    Streak5 = 6,
    SoldierS2 = 7,
    SergeantS2 = 8,
    LieutenantS2 = 9,
    GeneralS2 = 10,
    BonkOperative = 11,
    JitoJuggernaut = 12,
    Season2Champion = 13,
    ThePainmaker = 14,
    NiceGuy = 15,
}

impl TryFrom<u8> for Wallpaper {
    type Error = &'static str;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(Wallpaper::Zero),
            1 => Ok(Wallpaper::One),
            2 => Ok(Wallpaper::Two),
            3 => Ok(Wallpaper::Three),
            4 => Ok(Wallpaper::Four),
            5 => Ok(Wallpaper::VolumeKing),
            6 => Ok(Wallpaper::Streak5),
            7 => Ok(Wallpaper::SoldierS2),
            8 => Ok(Wallpaper::SergeantS2),
            9 => Ok(Wallpaper::LieutenantS2),
            10 => Ok(Wallpaper::GeneralS2),
            11 => Ok(Wallpaper::BonkOperative),
            12 => Ok(Wallpaper::JitoJuggernaut),
            13 => Ok(Wallpaper::Season2Champion),
            14 => Ok(Wallpaper::ThePainmaker),
            15 => Ok(Wallpaper::NiceGuy),
            _ => Err("Invalid value for Wallpaper enum"),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ProfilePicture {
    Zero = 0,
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    TopTier = 5,
    WhaleAmongMen = 6,
    Streak10 = 7,
    StakedHolder = 8,
    SoldierS2 = 9,
    SergeantS2 = 10,
    LieutenantS2 = 11,
    GeneralS2 = 12,
    BonkOperative = 13,
    JitoJuggernaut = 14,
    Season2Champion = 15,
    Season2Destroyer = 16,
    Saboteur = 17,
    Traitor = 18,
    Relentless = 19,
    BossMuncher = 20,
    ThePainmaker = 21,
    NiceGuy = 22,
    GoldenHands = 23,
    DiamondHands = 24,
}

impl TryFrom<u8> for ProfilePicture {
    type Error = &'static str;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(ProfilePicture::Zero),
            1 => Ok(ProfilePicture::One),
            2 => Ok(ProfilePicture::Two),
            3 => Ok(ProfilePicture::Three),
            4 => Ok(ProfilePicture::Four),
            5 => Ok(ProfilePicture::TopTier),
            6 => Ok(ProfilePicture::WhaleAmongMen),
            7 => Ok(ProfilePicture::Streak10),
            8 => Ok(ProfilePicture::StakedHolder),
            9 => Ok(ProfilePicture::SoldierS2),
            10 => Ok(ProfilePicture::SergeantS2),
            11 => Ok(ProfilePicture::LieutenantS2),
            12 => Ok(ProfilePicture::GeneralS2),
            13 => Ok(ProfilePicture::BonkOperative),
            14 => Ok(ProfilePicture::JitoJuggernaut),
            15 => Ok(ProfilePicture::Season2Champion),
            16 => Ok(ProfilePicture::Season2Destroyer),
            17 => Ok(ProfilePicture::Saboteur),
            18 => Ok(ProfilePicture::Traitor),
            19 => Ok(ProfilePicture::Relentless),
            20 => Ok(ProfilePicture::BossMuncher),
            21 => Ok(ProfilePicture::ThePainmaker),
            22 => Ok(ProfilePicture::NiceGuy),
            23 => Ok(ProfilePicture::GoldenHands),
            24 => Ok(ProfilePicture::DiamondHands),
            _ => Err("Invalid value for ProfilePicture enum"),
        }
    }
}

pub enum UserProfileVersion {
    V1 = 0,
    V2 = 2,
}

impl UserProfileVersion {
    pub const fn latest() -> Self {
        UserProfileVersion::V2
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Team {
    Default = 0,
    Bonk = 1,
    Jito = 2,
}

impl TryFrom<u8> for Team {
    type Error = &'static str;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(Team::Default),
            1 => Ok(Team::Bonk),
            2 => Ok(Team::Jito),
            _ => Err("Invalid value for Team enum"),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Continent {
    Default = 0,
    Europe = 1,
    NorthAmerica = 2,
    SouthAmerica = 3,
    Asia = 4,
    Africa = 5,
    Australia = 6,
    Antarctica = 7,
}

impl TryFrom<u8> for Continent {
    type Error = &'static str;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(Continent::Default),
            1 => Ok(Continent::Europe),
            2 => Ok(Continent::NorthAmerica),
            3 => Ok(Continent::SouthAmerica),
            4 => Ok(Continent::Asia),
            5 => Ok(Continent::Africa),
            6 => Ok(Continent::Australia),
            7 => Ok(Continent::Antarctica),
            _ => Err("Invalid value for Continent enum"),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Achievement {
    FirstTrade = 0,
    KeepADX50Percent = 1,
    KeepADX90Percent = 2,
    AwakeningRank1 = 3,
    AwakeningChallenger = 4,
    AwakeningContender = 5,
    ExpanseRank1 = 6,
    ExpanseChallenger = 7,
    ExpanseContender = 8,
    FirstProfitableTrade = 9,
    Volume1M = 10,
    Volume10M = 11,
    Volume100M = 12,
    Volume250M = 13,
    Volume500M = 14,
    Volume1B = 15,
    Loss5K = 16,
    Loss10K = 17,
    Loss50K = 18,
    Loss200K = 19,
    Loss500K = 20,
    Loss1M = 21,
    Profit5K = 22,
    Profit10K = 23,
    Profit50K = 24,
    Profit200K = 25,
    Profit500K = 26,
    Profit1M = 27,
    Streak5 = 28,
    Streak10 = 29,
    Streak20 = 30,
    StakedEarnings10 = 31,
    StakedEarnings1K = 32,
    StakedEarnings5K = 33,
    StakedEarnings10K = 34,
    StakedEarnings50K = 35,
    StakedEarnings100K = 36,
    StakedHoldings1M = 37,
    StakedHoldings3M = 38,
    StakedHoldings6M = 39,
    StakedHoldings10M = 40,
    StakedHoldings20M = 41,
    StakedHoldings50M = 42,
    Liquidity1K = 43,
    Liquidity50K = 44,
    Liquidity100K = 45,
    Liquidity250K = 46,
    Liquidity500K = 47,
    Liquidity1M = 48,
    TradeOpen30Days = 49,
    Liquidated1 = 50,
    Liquidated25 = 51,
    Liquidated50 = 52,
    Liquidated100 = 53,
    ChangeUsername10 = 54,
    Soldier = 55,
    Sergeant = 56,
    Lieutenant = 57,
    General = 58,
    BonkOperative = 59,
    JitoJuggernaut = 60,
    Season2Champion = 61,
    Season2Destroyer = 62,
    CrownSniffer = 63,
    CertifiedMenace = 64,
    TombRaider = 65,
    Saboteur = 66,
    Traitor = 67,
    Opportunist = 68,
    Relentless = 69,
    WetAndLosing = 70,
    Underwater = 71,
    BossMuncher = 72,
    Scratcher = 73,
    PaperBeatRock = 74,
    ThePainmaker = 75,
    ChickenWings = 76,
    NiceGuy = 77,
}

impl Achievement {
    /// Get the points for this achievement
    pub const fn points(&self) -> u32 {
        const POINTS_BY_ACHIEVEMENT: [u32; 78] = [
            5,   // FirstTrade
            25,  // KeepADX50Percent
            50,  // KeepADX90Percent
            25,  // AwakeningRank1
            25,  // AwakeningChallenger
            5,   // AwakeningContender
            25,  // ExpanseRank1
            25,  // ExpanseChallenger
            5,   // ExpanseContender
            5,   // FirstProfitableTrade
            5,   // Volume1M
            15,  // Volume10M
            25,  // Volume100M
            50,  // Volume250M
            100, // Volume500M
            200, // Volume1B
            5,   // Loss5K
            15,  // Loss10K
            25,  // Loss50K
            50,  // Loss200K
            100, // Loss500K
            200, // Loss1M
            5,   // Profit5K
            15,  // Profit10K
            25,  // Profit50K
            50,  // Profit200K
            100, // Profit500K
            200, // Profit1M
            10,  // Streak5
            50,  // Streak10
            200, // Streak20
            5,   // StakedEarnings10
            10,  // StakedEarnings1K
            15,  // StakedEarnings5K
            25,  // StakedEarnings10K
            50,  // StakedEarnings50K
            100, // StakedEarnings100K
            10,  // StakedHoldings1M
            15,  // StakedHoldings3M
            25,  // StakedHoldings6M
            50,  // StakedHoldings10M
            100, // StakedHoldings20M
            200, // StakedHoldings50M
            5,   // Liquidity1K
            5,   // Liquidity50K
            10,  // Liquidity100K
            10,  // Liquidity250K
            10,  // Liquidity500K
            15,  // Liquidity1M
            10,  // TradeOpen30Days
            5,   // Liquidated1
            5,   // Liquidated25
            10,  // Liquidated50
            25,  // Liquidated100
            5,   // ChangeUsername10
            5,   // Soldier S2
            15,  // Sergeant S2
            50,  // Lieutenant S2
            100, // General S2
            5,   // Bonk Operative
            5,   // Jito Juggernaut
            100, // Season 2 Champion
            25,  // Season 2 Destroyer
            50,  // Crown Sniffer
            10,  // Certified Menace
            50,  // Tomb Raider
            25,  // Saboteur
            5,   // Traitor
            25,  // Opportunist
            50,  // Relentless
            5,   // Wet and Losing
            15,  // Underwater
            15,  // Boss Muncher
            50,  // Scratcher
            100, // PaperBeatRock
            200, // The Painmaker
            15,  // ChickenWings
            100, // NiceGuy
        ];

        POINTS_BY_ACHIEVEMENT[*self as usize]
    }
}

impl TryFrom<u8> for Achievement {
    type Error = &'static str;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(Achievement::FirstTrade),
            1 => Ok(Achievement::KeepADX50Percent),
            2 => Ok(Achievement::KeepADX90Percent),
            3 => Ok(Achievement::AwakeningRank1),
            4 => Ok(Achievement::AwakeningChallenger),
            5 => Ok(Achievement::AwakeningContender),
            6 => Ok(Achievement::ExpanseRank1),
            7 => Ok(Achievement::ExpanseChallenger),
            8 => Ok(Achievement::ExpanseContender),
            9 => Ok(Achievement::FirstProfitableTrade),
            10 => Ok(Achievement::Volume1M),
            11 => Ok(Achievement::Volume10M),
            12 => Ok(Achievement::Volume100M),
            13 => Ok(Achievement::Volume250M),
            14 => Ok(Achievement::Volume500M),
            15 => Ok(Achievement::Volume1B),
            16 => Ok(Achievement::Loss5K),
            17 => Ok(Achievement::Loss10K),
            18 => Ok(Achievement::Loss50K),
            19 => Ok(Achievement::Loss200K),
            20 => Ok(Achievement::Loss500K),
            21 => Ok(Achievement::Loss1M),
            22 => Ok(Achievement::Profit5K),
            23 => Ok(Achievement::Profit10K),
            24 => Ok(Achievement::Profit50K),
            25 => Ok(Achievement::Profit200K),
            26 => Ok(Achievement::Profit500K),
            27 => Ok(Achievement::Profit1M),
            28 => Ok(Achievement::Streak5),
            29 => Ok(Achievement::Streak10),
            30 => Ok(Achievement::Streak20),
            31 => Ok(Achievement::StakedEarnings10),
            32 => Ok(Achievement::StakedEarnings1K),
            33 => Ok(Achievement::StakedEarnings5K),
            34 => Ok(Achievement::StakedEarnings10K),
            35 => Ok(Achievement::StakedEarnings50K),
            36 => Ok(Achievement::StakedEarnings100K),
            37 => Ok(Achievement::StakedHoldings1M),
            38 => Ok(Achievement::StakedHoldings3M),
            39 => Ok(Achievement::StakedHoldings6M),
            40 => Ok(Achievement::StakedHoldings10M),
            41 => Ok(Achievement::StakedHoldings20M),
            42 => Ok(Achievement::StakedHoldings50M),
            43 => Ok(Achievement::Liquidity1K),
            44 => Ok(Achievement::Liquidity50K),
            45 => Ok(Achievement::Liquidity100K),
            46 => Ok(Achievement::Liquidity250K),
            47 => Ok(Achievement::Liquidity500K),
            48 => Ok(Achievement::Liquidity1M),
            49 => Ok(Achievement::TradeOpen30Days),
            50 => Ok(Achievement::Liquidated1),
            51 => Ok(Achievement::Liquidated25),
            52 => Ok(Achievement::Liquidated50),
            53 => Ok(Achievement::Liquidated100),
            54 => Ok(Achievement::ChangeUsername10),
            55 => Ok(Achievement::Soldier),
            56 => Ok(Achievement::Sergeant),
            57 => Ok(Achievement::Lieutenant),
            58 => Ok(Achievement::General),
            59 => Ok(Achievement::BonkOperative),
            60 => Ok(Achievement::JitoJuggernaut),
            61 => Ok(Achievement::Season2Champion),
            62 => Ok(Achievement::Season2Destroyer),
            63 => Ok(Achievement::CrownSniffer),
            64 => Ok(Achievement::CertifiedMenace),
            65 => Ok(Achievement::TombRaider),
            66 => Ok(Achievement::Saboteur),
            67 => Ok(Achievement::Traitor),
            68 => Ok(Achievement::Opportunist),
            69 => Ok(Achievement::Relentless),
            70 => Ok(Achievement::WetAndLosing),
            71 => Ok(Achievement::Underwater),
            72 => Ok(Achievement::BossMuncher),
            73 => Ok(Achievement::Scratcher),
            74 => Ok(Achievement::PaperBeatRock),
            75 => Ok(Achievement::ThePainmaker),
            76 => Ok(Achievement::ChickenWings),
            77 => Ok(Achievement::NiceGuy),
            _ => Err("Invalid value for Achievement enum"),
        }
    }
}

#[account(zero_copy)]
#[derive(Debug, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct UserProfile {
    pub bump: u8,
    pub version: u8,
    pub profile_picture: u8, // Enum of profile pictures
    pub wallpaper: u8,       // Enum of wallpapers
    pub title: u8,           // Enum of title
    pub team: u8,
    pub continent: u8,
    pub _padding: u8,
    pub nickname: LimitedString,
    pub created_at: i64,
    pub owner: Pubkey,
    pub achievements: [u8; 256], // Enough to fit 255 achievements + be a multiple of 8 for memory alignment
    pub referrer_profile: Pubkey, // Pubkey of the referrer profile (not the wallet!)
    pub claimable_referral_fee_usd: u64, // Referral fee that can be claimed by the referrer right now
    pub total_referral_fee_usd: u64,     // Total referral fee earned by the referrer
    pub _padding2: [u8; 16],
}

impl UserProfile {
    // 8 bytes for anchor discriminator
    pub const LEN: usize = 8 + std::mem::size_of::<UserProfile>();

    // TODO: Update when mutating the User Profile structure
    pub const VERSION: u8 = UserProfileVersion::latest() as u8;

    pub const CHANGE_NICKNAME_TAX: u64 = 500_000_000; // 500 ADX

    pub fn has_achievement(&self, achievement: Achievement) -> bool {
        self.achievements[achievement as usize] > 0
    }

    pub fn unlock_achievement(&mut self, achievement: Achievement) {
        self.achievements[achievement as usize] = achievement.points() as u8;
    }

    pub fn can_use_title(&self, title: Title) -> bool {
        match title {
            Title::Zero => true, // Default title, always available
            Title::GoldenHands => self.has_achievement(Achievement::KeepADX50Percent),
            Title::DiamondHands => self.has_achievement(Achievement::KeepADX90Percent),
            Title::AwakeningRank1 => self.has_achievement(Achievement::AwakeningRank1),
            Title::AwakeningChallenger => self.has_achievement(Achievement::AwakeningChallenger),
            Title::AwakeningContender => self.has_achievement(Achievement::AwakeningContender),
            Title::ExpanseRank1 => self.has_achievement(Achievement::ExpanseRank1),
            Title::ExpanseChallenger => self.has_achievement(Achievement::ExpanseChallenger),
            Title::ExpanseContender => self.has_achievement(Achievement::ExpanseContender),
            Title::Trader => self.has_achievement(Achievement::FirstTrade),
            Title::EmergingTrader => self.has_achievement(Achievement::Volume1M),
            Title::TopTier => self.has_achievement(Achievement::Volume500M),
            Title::VolumeKing => self.has_achievement(Achievement::Volume1B),
            Title::FutureMcDonaldsEmployee => self.has_achievement(Achievement::Loss5K),
            Title::HighlyUnprofitableTrader => self.has_achievement(Achievement::Loss50K),
            Title::SeverelyWounded => self.has_achievement(Achievement::Loss200K),
            Title::DaddysMoney => self.has_achievement(Achievement::Loss500K),
            Title::AllInAllGone => self.has_achievement(Achievement::Loss1M),
            Title::HighlyProfitableTrader => self.has_achievement(Achievement::Profit50K),
            Title::CertifiedMoneyPrinter => self.has_achievement(Achievement::Profit200K),
            Title::WhaleAmongMen => self.has_achievement(Achievement::Profit500K),
            Title::ApexTrader => self.has_achievement(Achievement::Profit1M),
            Title::Unstoppable => self.has_achievement(Achievement::Streak20),
            Title::FreeKebab => self.has_achievement(Achievement::StakedEarnings10),
            Title::PassiveIncome => self.has_achievement(Achievement::StakedEarnings100K),
            Title::AdrenaStakeholder => self.has_achievement(Achievement::StakedHoldings6M),
            Title::BoardMember => self.has_achievement(Achievement::StakedHoldings50M),
            Title::LiquidityKing => self.has_achievement(Achievement::Liquidity1M),
            Title::BadLuckBrian => self.has_achievement(Achievement::Liquidated1),
            Title::LeCramer => self.has_achievement(Achievement::Liquidated100),
            Title::TheChameleon => self.has_achievement(Achievement::ChangeUsername10),
            Title::SoldierS2 => self.has_achievement(Achievement::Soldier),
            Title::SergeantS2 => self.has_achievement(Achievement::Sergeant),
            Title::LieutenantS2 => self.has_achievement(Achievement::Lieutenant),
            Title::GeneralS2 => self.has_achievement(Achievement::General),
            Title::BonkOperative => self.has_achievement(Achievement::BonkOperative),
            Title::JitoJuggernaut => self.has_achievement(Achievement::JitoJuggernaut),
            Title::Season2Champion => self.has_achievement(Achievement::Season2Champion),
            Title::Season2Destroyer => self.has_achievement(Achievement::Season2Destroyer),
            Title::CrownSniffer => self.has_achievement(Achievement::CrownSniffer),
            Title::CertifiedMenace => self.has_achievement(Achievement::CertifiedMenace),
            Title::TombRaider => self.has_achievement(Achievement::TombRaider),
            Title::Saboteur => self.has_achievement(Achievement::Saboteur),
            Title::Traitor => self.has_achievement(Achievement::Traitor),
            Title::Opportunist => self.has_achievement(Achievement::Opportunist),
            Title::Relentless => self.has_achievement(Achievement::Relentless),
            Title::WetAndLosing => self.has_achievement(Achievement::WetAndLosing),
            Title::Underwater => self.has_achievement(Achievement::Underwater),
            Title::BossMuncher => self.has_achievement(Achievement::BossMuncher),
            Title::Scratcher => self.has_achievement(Achievement::Scratcher),
            Title::PaperBeatRock => self.has_achievement(Achievement::PaperBeatRock),
            Title::ThePainmaker => self.has_achievement(Achievement::ThePainmaker),
            Title::ChickenWings => self.has_achievement(Achievement::ChickenWings),
            Title::NiceGuy => self.has_achievement(Achievement::NiceGuy),
        }
    }

    pub fn can_use_pfp(&self, pfp: ProfilePicture) -> bool {
        match pfp {
            // Default profile pictures, always available
            ProfilePicture::Zero
            | ProfilePicture::One
            | ProfilePicture::Two
            | ProfilePicture::Three
            | ProfilePicture::Four => true,

            // Special profile pictures unlocked by achievements
            ProfilePicture::TopTier => self.has_achievement(Achievement::Volume500M),
            ProfilePicture::WhaleAmongMen => self.has_achievement(Achievement::Profit500K),
            ProfilePicture::Streak10 => self.has_achievement(Achievement::Streak10),
            ProfilePicture::StakedHolder => self.has_achievement(Achievement::StakedHoldings3M),
            ProfilePicture::SoldierS2 => self.has_achievement(Achievement::Soldier),
            ProfilePicture::SergeantS2 => self.has_achievement(Achievement::Sergeant),
            ProfilePicture::LieutenantS2 => self.has_achievement(Achievement::Lieutenant),
            ProfilePicture::GeneralS2 => self.has_achievement(Achievement::General),
            ProfilePicture::BonkOperative => self.has_achievement(Achievement::BonkOperative),
            ProfilePicture::JitoJuggernaut => self.has_achievement(Achievement::JitoJuggernaut),
            ProfilePicture::Season2Champion => self.has_achievement(Achievement::Season2Champion),
            ProfilePicture::Season2Destroyer => self.has_achievement(Achievement::Season2Destroyer),
            ProfilePicture::Saboteur => self.has_achievement(Achievement::Saboteur),
            ProfilePicture::Traitor => self.has_achievement(Achievement::Traitor),
            ProfilePicture::Relentless => self.has_achievement(Achievement::Relentless),
            ProfilePicture::BossMuncher => self.has_achievement(Achievement::BossMuncher),
            ProfilePicture::ThePainmaker => self.has_achievement(Achievement::ThePainmaker),
            ProfilePicture::NiceGuy => self.has_achievement(Achievement::NiceGuy),
            ProfilePicture::GoldenHands => self.has_achievement(Achievement::KeepADX50Percent),
            ProfilePicture::DiamondHands => self.has_achievement(Achievement::KeepADX90Percent),
        }
    }

    pub fn can_use_wallpaper(&self, wallpaper: Wallpaper) -> bool {
        match wallpaper {
            // Default wallpapers, always available
            Wallpaper::Zero
            | Wallpaper::One
            | Wallpaper::Two
            | Wallpaper::Three
            | Wallpaper::Four => true,

            // Special wallpapers unlocked by achievements
            Wallpaper::VolumeKing => self.has_achievement(Achievement::Volume250M),
            Wallpaper::Streak5 => self.has_achievement(Achievement::Streak5),
            Wallpaper::SoldierS2 => self.has_achievement(Achievement::Soldier),
            Wallpaper::SergeantS2 => self.has_achievement(Achievement::Sergeant),
            Wallpaper::LieutenantS2 => self.has_achievement(Achievement::Lieutenant),
            Wallpaper::GeneralS2 => self.has_achievement(Achievement::General),
            Wallpaper::BonkOperative => self.has_achievement(Achievement::BonkOperative),
            Wallpaper::JitoJuggernaut => self.has_achievement(Achievement::JitoJuggernaut),
            Wallpaper::Season2Champion => self.has_achievement(Achievement::Season2Champion),
            Wallpaper::ThePainmaker => self.has_achievement(Achievement::ThePainmaker),
            Wallpaper::NiceGuy => self.has_achievement(Achievement::NiceGuy),
        }
    }

    // Say if the user profile exists or not
    pub fn exists(&self) -> bool {
        self.created_at != 0
    }

    /// Initialize a user profile with given parameters
    pub fn initialize(
        bump: u8,
        owner: Pubkey,
        created_at: i64,
        data: &mut [u8],
        referrer_profile: Option<Pubkey>,
    ) -> std::result::Result<(), ProgramError> {
        // Ensure the provided buffer is large enough
        if data.len() < Self::LEN {
            return Err(ProgramError::AccountDataTooSmall);
        }

        // Write the discriminator to the beginning of the data
        data[..8].copy_from_slice(&UserProfile::DISCRIMINATOR);

        // Create a new profile and serialize it
        let profile = UserProfile {
            bump,
            version: Self::VERSION,
            profile_picture: ProfilePicture::Zero as u8,
            wallpaper: Wallpaper::Zero as u8,
            title: Title::Zero as u8,
            team: Team::Default as u8,
            continent: Continent::Default as u8,
            _padding: 0,
            nickname: LimitedString::new(""),
            created_at,
            owner,
            achievements: [0; 256],
            referrer_profile: referrer_profile.unwrap_or_default(),
            claimable_referral_fee_usd: 0,
            total_referral_fee_usd: 0,
            _padding2: [0; 16],
        };

        profile.serialize(&mut &mut data[8..])?;

        Ok(())
    }

    pub fn remove_achievement(&mut self, achievement: Achievement) {
        self.achievements[achievement as usize] = 0;
    }

    pub fn get_title(&self) -> Title {
        Title::try_from(self.title).unwrap()
    }

    pub fn get_profile_picture(&self) -> ProfilePicture {
        ProfilePicture::try_from(self.profile_picture).unwrap()
    }

    pub fn get_wallpaper(&self) -> Wallpaper {
        Wallpaper::try_from(self.wallpaper).unwrap()
    }

    pub fn get_team(&self) -> Team {
        Team::try_from(self.team).unwrap()
    }

    pub fn get_continent(&self) -> Continent {
        Continent::try_from(self.continent).unwrap()
    }

    /// Validates if a nickname follows the required format for non-owner migrations
    /// The format must be "Monster" followed by digits
    pub fn is_autogenerated_default_nickname(nickname: &str) -> bool {
        let prefix = "Monster";

        // Check if nickname starts with prefix and has at least one digit
        if nickname.len() <= prefix.len() || !nickname.starts_with(prefix) {
            return false;
        }

        // Check that all remaining characters are digits
        nickname[prefix.len()..].chars().all(|c| c.is_ascii_digit())
    }
}

impl Default for UserProfile {
    fn default() -> Self {
        UserProfile {
            bump: 0,
            version: 0,
            profile_picture: 0,
            wallpaper: 0,
            title: 0,
            team: Team::Default as u8,
            continent: Continent::Default as u8,
            _padding: 0,
            nickname: LimitedString::default(),
            created_at: 0,
            owner: Pubkey::default(),
            achievements: [0; 256],
            referrer_profile: Pubkey::default(),
            claimable_referral_fee_usd: 0,
            total_referral_fee_usd: 0,
            _padding2: [0; 16],
        }
    }
}

//
// OLD AND DEPRECATED VERSION OF USER PROFILE
// KEPT FOR HISTORY AND MIGRATION PURPOSES
//
pub mod legacy {
    use {
        crate::{math, utils::limited_string::LimitedString},
        anchor_lang::prelude::*,
        borsh::{BorshDeserialize, BorshSerialize},
        bytemuck::{Pod, Zeroable},
    };

    #[account(zero_copy)]
    #[derive(Default, Debug, BorshDeserialize, BorshSerialize)]
    #[repr(C)]
    pub struct UserProfileV1 {
        pub bump: u8,
        pub version: u8,
        pub _padding: [u8; 6],
        pub nickname: LimitedString,
        pub created_at: i64,
        //
        pub owner: Pubkey,
        //
        pub swap_count: u64,
        pub swap_volume_usd: u64,
        pub swap_fee_paid_usd: u64,
        //
        pub short_stats: TradingStats,
        pub long_stats: TradingStats,
    }

    impl UserProfileV1 {
        pub const LEN: usize = 8 + std::mem::size_of::<UserProfileV1>();
    }

    #[derive(
        Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug, Pod, Zeroable,
    )]
    #[repr(C)]
    pub struct TradingStats {
        pub opened_position_count: u64,
        pub liquidated_position_count: u64,
        // At position opening, in bps
        pub opening_average_leverage: u64,
        pub opening_size_usd: u64,
        // Calculated when position close/get liquidated
        pub profits_usd: u64,
        pub losses_usd: u64,
        pub fee_paid_usd: u64,
    }

    impl TradingStats {
        // Calculate new average leverage when adding extra size_usd at leverage
        pub fn calculate_new_average_leverage(&self, size_usd: u64, leverage: u64) -> Result<u64> {
            let past_weighted_leverage =
                self.opening_size_usd as u128 * self.opening_average_leverage as u128;

            let position_weighted_leverage = size_usd as u128 * leverage as u128;

            let total_weighted_leverage = past_weighted_leverage + position_weighted_leverage;

            math::checked_as_u64(math::checked_ceil_div::<u128>(
                total_weighted_leverage,
                self.opening_size_usd as u128 + size_usd as u128,
            )?)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::state::{cortex::Cortex, user_profile::legacy::TradingStats};

    fn scale(amount: u64, decimals: u8) -> u64 {
        amount * 10u64.pow(decimals as u32)
    }

    #[test]
    fn test_calculate_new_average_leverage() {
        assert_eq!(
            1000,
            //
            // default
            TradingStats {
                ..TradingStats::default()
            }
            // adds $1k at x10
            .calculate_new_average_leverage(
                scale(1_000, Cortex::USD_DECIMALS),
                10 * Cortex::BPS_POWER as u64 / 100
            )
            .unwrap(),
        );

        assert_eq!(
            546,
            //
            // $10k at x5
            TradingStats {
                opening_average_leverage: 5 * Cortex::BPS_POWER as u64 / 100,
                opening_size_usd: scale(10_000, Cortex::USD_DECIMALS),
                ..TradingStats::default()
            }
            // adds $1k at x10
            .calculate_new_average_leverage(
                scale(1_000, Cortex::USD_DECIMALS),
                10 * Cortex::BPS_POWER as u64 / 100
            )
            .unwrap(),
        );

        assert_eq!(
            9_991,
            //
            // $500m at x100
            TradingStats {
                opening_average_leverage: 100 * Cortex::BPS_POWER as u64 / 100,
                opening_size_usd: scale(500_000_000, Cortex::USD_DECIMALS),
                ..TradingStats::default()
            }
            // adds $1m at x50
            .calculate_new_average_leverage(
                scale(1_000_000, Cortex::USD_DECIMALS),
                50 * Cortex::BPS_POWER as u64 / 100
            )
            .unwrap(),
        );
    }
}

/// Maps an achievement to the title it unlocks (if any)
#[inline]
pub fn achievement_to_title(achievement: &Achievement) -> Option<Title> {
    match achievement {
        Achievement::FirstTrade => Some(Title::Trader),
        Achievement::KeepADX50Percent => Some(Title::GoldenHands),
        Achievement::KeepADX90Percent => Some(Title::DiamondHands),
        Achievement::AwakeningRank1 => Some(Title::AwakeningRank1),
        Achievement::AwakeningChallenger => Some(Title::AwakeningChallenger),
        Achievement::AwakeningContender => Some(Title::AwakeningContender),
        Achievement::ExpanseRank1 => Some(Title::ExpanseRank1),
        Achievement::ExpanseChallenger => Some(Title::ExpanseChallenger),
        Achievement::ExpanseContender => Some(Title::ExpanseContender),
        Achievement::Volume1M => Some(Title::EmergingTrader),
        Achievement::Volume500M => Some(Title::TopTier),
        Achievement::Volume1B => Some(Title::VolumeKing),
        Achievement::Loss5K => Some(Title::FutureMcDonaldsEmployee),
        Achievement::Loss50K => Some(Title::HighlyUnprofitableTrader),
        Achievement::Loss200K => Some(Title::SeverelyWounded),
        Achievement::Loss500K => Some(Title::DaddysMoney),
        Achievement::Loss1M => Some(Title::AllInAllGone),
        Achievement::Profit50K => Some(Title::HighlyProfitableTrader),
        Achievement::Profit200K => Some(Title::CertifiedMoneyPrinter),
        Achievement::Profit500K => Some(Title::WhaleAmongMen),
        Achievement::Profit1M => Some(Title::ApexTrader),
        Achievement::Streak20 => Some(Title::Unstoppable),
        Achievement::StakedEarnings10 => Some(Title::FreeKebab),
        Achievement::StakedEarnings100K => Some(Title::PassiveIncome),
        Achievement::StakedHoldings6M => Some(Title::AdrenaStakeholder),
        Achievement::StakedHoldings50M => Some(Title::BoardMember),
        Achievement::Liquidity1M => Some(Title::LiquidityKing),
        Achievement::Liquidated1 => Some(Title::BadLuckBrian),
        Achievement::Liquidated100 => Some(Title::LeCramer),
        Achievement::ChangeUsername10 => Some(Title::TheChameleon),
        Achievement::Soldier => Some(Title::SoldierS2),
        Achievement::Sergeant => Some(Title::SergeantS2),
        Achievement::Lieutenant => Some(Title::LieutenantS2),
        Achievement::General => Some(Title::GeneralS2),
        Achievement::BonkOperative => Some(Title::BonkOperative),
        Achievement::JitoJuggernaut => Some(Title::JitoJuggernaut),
        Achievement::Season2Champion => Some(Title::Season2Champion),
        Achievement::Season2Destroyer => Some(Title::Season2Destroyer),
        Achievement::CrownSniffer => Some(Title::CrownSniffer),
        Achievement::CertifiedMenace => Some(Title::CertifiedMenace),
        Achievement::TombRaider => Some(Title::TombRaider),
        Achievement::Saboteur => Some(Title::Saboteur),
        Achievement::Traitor => Some(Title::Traitor),
        Achievement::Opportunist => Some(Title::Opportunist),
        Achievement::Relentless => Some(Title::Relentless),
        Achievement::WetAndLosing => Some(Title::WetAndLosing),
        Achievement::Underwater => Some(Title::Underwater),
        Achievement::BossMuncher => Some(Title::BossMuncher),
        Achievement::Scratcher => Some(Title::Scratcher),
        Achievement::PaperBeatRock => Some(Title::PaperBeatRock),
        Achievement::ThePainmaker => Some(Title::ThePainmaker),
        Achievement::ChickenWings => Some(Title::ChickenWings),
        Achievement::NiceGuy => Some(Title::NiceGuy),

        _ => None,
    }
}

/// Maps an achievement to the profile picture it unlocks (if any)
#[inline]
pub fn achievement_to_pfp(achievement: &Achievement) -> Option<ProfilePicture> {
    match achievement {
        Achievement::Volume500M => Some(ProfilePicture::TopTier),
        Achievement::Profit500K => Some(ProfilePicture::WhaleAmongMen),
        Achievement::Streak10 => Some(ProfilePicture::Streak10),
        Achievement::StakedHoldings3M => Some(ProfilePicture::StakedHolder),
        Achievement::Soldier => Some(ProfilePicture::SoldierS2),
        Achievement::Sergeant => Some(ProfilePicture::SergeantS2),
        Achievement::Lieutenant => Some(ProfilePicture::LieutenantS2),
        Achievement::General => Some(ProfilePicture::GeneralS2),
        Achievement::BonkOperative => Some(ProfilePicture::BonkOperative),
        Achievement::JitoJuggernaut => Some(ProfilePicture::JitoJuggernaut),
        Achievement::Season2Champion => Some(ProfilePicture::Season2Champion),
        Achievement::Season2Destroyer => Some(ProfilePicture::Season2Destroyer),
        Achievement::Saboteur => Some(ProfilePicture::Saboteur),
        Achievement::Traitor => Some(ProfilePicture::Traitor),
        Achievement::Relentless => Some(ProfilePicture::Relentless),
        Achievement::BossMuncher => Some(ProfilePicture::BossMuncher),
        Achievement::ThePainmaker => Some(ProfilePicture::ThePainmaker),
        Achievement::NiceGuy => Some(ProfilePicture::NiceGuy),
        Achievement::KeepADX50Percent => Some(ProfilePicture::GoldenHands),
        Achievement::KeepADX90Percent => Some(ProfilePicture::DiamondHands),

        _ => None,
    }
}

/// Maps an achievement to the wallpaper it unlocks (if any)
#[inline]
pub fn achievement_to_wallpaper(achievement: &Achievement) -> Option<Wallpaper> {
    match achievement {
        Achievement::Volume250M => Some(Wallpaper::VolumeKing),
        Achievement::Streak5 => Some(Wallpaper::Streak5),
        _ => None,
    }
}

/// Find achievements that unlock a specific title
#[inline]
pub fn find_achievements_by_title(title: &Title) -> Vec<Achievement> {
    match title {
        Title::Zero => vec![], // Default title doesn't require achievement
        Title::Trader => vec![Achievement::FirstTrade],
        Title::GoldenHands => vec![Achievement::KeepADX50Percent],
        Title::DiamondHands => vec![Achievement::KeepADX90Percent],
        Title::AwakeningRank1 => vec![Achievement::AwakeningRank1],
        Title::AwakeningChallenger => vec![Achievement::AwakeningChallenger],
        Title::AwakeningContender => vec![Achievement::AwakeningContender],
        Title::ExpanseRank1 => vec![Achievement::ExpanseRank1],
        Title::ExpanseChallenger => vec![Achievement::ExpanseChallenger],
        Title::ExpanseContender => vec![Achievement::ExpanseContender],
        Title::EmergingTrader => vec![Achievement::Volume1M],
        Title::TopTier => vec![Achievement::Volume500M],
        Title::VolumeKing => vec![Achievement::Volume1B],
        Title::FutureMcDonaldsEmployee => vec![Achievement::Loss5K],
        Title::HighlyUnprofitableTrader => vec![Achievement::Loss50K],
        Title::SeverelyWounded => vec![Achievement::Loss200K],
        Title::DaddysMoney => vec![Achievement::Loss500K],
        Title::AllInAllGone => vec![Achievement::Loss1M],
        Title::HighlyProfitableTrader => vec![Achievement::Profit50K],
        Title::CertifiedMoneyPrinter => vec![Achievement::Profit200K],
        Title::WhaleAmongMen => vec![Achievement::Profit500K],
        Title::ApexTrader => vec![Achievement::Profit1M],
        Title::Unstoppable => vec![Achievement::Streak20],
        Title::FreeKebab => vec![Achievement::StakedEarnings10],
        Title::PassiveIncome => vec![Achievement::StakedEarnings100K],
        Title::AdrenaStakeholder => vec![Achievement::StakedHoldings6M],
        Title::BoardMember => vec![Achievement::StakedHoldings50M],
        Title::LiquidityKing => vec![Achievement::Liquidity1M],
        Title::BadLuckBrian => vec![Achievement::Liquidated1],
        Title::LeCramer => vec![Achievement::Liquidated100],
        Title::TheChameleon => vec![Achievement::ChangeUsername10],
        Title::SoldierS2 => vec![Achievement::Soldier],
        Title::SergeantS2 => vec![Achievement::Sergeant],
        Title::LieutenantS2 => vec![Achievement::Lieutenant],
        Title::GeneralS2 => vec![Achievement::General],
        Title::BonkOperative => vec![Achievement::BonkOperative],
        Title::JitoJuggernaut => vec![Achievement::JitoJuggernaut],
        Title::Season2Champion => vec![Achievement::Season2Champion],
        Title::Season2Destroyer => vec![Achievement::Season2Destroyer],
        Title::CrownSniffer => vec![Achievement::CrownSniffer],
        Title::CertifiedMenace => vec![Achievement::CertifiedMenace],
        Title::TombRaider => vec![Achievement::TombRaider],
        Title::Saboteur => vec![Achievement::Saboteur],
        Title::Traitor => vec![Achievement::Traitor],
        Title::Opportunist => vec![Achievement::Opportunist],
        Title::Relentless => vec![Achievement::Relentless],
        Title::WetAndLosing => vec![Achievement::WetAndLosing],
        Title::Underwater => vec![Achievement::Underwater],
        Title::BossMuncher => vec![Achievement::BossMuncher],
        Title::Scratcher => vec![Achievement::Scratcher],
        Title::PaperBeatRock => vec![Achievement::PaperBeatRock],
        Title::ThePainmaker => vec![Achievement::ThePainmaker],
        Title::ChickenWings => vec![Achievement::ChickenWings],
        Title::NiceGuy => vec![Achievement::NiceGuy],
    }
}

/// Find achievements that unlock a specific profile picture
#[inline]
pub fn find_achievements_by_pfp(pfp: &ProfilePicture) -> Vec<Achievement> {
    match pfp {
        ProfilePicture::Zero
        | ProfilePicture::One
        | ProfilePicture::Two
        | ProfilePicture::Three
        | ProfilePicture::Four => vec![], // Default PFPs don't require achievements
        ProfilePicture::TopTier => vec![Achievement::Volume500M],
        ProfilePicture::WhaleAmongMen => vec![Achievement::Profit500K],
        ProfilePicture::Streak10 => vec![Achievement::Streak10],
        ProfilePicture::StakedHolder => vec![Achievement::StakedHoldings3M],
        ProfilePicture::SoldierS2 => vec![Achievement::Soldier],
        ProfilePicture::SergeantS2 => vec![Achievement::Sergeant],
        ProfilePicture::LieutenantS2 => vec![Achievement::Lieutenant],
        ProfilePicture::GeneralS2 => vec![Achievement::General],
        ProfilePicture::BonkOperative => vec![Achievement::BonkOperative],
        ProfilePicture::JitoJuggernaut => vec![Achievement::JitoJuggernaut],
        ProfilePicture::Season2Champion => vec![Achievement::Season2Champion],
        ProfilePicture::Season2Destroyer => vec![Achievement::Season2Destroyer],
        ProfilePicture::Saboteur => vec![Achievement::Saboteur],
        ProfilePicture::Traitor => vec![Achievement::Traitor],
        ProfilePicture::Relentless => vec![Achievement::Relentless],
        ProfilePicture::BossMuncher => vec![Achievement::BossMuncher],
        ProfilePicture::ThePainmaker => vec![Achievement::ThePainmaker],
        ProfilePicture::NiceGuy => vec![Achievement::NiceGuy],
        ProfilePicture::GoldenHands => vec![Achievement::KeepADX50Percent],
        ProfilePicture::DiamondHands => vec![Achievement::KeepADX90Percent],
    }
}

/// Find achievements that unlock a specific wallpaper
#[inline]
pub fn find_achievements_by_wallpaper(wallpaper: &Wallpaper) -> Vec<Achievement> {
    match wallpaper {
        Wallpaper::Zero | Wallpaper::One | Wallpaper::Two | Wallpaper::Three | Wallpaper::Four => {
            vec![]
        } // Default wallpapers don't require achievements
        Wallpaper::VolumeKing => vec![Achievement::Volume250M],
        Wallpaper::Streak5 => vec![Achievement::Streak5],
        Wallpaper::SoldierS2 => vec![Achievement::Soldier],
        Wallpaper::SergeantS2 => vec![Achievement::Sergeant],
        Wallpaper::LieutenantS2 => vec![Achievement::Lieutenant],
        Wallpaper::GeneralS2 => vec![Achievement::General],
        Wallpaper::BonkOperative => vec![Achievement::BonkOperative],
        Wallpaper::JitoJuggernaut => vec![Achievement::JitoJuggernaut],
        Wallpaper::Season2Champion => vec![Achievement::Season2Champion],
        Wallpaper::ThePainmaker => vec![Achievement::ThePainmaker],
        Wallpaper::NiceGuy => vec![Achievement::NiceGuy],
    }
}

// Static default collections
pub mod defaults {
    use super::{ProfilePicture, Wallpaper};

    /// Default profile pictures that are always available to all users
    pub static DEFAULT_PFPS: [ProfilePicture; 5] = [
        ProfilePicture::Zero,
        ProfilePicture::One,
        ProfilePicture::Two,
        ProfilePicture::Three,
        ProfilePicture::Four,
    ];

    /// Default wallpapers that are always available to all users
    pub static DEFAULT_WALLPAPERS: [Wallpaper; 5] = [
        Wallpaper::Zero,
        Wallpaper::One,
        Wallpaper::Two,
        Wallpaper::Three,
        Wallpaper::Four,
    ];
}
