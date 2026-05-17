use std::fmt;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow};
use bip39::Mnemonic;
use cdk::amount::{FeeAndAmounts, SplitTarget};
use cdk::dhke::{blind_message, construct_proofs};
use cdk::mint_url::MintUrl;
use cdk::nuts::nut10::Secret as Nut10Secret;
use cdk::nuts::{
    BlindedMessage, Conditions, CurrencyUnit, Id, Keys, PaymentMethod, PreMintSecrets, Proof,
    ProofsMethods, PublicKey, SecretKey, SigFlag, SpendingConditions, SwapRequest,
};
use cdk::wallet::{HttpClient, MintConnector, Wallet, WalletBuilder};
use cdk::{Amount, Error, StreamExt};
use cdk_mintd::config::{Database, DatabaseEngine, FakeWallet, Info, Ln, LnBackend, Settings};
use serde::Serialize;
use tokio_util::sync::CancellationToken;

const DEFAULT_JSON_REPORT_PATH: &str = "compat-report.json";
const DEFAULT_MINT_HOST: &str = "127.0.0.1";

fn standard_input_amount() -> Amount {
    Amount::from(10)
}

type ScenarioFuture = Pin<Box<dyn Future<Output = Result<String>> + Send>>;

#[derive(Debug, Clone, Serialize)]
struct ScenarioResult {
    name: String,
    target: String,
    passed: bool,
    duration_ms: u128,
    note: String,
}

#[derive(Debug, Serialize)]
struct Report {
    generated_at_unix_secs: u64,
    target: String,
    mint_url: String,
    results: Vec<ScenarioResult>,
}

struct LocalMintHandle {
    mint_url: String,
    work_dir: PathBuf,
    shutdown: CancellationToken,
    task: tokio::task::JoinHandle<Result<()>>,
}

#[derive(Clone)]
struct TestContext {
    wallet: Wallet,
    client: HttpClient,
}

struct LockedProofs {
    proofs: Vec<Proof>,
    keyset_id: Id,
}

struct Keypair {
    secret: SecretKey,
    public: PublicKey,
}

struct HtlcFixture {
    hash: String,
    preimage: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mint = LocalMintHandle::start().await?;
    let target = "cdk".to_string();
    let mint_url = mint.mint_url.clone();

    let scenarios: Vec<(&str, Box<dyn FnOnce(String) -> ScenarioFuture + Send>)> = vec![
        scenario(
            "p2pk_swap_unsigned_fails",
            scenario_p2pk_swap_unsigned_fails,
        ),
        scenario(
            "p2pk_partial_signatures_fail",
            scenario_p2pk_partial_signatures_fail,
        ),
        scenario(
            "p2pk_swap_signed_succeeds",
            scenario_p2pk_swap_signed_succeeds,
        ),
        scenario("p2pk_multisig_2of3", scenario_p2pk_multisig_2of3),
        scenario(
            "p2pk_locktime_before_expiry_primary_only",
            scenario_p2pk_locktime_before_expiry_primary_only,
        ),
        scenario(
            "p2pk_locktime_after_expiry_primary_still_works",
            scenario_p2pk_locktime_after_expiry_primary_still_works,
        ),
        scenario(
            "p2pk_locktime_after_expiry_no_refund_anyone_can_spend",
            scenario_p2pk_locktime_after_expiry_no_refund_anyone_can_spend,
        ),
        scenario(
            "p2pk_multisig_locktime_primary_still_works",
            scenario_p2pk_multisig_locktime_primary_still_works,
        ),
        scenario("p2pk_wrong_signer_fails", scenario_p2pk_wrong_signer_fails),
        scenario(
            "p2pk_duplicate_signatures_fail",
            scenario_p2pk_duplicate_signatures_fail,
        ),
        scenario(
            "htlc_preimage_only_fails",
            scenario_htlc_preimage_only_fails,
        ),
        scenario(
            "htlc_signature_only_fails",
            scenario_htlc_signature_only_fails,
        ),
        scenario(
            "htlc_swap_preimage_and_signature_succeeds",
            scenario_htlc_swap_preimage_and_signature_succeeds,
        ),
        scenario(
            "htlc_wrong_preimage_fails",
            scenario_htlc_wrong_preimage_fails,
        ),
        scenario(
            "htlc_locktime_after_expiry_refund_succeeds",
            scenario_htlc_locktime_after_expiry_refund_succeeds,
        ),
        scenario("htlc_multisig_2of3", scenario_htlc_multisig_2of3),
        scenario(
            "htlc_receiver_path_after_locktime",
            scenario_htlc_receiver_path_after_locktime,
        ),
        scenario(
            "p2pk_sigall_requires_transaction_signature",
            scenario_p2pk_sigall_requires_transaction_signature,
        ),
        scenario(
            "p2pk_sigall_sig_inputs_fail",
            scenario_p2pk_sigall_sig_inputs_fail,
        ),
        scenario(
            "p2pk_sigall_multisig_2of3",
            scenario_p2pk_sigall_multisig_2of3,
        ),
        scenario(
            "p2pk_sigall_wrong_signer_fails",
            scenario_p2pk_sigall_wrong_signer_fails,
        ),
        scenario(
            "p2pk_sigall_duplicate_signatures_fail",
            scenario_p2pk_sigall_duplicate_signatures_fail,
        ),
        scenario(
            "p2pk_sigall_locktime_before_expiry_primary_only",
            scenario_p2pk_sigall_locktime_before_expiry_primary_only,
        ),
        scenario(
            "p2pk_sigall_locktime_after_expiry_primary_still_works",
            scenario_p2pk_sigall_locktime_after_expiry_primary_still_works,
        ),
        scenario(
            "p2pk_sigall_locktime_after_expiry_no_refund_anyone_can_spend",
            scenario_p2pk_sigall_locktime_after_expiry_no_refund_anyone_can_spend,
        ),
        scenario(
            "p2pk_sigall_multisig_locktime_primary_still_works",
            scenario_p2pk_sigall_multisig_locktime_primary_still_works,
        ),
        scenario(
            "p2pk_sigall_mixed_proofs_different_data_fail",
            scenario_p2pk_sigall_mixed_proofs_different_data_fail,
        ),
        scenario(
            "p2pk_sigall_multisig_before_locktime",
            scenario_p2pk_sigall_multisig_before_locktime,
        ),
        scenario(
            "p2pk_sigall_more_signatures_than_required",
            scenario_p2pk_sigall_more_signatures_than_required,
        ),
        scenario(
            "p2pk_sigall_refund_multisig_2of2",
            scenario_p2pk_sigall_refund_multisig_2of2,
        ),
        scenario(
            "p2pk_sigall_output_amounts_swapped_fail",
            scenario_p2pk_sigall_output_amounts_swapped_fail,
        ),
        scenario(
            "htlc_sigall_preimage_only_fails",
            scenario_htlc_sigall_preimage_only_fails,
        ),
        scenario(
            "htlc_sigall_signature_only_fails",
            scenario_htlc_sigall_signature_only_fails,
        ),
        scenario(
            "htlc_sigall_requires_preimage_and_transaction_signature",
            scenario_htlc_sigall_requires_preimage_and_transaction_signature,
        ),
        scenario(
            "htlc_sigall_wrong_preimage_fails",
            scenario_htlc_sigall_wrong_preimage_fails,
        ),
        scenario(
            "htlc_sigall_locktime_after_expiry_refund_succeeds",
            scenario_htlc_sigall_locktime_after_expiry_refund_succeeds,
        ),
        scenario(
            "htlc_sigall_multisig_2of3",
            scenario_htlc_sigall_multisig_2of3,
        ),
        scenario(
            "htlc_sigall_receiver_path_after_locktime",
            scenario_htlc_sigall_receiver_path_after_locktime,
        ),
    ];

