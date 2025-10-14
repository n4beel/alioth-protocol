/// Minimum liquidity that must be locked forever in the pool
pub const MINIMUM_LIQUIDITY: u64 = 1000;

/// Default swap fee numerator (0.3%)
pub const DEFAULT_FEE_NUMERATOR: u64 = 3;

/// Default swap fee denominator (0.3%)
pub const DEFAULT_FEE_DENOMINATOR: u64 = 1000;

/// Flash loan fee numerator (0.09% - 30% of swap fee)
pub const FLASH_LOAN_FEE_NUMERATOR: u64 = 9;

/// Flash loan fee denominator
pub const FLASH_LOAN_FEE_DENOMINATOR: u64 = 10000;

/// Maximum oracle price age in seconds (5 minutes)
pub const MAX_ORACLE_AGE: i64 = 300;

/// Default oracle deviation tolerance in basis points (500 bps = 5%)
pub const DEFAULT_ORACLE_DEVIATION_BPS: u64 = 500;

/// Maximum basis points (100%)
pub const MAX_BPS: u64 = 10000;

/// Maximum number of hops in multi-hop swap
pub const MAX_SWAP_HOPS: u8 = 3;

/// Precision for price calculations
pub const PRICE_PRECISION: u128 = 1_000_000_000; // 10^9

/// Precision for reward calculations
pub const REWARD_PRECISION: u128 = 1_000_000_000_000; // 10^12

/// Minimum time window for TWAP (1 minute)
pub const MIN_TWAP_WINDOW: i64 = 60;

/// Pool seed prefix
pub const POOL_SEED: &[u8] = b"pool";

/// LP mint seed prefix
pub const LP_MINT_SEED: &[u8] = b"lp_mint";

/// Token A vault seed prefix
pub const TOKEN_A_VAULT_SEED: &[u8] = b"token_a_vault";

/// Token B vault seed prefix
pub const TOKEN_B_VAULT_SEED: &[u8] = b"token_b_vault";

/// Liquidity provider seed prefix
pub const LP_PROVIDER_SEED: &[u8] = b"lp_provider";

/// Farming pool seed prefix
pub const FARMING_POOL_SEED: &[u8] = b"farming_pool";

/// Reward vault seed prefix
pub const REWARD_VAULT_SEED: &[u8] = b"reward_vault";

/// User stake seed prefix
pub const USER_STAKE_SEED: &[u8] = b"user_stake";

/// Flash loan record seed prefix
pub const FLASH_LOAN_SEED: &[u8] = b"flash_loan";

/// Minimum farming duration in slots (approximately 1 hour at 400ms per slot)
pub const MIN_FARMING_DURATION: u64 = 9000;

/// Maximum farming duration in slots (approximately 30 days)
pub const MAX_FARMING_DURATION: u64 = 6_480_000;

