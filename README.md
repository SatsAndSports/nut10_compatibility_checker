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

## Current Results

> Generated from `_README.md` and `reports/*.json` via `python3 tools/build_readme.py`.

Jump to:

- [CDK](#cdk)
- [Nutshell](#nutshell)
- [Nutmix](#nutmix)

### CDK

| Field | Value |
|---|---|
| Version | `cdk-mintd/0.16.0` |
| Mint URL | `http://127.0.0.1:38555` |
| Started At | `2026-05-18T18:34:18Z` |
| Attempted | 54 |
| Passed | 54 ✅ |
| Failed | 0 ✅ |

<details>
<summary>Scenario Results (54 scenarios, 0 failure(s))</summary>

| Scenario | Result | Note |
|---|---|---|
| `p2pk_swap_unsigned_fails` | ✅ | swap rejected as expected: Signature missing or invalid |
| `p2pk_partial_signatures_fail` | ✅ | partial spend rejected: Signature missing or invalid |
| `p2pk_swap_signed_succeeds` | ✅ | swap succeeded with 2 output signature(s) |
| `p2pk_multisig_2of3` | ✅ | 2-of-3 multisig accepted only valid signer set |
| `p2pk_locktime_before_expiry_primary_only` | ✅ | primary path works before locktime; refund path rejected |
| `p2pk_locktime_after_expiry_primary_still_works` | ✅ | primary path still works after locktime |
| `p2pk_locktime_after_expiry_no_refund_anyone_can_spend` | ✅ | anyone-can-spend refund path worked after locktime |
| `p2pk_multisig_locktime_primary_still_works` | ✅ | primary multisig still works after locktime |
| `p2pk_wrong_signer_fails` | ✅ | wrong signer rejected: Signature missing or invalid |
| `p2pk_duplicate_signatures_fail` | ✅ | duplicate signatures rejected: Signature missing or invalid |
| `htlc_preimage_only_fails` | ✅ | preimage-only HTLC spend rejected: Unknown error response: `code: 50000, detail: Witness did not provide signatures` |
| `htlc_signature_only_fails` | ✅ | signature-only HTLC spend rejected: Unknown error response: `code: 50000, detail: Secret is not a HTLC secret` |
| `htlc_swap_preimage_and_signature_succeeds` | ✅ | HTLC swap succeeded with 2 output signature(s) |
| `htlc_wrong_preimage_fails` | ✅ | wrong HTLC preimage rejected: Unknown error response: `code: 50000, detail: Preimage must be valid hex encoding` |
| `htlc_locktime_after_expiry_refund_succeeds` | ✅ | HTLC refund path worked after locktime |
| `htlc_multisig_2of3` | ✅ | HTLC 2-of-3 multisig enforced correctly |
| `htlc_receiver_path_after_locktime` | ✅ | HTLC receiver path remains valid after locktime |
| `p2pk_sigall_requires_transaction_signature` | ✅ | SIG_ALL rejected unsigned spend: Signature missing or invalid |
| `p2pk_sigall_sig_inputs_fail` | ✅ | SIG_INPUTS signatures rejected for SIG_ALL: Signature missing or invalid |
| `p2pk_sigall_multisig_2of3` | ✅ | SIG_ALL 2-of-3 multisig enforced correctly |
| `p2pk_sigall_wrong_signer_fails` | ✅ | wrong SIG_ALL signer rejected: Signature missing or invalid |
| `p2pk_sigall_duplicate_signatures_fail` | ✅ | duplicate SIG_ALL signatures rejected: Signature missing or invalid |
| `p2pk_sigall_locktime_before_expiry_primary_only` | ✅ | SIG_ALL primary path works before locktime; refund path rejected |
| `p2pk_sigall_locktime_after_expiry_primary_still_works` | ✅ | SIG_ALL primary path still works after locktime |
| `p2pk_sigall_locktime_after_expiry_no_refund_anyone_can_spend` | ✅ | SIG_ALL anyone-can-spend refund path worked after locktime |
| `p2pk_sigall_multisig_locktime_primary_still_works` | ✅ | SIG_ALL primary multisig still works after locktime |
| `p2pk_sigall_mixed_proofs_different_data_fail` | ✅ | mixed SIG_ALL proofs rejected: Unknown error response: `code: 50000, detail: Spend conditions are not met` |
| `p2pk_sigall_mixed_proofs_different_kind_fail` | ✅ | mixed SIG_ALL proof kinds rejected: Unknown error response: `code: 50000, detail: Spend conditions are not met` |
| `p2pk_sigall_mixed_proofs_different_tags_fail` | ✅ | mixed SIG_ALL proof tags rejected: Unknown error response: `code: 50000, detail: Spend conditions are not met` |
| `p2pk_sigall_multisig_before_locktime` | ✅ | SIG_ALL 2-of-3 primary multisig works before locktime |
| `p2pk_sigall_more_signatures_than_required` | ✅ | SIG_ALL accepted more valid signatures than required |
| `p2pk_sigall_refund_multisig_2of2` | ✅ | SIG_ALL 2-of-2 refund multisig enforced correctly |
| `p2pk_sigall_output_amounts_swapped_fail` | ✅ | tampered SIG_ALL outputs rejected: Signature missing or invalid |
| `htlc_sigall_preimage_only_fails` | ✅ | SIG_ALL HTLC preimage-only rejected: Unknown error response: `code: 50000, detail: Witness signatures not provided` |
| `htlc_sigall_signature_only_fails` | ✅ | SIG_ALL HTLC signature-only rejected: Unknown error response: `code: 50000, detail: HTLC spend conditions are not met` |
| `htlc_sigall_requires_preimage_and_transaction_signature` | ✅ | SIG_ALL HTLC swap succeeded with 2 output signature(s) |
| `htlc_sigall_wrong_preimage_fails` | ✅ | wrong SIG_ALL HTLC preimage rejected: Unknown error response: `code: 50000, detail: HTLC spend conditions are not met` |
| `htlc_sigall_locktime_after_expiry_refund_succeeds` | ✅ | SIG_ALL HTLC refund path worked after locktime |
| `htlc_sigall_multisig_2of3` | ✅ | SIG_ALL HTLC 2-of-3 multisig enforced correctly |
| `htlc_sigall_receiver_path_after_locktime` | ✅ | SIG_ALL HTLC receiver path remains valid after locktime |
| `melt_p2pk_unsigned_fails` | ✅ | unsigned melt rejected as expected: Signature missing or invalid |
| `melt_p2pk_signed_succeeds` | ✅ | melt succeeded with state PAID |
| `melt_htlc_preimage_only_fails` | ✅ | preimage-only melt rejected as expected: Unknown error response: `code: 50000, detail: Witness did not provide signatures` |
| `melt_htlc_signature_only_fails` | ✅ | signature-only melt rejected as expected: Unknown error response: `code: 50000, detail: Secret is not a HTLC secret` |
| `melt_htlc_preimage_and_signature_succeeds` | ✅ | melt succeeded with state PAID |
| `melt_p2pk_sigall_unsigned_fails` | ✅ | unsigned SIG_ALL melt rejected as expected: Signature missing or invalid |
| `melt_p2pk_sigall_sig_inputs_fail` | ✅ | SIG_INPUTS melt rejected for SIG_ALL as expected: Signature missing or invalid |
| `melt_p2pk_sigall_transaction_signature_succeeds` | ✅ | melt succeeded with state PAID |
| `melt_htlc_sigall_preimage_only_fails` | ✅ | preimage-only SIG_ALL melt rejected as expected: Unknown error response: `code: 50000, detail: Witness signatures not provided` |
| `melt_htlc_sigall_sig_inputs_fail` | ✅ | SIG_INPUTS melt rejected for HTLC SIG_ALL as expected: Unknown error response: `code: 50000, detail: HTLC spend conditions are not met` |
| `melt_htlc_sigall_preimage_and_transaction_signature_succeeds` | ✅ | melt succeeded with state PAID |
| `melt_p2pk_post_locktime_anyone_can_spend` | ✅ | melt succeeded with state PAID |
| `melt_p2pk_before_locktime_wrong_key_fails` | ✅ | wrong-key melt rejected before locktime as expected: Signature missing or invalid |
| `melt_p2pk_before_locktime_correct_key_succeeds` | ✅ | melt succeeded with state PAID |

</details>

### Nutshell

| Field | Value |
|---|---|
| Version | `Nutshell/0.20.0` |
| Mint URL | `http://127.0.0.1:3339` |
| Mint Name | Local Nutshell Test Mint |
| Started At | `2026-05-18T18:34:47Z` |
| Attempted | 54 |
| Passed | 36 ✅ |
| Failed | 18 ❌ |

<details>
<summary>Scenario Results (54 scenarios, 18 failure(s))</summary>

| Scenario | Result | Note |
|---|---|---|
| `p2pk_swap_unsigned_fails` | ✅ | swap rejected as expected: Unknown error response: `code: 0, detail: Witness is missing for p2pk signature` |
| `p2pk_partial_signatures_fail` | ✅ | partial spend rejected: Unknown error response: `code: 0, detail: Witness is missing for p2pk signature` |
| `p2pk_swap_signed_succeeds` | ✅ | swap succeeded with 2 output signature(s) |
| `p2pk_multisig_2of3` | ✅ | 2-of-3 multisig accepted only valid signer set |
| `p2pk_locktime_before_expiry_primary_only` | ✅ | primary path works before locktime; refund path rejected |
| `p2pk_locktime_after_expiry_primary_still_works` | ✅ | primary path still works after locktime |
| `p2pk_locktime_after_expiry_no_refund_anyone_can_spend` | ✅ | anyone-can-spend refund path worked after locktime |
| `p2pk_multisig_locktime_primary_still_works` | ✅ | primary multisig still works after locktime |
| `p2pk_wrong_signer_fails` | ✅ | wrong signer rejected: Unknown error response: `code: 11000, detail: signature threshold not met. 0 < 1.` |
| `p2pk_duplicate_signatures_fail` | ✅ | duplicate signatures rejected: Unknown error response: `code: 11000, detail: signature threshold not met. 1 < 2.` |
| `htlc_preimage_only_fails` | ✅ | preimage-only HTLC spend rejected: Unknown error response: `code: 11000, detail: no signatures in proof.` |
| `htlc_signature_only_fails` | ✅ | signature-only HTLC spend rejected: Unknown error response: `code: 11000, detail: no HTLC preimage provided` |
| `htlc_swap_preimage_and_signature_succeeds` | ✅ | HTLC swap succeeded with 2 output signature(s) |
| `htlc_wrong_preimage_fails` | ✅ | wrong HTLC preimage rejected: Unknown error response: `code: 11000, detail: HTLC preimage must be 64 characters hex.` |
| `htlc_locktime_after_expiry_refund_succeeds` | ✅ | HTLC refund path worked after locktime |
| `htlc_multisig_2of3` | ✅ | HTLC 2-of-3 multisig enforced correctly |
| `htlc_receiver_path_after_locktime` | ✅ | HTLC receiver path remains valid after locktime |
| `p2pk_sigall_requires_transaction_signature` | ✅ | SIG_ALL rejected unsigned spend: Unknown error response: `code: 11000, detail: no witness in proof.` |
| `p2pk_sigall_sig_inputs_fail` | ✅ | SIG_INPUTS signatures rejected for SIG_ALL: Unknown error response: `code: 11000, detail: signature threshold not met. 0 < 1.` |
| `p2pk_sigall_multisig_2of3` | ❌ | SIG_ALL valid 2-of-3: Unknown error response: `code: 11000, detail: signature threshold not met. 0 < 2.` |
| `p2pk_sigall_wrong_signer_fails` | ✅ | wrong SIG_ALL signer rejected: Unknown error response: `code: 11000, detail: signature threshold not met. 0 < 1.` |
| `p2pk_sigall_duplicate_signatures_fail` | ✅ | duplicate SIG_ALL signatures rejected: Unknown error response: `code: 11000, detail: signature threshold not met. 0 < 2.` |
| `p2pk_sigall_locktime_before_expiry_primary_only` | ❌ | SIG_ALL primary before locktime: Unknown error response: `code: 11000, detail: signature threshold not met. 0 < 1.` |
| `p2pk_sigall_locktime_after_expiry_primary_still_works` | ❌ | SIG_ALL primary after locktime: Unknown error response: `code: 11000, detail: signature threshold not met. 0 < 1.` |
| `p2pk_sigall_locktime_after_expiry_no_refund_anyone_can_spend` | ✅ | SIG_ALL anyone-can-spend refund path worked after locktime |
| `p2pk_sigall_multisig_locktime_primary_still_works` | ❌ | SIG_ALL primary multisig after locktime: Unknown error response: `code: 11000, detail: signature threshold not met. 0 < 1.` |
| `p2pk_sigall_mixed_proofs_different_data_fail` | ❌ | alice-only SIG_ALL: Unknown error response: `code: 11000, detail: signature threshold not met. 0 < 1.` |
| `p2pk_sigall_mixed_proofs_different_kind_fail` | ❌ | p2pk-only mixed-kind control: Unknown error response: `code: 11000, detail: signature threshold not met. 0 < 1.` |
| `p2pk_sigall_mixed_proofs_different_tags_fail` | ❌ | plain-only mixed-tags control: Unknown error response: `code: 11000, detail: signature threshold not met. 0 < 1.` |
| `p2pk_sigall_multisig_before_locktime` | ❌ | SIG_ALL 2-of-3 before locktime: Unknown error response: `code: 11000, detail: signature threshold not met. 0 < 2.` |
| `p2pk_sigall_more_signatures_than_required` | ❌ | extra valid signatures: Unknown error response: `code: 11000, detail: signature threshold not met. 0 < 2.` |
| `p2pk_sigall_refund_multisig_2of2` | ❌ | 2-of-2 refund multisig: Unknown error response: `code: 11000, detail: signature threshold not met. 0 < 2.` |
| `p2pk_sigall_output_amounts_swapped_fail` | ❌ | restored original output amounts: Unknown error response: `code: 11000, detail: signature threshold not met. 0 < 1.` |
| `htlc_sigall_preimage_only_fails` | ✅ | SIG_ALL HTLC preimage-only rejected: Unknown error response: `code: 0, detail: Witness is missing for htlc preimage` |
| `htlc_sigall_signature_only_fails` | ✅ | SIG_ALL HTLC signature-only rejected: Unknown error response: `code: 11000, detail: no HTLC preimage provided` |
| `htlc_sigall_requires_preimage_and_transaction_signature` | ❌ | SIG_ALL HTLC valid spend: Unknown error response: `code: 0, detail: Witness is missing for htlc preimage` |
| `htlc_sigall_wrong_preimage_fails` | ✅ | wrong SIG_ALL HTLC preimage rejected: Unknown error response: `code: 11000, detail: HTLC preimage must be 64 characters hex.` |
| `htlc_sigall_locktime_after_expiry_refund_succeeds` | ❌ | SIG_ALL HTLC refund after locktime: Unknown error response: `code: 0, detail: Witness is missing for htlc preimage` |
| `htlc_sigall_multisig_2of3` | ❌ | SIG_ALL HTLC 2-of-3: Unknown error response: `code: 0, detail: Witness is missing for htlc preimage` |
| `htlc_sigall_receiver_path_after_locktime` | ❌ | SIG_ALL HTLC receiver after locktime: Unknown error response: `code: 0, detail: Witness is missing for htlc preimage` |
| `melt_p2pk_unsigned_fails` | ✅ | unsigned melt rejected as expected: Unknown error response: `code: 0, detail: Witness is missing for p2pk signature` |
| `melt_p2pk_signed_succeeds` | ✅ | melt succeeded with state PAID |
| `melt_htlc_preimage_only_fails` | ✅ | preimage-only melt rejected as expected: Unknown error response: `code: 11000, detail: no signatures in proof.` |
| `melt_htlc_signature_only_fails` | ✅ | signature-only melt rejected as expected: Unknown error response: `code: 11000, detail: no HTLC preimage provided` |
| `melt_htlc_preimage_and_signature_succeeds` | ✅ | melt succeeded with state PAID |
| `melt_p2pk_sigall_unsigned_fails` | ✅ | unsigned SIG_ALL melt rejected as expected: Unknown error response: `code: 11000, detail: no witness in proof.` |
| `melt_p2pk_sigall_sig_inputs_fail` | ✅ | SIG_INPUTS melt rejected for SIG_ALL as expected: Unknown error response: `code: 11000, detail: signature threshold not met. 0 < 1.` |
| `melt_p2pk_sigall_transaction_signature_succeeds` | ❌ | [submit] melt P2PK SIG_ALL valid |
| `melt_htlc_sigall_preimage_only_fails` | ❌ | melt HTLC SIG_ALL preimage-only: unexpected error `Http transport error Some(500): Internal Server Error`; expected one of ["Witness signatures not provided", "Witness did not provide signatures", "Witness is missing for htlc preimage"] |
| `melt_htlc_sigall_sig_inputs_fail` | ✅ | SIG_INPUTS melt rejected for HTLC SIG_ALL as expected: accepted protocol-like rejection: status=Some(400), code=Some(11000), detail=Some("signature threshold not met. 0 < 1.") |
| `melt_htlc_sigall_preimage_and_transaction_signature_succeeds` | ❌ | [submit] melt HTLC SIG_ALL valid |
| `melt_p2pk_post_locktime_anyone_can_spend` | ✅ | melt succeeded with state PAID |
| `melt_p2pk_before_locktime_wrong_key_fails` | ✅ | wrong-key melt rejected before locktime as expected: Unknown error response: `code: 11000, detail: signature threshold not met. 0 < 1.` |
| `melt_p2pk_before_locktime_correct_key_succeeds` | ✅ | melt succeeded with state PAID |

</details>

### Nutmix

| Field | Value |
|---|---|
| Version | `nutmix/0.4.0` |
| Mint URL | `http://127.0.0.1:3338` |
| Started At | `2026-05-18T18:34:34Z` |
| Attempted | 54 |
| Passed | 48 ✅ |
| Failed | 6 ❌ |

<details>
<summary>Scenario Results (54 scenarios, 6 failure(s))</summary>

| Scenario | Result | Note |
|---|---|---|
| `p2pk_swap_unsigned_fails` | ✅ | swap rejected as expected: accepted protocol-like rejection: status=Some(400), code=Some(99999), detail=None |
| `p2pk_partial_signatures_fail` | ✅ | partial spend rejected: accepted protocol-like rejection: status=Some(400), code=Some(99999), detail=None |
| `p2pk_swap_signed_succeeds` | ✅ | swap succeeded with 2 output signature(s) |
| `p2pk_multisig_2of3` | ✅ | 2-of-3 multisig accepted only valid signer set |
| `p2pk_locktime_before_expiry_primary_only` | ✅ | primary path works before locktime; refund path rejected |
| `p2pk_locktime_after_expiry_primary_still_works` | ✅ | primary path still works after locktime |
| `p2pk_locktime_after_expiry_no_refund_anyone_can_spend` | ❌ | anyone-can-spend after locktime: Http transport error Some(400): {"code":99999} |
| `p2pk_multisig_locktime_primary_still_works` | ✅ | primary multisig still works after locktime |
| `p2pk_wrong_signer_fails` | ✅ | wrong signer rejected: accepted protocol-like rejection: status=Some(400), code=Some(10001), detail=Some("Token not verified") |
| `p2pk_duplicate_signatures_fail` | ✅ | duplicate signatures rejected: accepted protocol-like rejection: status=Some(400), code=Some(10001), detail=Some("Token not verified") |
| `htlc_preimage_only_fails` | ✅ | preimage-only HTLC spend rejected: accepted protocol-like rejection: status=Some(400), code=Some(10001), detail=Some("Token not verified") |
| `htlc_signature_only_fails` | ✅ | signature-only HTLC spend rejected: accepted protocol-like rejection: status=Some(400), code=Some(10001), detail=Some("Token not verified") |
| `htlc_swap_preimage_and_signature_succeeds` | ✅ | HTLC swap succeeded with 2 output signature(s) |
| `htlc_wrong_preimage_fails` | ✅ | wrong HTLC preimage rejected: accepted protocol-like rejection: status=Some(400), code=Some(99999), detail=None |
| `htlc_locktime_after_expiry_refund_succeeds` | ❌ | HTLC refund after locktime: Token not verified |
| `htlc_multisig_2of3` | ✅ | HTLC 2-of-3 multisig enforced correctly |
| `htlc_receiver_path_after_locktime` | ✅ | HTLC receiver path remains valid after locktime |
| `p2pk_sigall_requires_transaction_signature` | ✅ | SIG_ALL rejected unsigned spend: accepted protocol-like rejection: status=Some(400), code=Some(99999), detail=None |
| `p2pk_sigall_sig_inputs_fail` | ✅ | SIG_INPUTS signatures rejected for SIG_ALL: accepted protocol-like rejection: status=Some(400), code=Some(10001), detail=Some("Token not verified") |
| `p2pk_sigall_multisig_2of3` | ✅ | SIG_ALL 2-of-3 multisig enforced correctly |
| `p2pk_sigall_wrong_signer_fails` | ✅ | wrong SIG_ALL signer rejected: accepted protocol-like rejection: status=Some(400), code=Some(10001), detail=Some("Token not verified") |
| `p2pk_sigall_duplicate_signatures_fail` | ✅ | duplicate SIG_ALL signatures rejected: accepted protocol-like rejection: status=Some(400), code=Some(10001), detail=Some("Token not verified") |
| `p2pk_sigall_locktime_before_expiry_primary_only` | ✅ | SIG_ALL primary path works before locktime; refund path rejected |
| `p2pk_sigall_locktime_after_expiry_primary_still_works` | ✅ | SIG_ALL primary path still works after locktime |
| `p2pk_sigall_locktime_after_expiry_no_refund_anyone_can_spend` | ❌ | SIG_ALL anyone-can-spend: Http transport error Some(400): {"code":99999} |
| `p2pk_sigall_multisig_locktime_primary_still_works` | ✅ | SIG_ALL primary multisig still works after locktime |
| `p2pk_sigall_mixed_proofs_different_data_fail` | ✅ | mixed SIG_ALL proofs rejected: accepted protocol-like rejection: status=Some(400), code=Some(99999), detail=None |
| `p2pk_sigall_mixed_proofs_different_kind_fail` | ✅ | mixed SIG_ALL proof kinds rejected: accepted protocol-like rejection: status=Some(400), code=Some(99999), detail=None |
| `p2pk_sigall_mixed_proofs_different_tags_fail` | ✅ | mixed SIG_ALL proof tags rejected: accepted protocol-like rejection: status=Some(400), code=Some(99999), detail=None |
| `p2pk_sigall_multisig_before_locktime` | ✅ | SIG_ALL 2-of-3 primary multisig works before locktime |
| `p2pk_sigall_more_signatures_than_required` | ✅ | SIG_ALL accepted more valid signatures than required |
| `p2pk_sigall_refund_multisig_2of2` | ✅ | SIG_ALL 2-of-2 refund multisig enforced correctly |
| `p2pk_sigall_output_amounts_swapped_fail` | ✅ | tampered SIG_ALL outputs rejected: accepted protocol-like rejection: status=Some(400), code=Some(10001), detail=Some("Token not verified") |
| `htlc_sigall_preimage_only_fails` | ✅ | SIG_ALL HTLC preimage-only rejected: accepted protocol-like rejection: status=Some(400), code=Some(10001), detail=Some("Token not verified") |
| `htlc_sigall_signature_only_fails` | ❌ | SIG_ALL HTLC signature-only: swap unexpectedly succeeded |
| `htlc_sigall_requires_preimage_and_transaction_signature` | ✅ | SIG_ALL HTLC swap succeeded with 2 output signature(s) |
| `htlc_sigall_wrong_preimage_fails` | ❌ | SIG_ALL HTLC wrong preimage: swap unexpectedly succeeded |
| `htlc_sigall_locktime_after_expiry_refund_succeeds` | ✅ | SIG_ALL HTLC refund path worked after locktime |
| `htlc_sigall_multisig_2of3` | ✅ | SIG_ALL HTLC 2-of-3 multisig enforced correctly |
| `htlc_sigall_receiver_path_after_locktime` | ✅ | SIG_ALL HTLC receiver path remains valid after locktime |
| `melt_p2pk_unsigned_fails` | ✅ | unsigned melt rejected as expected: accepted protocol-like rejection: status=Some(400), code=Some(99999), detail=None |
| `melt_p2pk_signed_succeeds` | ✅ | melt succeeded with state PAID |
| `melt_htlc_preimage_only_fails` | ✅ | preimage-only melt rejected as expected: accepted protocol-like rejection: status=Some(400), code=Some(10001), detail=Some("Token not verified") |
| `melt_htlc_signature_only_fails` | ✅ | signature-only melt rejected as expected: accepted protocol-like rejection: status=Some(400), code=Some(10001), detail=Some("Token not verified") |
| `melt_htlc_preimage_and_signature_succeeds` | ✅ | melt succeeded with state PAID |
| `melt_p2pk_sigall_unsigned_fails` | ✅ | unsigned SIG_ALL melt rejected as expected: accepted protocol-like rejection: status=Some(400), code=Some(99999), detail=None |
| `melt_p2pk_sigall_sig_inputs_fail` | ✅ | SIG_INPUTS melt rejected for SIG_ALL as expected: accepted protocol-like rejection: status=Some(400), code=Some(10001), detail=Some("Token not verified") |
| `melt_p2pk_sigall_transaction_signature_succeeds` | ✅ | melt succeeded with state PAID |
| `melt_htlc_sigall_preimage_only_fails` | ✅ | preimage-only SIG_ALL melt rejected as expected: accepted protocol-like rejection: status=Some(400), code=Some(10001), detail=Some("Token not verified") |
| `melt_htlc_sigall_sig_inputs_fail` | ✅ | SIG_INPUTS melt rejected for HTLC SIG_ALL as expected: accepted protocol-like rejection: status=Some(400), code=Some(10001), detail=Some("Token not verified") |
| `melt_htlc_sigall_preimage_and_transaction_signature_succeeds` | ✅ | melt succeeded with state PAID |
| `melt_p2pk_post_locktime_anyone_can_spend` | ❌ | [submit] melt anyone-can-spend after locktime |
| `melt_p2pk_before_locktime_wrong_key_fails` | ✅ | wrong-key melt rejected before locktime as expected: accepted protocol-like rejection: status=Some(400), code=Some(10001), detail=Some("Token not verified") |
| `melt_p2pk_before_locktime_correct_key_succeeds` | ✅ | melt succeeded with state PAID |

</details>

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
- `reports/`
  Tracked JSON compatibility reports used to build the top-of-README results sections
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
- optionally write JSON to `reports/<report-name>.json` when `--report-name` is provided

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
cargo run -- --mint-url http://127.0.0.1:3339 --report-name nutshell --suite swap
```

CLI behavior:

- `cargo run`
  default embedded CDK mode
- `--mint-url`
  external mint mode
- `--report-name`
  optional JSON artifact name written to `../reports/<report-name>.json`
- `--suite swap|melt|all`
  suite selection
- `--sigall-mode standard|legacy`
  choose CDK/spec `SIG_ALL` signing or legacy Nutshell-style signing
  legacy mode is also recorded in the JSON report and shown in stdout metadata

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
cargo run -- --mint-url http://127.0.0.1:3339 --report-name nutshell --suite swap
```

```bash
# external Nutshell, swap only, legacy SIG_ALL mode
cd compat-runner
cargo run -- --mint-url http://127.0.0.1:3339 --report-name nutshell --suite swap --sigall-mode legacy
```

```bash
# external Nutshell, full melt suite
cd compat-runner
cargo run -- --mint-url http://127.0.0.1:3339 --report-name nutshell --suite melt
```

```bash
# external Nutmix, full melt suite
cd compat-runner
cargo run -- --mint-url http://127.0.0.1:3338 --report-name nutmix --suite melt
```

## Refreshing Tracked Results

Regenerate the tracked report artifacts:

```bash
cd compat-runner
cargo run -- --report-name cdk
cargo run -- --mint-url http://127.0.0.1:3339 --report-name nutshell --suite all --sigall-mode standard
cargo run -- --mint-url http://127.0.0.1:3338 --report-name nutmix --suite all --sigall-mode standard
```

Then rebuild `README.md` from `_README.md` and the tracked reports:

```bash
python3 tools/build_readme.py
```

Current external target notes:

- for external targets, negative-case validation is now relaxed to accept protocol-like rejections even when exact error text differs from CDK
- exact error text is still shown in the report for human diagnosis

Current Nutshell status:

- external proof funding now works through explicit HTTP quote polling and minting
- non-SIG_ALL swap scenarios largely pass
- `--sigall-mode legacy` improves Nutshell compatibility substantially compared to the default `standard` mode
- remaining failures in legacy mode are concentrated in a smaller SIG_ALL subset, especially HTLC SIG_ALL and some post-locktime/tamper cases
- normalized swap counts are currently:
  - `standard`: 15 failures
  - `legacy`: 8 failures

Current interpretation of the remaining Nutshell legacy-mode failures:

- P2PK SIG_ALL mostly improves under legacy mode, which strongly suggests Nutshell still verifies the older SIG_ALL message format
- some post-locktime P2PK SIG_ALL scenarios still fail, which appears to be related to Nutshell preferring the refund path after locktime instead of keeping the primary path additive
- several HTLC SIG_ALL scenarios still fail, and Nutshell's verification path appears to require per-proof HTLC witness/preimage presence before aggregate SIG_ALL validation
- the output-amount tamper case still succeeds in legacy mode, which is consistent with the older SIG_ALL message format not binding output amounts

Remaining Nutshell legacy-mode failure clusters:

- post-locktime P2PK SIG_ALL primary-path cases
- HTLC SIG_ALL first-input-only witness cases
- output-amount tamper detection in legacy SIG_ALL mode

About `SIG_ALL` modes:

- `standard`
  Uses the current CDK/spec-style aggregated message construction.
- `legacy`
  Uses the older Nutshell-style aggregated message construction for runner-side `SIG_ALL` signatures.

The `legacy` mode exists as a diagnostic and interoperability mode. The default remains `standard`.

Current Nutmix status:

- external swap execution works against a local Nutmix mint
- Nutmix appears closer to `standard` SIG_ALL mode than to `legacy`
- after relaxing external negative-case validation, the standard-mode swap result set is much cleaner
- normalized swap count is currently:
  - `standard`: 5 failures
- in other words: many earlier Nutmix failures were negative cases rejected correctly but reported with generic error shapes

Current external melt status:

- Nutshell:
  - standard witness/preimage melt cases pass
  - post-locktime expired-locktime-no-refund melt passes
  - positive SIG_ALL melt cases still fail in both `standard` and `legacy`
  - HTLC SIG_ALL preimage-only negative currently returns HTTP 500 instead of a clean protocol rejection
- Nutmix:
  - standard witness/preimage melt cases pass
  - positive SIG_ALL melt cases pass in `standard`
  - the expired-locktime-no-refund melt case fails

Remaining Nutmix standard-mode failure clusters:

- anyone-can-spend after expired locktime
- HTLC refund-after-locktime
- HTLC SIG_ALL cases where missing or wrong preimage unexpectedly does not fail

## Notes

- The runner does not modify `cdk/` or `nutshell/`.
- For melt scenarios on the embedded CDK fakewallet mint, the runner is quote-driven.
- Some upstream CDK tests are intentionally split into multiple runner scenarios for clearer reporting.

## Next Steps

- investigate the remaining Nutshell SIG_ALL failures in more detail
- decide whether any additional target-specific diagnostic modes are worthwhile
- later generalize melt for non-fakewallet targets