    let mut results = Vec::with_capacity(scenarios.len());

    for (name, scenario) in scenarios {
        results.push(run_named_scenario(name, &target, &mint_url, scenario).await);
    }

    print_results_table(&results);

    let report = Report {
        generated_at_unix_secs: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system clock before unix epoch")?
            .as_secs(),
        target,
        mint_url,
        results,
    };

    write_json_report(Path::new(DEFAULT_JSON_REPORT_PATH), &report).await?;
    mint.stop().await?;

    Ok(())
}

fn scenario<F, Fut>(
    name: &'static str,
    f: F,
) -> (
    &'static str,
    Box<dyn FnOnce(String) -> ScenarioFuture + Send>,
)
where
    F: FnOnce(String) -> Fut + Send + 'static,
    Fut: Future<Output = Result<String>> + Send + 'static,
{
    (name, Box::new(move |mint_url| Box::pin(f(mint_url))))
}

async fn run_named_scenario(
    name: &str,
    target: &str,
    mint_url: &str,
    scenario: Box<dyn FnOnce(String) -> ScenarioFuture + Send>,
) -> ScenarioResult {
    let started = Instant::now();

    match scenario(mint_url.to_string()).await {
        Ok(note) => ScenarioResult {
            name: name.to_string(),
            target: target.to_string(),
            passed: true,
            duration_ms: started.elapsed().as_millis(),
            note,
        },
        Err(err) => ScenarioResult {
            name: name.to_string(),
            target: target.to_string(),
            passed: false,
            duration_ms: started.elapsed().as_millis(),
            note: err.to_string(),
        },
    }
}

impl LocalMintHandle {
    async fn start() -> Result<Self> {
        let port = reserve_local_port()?;
        let mint_url = format!("http://{DEFAULT_MINT_HOST}:{port}");
        let work_dir = create_temp_work_dir("compat-runner-cdk-mint")?;
        let settings = build_zero_fee_settings(&mint_url, port)?;
        let shutdown = CancellationToken::new();
        let shutdown_signal = shutdown.clone();
        let work_dir_for_task = work_dir.clone();

        let task = tokio::spawn(async move {
            cdk_mintd::run_mintd_with_shutdown(
                &work_dir_for_task,
                &settings,
                async move {
                    shutdown_signal.cancelled().await;
                },
                None,
                None,
                vec![],
            )
            .await
        });

        wait_for_mint_ready(&mint_url, Duration::from_secs(30)).await?;

        Ok(Self {
            mint_url,
            work_dir,
            shutdown,
            task,
        })
    }

    async fn stop(self) -> Result<()> {
        self.shutdown.cancel();

        match self.task.await {
            Ok(result) => result,
            Err(err) => Err(anyhow!("mint task join error: {err}")),
        }
    }
}

impl TestContext {
    async fn new(mint_url: &str) -> Result<Self> {
        let mint_url = MintUrl::from_str(mint_url)?;
        let seed = Mnemonic::generate(12)?.to_seed_normalized("");
        let localstore = Arc::new(cdk_sqlite::wallet::memory::empty().await?);
        let wallet = WalletBuilder::new()
            .mint_url(mint_url.clone())
            .unit(CurrencyUnit::Sat)
            .localstore(localstore)
            .seed(seed)
            .build()?;
        let client = HttpClient::new(mint_url, None);

        Ok(Self { wallet, client })
    }

    async fn with_funds(mint_url: &str, amount: Amount) -> Result<Self> {
        let ctx = Self::new(mint_url).await?;
        ctx.fund_wallet(amount).await?;
        Ok(ctx)
    }

    async fn fund_wallet(&self, amount: Amount) -> Result<()> {
        let quote = self
            .wallet
            .mint_quote(PaymentMethod::BOLT11, Some(amount), None, None)
            .await?;

        let proofs = self
            .wallet
            .proof_stream(quote, SplitTarget::default(), None)
            .next()
            .await
            .ok_or_else(|| anyhow!("proof stream ended before minting proofs"))??;

        let funded_amount = proofs.total_amount()?;
        if funded_amount != amount {
            return Err(anyhow!(
                "expected funded amount {amount}, got {funded_amount}"
            ));
        }

        Ok(())
    }

    async fn active_keyset_id(&self) -> Result<Id> {
        Ok(self.wallet.fetch_active_keyset().await?.id)
    }

    async fn active_keyset_keys(&self) -> Result<Keys> {
        let keyset_id = self.active_keyset_id().await?;
        Ok(self.client.get_mint_keyset(keyset_id).await?.keys)
    }

    async fn wallet_proofs(&self) -> Result<Vec<Proof>> {
        self.wallet.get_unspent_proofs().await.map_err(Into::into)
    }
}

fn standard_fee_and_amounts() -> FeeAndAmounts {
    (0, (0..32).map(|power| 2u64.pow(power)).collect::<Vec<_>>()).into()
}

fn create_test_keypair() -> Keypair {
    let secret = SecretKey::generate();
    let public = secret.public_key();
    Keypair { secret, public }
}

fn create_test_hash_and_preimage() -> HtlcFixture {
    use cdk::secp256k1::hashes::Hash;
    use cdk::secp256k1::hashes::sha256::Hash as Sha256Hash;

    let preimage_bytes = [0x42u8; 32];
    let hash = Sha256Hash::hash(&preimage_bytes);

    HtlcFixture {
        hash: hash.to_string(),
        preimage: cdk::util::hex::encode(preimage_bytes),
    }
}

fn spending_condition_secret(conditions: &SpendingConditions) -> Result<cdk::secret::Secret> {
    let secret: Nut10Secret = conditions.clone().into();
    let secret: cdk::secret::Secret = secret.try_into()?;
    Ok(secret)
}

fn premint_with_conditions_for_amounts(
    keyset_id: Id,
    amounts: Vec<Amount>,
    conditions: &SpendingConditions,
) -> Result<PreMintSecrets> {
    let mut blinded_messages = Vec::with_capacity(amounts.len());
    let mut secrets = Vec::with_capacity(amounts.len());
    let mut rs = Vec::with_capacity(amounts.len());

    for amount in amounts {
        let secret = spending_condition_secret(conditions)?;
        let (blinded, r) = blind_message(&secret.to_bytes(), None)?;
        blinded_messages.push(BlindedMessage::new(amount, keyset_id, blinded));
        secrets.push(secret);
        rs.push(r);
    }

    let premint = PreMintSecrets::from_secrets(
        keyset_id,
        blinded_messages.iter().map(|b| b.amount).collect(),
        secrets,
    )?;

    let blinded = premint.blinded_messages();
    if blinded.len() != rs.len() {
        return Err(anyhow!("premint secret/r count mismatch"));
    }

    Ok(premint)
}

async fn lock_proofs_with_conditions(
    ctx: &TestContext,
    input_amount: Amount,
    conditions: &SpendingConditions,
) -> Result<LockedProofs> {
    let keyset_id = ctx.active_keyset_id().await?;
    let amounts = split_amounts(input_amount);
    lock_proofs_with_conditions_and_amounts(ctx, input_amount, amounts, conditions, keyset_id).await
}

async fn lock_proofs_with_conditions_and_amounts(
    ctx: &TestContext,
    _input_amount: Amount,
    amounts: Vec<Amount>,
    conditions: &SpendingConditions,
    keyset_id: Id,
) -> Result<LockedProofs> {
    let input_proofs = ctx.wallet_proofs().await?;
    let keyset_keys = ctx.active_keyset_keys().await?;
    let premint = premint_with_conditions_for_amounts(keyset_id, amounts, conditions)?;

    let swap_request = SwapRequest::new(input_proofs, premint.blinded_messages().to_vec());
    let swap_response = ctx.client.post_swap(swap_request).await?;
    let proofs = construct_proofs(
        swap_response.signatures,
        premint.rs(),
        premint.secrets(),
        &keyset_keys,
    )?;

    Ok(LockedProofs { proofs, keyset_id })
}

