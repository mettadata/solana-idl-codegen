# List of generated crates
projects := "raydium_amm raydium_clmm raydium_cpmm pumpfun pumpfun_amm"

# IDL configurations: module_name:idl_path
idls := "raydium_amm:idl/raydium-idl/raydium_amm/idl.json raydium_clmm:idl/raydium-idl/raydium_clmm/amm_v3.json raydium_cpmm:idl/raydium-idl/raydium_cpmm/raydium_cp_swap.json pumpfun:idl/pump-public-docs/idl/pump.json pumpfun_amm:idl/pump-public-docs/idl/pump_amm.json"

clean:
    rm -rf generated
    cargo clean

generate: clean
    #!/usr/bin/env bash
    set -euo pipefail
    for idl in {{idls}}; do
        module="${idl%%:*}"
        path="${idl#*:}"
        cargo run -- -i "$path" -o generated -m "$module"
    done

check: generate
    #!/usr/bin/env bash
    set -euo pipefail
    for project in {{projects}}; do
        echo "Checking $project..."
        (cd "generated/$project" && cargo check)
    done

build:
    #!/usr/bin/env bash
    set -euo pipefail
    for project in {{projects}}; do
        echo "Building $project..."
        (cd "generated/$project" && cargo build)
    done

test:
    cargo test

# Run integration tests (requires generated code)
test-integration: generate
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Running integration tests..."
    cargo test --test integration_tests -- --nocapture

# Run all tests including integration tests
test-all: generate
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Running unit tests..."
    cargo test 
    echo ""
    echo "Running integration tests..."
    cargo test --test integration_tests -- --nocapture
    echo ""
    echo "✓ All tests passed!"

# Run performance tests
test-perf: generate
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Running performance tests..."
    cargo test --test performance_tests -- --nocapture

# Run benchmarks
bench:
    cargo bench

# Run all tests with timing information
test-with-timing: generate
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Running all tests with timing information..."
    time cargo test --test integration_tests -- --nocapture
    echo ""
    cargo test --test performance_tests -- --nocapture