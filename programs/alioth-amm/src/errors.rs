use anchor_lang::prelude::*;

#[error_code]
pub enum AmmError {
    #[msg("Insufficient liquidity in the pool")]
    InsufficientLiquidity,
    
    #[msg("Slippage tolerance exceeded")]
    SlippageExceeded,
    
    #[msg("Invalid fee parameters")]
    InvalidFeeParameters,
    
    #[msg("Pool is currently paused")]
    PoolPaused,
    
    #[msg("Oracle price is stale")]
    StaleOraclePrice,
    
    #[msg("Oracle price deviation too large")]
    OraclePriceDeviation,
    
    #[msg("Invalid oracle account")]
    InvalidOracle,
    
    #[msg("Math overflow occurred")]
    MathOverflow,
    
    #[msg("Cannot swap zero amount")]
    ZeroAmount,
    
    #[msg("Invalid token ratio")]
    InvalidRatio,
    
    #[msg("Minimum liquidity requirement not met")]
    MinimumLiquidityNotMet,
    
    #[msg("Flash loan not repaid in the same transaction")]
    FlashLoanNotRepaid,
    
    #[msg("Flash loan already repaid")]
    FlashLoanAlreadyRepaid,
    
    #[msg("Invalid flash loan fee")]
    InvalidFlashLoanFee,
    
    #[msg("Unauthorized access")]
    Unauthorized,
    
    #[msg("Invalid time range for TWAP calculation")]
    InvalidTimeRange,
    
    #[msg("Farming pool not active")]
    FarmingNotActive,
    
    #[msg("Farming period has not started yet")]
    FarmingNotStarted,
    
    #[msg("Farming period has ended")]
    FarmingEnded,
    
    #[msg("No rewards to claim")]
    NoRewards,
    
    #[msg("Insufficient staked amount")]
    InsufficientStake,
    
    #[msg("Invalid pool configuration")]
    InvalidPoolConfig,
    
    #[msg("Maximum hops exceeded")]
    MaxHopsExceeded,
    
    #[msg("Invalid swap route")]
    InvalidSwapRoute,
    
    #[msg("Cannot divide by zero")]
    DivisionByZero,
    
    #[msg("Token mint mismatch")]
    TokenMintMismatch,
    
    #[msg("Invalid authority")]
    InvalidAuthority,
    
    #[msg("Numerical overflow in calculation")]
    NumericalOverflow,
}