fn split_amounts(amount: Amount) -> Vec<Amount> {
    let mut result = Vec::new();
    let mut remaining = amount.to_u64();

    for power in (0..32).rev() {
        let denom = 2u64.pow(power);
        if remaining >= denom {
            result.push(Amount::from(denom));
            remaining -= denom;
        }
    }

    result
}

fn random_outputs(keyset_id: Id, amount: Amount) -> Result<Vec<BlindedMessage>> {
    Ok(PreMintSecrets::random(
        keyset_id,
        amount,
        &SplitTarget::default(),
        &standard_fee_and_amounts(),
    )?
    .blinded_messages()
    .to_vec())
}

fn sign_all_inputs(proofs: &mut [Proof], signers: &[SecretKey]) -> Result<()> {
    for proof in proofs {
        for signer in signers {
            proof.sign_p2pk(signer.clone())?;
        }
    }

    Ok(())
}

fn add_preimage_and_sign_all_inputs(
    proofs: &mut [Proof],
    preimage: &str,
    signers: &[SecretKey],
) -> Result<()> {
    for proof in proofs {
        proof.add_preimage(preimage.to_string());
        for signer in signers {
            proof.sign_p2pk(signer.clone())?;
        }
    }

    Ok(())
}

fn add_empty_preimage_and_sign_all_inputs(
    proofs: &mut [Proof],
    signers: &[SecretKey],
) -> Result<()> {
    for proof in proofs {
        proof.add_preimage(String::new());
        for signer in signers {
            proof.sign_p2pk(signer.clone())?;
        }
    }

    Ok(())
}

fn expect_swap_failure(
    result: std::result::Result<cdk::nuts::SwapResponse, Error>,
    msg: &str,
) -> Result<String> {
    match result {
        Ok(_) => Err(anyhow!("{msg}: swap unexpectedly succeeded")),
        Err(err) => Ok(err.to_string()),
    }
}

fn expect_swap_success(
    result: std::result::Result<cdk::nuts::SwapResponse, Error>,
    msg: &str,
) -> Result<cdk::nuts::SwapResponse> {
    result.map_err(|err| anyhow!("{msg}: {err}"))
}

async fn scenario_p2pk_swap_unsigned_fails(mint_url: String) -> Result<String> {
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let locked = lock_proofs_with_conditions(
        &ctx,
        standard_input_amount(),
        &SpendingConditions::new_p2pk(alice.public, None),
    )
    .await?;

    let request = SwapRequest::new(
        locked.proofs,
        random_outputs(locked.keyset_id, standard_input_amount())?,
    );

    let error = expect_swap_failure(ctx.client.post_swap(request).await, "unsigned P2PK spend")?;
    Ok(format!("swap rejected as expected: {error}"))
}

async fn scenario_p2pk_partial_signatures_fail(mint_url: String) -> Result<String> {
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let locked = lock_proofs_with_conditions(
        &ctx,
        standard_input_amount(),
        &SpendingConditions::new_p2pk(alice.public, None),
    )
    .await?;

    let outputs = random_outputs(locked.keyset_id, standard_input_amount())?;
    let mut request = SwapRequest::new(locked.proofs, outputs);
    request.inputs_mut()[0].sign_p2pk(alice.secret)?;

    let error = expect_swap_failure(
        ctx.client.post_swap(request).await,
        "partial-signature P2PK spend",
    )?;
    Ok(format!("partial spend rejected: {error}"))
}

async fn scenario_p2pk_swap_signed_succeeds(mint_url: String) -> Result<String> {
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let locked = lock_proofs_with_conditions(
        &ctx,
        standard_input_amount(),
        &SpendingConditions::new_p2pk(alice.public, None),
    )
    .await?;

    let outputs = random_outputs(locked.keyset_id, standard_input_amount())?;
    let mut request = SwapRequest::new(locked.proofs, outputs);
    sign_all_inputs(request.inputs_mut(), &[alice.secret])?;

    let response = expect_swap_success(ctx.client.post_swap(request).await, "signed P2PK spend")?;
    Ok(format!(
        "swap succeeded with {} output signature(s)",
        response.signatures.len()
    ))
}

async fn scenario_p2pk_multisig_2of3(mint_url: String) -> Result<String> {
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let bob = create_test_keypair();
    let carol = create_test_keypair();
    let dave = create_test_keypair();
    let eve = create_test_keypair();
    let conditions = SpendingConditions::new_p2pk(
        alice.public,
        Some(Conditions::new(
            None,
            Some(vec![bob.public, carol.public]),
            None,
            Some(2),
            None,
            None,
        )?),
    );
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;

    let outputs = random_outputs(locked.keyset_id, standard_input_amount())?;

    let mut one_sig = SwapRequest::new(locked.proofs.clone(), outputs.clone());
    sign_all_inputs(one_sig.inputs_mut(), &[alice.secret.clone()])?;
    expect_swap_failure(ctx.client.post_swap(one_sig).await, "1-of-2 multisig")?;

    let mut invalid = SwapRequest::new(locked.proofs.clone(), outputs.clone());
    sign_all_inputs(
        invalid.inputs_mut(),
        &[dave.secret.clone(), eve.secret.clone()],
    )?;
    expect_swap_failure(ctx.client.post_swap(invalid).await, "invalid multisig")?;

    let mut valid = SwapRequest::new(locked.proofs, outputs);
    sign_all_inputs(valid.inputs_mut(), &[alice.secret, bob.secret])?;
    expect_swap_success(ctx.client.post_swap(valid).await, "valid multisig")?;
    Ok("2-of-3 multisig accepted only valid signer set".to_string())
}

async fn scenario_p2pk_locktime_before_expiry_primary_only(mint_url: String) -> Result<String> {
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let bob = create_test_keypair();
    let conditions = SpendingConditions::new_p2pk(
        alice.public,
        Some(Conditions::new(
            Some(cdk::util::unix_time() + 3600),
            None,
            Some(vec![bob.public]),
            None,
            None,
            None,
        )?),
    );
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let outputs = random_outputs(locked.keyset_id, standard_input_amount())?;

    let mut refund = SwapRequest::new(locked.proofs.clone(), outputs.clone());
    sign_all_inputs(refund.inputs_mut(), &[bob.secret.clone()])?;
    expect_swap_failure(ctx.client.post_swap(refund).await, "refund before locktime")?;

    let mut primary = SwapRequest::new(locked.proofs, outputs);
    sign_all_inputs(primary.inputs_mut(), &[alice.secret])?;
    expect_swap_success(
        ctx.client.post_swap(primary).await,
        "primary before locktime",
    )?;
    Ok("primary path works before locktime; refund path rejected".to_string())
}

async fn scenario_p2pk_locktime_after_expiry_primary_still_works(
    mint_url: String,
) -> Result<String> {
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let bob = create_test_keypair();
    let conditions = SpendingConditions::new_p2pk(
        alice.public,
        Some(Conditions {
            locktime: Some(cdk::util::unix_time() - 3600),
            pubkeys: None,
            refund_keys: Some(vec![bob.public]),
            num_sigs: None,
            sig_flag: SigFlag::default(),
            num_sigs_refund: None,
        }),
    );
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let mut primary = SwapRequest::new(
        locked.proofs,
        random_outputs(locked.keyset_id, standard_input_amount())?,
    );
    sign_all_inputs(primary.inputs_mut(), &[alice.secret])?;
    expect_swap_success(
        ctx.client.post_swap(primary).await,
        "primary after locktime",
    )?;
    Ok("primary path still works after locktime".to_string())
}

