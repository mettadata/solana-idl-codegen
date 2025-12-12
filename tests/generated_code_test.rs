// Integration tests for generated code
//
// These tests require generated code to be present. Run `just generate` to create them.
//
// Note: These tests are commented out by default because they require generated files.
// Uncomment them after running `just generate` to test the generated code.

/*
#[allow(dead_code)]
mod raydium_amm {
    include!("../generated/raydium_amm/src/lib.rs");
}

#[allow(dead_code)]
mod raydium_clmm {
    include!("../generated/raydium_clmm/src/lib.rs");
}

#[allow(dead_code)]
mod raydium_cpmm {
    include!("../generated/raydium_cpmm/src/lib.rs");
}

// ============================================================================
// Raydium CPMM Tests
// ============================================================================

#[test]
fn test_raydium_cpmm_types() {
    use raydium_cpmm::*;
    use borsh::BorshSerialize;

    // Test that we can instantiate basic types
    let config = AmmConfig {
        bump: 1,
        disable_create_pool: false,
        index: 0,
        trade_fee_rate: 25,
        protocol_fee_rate: 10000,
        fund_fee_rate: 4000,
        create_pool_fee: 1000000000,
        protocol_owner: solana_program::pubkey::Pubkey::new_unique(),
        fund_owner: solana_program::pubkey::Pubkey::new_unique(),
        padding: [0; 16],
    };

    // Test serialization/deserialization with Borsh
    let mut serialized = Vec::new();
    config.serialize(&mut serialized).expect("Should serialize");
    let deserialized: AmmConfig = borsh::BorshDeserialize::try_from_slice(&serialized).expect("Should deserialize");
    assert_eq!(config, deserialized);
}

#[test]
fn test_raydium_cpmm_bytemuck_types() {
    use raydium_cpmm::*;
    use bytemuck::Zeroable;

    // Test bytemuck types
    let observation = Observation {
        block_timestamp: 12345,
        cumulative_token_0_price_x32: 100,
        cumulative_token_1_price_x32: 200,
    };

    // Test that we can use bytemuck functions
    let _zeroed: Observation = Observation::zeroed();
    let bytes = bytemuck::bytes_of(&observation);
    let from_bytes: &Observation = bytemuck::from_bytes(bytes);

    // Test that a bytemuck type can be read (using copy to avoid packed field reference)
    let timestamp = { observation.block_timestamp };
    let from_timestamp = { from_bytes.block_timestamp };
    assert_eq!(timestamp, 12345);
    assert_eq!(from_timestamp, 12345);
}

#[test]
fn test_raydium_cpmm_pool_state() {
    use raydium_cpmm::*;

    // Test creating and working with PoolState
    let pool = PoolState {
        auth_bump: 1,
        status: 1,
        lp_mint_decimals: 9,
        mint_0_decimals: 6,
        mint_1_decimals: 9,
        lp_supply: 1000000,
        protocol_fees_token_0: 100,
        protocol_fees_token_1: 200,
        fund_fees_token_0: 50,
        fund_fees_token_1: 75,
        open_time: 1234567890,
        padding: [0; 32],
        amm_config: solana_program::pubkey::Pubkey::new_unique(),
        pool_creator: solana_program::pubkey::Pubkey::new_unique(),
        token_0_vault: solana_program::pubkey::Pubkey::new_unique(),
        token_1_vault: solana_program::pubkey::Pubkey::new_unique(),
        lp_mint: solana_program::pubkey::Pubkey::new_unique(),
        token_0_mint: solana_program::pubkey::Pubkey::new_unique(),
        token_1_mint: solana_program::pubkey::Pubkey::new_unique(),
        token_0_program: solana_program::pubkey::Pubkey::new_unique(),
        token_1_program: solana_program::pubkey::Pubkey::new_unique(),
        observation_key: solana_program::pubkey::Pubkey::new_unique(),
    };

    // Test serialization
    let mut buffer = Vec::new();
    use borsh::BorshSerialize;
    pool.serialize(&mut buffer).expect("Should serialize");

    // Test deserialization
    use borsh::BorshDeserialize;
    let deserialized = PoolState::try_from_slice(&buffer).expect("Should deserialize");
    assert_eq!(pool, deserialized);
}

#[test]
fn test_raydium_cpmm_instruction_serialization() {
    use raydium_cpmm::*;

    // Test instruction enum serialization
    let init_args = InitializeArgs {
        init_amount_0: 1000,
        init_amount_1: 2000,
        open_time: 1234567890,
    };

    let instruction = Instruction::Initialize(init_args);

    // Test serialization
    let mut buffer = Vec::new();
    instruction.serialize(&mut buffer).expect("Should serialize instruction");

    // Buffer should contain discriminator + args
    assert!(buffer.len() > 8, "Buffer should contain discriminator and args");
}

#[test]
fn test_raydium_cpmm_instruction_deserialization() {
    use raydium_cpmm::*;
    use borsh::BorshSerialize;

    // Create instruction data with discriminator
    let init_args = InitializeArgs {
        init_amount_0: 1000,
        init_amount_1: 2000,
        open_time: 1234567890,
    };

    let instruction = Instruction::Initialize(init_args.clone());

    // Serialize
    let mut buffer = Vec::new();
    instruction.serialize(&mut buffer).expect("Should serialize");

    // Deserialize
    let deserialized = Instruction::try_from_slice(&buffer).expect("Should deserialize");

    match deserialized {
        Instruction::Initialize(args) => {
            assert_eq!(args.init_amount_0, 1000);
            assert_eq!(args.init_amount_1, 2000);
            assert_eq!(args.open_time, 1234567890);
        }
        _ => panic!("Expected Initialize instruction"),
    }
}

#[test]
fn test_raydium_cpmm_discriminators() {
    use raydium_cpmm::*;

    // Test that discriminators are properly defined
    assert_eq!(PoolState::DISCRIMINATOR.len(), 8);
    assert_eq!(AmmConfig::DISCRIMINATOR.len(), 8);

    // Discriminators should be unique (not all zeros)
    assert_ne!(PoolState::DISCRIMINATOR, [0; 8]);
    assert_ne!(AmmConfig::DISCRIMINATOR, [0; 8]);
}

#[test]
fn test_raydium_cpmm_try_from_slice_with_discriminator() {
    use raydium_cpmm::*;
    use borsh::BorshSerialize;

    // Create a PoolState
    let pool = PoolState {
        auth_bump: 1,
        status: 1,
        lp_mint_decimals: 9,
        mint_0_decimals: 6,
        mint_1_decimals: 9,
        lp_supply: 1000000,
        protocol_fees_token_0: 100,
        protocol_fees_token_1: 200,
        fund_fees_token_0: 50,
        fund_fees_token_1: 75,
        open_time: 1234567890,
        padding: [0; 32],
        amm_config: solana_program::pubkey::Pubkey::new_unique(),
        pool_creator: solana_program::pubkey::Pubkey::new_unique(),
        token_0_vault: solana_program::pubkey::Pubkey::new_unique(),
        token_1_vault: solana_program::pubkey::Pubkey::new_unique(),
        lp_mint: solana_program::pubkey::Pubkey::new_unique(),
        token_0_mint: solana_program::pubkey::Pubkey::new_unique(),
        token_1_mint: solana_program::pubkey::Pubkey::new_unique(),
        token_0_program: solana_program::pubkey::Pubkey::new_unique(),
        token_1_program: solana_program::pubkey::Pubkey::new_unique(),
        observation_key: solana_program::pubkey::Pubkey::new_unique(),
    };

    // Serialize with discriminator
    let mut buffer = Vec::new();
    pool.serialize_with_discriminator(&mut buffer)
        .expect("Should serialize with discriminator");

    // Should start with discriminator
    assert_eq!(&buffer[..8], &PoolState::DISCRIMINATOR);

    // Deserialize with discriminator
    let deserialized = PoolState::try_from_slice_with_discriminator(&buffer)
        .expect("Should deserialize with discriminator");

    assert_eq!(pool, deserialized);
}

#[test]
fn test_raydium_cpmm_wrong_discriminator_fails() {
    use raydium_cpmm::*;

    // Create data with wrong discriminator
    let mut buffer = vec![0; 100];
    buffer[..8].copy_from_slice(&[99, 99, 99, 99, 99, 99, 99, 99]);

    // Should fail with wrong discriminator
    let result = PoolState::try_from_slice_with_discriminator(&buffer);
    assert!(result.is_err(), "Should fail with invalid discriminator");
}

#[test]
fn test_raydium_cpmm_short_data_fails() {
    use raydium_cpmm::*;

    // Create data that's too short for discriminator
    let buffer = vec![1, 2, 3];

    let result = PoolState::try_from_slice_with_discriminator(&buffer);
    assert!(result.is_err(), "Should fail with data too short");
}

// ============================================================================
// Raydium AMM Tests
// ============================================================================

#[test]
fn test_raydium_amm_target_orders() {
    use raydium_amm::*;

    // Test TargetOrders type
    let target_orders = TargetOrders {
        owner: [1u64; 32],
        buy_orders: [2u128; 50],
        padding1: [0u64; 8],
        target_x: 1000,
        target_y: 2000,
        plan_x_buy: 100,
        plan_y_buy: 200,
        plan_x_sell: 150,
        plan_y_sell: 250,
        placed_x: 50,
        placed_y: 75,
        calc_pnl_x: 10,
        calc_pnl_y: 20,
        sell_orders: [3u128; 50],
        padding2: [0u64; 6],
        replace_buy_client_id: [4u64; 10],
        replace_sell_client_id: [5u64; 10],
        last_order_numerator: 100,
        last_order_denominator: 200,
        plan_orders_cur: 1,
        place_orders_cur: 2,
        valid_buy_order_num: 3,
        valid_sell_order_num: 4,
        padding3: [0u64; 10],
        free_slot_bits: 12345,
    };

    // Test bytemuck operations
    let bytes = bytemuck::bytes_of(&target_orders);
    let from_bytes: &TargetOrders = bytemuck::from_bytes(bytes);

    assert_eq!({ from_bytes.target_x }, 1000);
    assert_eq!({ from_bytes.target_y }, 2000);
}

// ============================================================================
// Cross-Module Tests
// ============================================================================

#[test]
fn test_all_modules_have_instruction_enum() {
    // Verify that all modules have an Instruction enum
    use raydium_cpmm::Instruction as CpmmInstruction;
    use raydium_amm::Instruction as AmmInstruction;
    use raydium_clmm::Instruction as ClmmInstruction;

    // These should compile if the enums exist
    let _cpmm: Option<CpmmInstruction> = None;
    let _amm: Option<AmmInstruction> = None;
    let _clmm: Option<ClmmInstruction> = None;
}

#[test]
fn test_debug_trait_implementation() {
    use raydium_cpmm::*;

    // Test that Debug is implemented
    let config = AmmConfig {
        bump: 1,
        disable_create_pool: false,
        index: 0,
        trade_fee_rate: 25,
        protocol_fee_rate: 10000,
        fund_fee_rate: 4000,
        create_pool_fee: 1000000000,
        protocol_owner: solana_program::pubkey::Pubkey::new_unique(),
        fund_owner: solana_program::pubkey::Pubkey::new_unique(),
        padding: [0; 16],
    };

    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("AmmConfig"));
}

#[test]
fn test_clone_trait_implementation() {
    use raydium_cpmm::*;

    let config = AmmConfig {
        bump: 1,
        disable_create_pool: false,
        index: 0,
        trade_fee_rate: 25,
        protocol_fee_rate: 10000,
        fund_fee_rate: 4000,
        create_pool_fee: 1000000000,
        protocol_owner: solana_program::pubkey::Pubkey::new_unique(),
        fund_owner: solana_program::pubkey::Pubkey::new_unique(),
        padding: [0; 16],
    };

    let cloned = config.clone();
    assert_eq!(config, cloned);
}

#[test]
fn test_partial_eq_trait_implementation() {
    use raydium_cpmm::*;

    let pubkey = solana_program::pubkey::Pubkey::new_unique();

    let config1 = AmmConfig {
        bump: 1,
        disable_create_pool: false,
        index: 0,
        trade_fee_rate: 25,
        protocol_fee_rate: 10000,
        fund_fee_rate: 4000,
        create_pool_fee: 1000000000,
        protocol_owner: pubkey,
        fund_owner: pubkey,
        padding: [0; 16],
    };

    let config2 = AmmConfig {
        bump: 1,
        disable_create_pool: false,
        index: 0,
        trade_fee_rate: 25,
        protocol_fee_rate: 10000,
        fund_fee_rate: 4000,
        create_pool_fee: 1000000000,
        protocol_owner: pubkey,
        fund_owner: pubkey,
        padding: [0; 16],
    };

    assert_eq!(config1, config2);
}
*/

#[test]
fn test_placeholder() {
    // This is a placeholder test so that the test suite runs even without generated code
    // To run the full integration tests, run `just generate` first, then uncomment the tests above
    assert!(
        true,
        "Integration tests require generated code. Run `just generate` first."
    );
}
