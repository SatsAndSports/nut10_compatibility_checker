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

Status: complete for the first milestone.

Initial candidate scenarios:

1. `p2pk_swap_unsigned_fails`
2. `p2pk_swap_signed_succeeds`
3. `htlc_swap_preimage_and_signature_succeeds`

Current result:

- all three scenarios are now implemented in `compat-runner/`
- all three pass against an embedded local zero-fee CDK mint
- JSON output is currently written to `compat-runner/compat-report.json`
- the runner has since been expanded to cover the full swap-side CDK spending-condition matrix

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

## Current Implementation

The standalone runner crate is now:

- `compat-runner/`

Current behavior:

- starts a local CDK mint with `cdk-mintd` as a library
- uses sqlite in a temp work directory
- configures zero input fees and zero fakewallet reserve fees
- creates a fresh wallet context per scenario
- prints a terminal results table
- writes a JSON report file
- exits non-zero if any scenario fails
- reports scenario status as `pass`, `fail`, or `skip`
- removes the temporary mint work directory during shutdown
- retries local mint startup across fresh ports
- creates fresh fake BOLT11 invoices for melt scenarios

The first implemented scenarios are:

- `p2pk_swap_unsigned_fails`
- `p2pk_swap_signed_succeeds`
- `htlc_swap_preimage_and_signature_succeeds`

The current swap coverage now includes:

- non-SIG_ALL P2PK scenarios
- non-SIG_ALL HTLC scenarios
- SIG_ALL P2PK scenarios
- SIG_ALL HTLC scenarios
- locktime and refund-path swap scenarios
- mixed-input and tampered-output SIG_ALL negatives
- explicit mixed-data, mixed-kind, and mixed-tags SIG_ALL negatives

The current melt coverage now includes:

- basic P2PK melt scenarios
- basic HTLC melt scenarios
- P2PK SIG_ALL melt scenarios
- HTLC SIG_ALL melt scenarios
- melt locktime and refund-path scenarios

Implemented melt scenarios:

- `melt_p2pk_unsigned_fails`
- `melt_p2pk_signed_succeeds`
- `melt_htlc_preimage_only_fails`
- `melt_htlc_signature_only_fails`
- `melt_htlc_preimage_and_signature_succeeds`
- `melt_p2pk_sigall_unsigned_fails`
- `melt_p2pk_sigall_sig_inputs_fail`
- `melt_p2pk_sigall_transaction_signature_succeeds`
- `melt_htlc_sigall_preimage_only_fails`
- `melt_htlc_sigall_sig_inputs_fail`
- `melt_htlc_sigall_preimage_and_transaction_signature_succeeds`
- `melt_p2pk_post_locktime_anyone_can_spend`
- `melt_p2pk_before_locktime_wrong_key_fails`
- `melt_p2pk_before_locktime_correct_key_succeeds`

Implementation notes for melt:

- melt scenarios are quote-driven
- the runner creates the melt quote first, then funds based on quote requirements
- for the current fakewallet-backed CDK mint, successful melt flows need one extra sat beyond `quote.amount + quote.fee_reserve`
- successful melt flows may return `PENDING` first, so the runner polls quote status until final settlement
- `UNPAID`, `FAILED`, and `UNKNOWN` are treated as terminal non-success states during melt polling
- current melt scenarios are explicitly scoped to fakewallet-backed targets in the runner capability model

Current verification state:

- the expanded swap suite passes against the embedded local CDK mint
- negative scenarios now assert expected error classes/messages rather than accepting any failure
- the melt suite also passes against the embedded local CDK mint
- the review-driven melt polling and target-scoping fixes are implemented and verified

## Next Steps

- add CLI arguments for target selection and report path
- expand toward the broader CDK NUT-10 matrix
- preserve parity notes between runner scenario names and the original CDK test files
- decide whether to rename runner scenarios to match CDK test function names more directly
- decide whether to keep the current split scenario naming or add an alternate reporting layer keyed by exact upstream CDK test names
- start evaluating the same suite against Nutshell
- replace fakewallet-specific melt invoice generation with target-specific invoice setup when moving beyond embedded CDK fakewallet

## Decisions Made

- keep `cdk/` unchanged for the runner work
- keep `nutshell/` unchanged for the runner work
- create a separate tool that depends on CDK by path
- produce terminal and JSON outputs
- start with a small number of simple CDK mint scenarios before broadening coverage
- place the standalone crate at `compat-runner/`
- embed local CDK mint startup through `cdk-mintd::run_mintd_with_shutdown(...)`
- reuse one local mint per runner invocation and create a fresh wallet context per scenario
- complete swap coverage first, then move to melt coverage
- fail the runner process if any scenario fails, even though JSON output is still emitted
- treat negative tests as protocol assertions with expected error matching, not just generic failure detection
- keep the embedded mint zero-input-fee, but make melt funding quote-driven and target-behavior-aware
- keep current melt scenarios explicitly fakewallet-scoped until a portable invoice/payment abstraction exists

## Open Questions

- whether to keep local mint startup embedded only, or also add process-based startup for parity with non-Rust mints

## Update Policy

Update this file as work progresses:

- record decisions
- record completed steps
- add blockers and discoveries
- keep the short-term goal current
