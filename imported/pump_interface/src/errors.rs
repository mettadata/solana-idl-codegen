use solana_program::program_error::ProgramError;
use thiserror::Error;
#[derive(Clone, Copy, Debug, Eq, Error, num_derive::FromPrimitive, PartialEq)]
pub enum PumpError {
    #[error("The given account is not authorized to execute this instruction.")]
    NotAuthorized = 6000,
    #[error("The program is already initialized.")]
    AlreadyInitialized = 6001,
    #[error("slippage: Too much SOL required to buy the given amount of tokens.")]
    TooMuchSolRequired = 6002,
    #[error("slippage: Too little SOL received to sell the given amount of tokens.")]
    TooLittleSolReceived = 6003,
    #[error("The mint does not match the bonding curve.")]
    MintDoesNotMatchBondingCurve = 6004,
    #[error("The bonding curve has completed and liquidity migrated to raydium.")]
    BondingCurveComplete = 6005,
    #[error("The bonding curve has not completed.")]
    BondingCurveNotComplete = 6006,
    #[error("The program is not initialized.")]
    NotInitialized = 6007,
}
impl From<PumpError> for ProgramError {
    fn from(e: PumpError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