async fn scenario_p2pk_locktime_after_expiry_no_refund_anyone_can_spend(
    mint_url: String,
) -> Result<String> {
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let conditions = SpendingConditions::new_p2pk(
        alice.public,
        Some(Conditions {
            locktime: Some(cdk::util::unix_time() - 3600),
            pubkeys: None,
            refund_keys: None,
            num_sigs: None,
            sig_flag: SigFlag::default(),
            num_sigs_refund: None,
        }),
    );
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let request = SwapRequest::new(
        locked.proofs,
        random_outputs(locked.keyset_id, standard_input_amount())?,
    );
    expect_swap_success(
        ctx.client.post_swap(request).await,
        "anyone-can-spend after locktime",
    )?;
    Ok("anyone-can-spend refund path worked after locktime".to_string())
}

async fn scenario_p2pk_multisig_locktime_primary_still_works(mint_url: String) -> Result<String> {
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let bob = create_test_keypair();
    let carol = create_test_keypair();
    let dave = create_test_keypair();
    let eve = create_test_keypair();
    let conditions = SpendingConditions::new_p2pk(
        alice.public,
        Some(Conditions {
            locktime: Some(cdk::util::unix_time() - 100),
            pubkeys: Some(vec![bob.public, carol.public]),
            refund_keys: Some(vec![dave.public, eve.public]),
            num_sigs: Some(2),
            sig_flag: SigFlag::default(),
            num_sigs_refund: Some(1),
        }),
    );
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let mut request = SwapRequest::new(
        locked.proofs,
        random_outputs(locked.keyset_id, standard_input_amount())?,
    );
    sign_all_inputs(request.inputs_mut(), &[alice.secret, bob.secret])?;
    expect_swap_success(
        ctx.client.post_swap(request).await,
        "multisig primary after locktime",
    )?;
    Ok("primary multisig still works after locktime".to_string())
}

async fn scenario_p2pk_wrong_signer_fails(mint_url: String) -> Result<String> {
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let bob = create_test_keypair();
    let locked = lock_proofs_with_conditions(
        &ctx,
        standard_input_amount(),
        &SpendingConditions::new_p2pk(alice.public, None),
    )
    .await?;
    let mut request = SwapRequest::new(
        locked.proofs,
        random_outputs(locked.keyset_id, standard_input_amount())?,
    );
    sign_all_inputs(request.inputs_mut(), &[bob.secret])?;
    let error = expect_swap_failure(ctx.client.post_swap(request).await, "wrong signer")?;
    Ok(format!("wrong signer rejected: {error}"))
}

async fn scenario_p2pk_duplicate_signatures_fail(mint_url: String) -> Result<String> {
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let bob = create_test_keypair();
    let conditions = SpendingConditions::new_p2pk(
        alice.public,
        Some(Conditions::new(
            None,
            Some(vec![bob.public]),
            None,
            Some(2),
            None,
            None,
        )?),
    );
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let mut request = SwapRequest::new(
        locked.proofs,
        random_outputs(locked.keyset_id, standard_input_amount())?,
    );
    sign_all_inputs(request.inputs_mut(), &[alice.secret.clone(), alice.secret])?;
    let error = expect_swap_failure(ctx.client.post_swap(request).await, "duplicate signatures")?;
    Ok(format!("duplicate signatures rejected: {error}"))
}

async fn scenario_htlc_preimage_only_fails(mint_url: String) -> Result<String> {
    let fixture = create_test_hash_and_preimage();
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let conditions = SpendingConditions::new_htlc_hash(
        &fixture.hash,
        Some(Conditions {
            locktime: None,
            pubkeys: Some(vec![alice.public]),
            refund_keys: None,
            num_sigs: None,
            sig_flag: SigFlag::default(),
            num_sigs_refund: None,
        }),
    )?;
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let mut request = SwapRequest::new(
        locked.proofs,
        random_outputs(locked.keyset_id, standard_input_amount())?,
    );

    for proof in request.inputs_mut() {
        proof.add_preimage(fixture.preimage.clone());
    }

    let error = expect_swap_failure(ctx.client.post_swap(request).await, "HTLC preimage-only")?;
    Ok(format!("preimage-only HTLC spend rejected: {error}"))
}

async fn scenario_htlc_signature_only_fails(mint_url: String) -> Result<String> {
    let fixture = create_test_hash_and_preimage();
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let conditions = SpendingConditions::new_htlc_hash(
        &fixture.hash,
        Some(Conditions {
            locktime: None,
            pubkeys: Some(vec![alice.public]),
            refund_keys: None,
            num_sigs: None,
            sig_flag: SigFlag::default(),
            num_sigs_refund: None,
        }),
    )?;
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let mut request = SwapRequest::new(
        locked.proofs,
        random_outputs(locked.keyset_id, standard_input_amount())?,
    );
    sign_all_inputs(request.inputs_mut(), &[alice.secret])?;
    let error = expect_swap_failure(ctx.client.post_swap(request).await, "HTLC signature-only")?;
    Ok(format!("signature-only HTLC spend rejected: {error}"))
}

async fn scenario_htlc_swap_preimage_and_signature_succeeds(mint_url: String) -> Result<String> {
    let fixture = create_test_hash_and_preimage();
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let conditions = SpendingConditions::new_htlc_hash(
        &fixture.hash,
        Some(Conditions {
            locktime: None,
            pubkeys: Some(vec![alice.public]),
            refund_keys: None,
            num_sigs: None,
            sig_flag: SigFlag::default(),
            num_sigs_refund: None,
        }),
    )?;
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let mut request = SwapRequest::new(
        locked.proofs,
        random_outputs(locked.keyset_id, standard_input_amount())?,
    );
    add_preimage_and_sign_all_inputs(request.inputs_mut(), &fixture.preimage, &[alice.secret])?;
    let response = expect_swap_success(ctx.client.post_swap(request).await, "HTLC valid spend")?;
    Ok(format!(
        "HTLC swap succeeded with {} output signature(s)",
        response.signatures.len()
    ))
}

async fn scenario_htlc_wrong_preimage_fails(mint_url: String) -> Result<String> {
    let fixture = create_test_hash_and_preimage();
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let conditions = SpendingConditions::new_htlc_hash(
        &fixture.hash,
        Some(Conditions {
            locktime: None,
            pubkeys: Some(vec![alice.public]),
            refund_keys: None,
            num_sigs: None,
            sig_flag: SigFlag::default(),
            num_sigs_refund: None,
        }),
    )?;
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let mut request = SwapRequest::new(
        locked.proofs,
        random_outputs(locked.keyset_id, standard_input_amount())?,
    );
    add_preimage_and_sign_all_inputs(
        request.inputs_mut(),
        "this_is_the_wrong_preimage",
        &[alice.secret],
    )?;
    let error = expect_swap_failure(ctx.client.post_swap(request).await, "wrong HTLC preimage")?;
    Ok(format!("wrong HTLC preimage rejected: {error}"))
}

async fn scenario_htlc_locktime_after_expiry_refund_succeeds(mint_url: String) -> Result<String> {
    let fixture = create_test_hash_and_preimage();
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let bob = create_test_keypair();
    let conditions = SpendingConditions::new_htlc_hash(
        &fixture.hash,
        Some(Conditions {
            locktime: Some(cdk::util::unix_time() - 1000),
            pubkeys: Some(vec![alice.public]),
            refund_keys: Some(vec![bob.public]),
            num_sigs: None,
            sig_flag: SigFlag::default(),
            num_sigs_refund: None,
        }),
    )?;
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let mut request = SwapRequest::new(
        locked.proofs,
        random_outputs(locked.keyset_id, standard_input_amount())?,
    );
    add_empty_preimage_and_sign_all_inputs(request.inputs_mut(), &[bob.secret])?;
    expect_swap_success(
        ctx.client.post_swap(request).await,
        "HTLC refund after locktime",
    )?;
    Ok("HTLC refund path worked after locktime".to_string())
}

