use crate::domain::rarity_code::RarityCode;

pub const MAX_KEPT_COPIES: u8 = 4;

#[derive(Debug)]
pub struct RarityTradeFilter {
    pub rarity: RarityCode,
    pub is_open: bool,
    pub kept_copies: u8,
    pub copies: u64,
    pub proposed: u64,
}

pub struct RarityTradeFilterRule {
    pub rarity: RarityCode,
    pub is_open: bool,
    pub kept_copies: u8,
}
