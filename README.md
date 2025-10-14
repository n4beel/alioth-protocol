# Alioth Protocol - Advanced Solana AMM

A production-ready Automated Market Maker (AMM) protocol built on Solana with Pyth Network oracle integration, flash loans, yield farming, and multi-hop routing.

## 🌟 Features

### Core AMM Functionality
- **Constant Product Market Maker** (x*y=k formula)
- **Liquidity Pool Management** - Create and manage token pair pools
- **Add/Remove Liquidity** - Provide liquidity and earn LP tokens
- **Token Swaps** - Exchange tokens with minimal slippage
- **Configurable Fees** - Customizable swap fees per pool

### Advanced Features

#### 🔮 Oracle Integration (Pyth Network)
- Real-time price validation for all swaps
- Configurable price staleness checks
- Maximum deviation tolerance (default 5%)
- TWAP (Time-Weighted Average Price) calculations
- Protection against price manipulation

#### ⚡ Flash Loans
- Borrow tokens without collateral
- Must repay in the same transaction
- 0.09% flash loan fee (30% of swap fee)
- Automatic repayment validation
- Perfect for arbitrage and liquidations

#### 🌾 Yield Farming & Staking
- Stake LP tokens to earn rewards
- Flexible reward distribution periods
- Real-time reward calculations
- Multiple farming pools per LP token
- Claim rewards anytime

#### 🔀 Multi-Hop Routing
- Route swaps through up to 3 pools
- Optimal price discovery
- Gas-efficient execution
- Oracle validation at each hop
- Perfect for illiquid token pairs

#### 🔐 Security Features
- Pool pause/unpause mechanism
- Admin access controls
- Comprehensive error handling
- Slippage protection
- Authority transfer capability

## 📁 Project Structure

```
alioth-protocol/
├── programs/
│   └── alioth-amm/
│       └── src/
│           ├── lib.rs                    # Program entry point
│           ├── constants.rs              # Global constants
│           ├── errors.rs                 # Custom error types
│           ├── state/
│           │   ├── mod.rs
│           │   ├── pool.rs              # Pool & LP state
│           │   ├── farming.rs           # Farming pool state
│           │   └── flash_loan.rs        # Flash loan state
│           ├── instructions/
│           │   ├── mod.rs
│           │   ├── initialize_pool.rs   # Pool initialization
│           │   ├── add_liquidity.rs     # Add liquidity
│           │   ├── remove_liquidity.rs  # Remove liquidity
│           │   ├── swap.rs              # Token swaps
│           │   ├── flash_loan.rs        # Flash loans
│           │   ├── farming.rs           # Staking & farming
│           │   ├── multi_hop.rs         # Multi-hop routing
│           │   └── admin.rs             # Admin functions
│           └── utils/
│               ├── mod.rs
│               ├── math.rs              # AMM math utilities
│               └── oracle.rs            # Pyth oracle helpers
├── tests/
│   └── alioth-amm.ts                    # Comprehensive test suite
├── Anchor.toml
├── Cargo.toml
└── package.json
```

## 🚀 Getting Started

### Prerequisites

- Rust 1.70+
- Solana CLI 1.18+
- Anchor Framework 0.30.1
- Node.js 16+
- Yarn or npm

### Installation

1. **Clone the repository**
```bash
git clone https://github.com/yourusername/alioth-protocol.git
cd alioth-protocol
```

2. **Install dependencies**
```bash
yarn install
```

3. **Build the program**
```bash
anchor build
```

4. **Run tests**
```bash
anchor test
```

### Deployment

#### Localnet
```bash
# Start local validator
solana-test-validator

# Deploy
anchor deploy
```

#### Devnet
```bash
# Configure CLI for devnet
solana config set --url devnet

# Airdrop SOL for deployment
solana airdrop 2

# Deploy
anchor deploy --provider.cluster devnet
```

#### Mainnet
```bash
# Configure CLI for mainnet
solana config set --url mainnet-beta

# Deploy (ensure you have enough SOL)
anchor deploy --provider.cluster mainnet
```

## 📖 Usage Examples

### Initialize a Pool

```typescript
const [pool] = PublicKey.findProgramAddressSync(
  [Buffer.from("pool"), tokenAMint.toBuffer(), tokenBMint.toBuffer()],
  program.programId
);

await program.methods
  .initializePool(
    new BN(3),      // 0.3% fee numerator
    new BN(1000),   // fee denominator
    new BN(300),    // 5 min oracle max age
    new BN(500)     // 5% max deviation
  )
  .accounts({
    pool,
    authority: wallet.publicKey,
    tokenAMint,
    tokenBMint,
    lpMint,
    tokenAVault,
    tokenBVault,
    oracleA: pythOracleA,
    oracleB: pythOracleB,
    // ... other accounts
  })
  .rpc();
```

### Add Liquidity

```typescript
await program.methods
  .addLiquidity(
    new BN(100_000_000_000),  // 100 Token A
    new BN(100_000_000_000),  // 100 Token B
    new BN(1)                 // Min LP tokens
  )
  .accounts({
    pool,
    lpProvider,
    user: wallet.publicKey,
    userTokenA,
    userTokenB,
    tokenAVault,
    tokenBVault,
    lpMint,
    userLpToken,
    // ... other accounts
  })
  .rpc();
```