async fn scenario_htlc_multisig_2of3(mint_url: String) -> Result<String> {
    let fixture = create_test_hash_and_preimage();
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let bob = create_test_keypair();
    let charlie = create_test_keypair();
    let conditions = SpendingConditions::new_htlc_hash(
        &fixture.hash,
        Some(Conditions {
            locktime: None,
            pubkeys: Some(vec![alice.public, bob.public, charlie.public]),
            refund_keys: None,
            num_sigs: Some(2),
            sig_flag: SigFlag::default(),
            num_sigs_refund: None,
        }),
    )?;
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let outputs = random_outputs(locked.keyset_id, standard_input_amount())?;

    let mut one_sig = SwapRequest::new(locked.proofs.clone(), outputs.clone());
    add_preimage_and_sign_all_inputs(
        one_sig.inputs_mut(),
        &fixture.preimage,
        &[alice.secret.clone()],
    )?;
    expect_swap_failure(ctx.client.post_swap(one_sig).await, "HTLC 1-of-3")?;

    let mut two_sig = SwapRequest::new(locked.proofs, outputs);
    add_preimage_and_sign_all_inputs(
        two_sig.inputs_mut(),
        &fixture.preimage,
        &[alice.secret, bob.secret],
    )?;
    expect_swap_success(ctx.client.post_swap(two_sig).await, "HTLC 2-of-3")?;
    Ok("HTLC 2-of-3 multisig enforced correctly".to_string())
}

async fn scenario_htlc_receiver_path_after_locktime(mint_url: String) -> Result<String> {
    let fixture = create_test_hash_and_preimage();
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let bob = create_test_keypair();
    let conditions = SpendingConditions::new_htlc_hash(
        &fixture.hash,
        Some(Conditions {
            locktime: Some(cdk::util::unix_time() - 1000),
            pubkeys: Some(vec![alice.public]),
            refund_keys: Some(vec![bob.public]),
            num_sigs: None,
            sig_flag: SigFlag::default(),
            num_sigs_refund: None,
        }),
    )?;
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let mut request = SwapRequest::new(
        locked.proofs,
        random_outputs(locked.keyset_id, standard_input_amount())?,
    );
    add_preimage_and_sign_all_inputs(request.inputs_mut(), &fixture.preimage, &[alice.secret])?;
    expect_swap_success(
        ctx.client.post_swap(request).await,
        "HTLC receiver path after locktime",
    )?;
    Ok("HTLC receiver path remains valid after locktime".to_string())
}

async fn scenario_p2pk_sigall_requires_transaction_signature(mint_url: String) -> Result<String> {
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let conditions = SpendingConditions::new_p2pk(
        alice.public,
        Some(Conditions::new(
            None,
            None,
            None,
            None,
            Some(SigFlag::SigAll),
            None,
        )?),
    );
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let request = SwapRequest::new(
        locked.proofs,
        random_outputs(locked.keyset_id, standard_input_amount())?,
    );
    let error = expect_swap_failure(ctx.client.post_swap(request).await, "SIG_ALL unsigned")?;
    Ok(format!("SIG_ALL rejected unsigned spend: {error}"))
}

async fn scenario_p2pk_sigall_sig_inputs_fail(mint_url: String) -> Result<String> {
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let conditions = SpendingConditions::new_p2pk(
        alice.public,
        Some(Conditions::new(
            None,
            None,
            None,
            None,
            Some(SigFlag::SigAll),
            None,
        )?),
    );
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let mut request = SwapRequest::new(
        locked.proofs,
        random_outputs(locked.keyset_id, standard_input_amount())?,
    );
    sign_all_inputs(request.inputs_mut(), &[alice.secret])?;
    let error = expect_swap_failure(
        ctx.client.post_swap(request).await,
        "SIG_INPUTS used for SIG_ALL",
    )?;
    Ok(format!(
        "SIG_INPUTS signatures rejected for SIG_ALL: {error}"
    ))
}

async fn scenario_p2pk_sigall_multisig_2of3(mint_url: String) -> Result<String> {
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let bob = create_test_keypair();
    let carol = create_test_keypair();
    let dave = create_test_keypair();
    let eve = create_test_keypair();
    let conditions = SpendingConditions::new_p2pk(
        alice.public,
        Some(Conditions::new(
            None,
            Some(vec![bob.public, carol.public]),
            None,
            Some(2),
            Some(SigFlag::SigAll),
            None,
        )?),
    );
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let outputs = random_outputs(locked.keyset_id, standard_input_amount())?;

    let mut one_sig = SwapRequest::new(locked.proofs.clone(), outputs.clone());
    one_sig.sign_sig_all(alice.secret.clone())?;
    expect_swap_failure(ctx.client.post_swap(one_sig).await, "SIG_ALL 1-of-3")?;

    let mut invalid = SwapRequest::new(locked.proofs.clone(), outputs.clone());
    invalid.sign_sig_all(dave.secret.clone())?;
    invalid.sign_sig_all(eve.secret.clone())?;
    expect_swap_failure(
        ctx.client.post_swap(invalid).await,
        "SIG_ALL invalid signers",
    )?;

    let mut valid = SwapRequest::new(locked.proofs, outputs);
    valid.sign_sig_all(alice.secret)?;
    valid.sign_sig_all(bob.secret)?;
    expect_swap_success(ctx.client.post_swap(valid).await, "SIG_ALL valid 2-of-3")?;
    Ok("SIG_ALL 2-of-3 multisig enforced correctly".to_string())
}

async fn scenario_p2pk_sigall_wrong_signer_fails(mint_url: String) -> Result<String> {
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let bob = create_test_keypair();
    let conditions = SpendingConditions::new_p2pk(
        alice.public,
        Some(Conditions::new(
            None,
            None,
            None,
            None,
            Some(SigFlag::SigAll),
            None,
        )?),
    );
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let mut request = SwapRequest::new(
        locked.proofs,
        random_outputs(locked.keyset_id, standard_input_amount())?,
    );
    request.sign_sig_all(bob.secret)?;
    let error = expect_swap_failure(ctx.client.post_swap(request).await, "SIG_ALL wrong signer")?;
    Ok(format!("wrong SIG_ALL signer rejected: {error}"))
}

async fn scenario_p2pk_sigall_duplicate_signatures_fail(mint_url: String) -> Result<String> {
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let bob = create_test_keypair();
    let conditions = SpendingConditions::new_p2pk(
        alice.public,
        Some(Conditions::new(
            None,
            Some(vec![bob.public]),
            None,
            Some(2),
            Some(SigFlag::SigAll),
            None,
        )?),
    );
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let mut request = SwapRequest::new(
        locked.proofs,
        random_outputs(locked.keyset_id, standard_input_amount())?,
    );
    request.sign_sig_all(alice.secret.clone())?;
    request.sign_sig_all(alice.secret)?;
    let error = expect_swap_failure(
        ctx.client.post_swap(request).await,
        "SIG_ALL duplicate signatures",
    )?;
    Ok(format!("duplicate SIG_ALL signatures rejected: {error}"))
}

