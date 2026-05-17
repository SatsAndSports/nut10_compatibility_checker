# Cashu Compatibility Runner Status

## Goal

Build a new standalone tool in this workspace that depends on the local `cdk/` checkout and runs a compatibility suite for Cashu mint implementations.

The immediate target is the CDK mint. Once the runner works there, we will point the same tests at Nutshell and later other mints such as Nutmix.

The output format should be:

- terminal table
- JSON artifact

## Architectural Decision

We are not modifying the existing `cdk/` or `nutshell/` code for the initial runner.

Instead, we will:

- create a new separate tool in this workspace
- depend on `cdk/` by local path
- use that tool to start or target mints and execute compatibility scenarios

This keeps upstream repos clean and makes the runner reusable across multiple implementations.

## Current Understanding

### CDK implementation locations

- `cdk/crates/cashu/src/nuts/nut10/mod.rs`
  - NUT-10 orchestration
  - locktime/refund path logic
  - SIG_ALL group verification flow
- `cdk/crates/cashu/src/nuts/nut10/spending_conditions.rs`
  - `SpendingConditions` and `Conditions`
  - validation and tag conversion
- `cdk/crates/cashu/src/nuts/nut11/mod.rs`
  - P2PK verification
  - SIG_ALL signing and verification
- `cdk/crates/cashu/src/nuts/nut14/mod.rs`
  - HTLC verification
  - receiver and refund path handling
- `cdk/crates/cashu/src/nuts/nut03.rs`
  - swap `SIG_ALL` message construction
- `cdk/crates/cashu/src/nuts/nut05.rs`
  - melt `SIG_ALL` message construction

### CDK test helpers

- `cdk/crates/cdk/src/test_helpers/nut10.rs`
- `cdk/crates/cdk/src/test_helpers/mint.rs`

These provide a good model for funding proofs, constructing blinded outputs, and generating test key material.

### CDK mint spending-condition test files

Swap tests:

- `cdk/crates/cdk/src/mint/swap/tests/p2pk_spending_conditions_tests.rs`
- `cdk/crates/cdk/src/mint/swap/tests/p2pk_sigall_spending_conditions_tests.rs`
- `cdk/crates/cdk/src/mint/swap/tests/htlc_spending_conditions_tests.rs`
- `cdk/crates/cdk/src/mint/swap/tests/htlc_sigall_spending_conditions_tests.rs`

Melt tests:

- `cdk/crates/cdk/src/mint/melt/tests/p2pk_spending_conditions_tests.rs`
- `cdk/crates/cdk/src/mint/melt/tests/p2pk_sigall_spending_conditions_tests.rs`
- `cdk/crates/cdk/src/mint/melt/tests/htlc_spending_conditions_tests.rs`
- `cdk/crates/cdk/src/mint/melt/tests/htlc_sigall_spending_conditions_tests.rs`
- `cdk/crates/cdk/src/mint/melt/tests/locktime_spending_conditions_tests.rs`

### Key behavior to preserve in the runner

- `SIG_ALL` signatures are attached to the first input witness.
- For `SIG_ALL`, all inputs must match on kind, `Secret.data`, and tags.
- P2PK primary path remains valid after locktime; refund path is additive.
- HTLC receiver path remains valid after locktime; refund path is additive.
- After locktime, no refund keys means an anyone-can-spend refund path.

## Local CDK Mint Strategy

Use `cdk-mintd` as a library rather than manually wiring the mint and HTTP server.

Recommended startup path:

- construct `cdk_mintd::config::Settings`
- use fakewallet backend
- use sqlite in a temp work dir
- call `cdk_mintd::run_mintd_with_shutdown(...)`

Zero-fee settings:

- `input_fee_ppk = 0`
- fakewallet `fee_percent = 0.0`
- fakewallet `reserve_fee_min = 0`
- fakewallet delays set to `0`

## Short-Term Goal

Get two or three simple compatibility tests running against a local zero-fee CDK mint.

Initial candidate scenarios:

1. `p2pk_swap_unsigned_fails`
2. `p2pk_swap_signed_succeeds`
3. `htlc_swap_preimage_and_signature_succeeds`

These are intentionally smaller than the full suite and should validate:

- local CDK mint startup
- HTTP wallet/mint interaction
- proof funding
- spending-condition scenario execution
- terminal and JSON reporting

## Planned Runner Shape

Expected components:

- standalone Rust crate in this workspace
- mint target abstraction
  - embedded local CDK mint
  - external HTTP mint target
  - later Nutshell process target
- scenario registry
- terminal table renderer
- JSON report writer

## Next Steps

- create the standalone runner crate
- implement local CDK mint startup and shutdown
- implement report format
- port the first 2-3 simple swap scenarios
- run them against CDK mint
- expand toward the broader CDK NUT-10 matrix

## Decisions Made

- keep `cdk/` unchanged for the runner work
- keep `nutshell/` unchanged for the runner work
- create a separate tool that depends on CDK by path
- produce terminal and JSON outputs
- start with a small number of simple CDK mint scenarios before broadening coverage

## Open Questions

- where exactly to place the new runner crate in this workspace
- JSON schema details for the report output
- whether to keep local mint startup embedded only, or also add process-based startup for parity with non-Rust mints

## Update Policy

Update this file as work progresses:

- record decisions
- record completed steps
- add blockers and discoveries
- keep the short-term goal current
