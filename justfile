
generate:
    cargo run -- -i idl/raydium-idl/raydium_amm/idl.json -o generated -m raydium_amm
    cargo run -- -i idl/raydium-idl/raydium_clmm/amm_v3.json -o generated -m raydium_clmm
    cargo run -- -i idl/raydium-idl/raydium_cpmm/raydium_cp_swap.json -o generated -m raydium_cpmm

format:
    rustfmt ./generated/*.rs