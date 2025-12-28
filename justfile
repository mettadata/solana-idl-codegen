# List of generated crates
projects := "raydium_amm raydium_clmm raydium_cpmm pumpfun pumpfun_amm"

# IDL configurations: module_name:idl_path
idls := "raydium_amm:idl/raydium-idl/raydium_amm/idl.json raydium_clmm:idl/raydium-idl/raydium_clmm/amm_v3.json raydium_cpmm:idl/raydium-idl/raydium_cpmm/raydium_cp_swap.json pumpfun:idl/pump-public-docs/idl/pump.json pumpfun_amm:idl/pump-public-docs/idl/pump_amm.json"

clean:
    rm -rf generated
    cargo clean

# Check the main workspace without generating code
check-workspace: clean
    cargo check --all --all-targets --all-features

fmt-fix:
    cargo fmt --all

# Check formatting without modifying files
fmt:
    cargo fmt --all --check

clippy:
    cargo clippy --all --all-targets --all-features -- --deny warnings

clippy-fix:
    cargo clippy --all --all-targets --all-features -- --deny warnings --fix

# Run cargo fmt on all generated crates
fmt-generated:
    #!/usr/bin/env bash
    set -euo pipefail
    for project in {{projects}}; do
        if [ -d "generated/$project" ]; then
            echo "Formatting $project..."
            (cd "generated/$project" && cargo fmt --all --check)
        fi
    done

# Run cargo clippy on all generated crates (excluding examples which are templates)
clippy-generated:
    #!/usr/bin/env bash
    set -euo pipefail
    for project in {{projects}}; do
        if [ -d "generated/$project" ]; then
            echo "Checking $project with clippy..."
            (cd "generated/$project" && cargo clippy --lib --bins --all-features -- --deny warnings)
        fi
    done

# Run both fmt and clippy on all generated crates
lint-generated: generate fmt-generated clippy-generated

generate: clean
    #!/usr/bin/env bash
    set -euo pipefail
    for idl in {{idls}}; do
        module="${idl%%:*}"
        path="${idl#*:}"
        cargo run -- -i "$path" -o generated -m "$module"
    done

# Generate code and validate all generated crates (original behavior of 'check')
check-generated: generate
    #!/usr/bin/env bash
    set -euo pipefail
    for project in {{projects}}; do
        echo "Checking $project..."
        (cd "generated/$project" && cargo check)
    done

build-generated: clean generate
    #!/usr/bin/env bash
    set -euo pipefail
    for project in {{projects}}; do
        echo "Building $project..."
        (cd "generated/$project" && cargo build)
    done

test:
    cargo test -- --test-threads=1

# Run integration tests (requires generated code)
test-integration: generate
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Running integration tests..."
    cargo test --test integration_tests -- --nocapture --test-threads=1

# Run all tests including integration tests
test-all: generate
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Running all tests (sequential to avoid race conditions)..."
    cargo test --all -- --test-threads=1
    echo ""
    echo "✓ All tests passed!"

# Run performance tests
test-perf: generate
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Running performance tests..."
    cargo test --test performance_tests -- --nocapture --test-threads=1

# Run benchmarks
bench:
    cargo bench

# Run all tests with timing information
test-with-timing: generate
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Running all tests with timing information..."
    time cargo test --all -- --test-threads=1 --nocapture

# Run all checks: fmt, clippy, and tests for both codegen and generated code
check-all:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "=== Checking codegen workspace ==="
    echo ""
    echo "1. Checking formatting..."
    just fmt
    echo ""
    echo "2. Running clippy..."
    just clippy
    echo ""
    echo "3. Running unit tests..."
    just test
    echo ""
    echo "=== Checking generated code ==="
    echo ""
    echo "4. Generating code..."
    just generate
    echo ""
    echo "5. Checking formatting on generated code..."
    just fmt-generated
    echo ""
    echo "6. Running clippy on generated code..."
    just clippy-generated
    echo ""
    echo "7. Running integration tests..."
    just test-integration
    echo ""
    echo "✓ All checks passed!"