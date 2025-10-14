use anchor_lang::prelude::*;

/// Flash loan state - tracks active flash loans in a transaction
#[account]
#[derive(Default)]
pub struct FlashLoanRecord {
    /// Pool the flash loan is from
    pub pool: Pubkey,
    
    /// Borrower's authority
    pub borrower: Pubkey,
    
    /// Amount of token A borrowed
    pub amount_a_borrowed: u64,
    
    /// Amount of token B borrowed
    pub amount_b_borrowed: u64,
    
    /// Fee for token A (must be repaid in addition to principal)
    pub fee_a: u64,
    
    /// Fee for token B (must be repaid in addition to principal)
    pub fee_b: u64,
    
    /// Slot when the flash loan was initiated
    pub initiated_slot: u64,
    
    /// Whether the flash loan has been repaid
    pub is_repaid: bool,
    
    /// Bump seed
    pub bump: u8,
}

impl FlashLoanRecord {
    pub const LEN: usize = 8 + // discriminator
        32 + // pool
        32 + // borrower
        8 + // amount_a_borrowed
        8 + // amount_b_borrowed
        8 + // fee_a
        8 + // fee_b
        8 + // initiated_slot
        1 + // is_repaid
        1; // bump

    /// Calculate total amount to be repaid for token A
    pub fn total_repay_a(&self) -> u64 {
        self.amount_a_borrowed
            .checked_add(self.fee_a)
            .unwrap()
    }

    /// Calculate total amount to be repaid for token B
    pub fn total_repay_b(&self) -> u64 {
        self.amount_b_borrowed
            .checked_add(self.fee_b)
            .unwrap()
    }
}

