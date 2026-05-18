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
- supports external mint URL mode with suite filtering
- supports `SIG_ALL` signing modes: `standard` and `legacy`

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
- external target mode works against a running Nutshell mint for swap execution
- external proof funding now uses explicit HTTP quote polling and minting instead of websocket-driven proof streaming
- current Nutshell swap results show broad non-SIG_ALL compatibility, with remaining failures concentrated around SIG_ALL behavior
- legacy `SIG_ALL` mode significantly improves Nutshell swap compatibility versus standard mode
- external target mode also works against a running Nutmix mint for swap execution
- current Nutmix swap results suggest stronger alignment with `standard` SIG_ALL mode than with `legacy`
- the current harness is likely still too strict on external negative-case error messages, which inflates apparent failure counts for mints that return generic errors

## Nutshell Analysis

Current external Nutshell summary:

- non-SIG_ALL swap scenarios broadly pass
- `SIG_ALL` legacy mode materially improves results compared to `standard`
- the remaining failing scenarios are a smaller set concentrated around `SIG_ALL`

Likely causes of the remaining legacy-mode failures:

- Nutshell appears to verify the older SIG_ALL message format for swap: concatenated `secret` values and output `B_` values, without `C` or output `amount`
- Nutshell's SIG_ALL P2PK verification appears to switch to refund pubkeys after locktime instead of keeping the primary path additive after expiry
- Nutshell's HTLC verification checks per-proof witness/preimage presence before shared SIG_ALL validation, which conflicts with spec-compliant first-input-only SIG_ALL witness placement
- the output-amount tamper case still succeeds in legacy mode, which is consistent with the older SIG_ALL format not binding output amounts

Interpretation:

- the legacy SIG_ALL option is useful as a diagnostic and interoperability mode
- the remaining failing Nutshell scenarios should currently be treated as likely real compatibility gaps, not runner setup failures

## Nutmix Analysis

Current external Nutmix summary:

- non-SIG_ALL positive swap scenarios broadly pass
- `standard` SIG_ALL mode performs better than `legacy`
- many negative cases fail for the expected behavioral reason but return generic errors such as `Token not verified` or HTTP `400` with `{"code":99999}`

Interpretation:

- Nutmix appears closer to CDK/spec SIG_ALL semantics than to the older legacy Nutshell format
- the raw failure count overstates the true compatibility gap because the runner still expects more specific error messages than Nutmix provides for many negative tests
- after normalizing generic external negative-case errors, the remaining meaningful Nutmix failures are likely concentrated in a smaller subset, especially some locktime/refund-path cases and a few HTLC SIG_ALL cases

Harness caveat for external targets:

- for external mints, the current harness can be too strict about exact failure text
- generic rejections such as `Token not verified` or opaque error codes may still represent correct negative behavior
- this means external-target negative results should be interpreted carefully until error expectation matching is widened or target-specific

## Next Steps

- expand toward the broader CDK NUT-10 matrix
- preserve parity notes between runner scenario names and the original CDK test files
- decide whether to rename runner scenarios to match CDK test function names more directly
- decide whether to keep the current split scenario naming or add an alternate reporting layer keyed by exact upstream CDK test names
- decide whether to keep the remaining Nutshell SIG_ALL failures as explicit compatibility failures or add additional diagnostic target-specific modes
- investigate whether a Nutshell-specific HTLC SIG_ALL witness-broadcast experiment is useful for diagnosis only
- reduce false negatives for external targets by relaxing or target-scoping expected error matching for negative cases
- rerun Nutmix after that normalization to isolate the smaller set of real compatibility gaps
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
- keep `standard` as the default `SIG_ALL` mode and expose `legacy` as an explicit interoperability option
- keep the current spec-compliant first-input-only SIG_ALL witness placement unchanged in the runner
- accept that some external mints currently need broader negative-case error matching before raw pass/fail counts become fully comparable

## Open Questions

- whether to keep local mint startup embedded only, or also add process-based startup for parity with non-Rust mints

## Update Policy

Update this file as work progresses:

- record decisions
- record completed steps
- add blockers and discoveries
- keep the short-term goal current
