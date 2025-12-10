// Test that generated code compiles and is valid

#[allow(dead_code)]
mod raydium_amm {
    include!("../generated/raydium_amm.rs");
}

#[allow(dead_code)]
mod raydium_clmm {
    include!("../generated/raydium_clmm.rs");
}

#[allow(dead_code)]
mod raydium_cpmm {
    include!("../generated/raydium_cpmm.rs");
}

#[test]
fn test_generated_code_compiles() {
    // Just testing that the code compiles
    // The modules are included above which will cause compilation errors if there are issues
    assert!(true);
}

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