async fn scenario_p2pk_sigall_locktime_before_expiry_primary_only(
    mint_url: String,
) -> Result<String> {
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let bob = create_test_keypair();
    let conditions = SpendingConditions::new_p2pk(
        alice.public,
        Some(Conditions::new(
            Some(cdk::util::unix_time() + 3600),
            None,
            Some(vec![bob.public]),
            None,
            Some(SigFlag::SigAll),
            None,
        )?),
    );
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let outputs = random_outputs(locked.keyset_id, standard_input_amount())?;

    let mut refund = SwapRequest::new(locked.proofs.clone(), outputs.clone());
    refund.sign_sig_all(bob.secret.clone())?;
    expect_swap_failure(
        ctx.client.post_swap(refund).await,
        "SIG_ALL refund before locktime",
    )?;

    let mut primary = SwapRequest::new(locked.proofs, outputs);
    primary.sign_sig_all(alice.secret)?;
    expect_swap_success(
        ctx.client.post_swap(primary).await,
        "SIG_ALL primary before locktime",
    )?;
    Ok("SIG_ALL primary path works before locktime; refund path rejected".to_string())
}

async fn scenario_p2pk_sigall_locktime_after_expiry_primary_still_works(
    mint_url: String,
) -> Result<String> {
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let bob = create_test_keypair();
    let conditions = SpendingConditions::new_p2pk(
        alice.public,
        Some(Conditions {
            locktime: Some(cdk::util::unix_time() - 3600),
            pubkeys: None,
            refund_keys: Some(vec![bob.public]),
            num_sigs: None,
            sig_flag: SigFlag::SigAll,
            num_sigs_refund: None,
        }),
    );
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let mut primary = SwapRequest::new(
        locked.proofs,
        random_outputs(locked.keyset_id, standard_input_amount())?,
    );
    primary.sign_sig_all(alice.secret)?;
    expect_swap_success(
        ctx.client.post_swap(primary).await,
        "SIG_ALL primary after locktime",
    )?;
    Ok("SIG_ALL primary path still works after locktime".to_string())
}

async fn scenario_p2pk_sigall_locktime_after_expiry_no_refund_anyone_can_spend(
    mint_url: String,
) -> Result<String> {
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let conditions = SpendingConditions::new_p2pk(
        alice.public,
        Some(Conditions {
            locktime: Some(cdk::util::unix_time() - 3600),
            pubkeys: None,
            refund_keys: None,
            num_sigs: None,
            sig_flag: SigFlag::SigAll,
            num_sigs_refund: None,
        }),
    );
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let request = SwapRequest::new(
        locked.proofs,
        random_outputs(locked.keyset_id, standard_input_amount())?,
    );
    expect_swap_success(
        ctx.client.post_swap(request).await,
        "SIG_ALL anyone-can-spend",
    )?;
    Ok("SIG_ALL anyone-can-spend refund path worked after locktime".to_string())
}

async fn scenario_p2pk_sigall_multisig_locktime_primary_still_works(
    mint_url: String,
) -> Result<String> {
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let bob = create_test_keypair();
    let carol = create_test_keypair();
    let dave = create_test_keypair();
    let eve = create_test_keypair();
    let conditions = SpendingConditions::new_p2pk(
        alice.public,
        Some(Conditions {
            locktime: Some(cdk::util::unix_time() - 100),
            pubkeys: Some(vec![bob.public, carol.public]),
            refund_keys: Some(vec![dave.public, eve.public]),
            num_sigs: Some(2),
            sig_flag: SigFlag::SigAll,
            num_sigs_refund: Some(1),
        }),
    );
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let mut request = SwapRequest::new(
        locked.proofs,
        random_outputs(locked.keyset_id, standard_input_amount())?,
    );
    request.sign_sig_all(alice.secret)?;
    request.sign_sig_all(bob.secret)?;
    expect_swap_success(
        ctx.client.post_swap(request).await,
        "SIG_ALL primary multisig after locktime",
    )?;
    Ok("SIG_ALL primary multisig still works after locktime".to_string())
}

async fn scenario_p2pk_sigall_mixed_proofs_different_data_fail(mint_url: String) -> Result<String> {
    let ctx_alice = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let alice_conditions = SpendingConditions::new_p2pk(
        alice.public,
        Some(Conditions {
            locktime: None,
            pubkeys: None,
            refund_keys: None,
            num_sigs: None,
            sig_flag: SigFlag::SigAll,
            num_sigs_refund: None,
        }),
    );
    let alice_locked =
        lock_proofs_with_conditions(&ctx_alice, standard_input_amount(), &alice_conditions).await?;

    let ctx_bob = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let bob = create_test_keypair();
    let bob_conditions = SpendingConditions::new_p2pk(
        bob.public,
        Some(Conditions {
            locktime: None,
            pubkeys: None,
            refund_keys: None,
            num_sigs: None,
            sig_flag: SigFlag::SigAll,
            num_sigs_refund: None,
        }),
    );
    let bob_locked =
        lock_proofs_with_conditions(&ctx_bob, standard_input_amount(), &bob_conditions).await?;

    let mixed_amount = standard_input_amount() + standard_input_amount();
    let client = HttpClient::new(MintUrl::from_str(&mint_url)?, None);
    let keyset_id = ctx_alice.active_keyset_id().await?;
    let mut mixed_proofs = alice_locked.proofs.clone();
    mixed_proofs.extend(bob_locked.proofs.clone());
    let mut mixed_request =
        SwapRequest::new(mixed_proofs, random_outputs(keyset_id, mixed_amount)?);
    mixed_request.sign_sig_all(alice.secret.clone())?;
    mixed_request.sign_sig_all(bob.secret.clone())?;
    let error = expect_swap_failure(client.post_swap(mixed_request).await, "mixed SIG_ALL data")?;

    let mut alice_only = SwapRequest::new(
        alice_locked.proofs,
        random_outputs(alice_locked.keyset_id, standard_input_amount())?,
    );
    alice_only.sign_sig_all(alice.secret)?;
    expect_swap_success(
        ctx_alice.client.post_swap(alice_only).await,
        "alice-only SIG_ALL",
    )?;

    let mut bob_only = SwapRequest::new(
        bob_locked.proofs,
        random_outputs(bob_locked.keyset_id, standard_input_amount())?,
    );
    bob_only.sign_sig_all(bob.secret)?;
    expect_swap_success(ctx_bob.client.post_swap(bob_only).await, "bob-only SIG_ALL")?;

    Ok(format!("mixed SIG_ALL proofs rejected: {error}"))
}

async fn scenario_p2pk_sigall_multisig_before_locktime(mint_url: String) -> Result<String> {
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let bob = create_test_keypair();
    let carol = create_test_keypair();
    let dave = create_test_keypair();
    let eve = create_test_keypair();
    let conditions = SpendingConditions::new_p2pk(
        alice.public,
        Some(Conditions {
            locktime: Some(cdk::util::unix_time() + 3600),
            pubkeys: Some(vec![bob.public, carol.public]),
            refund_keys: Some(vec![dave.public, eve.public]),
            num_sigs: Some(2),
            sig_flag: SigFlag::SigAll,
            num_sigs_refund: Some(1),
        }),
    );
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let outputs = random_outputs(locked.keyset_id, standard_input_amount())?;

    let mut one_sig = SwapRequest::new(locked.proofs.clone(), outputs.clone());
    one_sig.sign_sig_all(alice.secret.clone())?;
    expect_swap_failure(
        ctx.client.post_swap(one_sig).await,
        "SIG_ALL 1-of-3 before locktime",
    )?;

    let mut two_sig = SwapRequest::new(locked.proofs, outputs);
    two_sig.sign_sig_all(alice.secret)?;
    two_sig.sign_sig_all(bob.secret)?;
    expect_swap_success(
        ctx.client.post_swap(two_sig).await,
        "SIG_ALL 2-of-3 before locktime",
    )?;
    Ok("SIG_ALL 2-of-3 primary multisig works before locktime".to_string())
}

