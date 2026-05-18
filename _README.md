# Cashu Compatibility Runner

Compatibility runner for Cashu spending-condition behavior across `cdk-mintd`, `Nutshell`, and `Nutmix`.

This repo publishes tracked compatibility results for swap and melt scenarios covering:

- P2PK
- HTLC
- `SIG_ALL`
- locktime and refund behavior

{{RESULTS_TABLES}}

## What This Repo Contains

- `compat-runner/`
  Standalone Rust compatibility runner
- `reports/`
  Tracked JSON reports used to generate the top-of-README results
- `cdk/`
  Local upstream CDK checkout used by the runner
- `nutshell/`
  Local upstream Nutshell checkout used for local testing

The runner stays separate from the upstream mint repositories.

## Running The Runner

From the workspace root:

```bash
cd compat-runner
cargo run
```

This starts an embedded zero-fee local `cdk-mintd` mint and runs the full suite.

Useful variants:

```bash
# embedded CDK
cd compat-runner
cargo run -- --report-name cdk
```

```bash
# Nutshell, standard SIG_ALL
cd compat-runner
cargo run -- --mint-url http://127.0.0.1:3339 --report-name nutshell --suite all --sigall-mode standard
```

```bash
# Nutshell, legacy SIG_ALL
cd compat-runner
cargo run -- --mint-url http://127.0.0.1:3339 --report-name nutshell-legacySIGALL --suite all --sigall-mode legacy
```

```bash
# Nutmix, standard SIG_ALL
cd compat-runner
cargo run -- --mint-url http://127.0.0.1:3338 --report-name nutmix --suite all --sigall-mode standard
```

CLI options:

- `--mint-url`
  Run against an external mint instead of the embedded CDK mint
- `--report-name`
  Write JSON to `../reports/<report-name>.json`
- `--suite swap|melt|all`
  Select which scenarios to run
- `--sigall-mode standard|legacy`
  Choose CDK/spec-style or legacy Nutshell-style `SIG_ALL` signing

If `legacy` mode is selected, it is shown in stdout metadata and recorded in the JSON report.

## Refreshing Tracked Results

Regenerate the tracked report artifacts:

```bash
cd compat-runner
cargo run -- --report-name cdk
cargo run -- --mint-url http://127.0.0.1:3339 --report-name nutshell --suite all --sigall-mode standard
cargo run -- --mint-url http://127.0.0.1:3339 --report-name nutshell-legacySIGALL --suite all --sigall-mode legacy
cargo run -- --mint-url http://127.0.0.1:3338 --report-name nutmix --suite all --sigall-mode standard
```

Then rebuild `README.md`:

```bash
python3 tools/build_readme.py
```

## Known Differences

- `Nutshell` benefits from `--sigall-mode legacy` for swap compatibility, but still has remaining `SIG_ALL` differences.
- `Nutmix` aligns better with `--sigall-mode standard` than with `legacy`.
- External negative cases are accepted as passes when the mint returns a protocol-shaped rejection even if the exact error text differs from CDK.

## Notes

- The runner does not modify `cdk/` or `nutshell/`.
- The generated `README.md` is built from `_README.md` plus the tracked `reports/*.json` files.