### Swap Tokens

```typescript
await program.methods
  .swap(
    new BN(1_000_000_000),  // 1 Token A
    new BN(990_000_000),    // Min 0.99 Token B out
    true                     // A to B
  )
  .accounts({
    pool,
    user: wallet.publicKey,
    userTokenIn,
    userTokenOut,
    poolTokenIn,
    poolTokenOut,
    oracleA: pythOracleA,
    oracleB: pythOracleB,
    // ... other accounts
  })
  .rpc();
```

### Execute Flash Loan

```typescript
// Flash loan must be executed and repaid in the same transaction
const tx = new Transaction();

tx.add(
  program.instruction.flashLoan(
    new BN(10_000_000_000),  // Borrow 10 Token A
    new BN(10_000_000_000),  // Borrow 10 Token B
    {
      accounts: {
        pool,
        flashLoanRecord,
        borrower: wallet.publicKey,
        // ... other accounts
      }
    }
  )
);

// Add your arbitrage/strategy instructions here

tx.add(
  program.instruction.flashLoanRepay({
    accounts: {
      pool,
      flashLoanRecord,
      borrower: wallet.publicKey,
      // ... other accounts
    }
  })
);

await provider.sendAndConfirm(tx);
```

### Stake LP Tokens

```typescript
await program.methods
  .stake(new BN(50_000_000_000))  // Stake 50 LP tokens
  .accounts({
    pool,
    farmingPool,
    userStake,
    user: wallet.publicKey,
    userLpToken,
    lpTokenVault,
    // ... other accounts
  })
  .rpc();
```

### Multi-Hop Swap

```typescript
await program.methods
  .multiHopSwap(
    new BN(1_000_000_000),   // Input amount
    new BN(950_000_000),     // Min output
    3                        // Number of hops
  )
  .accounts({
    user: wallet.publicKey,
    pool1,
    pool2,
    pool3,
    userTokenIn,
    userTokenOut,
    intermediateToken1,
    intermediateToken2,
    // ... vault and oracle accounts
  })
  .rpc();
```

## 🔒 Security Considerations

### Oracle Integration
- Always use fresh Pyth oracle data
- Configure appropriate staleness thresholds
- Set reasonable deviation limits
- Monitor oracle health

### Flash Loans
- Ensure repayment logic is robust
- Validate all calculations
- Test thoroughly before mainnet
- Consider MEV implications

### Admin Operations
- Use multi-sig for pool authority
- Implement time-locks for critical changes
- Monitor admin actions
- Regular security audits

## 🧪 Testing

The project includes comprehensive tests covering:
- Pool initialization
- Liquidity operations (add/remove)
- Token swaps with oracle validation
- Flash loan execution and repayment
- Farming operations (stake/unstake/claim)
- Multi-hop routing
- Admin functions
- Edge cases and error handling

Run tests with:
```bash
anchor test
```

## 📊 Program Constants

| Constant | Value | Description |
|----------|-------|-------------|
| MINIMUM_LIQUIDITY | 1000 | Minimum liquidity locked forever |
| DEFAULT_FEE | 0.3% | Default swap fee |
| FLASH_LOAN_FEE | 0.09% | Flash loan fee |
| MAX_ORACLE_AGE | 300s | Maximum oracle price age |
| DEFAULT_ORACLE_DEVIATION | 5% | Default price deviation tolerance |
| MAX_SWAP_HOPS | 3 | Maximum multi-hop routes |

## 🛠️ Tech Stack

- **Solana** - High-performance blockchain
- **Anchor** - Solana development framework
- **Pyth Network** - Real-time price oracles
- **SPL Token** - Solana token standard
- **TypeScript** - Testing framework

## 📝 License

MIT License - see LICENSE file for details

## 🤝 Contributing

Contributions are welcome! Please:
1. Fork the repository
2. Create a feature branch
3. Commit your changes
4. Push to the branch
5. Open a Pull Request

## 🔧 Oracle Integration Note

**Important**: The current implementation uses a simplified mock oracle for demonstration and testing purposes. For production deployment:

1. **Install Pyth SDK properly**: 
   ```toml
   pyth-sdk-solana = "0.10.0"
   ```

2. **Replace the mock implementation** in `utils/oracle.rs` with real Pyth oracle calls:
   ```rust
   use pyth_sdk_solana::state::SolanaPriceAccount;
   
   pub fn get_price(oracle_account: &AccountInfo, max_age: i64) -> Result<(i64, u64, i32)> {
       let price_account = SolanaPriceAccount::account_info_to_feed(oracle_account)
           .map_err(|_| AmmError::InvalidOracle)?;
       let price = price_account.get_price_no_older_than(
           &Clock::get()?, 
           max_age as u64
       ).ok_or(AmmError::StaleOraclePrice)?;
       Ok((price.price, price.conf, price.expo))
   }
   ```

3. **Use real Pyth oracle accounts** from [Pyth Network](https://pyth.network/)

## ⚠️ Disclaimer

This software is provided "as is" without warranty. Use at your own risk. Always audit smart contracts before deploying to mainnet with real funds.

## 📧 Contact

For questions and support, please open an issue on GitHub.

---

Built with ❤️ for the Solana ecosystem

# alioth-protocol