async fn scenario_p2pk_sigall_more_signatures_than_required(mint_url: String) -> Result<String> {
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let bob = create_test_keypair();
    let carol = create_test_keypair();
    let conditions = SpendingConditions::new_p2pk(
        alice.public,
        Some(Conditions {
            locktime: None,
            pubkeys: Some(vec![bob.public, carol.public]),
            refund_keys: None,
            num_sigs: Some(2),
            sig_flag: SigFlag::SigAll,
            num_sigs_refund: None,
        }),
    );
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let mut request = SwapRequest::new(
        locked.proofs,
        random_outputs(locked.keyset_id, standard_input_amount())?,
    );
    request.sign_sig_all(alice.secret)?;
    request.sign_sig_all(bob.secret)?;
    request.sign_sig_all(carol.secret)?;
    expect_swap_success(
        ctx.client.post_swap(request).await,
        "extra valid signatures",
    )?;
    Ok("SIG_ALL accepted more valid signatures than required".to_string())
}

async fn scenario_p2pk_sigall_refund_multisig_2of2(mint_url: String) -> Result<String> {
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let dave = create_test_keypair();
    let eve = create_test_keypair();
    let conditions = SpendingConditions::new_p2pk(
        alice.public,
        Some(Conditions {
            locktime: Some(cdk::util::unix_time() - 3600),
            pubkeys: None,
            refund_keys: Some(vec![dave.public, eve.public]),
            num_sigs: None,
            sig_flag: SigFlag::SigAll,
            num_sigs_refund: Some(2),
        }),
    );
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let outputs = random_outputs(locked.keyset_id, standard_input_amount())?;

    let mut one = SwapRequest::new(locked.proofs.clone(), outputs.clone());
    one.sign_sig_all(dave.secret.clone())?;
    expect_swap_failure(ctx.client.post_swap(one).await, "1-of-2 refund multisig")?;

    let mut both = SwapRequest::new(locked.proofs, outputs);
    both.sign_sig_all(dave.secret)?;
    both.sign_sig_all(eve.secret)?;
    expect_swap_success(ctx.client.post_swap(both).await, "2-of-2 refund multisig")?;
    Ok("SIG_ALL 2-of-2 refund multisig enforced correctly".to_string())
}

async fn scenario_p2pk_sigall_output_amounts_swapped_fail(mint_url: String) -> Result<String> {
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let conditions = SpendingConditions::new_p2pk(
        alice.public,
        Some(Conditions::new(
            None,
            None,
            None,
            None,
            Some(SigFlag::SigAll),
            None,
        )?),
    );
    let keyset_id = ctx.active_keyset_id().await?;
    let locked = lock_proofs_with_conditions_and_amounts(
        &ctx,
        standard_input_amount(),
        vec![Amount::from(8), Amount::from(2)],
        &conditions,
        keyset_id,
    )
    .await?;

    let mut request = SwapRequest::new(
        locked.proofs,
        random_outputs(locked.keyset_id, standard_input_amount())?,
    );
    request.sign_sig_all(alice.secret)?;

    let outputs = request.outputs_mut();
    (outputs[0].amount, outputs[1].amount) = (outputs[1].amount, outputs[0].amount);

    let error = expect_swap_failure(
        ctx.client.post_swap(request.clone()).await,
        "tampered output amounts",
    )?;

    let outputs = request.outputs_mut();
    (outputs[0].amount, outputs[1].amount) = (outputs[1].amount, outputs[0].amount);
    expect_swap_success(
        ctx.client.post_swap(request).await,
        "restored original output amounts",
    )?;

    Ok(format!("tampered SIG_ALL outputs rejected: {error}"))
}

async fn scenario_htlc_sigall_preimage_only_fails(mint_url: String) -> Result<String> {
    let fixture = create_test_hash_and_preimage();
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let conditions = SpendingConditions::new_htlc_hash(
        &fixture.hash,
        Some(Conditions {
            locktime: None,
            pubkeys: Some(vec![alice.public]),
            refund_keys: None,
            num_sigs: None,
            sig_flag: SigFlag::SigAll,
            num_sigs_refund: None,
        }),
    )?;
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let mut request = SwapRequest::new(
        locked.proofs,
        random_outputs(locked.keyset_id, standard_input_amount())?,
    );
    request.inputs_mut()[0].add_preimage(fixture.preimage.clone());
    let error = expect_swap_failure(
        ctx.client.post_swap(request).await,
        "SIG_ALL HTLC preimage-only",
    )?;
    Ok(format!("SIG_ALL HTLC preimage-only rejected: {error}"))
}

async fn scenario_htlc_sigall_signature_only_fails(mint_url: String) -> Result<String> {
    let fixture = create_test_hash_and_preimage();
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let conditions = SpendingConditions::new_htlc_hash(
        &fixture.hash,
        Some(Conditions {
            locktime: None,
            pubkeys: Some(vec![alice.public]),
            refund_keys: None,
            num_sigs: None,
            sig_flag: SigFlag::SigAll,
            num_sigs_refund: None,
        }),
    )?;
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let mut request = SwapRequest::new(
        locked.proofs,
        random_outputs(locked.keyset_id, standard_input_amount())?,
    );
    request.inputs_mut()[0].add_preimage(String::new());
    request.sign_sig_all(alice.secret)?;
    let error = expect_swap_failure(
        ctx.client.post_swap(request).await,
        "SIG_ALL HTLC signature-only",
    )?;
    Ok(format!("SIG_ALL HTLC signature-only rejected: {error}"))
}

async fn scenario_htlc_sigall_requires_preimage_and_transaction_signature(
    mint_url: String,
) -> Result<String> {
    let fixture = create_test_hash_and_preimage();
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let conditions = SpendingConditions::new_htlc_hash(
        &fixture.hash,
        Some(Conditions {
            locktime: None,
            pubkeys: Some(vec![alice.public]),
            refund_keys: None,
            num_sigs: None,
            sig_flag: SigFlag::SigAll,
            num_sigs_refund: None,
        }),
    )?;
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let mut request = SwapRequest::new(
        locked.proofs,
        random_outputs(locked.keyset_id, standard_input_amount())?,
    );
    request.inputs_mut()[0].add_preimage(fixture.preimage);
    request.sign_sig_all(alice.secret)?;
    let response = expect_swap_success(
        ctx.client.post_swap(request).await,
        "SIG_ALL HTLC valid spend",
    )?;
    Ok(format!(
        "SIG_ALL HTLC swap succeeded with {} output signature(s)",
        response.signatures.len()
    ))
}

async fn scenario_htlc_sigall_wrong_preimage_fails(mint_url: String) -> Result<String> {
    let fixture = create_test_hash_and_preimage();
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let conditions = SpendingConditions::new_htlc_hash(
        &fixture.hash,
        Some(Conditions {
            locktime: None,
            pubkeys: Some(vec![alice.public]),
            refund_keys: None,
            num_sigs: None,
            sig_flag: SigFlag::SigAll,
            num_sigs_refund: None,
        }),
    )?;
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let mut request = SwapRequest::new(
        locked.proofs,
        random_outputs(locked.keyset_id, standard_input_amount())?,
    );
    request.inputs_mut()[0].add_preimage("this_is_the_wrong_preimage".to_string());
    request.sign_sig_all(alice.secret)?;
    let error = expect_swap_failure(
        ctx.client.post_swap(request).await,
        "SIG_ALL HTLC wrong preimage",
    )?;
    Ok(format!("wrong SIG_ALL HTLC preimage rejected: {error}"))
}

