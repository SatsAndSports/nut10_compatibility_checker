# Cashu Compatibility Runner

Standalone compatibility runner for Cashu mint implementations.

Current targets:

- embedded local CDK mint
- external mint URL mode in progress
- first external target: Nutshell

Current output:

- terminal table
- JSON report

The runner lives in:

- `compat-runner/`

Project tracking lives in:

- `STATUS.md`

## Current Status

Implemented and verified against the embedded local CDK mint:

- full swap spending-condition coverage
- full melt spending-condition coverage

The suite currently covers:

- P2PK
- HTLC
- SIG_ALL
- locktime and refund-path behavior

Important current limitation:

- melt scenarios are currently fakewallet-scoped in the runner design
- swap scenarios are the first priority for external mint interoperability checks

## Repository Layout

- `compat-runner/`
  Standalone Rust runner crate
- `cdk/`
  Upstream CDK checkout
- `nutshell/`
  Upstream Nutshell checkout
- `STATUS.md`
  Current progress, decisions, and caveats
- `AGENTS.md`
  Project instructions

## Running The Embedded CDK Suite

From the workspace root:

```bash
cd compat-runner
cargo run
```

This will:

- start an embedded zero-fee local CDK mint
- run the current suite
- print a terminal table
- write JSON to `compat-runner/compat-report.json`

You can also build-check first:

```bash
cd compat-runner
cargo check
```

## Running Nutshell Locally

The first external mint target is a local Nutshell mint.

### 1. Install dependencies

From `nutshell/`:

```bash
poetry install
```

Note:

- if Poetry creates a Python 3.13 environment and Nutshell has Python-version issues, switch to the Python version expected by Nutshell before reinstalling

### 2. Create `.env`

Create `nutshell/.env` with these settings:

```dotenv
DEBUG=FALSE
CASHU_DIR=~/.cashu

# --------- WALLET ---------

MINT_HOST=127.0.0.1
MINT_PORT=3339
TOR=FALSE
API_PORT=4448
WALLET_UNIT="sat"

# --------- MINT ---------

MINT_LISTEN_HOST=127.0.0.1
MINT_LISTEN_PORT=3339

MINT_INFO_NAME="Local Nutshell Test Mint"
MINT_INFO_DESCRIPTION="Zero-fee local fakewallet mint for compatibility testing"

MINT_PRIVATE_KEY=TEST_PRIVATE_KEY_FOR_LOCAL_DEV
MINT_DERIVATION_PATH="m/0'/0'/0'"

MINT_INPUT_FEE_PPK=0
MINT_DATABASE=data/mint

MINT_BACKEND_BOLT11_SAT=FakeWallet

LIGHTNING_FEE_PERCENT=0
LIGHTNING_RESERVE_FEE_MIN=0

FAKEWALLET_BRR=TRUE
FAKEWALLET_DELAY_INCOMING_PAYMENT=0
FAKEWALLET_DELAY_OUTGOING_PAYMENT=0
```

This repo already has a working local file at:

- `nutshell/.env`

### 3. Start the mint

```bash
cd nutshell
poetry run mint
```

### 4. Verify that it is up

```bash
curl localhost:3339/v1/info
```

You should see `nuts` support including:

- `10`
- `11`
- `14`

## External Mint Mode

Current runner CLI shape:

```bash
cd compat-runner
cargo run -- --mint-url http://127.0.0.1:3339 --target-name nutshell --suite swap
```

CLI behavior:

- `cargo run`
  default embedded CDK mode
- `--mint-url`
  external mint mode
- `--target-name`
  label for reports
- `--suite swap|melt|all`
  suite selection
- `--sigall-mode standard|legacy`
  choose CDK/spec `SIG_ALL` signing or legacy Nutshell-style signing

Examples:

```bash
# embedded CDK, full suite
cd compat-runner
cargo run
```

```bash
# embedded CDK, swap only
cd compat-runner
cargo run -- --suite swap
```

```bash
# external Nutshell, swap only
cd compat-runner
cargo run -- --mint-url http://127.0.0.1:3339 --target-name nutshell --suite swap
```

```bash
# external Nutshell, swap only, legacy SIG_ALL mode
cd compat-runner
cargo run -- --mint-url http://127.0.0.1:3339 --target-name nutshell --suite swap --sigall-mode legacy
```

Current external target notes:

- start with `swap`
- keep `melt` fakewallet-scoped until invoice/payment setup is abstracted by target

Current Nutshell status:

- external proof funding now works through explicit HTTP quote polling and minting
- non-SIG_ALL swap scenarios largely pass
- `--sigall-mode legacy` improves Nutshell compatibility substantially compared to the default `standard` mode
- remaining failures in legacy mode are concentrated in a smaller SIG_ALL subset, especially HTLC SIG_ALL and some post-locktime/tamper cases

Current interpretation of the remaining Nutshell legacy-mode failures:

- P2PK SIG_ALL mostly improves under legacy mode, which strongly suggests Nutshell still verifies the older SIG_ALL message format
- some post-locktime P2PK SIG_ALL scenarios still fail, which appears to be related to Nutshell preferring the refund path after locktime instead of keeping the primary path additive
- several HTLC SIG_ALL scenarios still fail, and Nutshell's verification path appears to require per-proof HTLC witness/preimage presence before aggregate SIG_ALL validation
- the output-amount tamper case still succeeds in legacy mode, which is consistent with the older SIG_ALL message format not binding output amounts

About `SIG_ALL` modes:

- `standard`
  Uses the current CDK/spec-style aggregated message construction.
- `legacy`
  Uses the older Nutshell-style aggregated message construction for runner-side `SIG_ALL` signatures.

The `legacy` mode exists as a diagnostic and interoperability mode. The default remains `standard`.

## Notes

- The runner does not modify `cdk/` or `nutshell/`.
- For melt scenarios on the embedded CDK fakewallet mint, the runner is quote-driven.
- Some upstream CDK tests are intentionally split into multiple runner scenarios for clearer reporting.

## Next Steps

- investigate the remaining Nutshell SIG_ALL failures in more detail
- decide whether any additional target-specific diagnostic modes are worthwhile
- later generalize melt for non-fakewallet targets
