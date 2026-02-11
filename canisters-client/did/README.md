# Canister IDs

This directory contains the Candid interface definitions (`.did` files) and canister ID mappings for all canisters used in the yral ecosystem.

## Canister ID Configuration

The `canister_ids.json` file maps canister names to their Principal IDs for both IC mainnet (`ic`) and local development (`local`) environments.

### Daily Missions Canister

The `daily_missions` canister IDs in `canister_ids.json` are currently **placeholder values** borrowed from existing canisters:

- **IC**: Uses the same ID as `notification_store` 
- **Local**: Uses a generic local canister ID

**⚠️ Important**: Before deploying the daily missions canister to production, you must:

1. Deploy the actual daily missions canister to both local and IC networks
2. Replace the placeholder Principal IDs in `canister_ids.json` with the real deployed canister IDs
3. Update any downstream services that depend on these IDs

## Adding New Canisters

To add a new canister:

1. Create the `.did` file with the canister's Candid interface
2. Add the canister name to the `DID_WHITELIST` in `build.rs` with appropriate feature flag
3. Add the feature flag to `Cargo.toml`
4. Add the canister IDs to `canister_ids.json`
5. Deploy the canister and update with real Principal IDs

## Build System

The `build.rs` script processes these files during compilation:

- Parses `.did` files for canisters enabled by feature flags
- Generates Rust client code for canister interactions
- Includes retry logic for robust canister communication
- Generates constants for canister Principal IDs