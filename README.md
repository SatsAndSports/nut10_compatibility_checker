# Cashu Compatibility Runner

Compatibility runner for Cashu spending-condition behavior across `cdk-mintd`, `Nutshell`, and `Nutmix`.

This repo publishes tracked compatibility results for swap and melt scenarios covering:

- P2PK
- HTLC
- `SIG_ALL`
- locktime and refund behavior

## Current Results

> Generated from `_README.md` and `reports/*.json` via `python3 tools/build_readme.py`.

Jump to:

- [CDK](#cdk)
- [Nutmix](#nutmix)
- [Nutshell](#nutshell)
- [Nutshell (Legacy SIG_ALL)](#nutshell-legacy-sigall)

<a id="cdk"></a>

### CDK

| Field | Value |
|---|---|
| Version | `cdk-mintd/0.16.0` |
| Mint URL | `http://127.0.0.1:33985` |
| Started At | `2026-05-18T19:05:18Z` |
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

<a id="nutmix"></a>

### Nutmix

| Field | Value |
|---|---|
| Version | `nutmix/0.4.0` |
| Mint URL | `http://127.0.0.1:3338` |
| Started At | `2026-05-18T19:05:34Z` |
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

<a id="nutshell"></a>

### Nutshell

| Field | Value |
|---|---|
| Version | `Nutshell/0.20.0` |
| Mint URL | `http://127.0.0.1:3339` |
| Mint Name | Local Nutshell Test Mint |
| Started At | `2026-05-18T19:05:45Z` |
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

<a id="nutshell-legacy-sigall"></a>

### Nutshell (Legacy SIG_ALL)

| Field | Value |
|---|---|
| Version | `Nutshell/0.20.0` |
| Mint URL | `http://127.0.0.1:3339` |
| Mint Name | Local Nutshell Test Mint |
| SIG_ALL Mode | `legacy` |
| Started At | `2026-05-18T19:06:09Z` |
| Attempted | 54 |
| Passed | 43 ✅ |
| Failed | 11 ❌ |

<details>
<summary>Scenario Results (54 scenarios, 11 failure(s))</summary>

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
| `p2pk_sigall_multisig_2of3` | ✅ | SIG_ALL 2-of-3 multisig enforced correctly |
| `p2pk_sigall_wrong_signer_fails` | ✅ | wrong SIG_ALL signer rejected: Unknown error response: `code: 11000, detail: signature threshold not met. 0 < 1.` |
| `p2pk_sigall_duplicate_signatures_fail` | ✅ | duplicate SIG_ALL signatures rejected: Unknown error response: `code: 11000, detail: signature threshold not met. 1 < 2.` |
| `p2pk_sigall_locktime_before_expiry_primary_only` | ✅ | SIG_ALL primary path works before locktime; refund path rejected |
| `p2pk_sigall_locktime_after_expiry_primary_still_works` | ❌ | SIG_ALL primary after locktime: Unknown error response: `code: 11000, detail: signature threshold not met. 0 < 1.` |
| `p2pk_sigall_locktime_after_expiry_no_refund_anyone_can_spend` | ✅ | SIG_ALL anyone-can-spend refund path worked after locktime |
| `p2pk_sigall_multisig_locktime_primary_still_works` | ❌ | SIG_ALL primary multisig after locktime: Unknown error response: `code: 11000, detail: signature threshold not met. 0 < 1.` |
| `p2pk_sigall_mixed_proofs_different_data_fail` | ✅ | mixed SIG_ALL proofs rejected: Unknown error response: `code: 11000, detail: not all secrets are equal.` |
| `p2pk_sigall_mixed_proofs_different_kind_fail` | ❌ | htlc-only mixed-kind control: Unknown error response: `code: 0, detail: Witness is missing for htlc preimage` |
| `p2pk_sigall_mixed_proofs_different_tags_fail` | ✅ | mixed SIG_ALL proof tags rejected: Unknown error response: `code: 11000, detail: not all secrets are equal.` |
| `p2pk_sigall_multisig_before_locktime` | ✅ | SIG_ALL 2-of-3 primary multisig works before locktime |
| `p2pk_sigall_more_signatures_than_required` | ✅ | SIG_ALL accepted more valid signatures than required |
| `p2pk_sigall_refund_multisig_2of2` | ✅ | SIG_ALL 2-of-2 refund multisig enforced correctly |
| `p2pk_sigall_output_amounts_swapped_fail` | ❌ | tampered output amounts: swap unexpectedly succeeded |
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
| `melt_p2pk_sigall_transaction_signature_succeeds` | ❌ | P2PK spend conditions are not met |
| `melt_htlc_sigall_preimage_only_fails` | ❌ | melt HTLC SIG_ALL preimage-only: unexpected error `Http transport error Some(500): Internal Server Error`; expected one of ["Witness signatures not provided", "Witness did not provide signatures", "Witness is missing for htlc preimage"] |
| `melt_htlc_sigall_sig_inputs_fail` | ✅ | SIG_INPUTS melt rejected for HTLC SIG_ALL as expected: accepted protocol-like rejection: status=Some(400), code=Some(11000), detail=Some("signature threshold not met. 0 < 1.") |
| `melt_htlc_sigall_preimage_and_transaction_signature_succeeds` | ❌ | HTLC spend conditions are not met |
| `melt_p2pk_post_locktime_anyone_can_spend` | ✅ | melt succeeded with state PAID |
| `melt_p2pk_before_locktime_wrong_key_fails` | ✅ | wrong-key melt rejected before locktime as expected: Unknown error response: `code: 11000, detail: signature threshold not met. 0 < 1.` |
| `melt_p2pk_before_locktime_correct_key_succeeds` | ✅ | melt succeeded with state PAID |

</details>

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

## Analysis

### CDK

`cdk-mintd` currently passes the full tracked suite and serves as the reference behavior for this runner.

### Nutmix

`Nutmix` aligns more closely with `standard` `SIG_ALL` behavior than with `legacy`. The remaining differences appear concentrated in a smaller set of locktime/refund-path and HTLC `SIG_ALL` cases.

### Nutshell

With `standard` `SIG_ALL`, `Nutshell` shows broader compatibility gaps concentrated around `SIG_ALL`, including post-locktime P2PK behavior, HTLC first-input-only witness handling, and positive melt `SIG_ALL` cases.

### Nutshell (Legacy SIG_ALL)

`legacy` `SIG_ALL` materially improves Nutshell swap compatibility, which suggests compatibility with an older `SIG_ALL` message format. Even under `legacy`, some post-locktime P2PK, HTLC `SIG_ALL`, output-tamper, and melt `SIG_ALL` differences remain.

## Notes

- The runner does not modify `cdk/` or `nutshell/`.
- The generated `README.md` is built from `_README.md` plus the tracked `reports/*.json` files.