async fn scenario_htlc_sigall_locktime_after_expiry_refund_succeeds(
    mint_url: String,
) -> Result<String> {
    let fixture = create_test_hash_and_preimage();
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let bob = create_test_keypair();
    let conditions = SpendingConditions::new_htlc_hash(
        &fixture.hash,
        Some(Conditions {
            locktime: Some(cdk::util::unix_time() - 1000),
            pubkeys: Some(vec![alice.public]),
            refund_keys: Some(vec![bob.public]),
            num_sigs: None,
            sig_flag: SigFlag::SigAll,
            num_sigs_refund: None,
        }),
    )?;
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let mut request = SwapRequest::new(
        locked.proofs,
        random_outputs(locked.keyset_id, standard_input_amount())?,
    );
    request.inputs_mut()[0].add_preimage(String::new());
    request.sign_sig_all(bob.secret)?;
    expect_swap_success(
        ctx.client.post_swap(request).await,
        "SIG_ALL HTLC refund after locktime",
    )?;
    Ok("SIG_ALL HTLC refund path worked after locktime".to_string())
}

async fn scenario_htlc_sigall_multisig_2of3(mint_url: String) -> Result<String> {
    let fixture = create_test_hash_and_preimage();
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let bob = create_test_keypair();
    let charlie = create_test_keypair();
    let conditions = SpendingConditions::new_htlc_hash(
        &fixture.hash,
        Some(Conditions {
            locktime: None,
            pubkeys: Some(vec![alice.public, bob.public, charlie.public]),
            refund_keys: None,
            num_sigs: Some(2),
            sig_flag: SigFlag::SigAll,
            num_sigs_refund: None,
        }),
    )?;
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let outputs = random_outputs(locked.keyset_id, standard_input_amount())?;

    let mut one = SwapRequest::new(locked.proofs.clone(), outputs.clone());
    one.inputs_mut()[0].add_preimage(fixture.preimage.clone());
    one.sign_sig_all(alice.secret.clone())?;
    expect_swap_failure(ctx.client.post_swap(one).await, "SIG_ALL HTLC 1-of-3")?;

    let mut two = SwapRequest::new(locked.proofs, outputs);
    two.inputs_mut()[0].add_preimage(fixture.preimage);
    two.sign_sig_all(alice.secret)?;
    two.sign_sig_all(bob.secret)?;
    expect_swap_success(ctx.client.post_swap(two).await, "SIG_ALL HTLC 2-of-3")?;
    Ok("SIG_ALL HTLC 2-of-3 multisig enforced correctly".to_string())
}

async fn scenario_htlc_sigall_receiver_path_after_locktime(mint_url: String) -> Result<String> {
    let fixture = create_test_hash_and_preimage();
    let ctx = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let bob = create_test_keypair();
    let conditions = SpendingConditions::new_htlc_hash(
        &fixture.hash,
        Some(Conditions {
            locktime: Some(cdk::util::unix_time() - 1000),
            pubkeys: Some(vec![alice.public]),
            refund_keys: Some(vec![bob.public]),
            num_sigs: None,
            sig_flag: SigFlag::SigAll,
            num_sigs_refund: None,
        }),
    )?;
    let locked = lock_proofs_with_conditions(&ctx, standard_input_amount(), &conditions).await?;
    let mut request = SwapRequest::new(
        locked.proofs,
        random_outputs(locked.keyset_id, standard_input_amount())?,
    );
    request.inputs_mut()[0].add_preimage(fixture.preimage);
    request.sign_sig_all(alice.secret)?;
    expect_swap_success(
        ctx.client.post_swap(request).await,
        "SIG_ALL HTLC receiver after locktime",
    )?;
    Ok("SIG_ALL HTLC receiver path remains valid after locktime".to_string())
}

fn build_zero_fee_settings(mint_url: &str, port: u16) -> Result<Settings> {
    let mnemonic = Mnemonic::generate(12)?.to_string();

    Ok(Settings {
        info: Info {
            url: mint_url.to_string(),
            listen_host: DEFAULT_MINT_HOST.to_string(),
            listen_port: port,
            seed: None,
            mnemonic: Some(mnemonic),
            signatory_url: None,
            signatory_certs: None,
            input_fee_ppk: Some(0),
            use_keyset_v2: None,
            http_cache: Default::default(),
            logging: Default::default(),
            enable_swagger_ui: None,
            enable_info_page: Some(false),
            quote_ttl: None,
        },
        mint_info: Default::default(),
        ln: Ln {
            ln_backend: LnBackend::FakeWallet,
            invoice_description: None,
            min_mint: Amount::ONE,
            max_mint: Amount::from(500_000),
            min_melt: Amount::ONE,
            max_melt: Amount::from(500_000),
        },
        limits: Default::default(),
        fake_wallet: Some(FakeWallet {
            supported_units: vec![CurrencyUnit::Sat],
            fee_percent: 0.0,
            reserve_fee_min: Amount::ZERO,
            min_delay_time: 0,
            max_delay_time: 0,
            keyset_rotations: Vec::new(),
        }),
        grpc_processor: None,
        database: Database {
            engine: DatabaseEngine::Sqlite,
            postgres: None,
        },
        auth_database: None,
        auth: None,
    })
}

fn reserve_local_port() -> Result<u16> {
    let listener = std::net::TcpListener::bind((DEFAULT_MINT_HOST, 0))
        .context("failed to reserve local port")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

async fn wait_for_mint_ready(mint_url: &str, timeout: Duration) -> Result<()> {
    let client = reqwest::Client::new();
    let endpoint = format!("{mint_url}/v1/info");
    let started = Instant::now();

    loop {
        if started.elapsed() > timeout {
            return Err(anyhow!("timed out waiting for mint at {endpoint}"));
        }

        match client.get(&endpoint).send().await {
            Ok(response) if response.status().is_success() => return Ok(()),
            Ok(_) | Err(_) => tokio::time::sleep(Duration::from_millis(200)).await,
        }
    }
}

fn create_temp_work_dir(prefix: &str) -> Result<PathBuf> {
    let unique = format!(
        "{}-{}-{}",
        prefix,
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system clock before unix epoch")?
            .as_nanos()
    );
    let path = std::env::temp_dir().join(unique);
    std::fs::create_dir_all(&path)?;
    Ok(path)
}

async fn write_json_report(path: &Path, report: &Report) -> Result<()> {
    let json = serde_json::to_string_pretty(report)?;
    tokio::fs::write(path, json).await?;
    Ok(())
}

fn print_results_table(results: &[ScenarioResult]) {
    let headers = ["Scenario", "Status", "Duration", "Note"];
    let mut rows = Vec::with_capacity(results.len());

    for result in results {
        rows.push(vec![
            result.name.clone(),
            if result.passed {
                "PASS".to_string()
            } else {
                "FAIL".to_string()
            },
            format!("{} ms", result.duration_ms),
            result.note.clone(),
        ]);
    }

    let mut widths = headers.map(str::len);
    for row in &rows {
        for (index, value) in row.iter().enumerate() {
            widths[index] = widths[index].max(value.len());
        }
    }

    println!(
        "{:<w0$}  {:<w1$}  {:<w2$}  {:<w3$}",
        headers[0],
        headers[1],
        headers[2],
        headers[3],
        w0 = widths[0],
        w1 = widths[1],
        w2 = widths[2],
        w3 = widths[3],
    );
    println!(
        "{:-<w0$}  {:-<w1$}  {:-<w2$}  {:-<w3$}",
        "",
        "",
        "",
        "",
        w0 = widths[0],
        w1 = widths[1],
        w2 = widths[2],
        w3 = widths[3],
    );

    for row in rows {
        println!(
            "{:<w0$}  {:<w1$}  {:<w2$}  {:<w3$}",
            row[0],
            row[1],
            row[2],
            row[3],
            w0 = widths[0],
            w1 = widths[1],
            w2 = widths[2],
            w3 = widths[3],
        );
    }
}

impl fmt::Debug for LocalMintHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalMintHandle")
            .field("mint_url", &self.mint_url)
            .field("work_dir", &self.work_dir)
            .finish_non_exhaustive()
    }
}
