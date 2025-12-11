clean:
    rm -rf generated
    cargo clean

generate: clean
    cargo run -- -i idl/raydium-idl/raydium_amm/idl.json -o generated -m raydium_amm
    cargo run -- -i idl/raydium-idl/raydium_clmm/amm_v3.json -o generated -m raydium_clmm
    cargo run -- -i idl/raydium-idl/raydium_cpmm/raydium_cp_swap.json -o generated -m raydium_cpmm
    cargo run -- -i idl/pump-public-docs/idl/pump.json -o generated -m pumpfun
    cargo run -- -i idl/pump-public-docs/idl/pump_amm.json -o generated -m pumpfun_amm

check: generate
    cd generated/raydium_amm && cargo check
    cd generated/raydium_clmm && cargo check
    cd generated/raydium_cpmm && cargo check
    cd generated/pumpfun && cargo check
    cd generated/pumpfun_amm && cargo check

build:
    cd generated/raydium_amm && cargo build
    cd generated/raydium_clmm && cargo build
    cd generated/raydium_cpmm && cargo build
    cd generated/pumpfun && cargo build
    cd generated/pumpfun_amm && cargo build

test:
    cargo test 