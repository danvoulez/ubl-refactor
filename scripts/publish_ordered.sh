#!/bin/bash
# UBL Autopoietic Publishing Script
# Publishes crates in correct dependency order using topological sorting

set -euo pipefail

echo "🚀 Starting UBL crate publishing in dependency order..."

# Define the workspace crates in dependency order
# Lower-level crates (no internal dependencies) first
CRATE_ORDER=(
    "ubl_config"
    "ubl_did"
    "ubl_receipt"
    "ubl_vm"
    "ubl_ledger"
    "ubl_nrf"
    "ubl_chipstore"
    "ubl_runtime"
    "ubl_cli"
    "ubl_gate"
)

# Function to check if crate needs publishing
needs_publishing() {
    local crate_path="$1"
    local crate_name="$2"

    cd "$crate_path" || return 1

    # Get local version
    local local_version
    local_version=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')

    if [[ -z "$local_version" ]]; then
        echo "Warning: Could not determine version for $crate_name"
        return 1
    fi

    # Check if version exists on crates.io
    echo "Checking if $crate_name v$local_version exists on crates.io..."

    if cargo search "$crate_name" --limit 1 | grep -q "\"$local_version\""; then
        echo "✅ $crate_name v$local_version already exists on crates.io"
        return 1
    else
        echo "📦 $crate_name v$local_version needs to be published"
        return 0
    fi
}

# Function to publish a single crate
publish_crate() {
    local crate_path="$1"
    local crate_name="$2"

    echo ""
    echo "📦 Publishing $crate_name..."
    echo "==================================="

    cd "$crate_path" || {
        echo "❌ Failed to cd to $crate_path"
        return 1
    }

    # Verify the crate builds
    echo "🔨 Building $crate_name..."
    if ! cargo build --release; then
        echo "❌ Build failed for $crate_name"
        return 1
    fi

    # Run tests
    echo "🧪 Testing $crate_name..."
    if ! cargo test; then
        echo "❌ Tests failed for $crate_name"
        return 1
    fi

    # Check if already published
    if ! needs_publishing "$crate_path" "$crate_name"; then
        echo "⏭️  Skipping $crate_name (already published)"
        return 0
    fi

    # Publish with retry logic
    local attempts=0
    local max_attempts=3

    while [[ $attempts -lt $max_attempts ]]; do
        echo "📤 Publishing attempt $((attempts + 1))/$max_attempts for $crate_name..."

        if cargo publish --no-verify; then
            echo "✅ Successfully published $crate_name"

            # Wait for crates.io to update (important for dependent crates)
            echo "⏳ Waiting 30 seconds for crates.io to update..."
            sleep 30

            return 0
        else
            attempts=$((attempts + 1))
            if [[ $attempts -lt $max_attempts ]]; then
                echo "⚠️  Publish failed, retrying in 10 seconds..."
                sleep 10
            else
                echo "❌ Failed to publish $crate_name after $max_attempts attempts"
                return 1
            fi
        fi
    done
}

# Main publishing logic
main() {
    local workspace_root
    workspace_root=$(pwd)
    local published_count=0
    local failed_crates=()

    echo "🏠 Workspace root: $workspace_root"
    echo "📋 Publishing order: ${CRATE_ORDER[*]}"
    echo ""

    # Verify CARGO_REGISTRY_TOKEN is set
    if [[ -z "${CARGO_REGISTRY_TOKEN:-}" ]]; then
        echo "❌ CARGO_REGISTRY_TOKEN environment variable is not set"
        echo "Please set it to your crates.io API token"
        exit 1
    fi

    # Process each crate in order
    for crate_name in "${CRATE_ORDER[@]}"; do
        local crate_path

        # Determine crate path
        if [[ "$crate_name" == "ubl_gate" ]]; then
            crate_path="$workspace_root/services/$crate_name"
        else
            crate_path="$workspace_root/crates/$crate_name"
        fi

        if [[ ! -d "$crate_path" ]]; then
            echo "⚠️  Crate directory not found: $crate_path"
            failed_crates+=("$crate_name")
            continue
        fi

        if publish_crate "$crate_path" "$crate_name"; then
            published_count=$((published_count + 1))
        else
            failed_crates+=("$crate_name")
        fi

        cd "$workspace_root"
    done

    # Summary
    echo ""
    echo "📊 PUBLISHING SUMMARY"
    echo "===================="
    echo "✅ Successfully published: $published_count crates"

    if [[ ${#failed_crates[@]} -gt 0 ]]; then
        echo "❌ Failed to publish: ${failed_crates[*]}"
        exit 1
    else
        echo "🎉 All crates published successfully!"
    fi
}

# Dry run mode
if [[ "${1:-}" == "--dry-run" ]]; then
    echo "🔍 DRY RUN MODE - No actual publishing will occur"
    echo ""

    for crate_name in "${CRATE_ORDER[@]}"; do
        crate_path=""

        if [[ "$crate_name" == "ubl_gate" ]]; then
            crate_path="./services/$crate_name"
        else
            crate_path="./crates/$crate_name"
        fi

        if [[ -d "$crate_path" ]]; then
            if needs_publishing "$crate_path" "$crate_name"; then
                echo "📦 Would publish: $crate_name"
            fi
        else
            echo "⚠️  Missing: $crate_path"
        fi
    done

    exit 0
fi

# Run main function
main "$@"