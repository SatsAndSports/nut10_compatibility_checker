use std::fmt;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::str::FromStr;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow};
use bip39::Mnemonic;
use chrono::{DateTime, SecondsFormat, Utc};
use cdk::amount::{FeeAndAmounts, SplitTarget};
use cdk::dhke::{blind_message, construct_proofs};
use cdk::mint_url::MintUrl;
use cdk::nuts::nut10::Secret as Nut10Secret;
use cdk::nuts::{
    BlindedMessage, Conditions, CurrencyUnit, Id, Keys, MeltQuoteBolt11Request, MeltQuoteState,
    MeltRequest, MintQuoteState, MintRequest, P2PKWitness, PaymentMethod, PreMintSecrets, Proof,
    ProofsMethods, PublicKey, SecretKey, SigFlag, SpendingConditionVerification,
    SpendingConditions, SwapRequest, Witness,
};
use cdk::wallet::{HttpClient, MintConnector, Wallet, WalletBuilder};
use cdk::{Amount, Error, MeltQuoteCreateResponse, MeltQuoteRequest, MeltQuoteResponse, StreamExt};
use cdk_fake_wallet::create_fake_invoice;
use cdk_mintd::config::{Database, DatabaseEngine, FakeWallet, Info, Ln, LnBackend, Settings};
use clap::{Parser, ValueEnum};
use serde::Serialize;
use tokio_util::sync::CancellationToken;

const DEFAULT_MINT_HOST: &str = "127.0.0.1";
const MINT_STARTUP_ATTEMPTS: u8 = 5;
const MINT_STARTUP_TIMEOUT_SECS: u64 = 5;

const EXPECT_SIGNATURE_INVALID: &[&str] = &[
    "Signature missing or invalid",
    "signature threshold not met",
    "Witness is missing for p2pk signature",
    "no witness in proof",
];
const EXPECT_WITNESS_NO_SIGNATURES: &[&str] = &[
    "Witness did not provide signatures",
    "no signatures in proof",
];
const EXPECT_SIGALL_WITNESS_NO_SIGNATURES: &[&str] = &[
    "Witness signatures not provided",
    "Witness did not provide signatures",
    "Witness is missing for htlc preimage",
];
const EXPECT_NOT_HTLC_SECRET: &[&str] =
    &["Secret is not a HTLC secret", "no HTLC preimage provided"];
const EXPECT_PREIMAGE_INVALID_HEX: &[&str] = &[
    "Preimage must be valid hex encoding",
    "HTLC preimage must be 64 characters hex.",
];
const EXPECT_HTLC_SPEND_NOT_MET: &[&str] = &[
    "HTLC spend conditions are not met",
    "P2PK spend conditions are not met",
    "no HTLC preimage provided",
    "Witness is missing for htlc preimage",
    "not enough pubkeys",
    "HTLC preimage must be 64 characters hex.",
];
const EXPECT_P2PK_OR_SIGNATURE_FAILURE: &[&str] = &[
    "P2PK spend conditions are not met",
    "Signature missing or invalid",
    "signature threshold not met",
    "not enough pubkeys",
    "Witness is missing for p2pk signature",
    "no witness in proof",
];
const EXPECT_SIGALL_INPUT_MISMATCH: &[&str] = &[
    "Spend conditions are not met",
    "not all secrets are equal.",
    "Witness is missing for htlc preimage",
];
const SCENARIO_COLUMN_WIDTH: usize =
    "p2pk_sigall_locktime_after_expiry_no_refund_anyone_can_spend".len();
const STATUS_COLUMN_WIDTH: usize = 6;
const DURATION_COLUMN_WIDTH: usize = 8;

fn standard_input_amount() -> Amount {
    Amount::from(10)
}

type ScenarioFuture = Pin<Box<dyn Future<Output = Result<String>> + Send>>;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
enum ScenarioStatus {
    Pass,
    Fail,
}

#[derive(Debug, Clone, Serialize)]
struct ScenarioResult {
    name: String,
    target: String,
    status: ScenarioStatus,
    duration_ms: u128,
    note: String,
}

#[derive(Debug, Clone, Serialize)]
struct MintMetadata {
    name: Option<String>,
    version: Option<String>,
}

#[derive(Debug, Serialize)]
struct Report {
    generated_at_unix_secs: u64,
    generated_at_utc: String,
    target: String,
    mint_url: String,
    mint: MintMetadata,
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
    manual_http_funding: bool,
    funded_proofs: Arc<Mutex<Vec<Proof>>>,
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

#[derive(Clone)]
struct MeltQuoteInfo {
    quote_id: String,
    amount: Amount,
    fee_reserve: Amount,
}

#[derive(Debug, Clone)]
struct ParsedProtocolError {
    http_status: Option<u16>,
    code: Option<i64>,
    detail: Option<String>,
}

#[derive(Clone)]
struct TargetProfile {
    name: String,
    mint_url: String,
    manual_http_funding: bool,
    relaxed_external_negative_errors: bool,
}

#[derive(Debug, Clone)]
struct RunMetadata {
    generated_at_unix_secs: u64,
    generated_at_utc: String,
    mint: MintMetadata,
}

static TARGET_PROFILE: OnceLock<TargetProfile> = OnceLock::new();
static SIGALL_MODE: OnceLock<SigAllMode> = OnceLock::new();

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Suite {
    Swap,
    Melt,
    All,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum SigAllMode {
    Standard,
    Legacy,
}

#[derive(Debug, Parser)]
struct Args {
    /// External mint URL. If omitted, the runner starts an embedded local CDK mint.
    #[arg(long)]
    mint_url: Option<String>,

    /// Report file name written to ../reports/<report-name>.json.
    #[arg(long)]
    report_name: Option<String>,

    /// Which scenario set to run.
    #[arg(long, value_enum, default_value_t = Suite::All)]
    suite: Suite,

    /// SIG_ALL signing mode: standard CDK/spec or legacy Nutshell-style.
    #[arg(long, value_enum, default_value_t = SigAllMode::Standard)]
    sigall_mode: SigAllMode,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let embedded_mint = if args.mint_url.is_none() {
        Some(LocalMintHandle::start().await?)
    } else {
        None
    };
    let target = match (&args.mint_url, &embedded_mint) {
        (Some(mint_url), _) => TargetProfile {
            name: "external".to_string(),
            mint_url: mint_url.clone(),
            manual_http_funding: true,
            relaxed_external_negative_errors: true,
        },
        (None, Some(mint)) => TargetProfile {
            name: "cdk".to_string(),
            mint_url: mint.mint_url.clone(),
            manual_http_funding: false,
            relaxed_external_negative_errors: false,
        },
        (None, None) => return Err(anyhow!("internal error: no mint target available")),
    };
    let _ = TARGET_PROFILE.set(target.clone());
    let _ = SIGALL_MODE.set(args.sigall_mode);
    let run_metadata = fetch_run_metadata(&target).await?;

    let suite_result: Result<Report> = async {
        print_run_metadata(&target, &run_metadata);
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
                "p2pk_sigall_mixed_proofs_different_kind_fail",
                scenario_p2pk_sigall_mixed_proofs_different_kind_fail,
            ),
            scenario(
                "p2pk_sigall_mixed_proofs_different_tags_fail",
                scenario_p2pk_sigall_mixed_proofs_different_tags_fail,
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
            scenario(
                "melt_p2pk_unsigned_fails",
                scenario_melt_p2pk_unsigned_fails,
            ),
            scenario(
                "melt_p2pk_signed_succeeds",
                scenario_melt_p2pk_signed_succeeds,
            ),
            scenario(
                "melt_htlc_preimage_only_fails",
                scenario_melt_htlc_preimage_only_fails,
            ),
            scenario(
                "melt_htlc_signature_only_fails",
                scenario_melt_htlc_signature_only_fails,
            ),
            scenario(
                "melt_htlc_preimage_and_signature_succeeds",
                scenario_melt_htlc_preimage_and_signature_succeeds,
            ),
            scenario(
                "melt_p2pk_sigall_unsigned_fails",
                scenario_melt_p2pk_sigall_unsigned_fails,
            ),
            scenario(
                "melt_p2pk_sigall_sig_inputs_fail",
                scenario_melt_p2pk_sigall_sig_inputs_fail,
            ),
            scenario(
                "melt_p2pk_sigall_transaction_signature_succeeds",
                scenario_melt_p2pk_sigall_transaction_signature_succeeds,
            ),
            scenario(
                "melt_htlc_sigall_preimage_only_fails",
                scenario_melt_htlc_sigall_preimage_only_fails,
            ),
            scenario(
                "melt_htlc_sigall_sig_inputs_fail",
                scenario_melt_htlc_sigall_sig_inputs_fail,
            ),
            scenario(
                "melt_htlc_sigall_preimage_and_transaction_signature_succeeds",
                scenario_melt_htlc_sigall_preimage_and_transaction_signature_succeeds,
            ),
            scenario(
                "melt_p2pk_post_locktime_anyone_can_spend",
                scenario_melt_p2pk_post_locktime_anyone_can_spend,
            ),
            scenario(
                "melt_p2pk_before_locktime_wrong_key_fails",
                scenario_melt_p2pk_before_locktime_wrong_key_fails,
            ),
            scenario(
                "melt_p2pk_before_locktime_correct_key_succeeds",
                scenario_melt_p2pk_before_locktime_correct_key_succeeds,
            ),
        ];

        let mut results = Vec::with_capacity(scenarios.len());
        print_results_header();

        for (name, scenario) in scenarios {
            if scenario_in_suite(name, args.suite) {
                let result = run_named_scenario(name, &target, scenario).await;
                print_result_row(&result);
                results.push(result);
            }
        }

        let report = Report {
            generated_at_unix_secs: run_metadata.generated_at_unix_secs,
            generated_at_utc: run_metadata.generated_at_utc.clone(),
            target: target.name.clone(),
            mint_url: target.mint_url.clone(),
            mint: run_metadata.mint.clone(),
            results,
        };

        if let Some(report_name) = args.report_name.as_deref() {
            let report_path = report_path_for_name(report_name);
            write_json_report(&report_path, &report).await?;
            println!("Wrote JSON report: {}", report_path.display());
        }

        Ok(report)
    }
    .await;

    let stop_result = match embedded_mint {
        Some(mint) => mint.stop().await,
        None => Ok(()),
    };

    if let Err(stop_err) = stop_result {
        match suite_result {
            Ok(_) => return Err(stop_err),
            Err(run_err) => return Err(anyhow!("{run_err}; cleanup failed: {stop_err}")),
        }
    }

    let report = suite_result?;
    let failure_count = report
        .results
        .iter()
        .filter(|result| matches!(result.status, ScenarioStatus::Fail))
        .count();
    let pass_count = report
        .results
        .iter()
        .filter(|result| matches!(result.status, ScenarioStatus::Pass))
        .count();
    println!();
    println!(
        "Completed {} scenarios: {} pass, {} fail",
        report.results.len(),
        pass_count,
        failure_count
    );
    if failure_count > 0 {
        return Err(anyhow!("{failure_count} scenario(s) failed"));
    }

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
    target: &TargetProfile,
    scenario: Box<dyn FnOnce(String) -> ScenarioFuture + Send>,
) -> ScenarioResult {
    let started = Instant::now();

    match scenario(target.mint_url.clone()).await {
        Ok(note) => ScenarioResult {
            name: name.to_string(),
            target: target.name.clone(),
            status: ScenarioStatus::Pass,
            duration_ms: started.elapsed().as_millis(),
            note,
        },
        Err(err) => ScenarioResult {
            name: name.to_string(),
            target: target.name.clone(),
            status: ScenarioStatus::Fail,
            duration_ms: started.elapsed().as_millis(),
            note: err.to_string(),
        },
    }
}

fn scenario_in_suite(name: &str, suite: Suite) -> bool {
    match suite {
        Suite::Swap => !name.starts_with("melt_"),
        Suite::Melt => name.starts_with("melt_"),
        Suite::All => true,
    }
}

fn current_sigall_mode() -> SigAllMode {
    SIGALL_MODE.get().copied().unwrap_or(SigAllMode::Standard)
}

trait SigAllRequest: SpendingConditionVerification {
    fn first_input_mut(&mut self) -> Result<&mut Proof>;
    fn legacy_sig_all_msg_to_sign(&self) -> String;
}

impl SigAllRequest for SwapRequest {
    fn first_input_mut(&mut self) -> Result<&mut Proof> {
        self.inputs_mut()
            .first_mut()
            .ok_or_else(|| anyhow!("swap request has no inputs"))
    }

    fn legacy_sig_all_msg_to_sign(&self) -> String {
        let mut msg = String::new();
        for proof in self.inputs() {
            msg.push_str(&proof.secret.to_string());
        }
        for output in self.outputs() {
            msg.push_str(&output.blinded_secret.to_hex());
        }
        msg
    }
}

impl SigAllRequest for MeltRequest<String> {
    fn first_input_mut(&mut self) -> Result<&mut Proof> {
        self.inputs_mut()
            .first_mut()
            .ok_or_else(|| anyhow!("melt request has no inputs"))
    }

    fn legacy_sig_all_msg_to_sign(&self) -> String {
        let mut msg = String::new();
        for proof in self.inputs() {
            msg.push_str(&proof.secret.to_string());
        }
        if let Some(outputs) = self.outputs() {
            for output in outputs {
                msg.push_str(&output.blinded_secret.to_hex());
            }
        }
        msg.push_str(self.quote());
        msg
    }
}

fn sign_sig_all_request<T>(request: &mut T, secret_key: SecretKey) -> Result<()>
where
    T: SigAllRequest,
{
    let message = match current_sigall_mode() {
        SigAllMode::Standard => request.sig_all_msg_to_sign(),
        SigAllMode::Legacy => request.legacy_sig_all_msg_to_sign(),
    };
    let signature = secret_key.sign(message.as_bytes())?;
    let first_input = request.first_input_mut()?;

    match first_input.witness.as_mut() {
        Some(witness) => {
            witness.add_signatures(vec![signature.to_string()]);
        }
        None => {
            let mut witness = Witness::P2PKWitness(P2PKWitness::default());
            witness.add_signatures(vec![signature.to_string()]);
            first_input.witness = Some(witness);
        }
    }

    Ok(())
}

impl LocalMintHandle {
    async fn start() -> Result<Self> {
        let mut last_error = String::new();

        for attempt in 1..=MINT_STARTUP_ATTEMPTS {
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

            match wait_for_mint_ready(&mint_url, Duration::from_secs(MINT_STARTUP_TIMEOUT_SECS))
                .await
            {
                Ok(()) => {
                    return Ok(Self {
                        mint_url,
                        work_dir,
                        shutdown,
                        task,
                    });
                }
                Err(err) => {
                    shutdown.cancel();
                    let task_result = task.await;
                    let _ = std::fs::remove_dir_all(&work_dir);
                    last_error =
                        format!("attempt {attempt} failed: {err}; task_result={task_result:?}");
                }
            }
        }

        Err(anyhow!(
            "failed to start local CDK mint after {MINT_STARTUP_ATTEMPTS} attempts: {last_error}"
        ))
    }

    async fn stop(self) -> Result<()> {
        self.shutdown.cancel();

        let task_result = match self.task.await {
            Ok(result) => result,
            Err(err) => Err(anyhow!("mint task join error: {err}")),
        };
        let cleanup_result = std::fs::remove_dir_all(&self.work_dir)
            .or_else(|err| match err.kind() {
                std::io::ErrorKind::NotFound => Ok(()),
                _ => Err(err),
            })
            .map_err(|err| {
                anyhow!(
                    "failed to remove temp work dir {}: {err}",
                    self.work_dir.display()
                )
            });

        match (task_result, cleanup_result) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(task_err), Ok(())) => Err(task_err),
            (Ok(()), Err(cleanup_err)) => Err(cleanup_err),
            (Err(task_err), Err(cleanup_err)) => Err(anyhow!("{task_err}; {cleanup_err}")),
        }
    }
}

async fn fetch_run_metadata(target: &TargetProfile) -> Result<RunMetadata> {
    let now = SystemTime::now();
    let generated_at_unix_secs = now
        .duration_since(UNIX_EPOCH)
        .context("system clock before unix epoch")?
        .as_secs();
    let generated_at_utc = DateTime::<Utc>::from(now).to_rfc3339_opts(SecondsFormat::Secs, true);

    let mint_url = MintUrl::from_str(&target.mint_url)?;
    let client = HttpClient::new(mint_url, None);
    let mint_info = client.get_mint_info().await?;
    let mint = MintMetadata {
        name: mint_info.name.clone(),
        version: mint_info.version.as_ref().map(ToString::to_string),
    };

    Ok(RunMetadata {
        generated_at_unix_secs,
        generated_at_utc,
        mint,
    })
}

fn print_run_metadata(target: &TargetProfile, metadata: &RunMetadata) {
    println!(
        "Version:    {}",
        metadata.mint.version.as_deref().unwrap_or("<unknown>")
    );
    println!("Target:     {}", target.name);
    println!("Mint URL:   {}", target.mint_url);
    if let Some(name) = metadata.mint.name.as_deref() {
        println!("Mint Name:  {}", name);
    }
    println!("Started At: {}", metadata.generated_at_utc);
    println!();
}

impl TestContext {
    async fn new(mint_url: &str) -> Result<Self> {
        let mint_url = MintUrl::from_str(mint_url)?;
        let manual_http_funding = TARGET_PROFILE
            .get()
            .map(|target| target.manual_http_funding)
            .unwrap_or(false);
        let seed = Mnemonic::generate(12)?.to_seed_normalized("");
        let localstore = Arc::new(cdk_sqlite::wallet::memory::empty().await?);
        let wallet = WalletBuilder::new()
            .mint_url(mint_url.clone())
            .unit(CurrencyUnit::Sat)
            .localstore(localstore)
            .seed(seed)
            .build()?;
        let client = HttpClient::new(mint_url, None);

        Ok(Self {
            wallet,
            client,
            manual_http_funding,
            funded_proofs: Arc::new(Mutex::new(Vec::new())),
        })
    }

    async fn with_funds(mint_url: &str, amount: Amount) -> Result<Self> {
        let ctx = Self::new(mint_url).await?;
        ctx.fund_wallet(amount).await?;
        Ok(ctx)
    }

    async fn fund_wallet(&self, amount: Amount) -> Result<()> {
        if self.manual_http_funding {
            let proofs = self.manual_http_fund_proofs(amount).await?;
            let funded_amount = proofs.total_amount()?;
            if funded_amount != amount {
                return Err(anyhow!(
                    "expected funded amount {amount}, got {funded_amount}"
                ));
            }
            let mut stored = self
                .funded_proofs
                .lock()
                .map_err(|_| anyhow!("funded proofs mutex poisoned"))?;
            stored.extend(proofs);
            return Ok(());
        }

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
        if self.manual_http_funding {
            let stored = self
                .funded_proofs
                .lock()
                .map_err(|_| anyhow!("funded proofs mutex poisoned"))?;
            return Ok(stored.clone());
        }

        self.wallet.get_unspent_proofs().await.map_err(Into::into)
    }

    async fn manual_http_fund_proofs(&self, amount: Amount) -> Result<Vec<Proof>> {
        let quote = self
            .wallet
            .mint_quote(PaymentMethod::BOLT11, Some(amount), None, None)
            .await?;

        let mut attempts = 0u8;
        loop {
            let current = self.wallet.check_mint_quote_status(&quote.id).await?;
            if matches!(current.state, MintQuoteState::Paid | MintQuoteState::Issued) {
                break;
            }

            attempts = attempts.saturating_add(1);
            if attempts > 100 {
                return Err(anyhow!(
                    "timed out waiting for external mint quote {} to become payable",
                    quote.id
                ));
            }

            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        let active_keyset_id = self.active_keyset_id().await?;
        let keyset_keys = self.active_keyset_keys().await?;
        let premint = PreMintSecrets::random(
            active_keyset_id,
            amount,
            &SplitTarget::default(),
            &standard_fee_and_amounts(),
        )?;

        let mut request = MintRequest {
            quote: quote.id.clone(),
            outputs: premint.blinded_messages(),
            signature: None,
        };

        request.sign(
            quote
                .secret_key
                .clone()
                .ok_or_else(|| anyhow!("mint quote missing signing key"))?,
        )?;

        let response = self
            .client
            .post_mint(&PaymentMethod::BOLT11, request)
            .await?;

        Ok(construct_proofs(
            response.signatures,
            premint.rs(),
            premint.secrets(),
            &keyset_keys,
        )?)
    }
}

fn report_path_for_name(report_name: &str) -> PathBuf {
    Path::new("..")
        .join("reports")
        .join(format!("{}.json", report_name))
}

fn standard_fee_and_amounts() -> FeeAndAmounts {
    (0, (0..32).map(|power| 2u64.pow(power)).collect::<Vec<_>>()).into()
}

fn test_bolt11_invoice() -> Result<cdk::Bolt11Invoice> {
    Ok(create_fake_invoice(
        10_000,
        format!(
            "compat-melt-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .context("system clock before unix epoch")?
                .as_nanos()
        ),
    ))
}

fn required_melt_input_amount(quote: &MeltQuoteInfo) -> Amount {
    quote.amount + quote.fee_reserve
}

fn successful_melt_input_amount(quote: &MeltQuoteInfo) -> Amount {
    // CDK fakewallet currently reports total_spent = amount + 1 unit during payment,
    // so successful melt flows need one extra unit beyond quote.amount + fee_reserve.
    required_melt_input_amount(quote) + Amount::ONE
}

async fn create_melt_quote(client: &HttpClient) -> Result<MeltQuoteInfo> {
    let request = MeltQuoteRequest::Bolt11(MeltQuoteBolt11Request {
        request: test_bolt11_invoice()?,
        unit: CurrencyUnit::Sat,
        options: None,
    });
    let quote = client.post_melt_quote(request).await?;

    Ok(MeltQuoteInfo {
        quote_id: quote.quote().clone(),
        amount: match &quote {
            MeltQuoteCreateResponse::Bolt11(r) => r.amount,
            MeltQuoteCreateResponse::Bolt12(r) => r.amount,
            MeltQuoteCreateResponse::Custom((_, r)) => r.amount,
        },
        fee_reserve: match &quote {
            MeltQuoteCreateResponse::Bolt11(r) => r.fee_reserve,
            MeltQuoteCreateResponse::Bolt12(r) => r.fee_reserve,
            MeltQuoteCreateResponse::Custom((_, r)) => r.fee_reserve,
        },
    })
}

fn melt_request_from_proofs(quote_id: String, proofs: Vec<Proof>) -> MeltRequest<String> {
    MeltRequest::new(quote_id, proofs, None)
}

async fn prepare_locked_melt_proofs(
    mint_url: &str,
    conditions: &SpendingConditions,
) -> Result<(TestContext, MeltQuoteInfo, Vec<Proof>)> {
    let ctx = TestContext::new(mint_url).await?;
    let quote = create_melt_quote(&ctx.client)
        .await
        .context("[quote] failed to create melt quote")?;
    let input_amount = successful_melt_input_amount(&quote);
    ctx.fund_wallet(input_amount)
        .await
        .context("[fund] failed to fund melt inputs")?;
    let locked = lock_proofs_with_conditions(&ctx, input_amount, conditions)
        .await
        .context("[fund] failed to prepare locked melt proofs")?;
    Ok((ctx, quote, locked.proofs))
}

async fn wait_for_melt_completion(
    client: &HttpClient,
    quote_id: &str,
    timeout: Duration,
) -> Result<MeltQuoteResponse<String>> {
    let started = Instant::now();

    loop {
        if started.elapsed() > timeout {
            return Err(anyhow!(
                "timed out waiting for melt quote {quote_id} to settle"
            ));
        }

        let response = client
            .get_melt_quote_status(PaymentMethod::BOLT11, quote_id)
            .await?;

        match response.state() {
            MeltQuoteState::Paid => return Ok(response),
            MeltQuoteState::Failed | MeltQuoteState::Unknown | MeltQuoteState::Unpaid => {
                return Err(anyhow!(
                    "melt quote {quote_id} settled unexpectedly with state {}",
                    response.state()
                ));
            }
            MeltQuoteState::Pending => {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    }
}

async fn post_melt_and_wait_for_success(
    client: &HttpClient,
    request: MeltRequest<String>,
    msg: &str,
) -> Result<MeltQuoteResponse<String>> {
    let initial = expect_melt_success(client.post_melt(&PaymentMethod::BOLT11, request).await, msg)
        .with_context(|| format!("[submit] {msg}"))?;

    match initial.state() {
        MeltQuoteState::Paid => Ok(initial),
        MeltQuoteState::Pending => {
            wait_for_melt_completion(client, initial.quote(), Duration::from_secs(5))
                .await
                .with_context(|| format!("[settle] {msg}"))
        }
        MeltQuoteState::Failed | MeltQuoteState::Unknown | MeltQuoteState::Unpaid => {
            Err(anyhow!("{msg}: unexpected melt state {}", initial.state()))
        }
    }
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

fn error_contains_any(err: &Error, expected_substrings: &[&str]) -> bool {
    let display = err.to_string();
    let debug = format!("{err:?}");

    expected_substrings
        .iter()
        .any(|substring| display.contains(substring) || debug.contains(substring))
}

fn parse_unknown_error_response(value: &str) -> ParsedProtocolError {
    let mut code = None;
    let mut detail = None;

    if let Some(rest) = value.strip_prefix("code: ") {
        let mut parts = rest.splitn(2, ", detail: ");
        if let Some(code_str) = parts.next() {
            code = code_str.trim().parse::<i64>().ok();
        }
        if let Some(detail_str) = parts.next() {
            detail = Some(detail_str.to_string());
        }
    }

    ParsedProtocolError {
        http_status: Some(400),
        code,
        detail,
    }
}

fn parse_http_error_body(status: Option<u16>, body: &str) -> Option<ParsedProtocolError> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    let code = value.get("code").and_then(serde_json::Value::as_i64);
    let detail = value
        .get("detail")
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string);

    Some(ParsedProtocolError {
        http_status: status,
        code,
        detail,
    })
}

fn parsed_protocol_error(err: &Error) -> Option<ParsedProtocolError> {
    match err {
        Error::UnknownErrorResponse(value) => Some(parse_unknown_error_response(value)),
        Error::HttpError(status, body) => parse_http_error_body(*status, body),
        Error::DHKE(cdk::dhke::Error::TokenNotVerified) => Some(ParsedProtocolError {
            http_status: Some(400),
            code: Some(10001),
            detail: Some(err.to_string()),
        }),
        Error::SignatureMissingOrInvalid => Some(ParsedProtocolError {
            http_status: Some(400),
            code: Some(20008),
            detail: Some(err.to_string()),
        }),
        _ => None,
    }
}

fn is_protocol_like_negative_error(err: &Error) -> bool {
    let Some(parsed) = parsed_protocol_error(err) else {
        return false;
    };

    if parsed.http_status != Some(400) {
        return false;
    }

    parsed.code.is_some() || parsed.detail.is_some()
}

fn protocol_error_note(err: &Error) -> String {
    match parsed_protocol_error(err) {
        Some(parsed) => format!(
            "accepted protocol-like rejection: status={:?}, code={:?}, detail={:?}",
            parsed.http_status, parsed.code, parsed.detail
        ),
        None => err.to_string(),
    }
}

fn current_target_profile() -> Option<&'static TargetProfile> {
    TARGET_PROFILE.get()
}

fn expect_swap_failure(
    result: std::result::Result<cdk::nuts::SwapResponse, Error>,
    msg: &str,
) -> Result<String> {
    match result {
        Ok(_) => Err(anyhow!("{msg}: swap unexpectedly succeeded")),
        Err(err) => {
            let expected_substrings = match msg {
                "unsigned P2PK spend" => EXPECT_SIGNATURE_INVALID,
                "partial-signature P2PK spend" => EXPECT_SIGNATURE_INVALID,
                "1-of-2 multisig" => EXPECT_P2PK_OR_SIGNATURE_FAILURE,
                "invalid multisig" => EXPECT_P2PK_OR_SIGNATURE_FAILURE,
                "refund before locktime" => EXPECT_SIGNATURE_INVALID,
                "wrong signer" => EXPECT_SIGNATURE_INVALID,
                "duplicate signatures" => EXPECT_SIGNATURE_INVALID,
                "HTLC preimage-only" => EXPECT_WITNESS_NO_SIGNATURES,
                "HTLC signature-only" => EXPECT_NOT_HTLC_SECRET,
                "wrong HTLC preimage" => EXPECT_PREIMAGE_INVALID_HEX,
                "HTLC 1-of-3" => EXPECT_HTLC_SPEND_NOT_MET,
                "SIG_ALL unsigned" => EXPECT_SIGNATURE_INVALID,
                "SIG_INPUTS used for SIG_ALL" => EXPECT_SIGNATURE_INVALID,
                "SIG_ALL 1-of-3" => EXPECT_P2PK_OR_SIGNATURE_FAILURE,
                "SIG_ALL invalid signers" => EXPECT_P2PK_OR_SIGNATURE_FAILURE,
                "SIG_ALL wrong signer" => EXPECT_SIGNATURE_INVALID,
                "SIG_ALL duplicate signatures" => EXPECT_P2PK_OR_SIGNATURE_FAILURE,
                "SIG_ALL refund before locktime" => EXPECT_SIGNATURE_INVALID,
                "mixed SIG_ALL data" => EXPECT_SIGALL_INPUT_MISMATCH,
                "mixed SIG_ALL kind" => EXPECT_SIGALL_INPUT_MISMATCH,
                "mixed SIG_ALL tags" => EXPECT_SIGALL_INPUT_MISMATCH,
                "SIG_ALL 1-of-3 before locktime" => EXPECT_P2PK_OR_SIGNATURE_FAILURE,
                "1-of-2 refund multisig" => EXPECT_P2PK_OR_SIGNATURE_FAILURE,
                "tampered output amounts" => EXPECT_SIGNATURE_INVALID,
                "SIG_ALL HTLC preimage-only" => EXPECT_SIGALL_WITNESS_NO_SIGNATURES,
                "SIG_ALL HTLC signature-only" => EXPECT_HTLC_SPEND_NOT_MET,
                "SIG_ALL HTLC wrong preimage" => EXPECT_HTLC_SPEND_NOT_MET,
                "SIG_ALL HTLC 1-of-3" => EXPECT_HTLC_SPEND_NOT_MET,
                _ => {
                    return Err(anyhow!(
                        "{msg}: no expected error mapping for actual error `{}`",
                        err
                    ));
                }
            };

            if error_contains_any(&err, expected_substrings) {
                Ok(err.to_string())
            } else if current_target_profile()
                .is_some_and(|target| target.relaxed_external_negative_errors)
                && is_protocol_like_negative_error(&err)
            {
                Ok(protocol_error_note(&err))
            } else {
                Err(anyhow!(
                    "{msg}: unexpected error `{}`; expected one of {:?}",
                    err,
                    expected_substrings
                ))
            }
        }
    }
}

fn expect_swap_success(
    result: std::result::Result<cdk::nuts::SwapResponse, Error>,
    msg: &str,
) -> Result<cdk::nuts::SwapResponse> {
    result.map_err(|err| anyhow!("{msg}: {err}"))
}

fn expect_melt_failure(
    result: std::result::Result<MeltQuoteResponse<String>, Error>,
    msg: &str,
) -> Result<String> {
    match result {
        Ok(_) => Err(anyhow!("{msg}: melt unexpectedly succeeded")),
        Err(err) => {
            let expected_substrings = match msg {
                "melt P2PK unsigned" => EXPECT_SIGNATURE_INVALID,
                "melt HTLC preimage-only" => EXPECT_WITNESS_NO_SIGNATURES,
                "melt HTLC signature-only" => EXPECT_NOT_HTLC_SECRET,
                "melt P2PK SIG_ALL unsigned" => EXPECT_SIGNATURE_INVALID,
                "melt P2PK SIG_ALL sig-inputs" => EXPECT_SIGNATURE_INVALID,
                "melt HTLC SIG_ALL preimage-only" => EXPECT_SIGALL_WITNESS_NO_SIGNATURES,
                "melt HTLC SIG_ALL sig-inputs" => EXPECT_HTLC_SPEND_NOT_MET,
                "melt P2PK wrong key before locktime" => EXPECT_SIGNATURE_INVALID,
                _ => {
                    return Err(anyhow!(
                        "{msg}: no expected error mapping for actual error `{}`",
                        err
                    ));
                }
            };

            if error_contains_any(&err, expected_substrings) {
                Ok(err.to_string())
            } else if current_target_profile()
                .is_some_and(|target| target.relaxed_external_negative_errors)
                && is_protocol_like_negative_error(&err)
            {
                Ok(protocol_error_note(&err))
            } else {
                Err(anyhow!(
                    "{msg}: unexpected error `{}`; expected one of {:?}",
                    err,
                    expected_substrings
                ))
            }
        }
    }
}

fn expect_melt_success(
    result: std::result::Result<MeltQuoteResponse<String>, Error>,
    msg: &str,
) -> Result<MeltQuoteResponse<String>> {
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
    sign_sig_all_request(&mut one_sig, alice.secret.clone())?;
    expect_swap_failure(ctx.client.post_swap(one_sig).await, "SIG_ALL 1-of-3")?;

    let mut invalid = SwapRequest::new(locked.proofs.clone(), outputs.clone());
    sign_sig_all_request(&mut invalid, dave.secret.clone())?;
    sign_sig_all_request(&mut invalid, eve.secret.clone())?;
    expect_swap_failure(
        ctx.client.post_swap(invalid).await,
        "SIG_ALL invalid signers",
    )?;

    let mut valid = SwapRequest::new(locked.proofs, outputs);
    sign_sig_all_request(&mut valid, alice.secret)?;
    sign_sig_all_request(&mut valid, bob.secret)?;
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
    sign_sig_all_request(&mut request, bob.secret)?;
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
    sign_sig_all_request(&mut request, alice.secret.clone())?;
    sign_sig_all_request(&mut request, alice.secret)?;
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
    sign_sig_all_request(&mut refund, bob.secret.clone())?;
    expect_swap_failure(
        ctx.client.post_swap(refund).await,
        "SIG_ALL refund before locktime",
    )?;

    let mut primary = SwapRequest::new(locked.proofs, outputs);
    sign_sig_all_request(&mut primary, alice.secret)?;
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
    sign_sig_all_request(&mut primary, alice.secret)?;
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
    sign_sig_all_request(&mut request, alice.secret)?;
    sign_sig_all_request(&mut request, bob.secret)?;
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
    sign_sig_all_request(&mut mixed_request, alice.secret.clone())?;
    sign_sig_all_request(&mut mixed_request, bob.secret.clone())?;
    let error = expect_swap_failure(client.post_swap(mixed_request).await, "mixed SIG_ALL data")?;

    let mut alice_only = SwapRequest::new(
        alice_locked.proofs,
        random_outputs(alice_locked.keyset_id, standard_input_amount())?,
    );
    sign_sig_all_request(&mut alice_only, alice.secret)?;
    expect_swap_success(
        ctx_alice.client.post_swap(alice_only).await,
        "alice-only SIG_ALL",
    )?;

    let mut bob_only = SwapRequest::new(
        bob_locked.proofs,
        random_outputs(bob_locked.keyset_id, standard_input_amount())?,
    );
    sign_sig_all_request(&mut bob_only, bob.secret)?;
    expect_swap_success(ctx_bob.client.post_swap(bob_only).await, "bob-only SIG_ALL")?;

    Ok(format!("mixed SIG_ALL proofs rejected: {error}"))
}

async fn scenario_p2pk_sigall_mixed_proofs_different_kind_fail(mint_url: String) -> Result<String> {
    let ctx_p2pk = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let p2pk_conditions = SpendingConditions::new_p2pk(
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
    let p2pk_locked =
        lock_proofs_with_conditions(&ctx_p2pk, standard_input_amount(), &p2pk_conditions).await?;

    let ctx_htlc = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let htlc_fixture = create_test_hash_and_preimage();
    let htlc_conditions = SpendingConditions::new_htlc_hash(
        &htlc_fixture.hash,
        Some(Conditions {
            locktime: None,
            pubkeys: Some(vec![alice.public]),
            refund_keys: None,
            num_sigs: None,
            sig_flag: SigFlag::SigAll,
            num_sigs_refund: None,
        }),
    )?;
    let htlc_locked =
        lock_proofs_with_conditions(&ctx_htlc, standard_input_amount(), &htlc_conditions).await?;

    let client = HttpClient::new(MintUrl::from_str(&mint_url)?, None);
    let keyset_id = ctx_p2pk.active_keyset_id().await?;
    let mut mixed_proofs = p2pk_locked.proofs.clone();
    mixed_proofs.extend(htlc_locked.proofs.clone());
    let mixed_amount = standard_input_amount() + standard_input_amount();
    let mixed_request = SwapRequest::new(mixed_proofs, random_outputs(keyset_id, mixed_amount)?);
    let error = expect_swap_failure(client.post_swap(mixed_request).await, "mixed SIG_ALL kind")?;

    let mut p2pk_only = SwapRequest::new(
        p2pk_locked.proofs,
        random_outputs(p2pk_locked.keyset_id, standard_input_amount())?,
    );
    sign_sig_all_request(&mut p2pk_only, alice.secret.clone())?;
    expect_swap_success(
        ctx_p2pk.client.post_swap(p2pk_only).await,
        "p2pk-only mixed-kind control",
    )?;

    let mut htlc_only = SwapRequest::new(
        htlc_locked.proofs,
        random_outputs(htlc_locked.keyset_id, standard_input_amount())?,
    );
    htlc_only.inputs_mut()[0].add_preimage(htlc_fixture.preimage);
    sign_sig_all_request(&mut htlc_only, alice.secret)?;
    expect_swap_success(
        ctx_htlc.client.post_swap(htlc_only).await,
        "htlc-only mixed-kind control",
    )?;

    Ok(format!("mixed SIG_ALL proof kinds rejected: {error}"))
}

async fn scenario_p2pk_sigall_mixed_proofs_different_tags_fail(mint_url: String) -> Result<String> {
    let ctx_plain = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let alice = create_test_keypair();
    let plain_conditions = SpendingConditions::new_p2pk(
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
    let plain_locked =
        lock_proofs_with_conditions(&ctx_plain, standard_input_amount(), &plain_conditions).await?;

    let ctx_tagged = TestContext::with_funds(&mint_url, standard_input_amount()).await?;
    let tagged_conditions = SpendingConditions::new_p2pk(
        alice.public,
        Some(Conditions {
            locktime: Some(cdk::util::unix_time() + 3600),
            pubkeys: None,
            refund_keys: None,
            num_sigs: None,
            sig_flag: SigFlag::SigAll,
            num_sigs_refund: None,
        }),
    );
    let tagged_locked =
        lock_proofs_with_conditions(&ctx_tagged, standard_input_amount(), &tagged_conditions)
            .await?;

    let client = HttpClient::new(MintUrl::from_str(&mint_url)?, None);
    let keyset_id = ctx_plain.active_keyset_id().await?;
    let mut mixed_proofs = plain_locked.proofs.clone();
    mixed_proofs.extend(tagged_locked.proofs.clone());
    let mixed_amount = standard_input_amount() + standard_input_amount();
    let mixed_request = SwapRequest::new(mixed_proofs, random_outputs(keyset_id, mixed_amount)?);
    let error = expect_swap_failure(client.post_swap(mixed_request).await, "mixed SIG_ALL tags")?;

    let mut plain_only = SwapRequest::new(
        plain_locked.proofs,
        random_outputs(plain_locked.keyset_id, standard_input_amount())?,
    );
    sign_sig_all_request(&mut plain_only, alice.secret.clone())?;
    expect_swap_success(
        ctx_plain.client.post_swap(plain_only).await,
        "plain-only mixed-tags control",
    )?;

    let mut tagged_only = SwapRequest::new(
        tagged_locked.proofs,
        random_outputs(tagged_locked.keyset_id, standard_input_amount())?,
    );
    sign_sig_all_request(&mut tagged_only, alice.secret)?;
    expect_swap_success(
        ctx_tagged.client.post_swap(tagged_only).await,
        "tagged-only mixed-tags control",
    )?;

    Ok(format!("mixed SIG_ALL proof tags rejected: {error}"))
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
    sign_sig_all_request(&mut one_sig, alice.secret.clone())?;
    expect_swap_failure(
        ctx.client.post_swap(one_sig).await,
        "SIG_ALL 1-of-3 before locktime",
    )?;

    let mut two_sig = SwapRequest::new(locked.proofs, outputs);
    sign_sig_all_request(&mut two_sig, alice.secret)?;
    sign_sig_all_request(&mut two_sig, bob.secret)?;
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
    sign_sig_all_request(&mut request, alice.secret)?;
    sign_sig_all_request(&mut request, bob.secret)?;
    sign_sig_all_request(&mut request, carol.secret)?;
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
    sign_sig_all_request(&mut one, dave.secret.clone())?;
    expect_swap_failure(ctx.client.post_swap(one).await, "1-of-2 refund multisig")?;

    let mut both = SwapRequest::new(locked.proofs, outputs);
    sign_sig_all_request(&mut both, dave.secret)?;
    sign_sig_all_request(&mut both, eve.secret)?;
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
    sign_sig_all_request(&mut request, alice.secret)?;

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
    sign_sig_all_request(&mut request, alice.secret)?;
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
    sign_sig_all_request(&mut request, alice.secret)?;
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
    sign_sig_all_request(&mut request, alice.secret)?;
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
    sign_sig_all_request(&mut request, bob.secret)?;
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
    sign_sig_all_request(&mut one, alice.secret.clone())?;
    expect_swap_failure(ctx.client.post_swap(one).await, "SIG_ALL HTLC 1-of-3")?;

    let mut two = SwapRequest::new(locked.proofs, outputs);
    two.inputs_mut()[0].add_preimage(fixture.preimage);
    sign_sig_all_request(&mut two, alice.secret)?;
    sign_sig_all_request(&mut two, bob.secret)?;
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
    sign_sig_all_request(&mut request, alice.secret)?;
    expect_swap_success(
        ctx.client.post_swap(request).await,
        "SIG_ALL HTLC receiver after locktime",
    )?;
    Ok("SIG_ALL HTLC receiver path remains valid after locktime".to_string())
}

async fn scenario_melt_p2pk_unsigned_fails(mint_url: String) -> Result<String> {
    let alice = create_test_keypair();
    let conditions = SpendingConditions::new_p2pk(alice.public, None);
    let (ctx, quote, proofs) = prepare_locked_melt_proofs(&mint_url, &conditions).await?;

    let request = melt_request_from_proofs(quote.quote_id.clone(), proofs);
    if request.verify_spending_conditions().is_ok() {
        return Err(anyhow!(
            "melt P2PK unsigned: local verification unexpectedly succeeded"
        ));
    }

    let error = expect_melt_failure(
        ctx.client.post_melt(&PaymentMethod::BOLT11, request).await,
        "melt P2PK unsigned",
    )?;
    Ok(format!("unsigned melt rejected as expected: {error}"))
}

async fn scenario_melt_p2pk_signed_succeeds(mint_url: String) -> Result<String> {
    let alice = create_test_keypair();
    let conditions = SpendingConditions::new_p2pk(alice.public, None);
    let (ctx, quote, mut proofs) = prepare_locked_melt_proofs(&mint_url, &conditions).await?;

    sign_all_inputs(&mut proofs, &[alice.secret])?;
    let request = melt_request_from_proofs(quote.quote_id.clone(), proofs);
    request.verify_spending_conditions()?;

    let response = post_melt_and_wait_for_success(&ctx.client, request, "melt P2PK signed").await?;
    if response.quote() != &quote.quote_id {
        return Err(anyhow!("melt P2PK signed: quote id mismatch"));
    }
    Ok(format!("melt succeeded with state {}", response.state()))
}

async fn scenario_melt_htlc_preimage_only_fails(mint_url: String) -> Result<String> {
    let alice = create_test_keypair();
    let fixture = create_test_hash_and_preimage();
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
    let (ctx, quote, mut proofs) = prepare_locked_melt_proofs(&mint_url, &conditions).await?;

    for proof in &mut proofs {
        proof.add_preimage(fixture.preimage.clone());
    }

    let request = melt_request_from_proofs(quote.quote_id.clone(), proofs);
    if request.verify_spending_conditions().is_ok() {
        return Err(anyhow!(
            "melt HTLC preimage-only: local verification unexpectedly succeeded"
        ));
    }

    let error = expect_melt_failure(
        ctx.client.post_melt(&PaymentMethod::BOLT11, request).await,
        "melt HTLC preimage-only",
    )?;
    Ok(format!("preimage-only melt rejected as expected: {error}"))
}

async fn scenario_melt_htlc_signature_only_fails(mint_url: String) -> Result<String> {
    let alice = create_test_keypair();
    let fixture = create_test_hash_and_preimage();
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
    let (ctx, quote, mut proofs) = prepare_locked_melt_proofs(&mint_url, &conditions).await?;

    sign_all_inputs(&mut proofs, &[alice.secret])?;
    let request = melt_request_from_proofs(quote.quote_id.clone(), proofs);
    if request.verify_spending_conditions().is_ok() {
        return Err(anyhow!(
            "melt HTLC signature-only: local verification unexpectedly succeeded"
        ));
    }

    let error = expect_melt_failure(
        ctx.client.post_melt(&PaymentMethod::BOLT11, request).await,
        "melt HTLC signature-only",
    )?;
    Ok(format!("signature-only melt rejected as expected: {error}"))
}

async fn scenario_melt_htlc_preimage_and_signature_succeeds(mint_url: String) -> Result<String> {
    let alice = create_test_keypair();
    let fixture = create_test_hash_and_preimage();
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
    let (ctx, quote, mut proofs) = prepare_locked_melt_proofs(&mint_url, &conditions).await?;

    add_preimage_and_sign_all_inputs(&mut proofs, &fixture.preimage, &[alice.secret])?;
    let request = melt_request_from_proofs(quote.quote_id.clone(), proofs);
    request.verify_spending_conditions()?;

    let response = post_melt_and_wait_for_success(&ctx.client, request, "melt HTLC valid").await?;
    Ok(format!("melt succeeded with state {}", response.state()))
}

async fn scenario_melt_p2pk_sigall_unsigned_fails(mint_url: String) -> Result<String> {
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
    let (ctx, quote, proofs) = prepare_locked_melt_proofs(&mint_url, &conditions).await?;

    let request = melt_request_from_proofs(quote.quote_id.clone(), proofs);
    if request.verify_spending_conditions().is_ok() {
        return Err(anyhow!(
            "melt P2PK SIG_ALL unsigned: local verification unexpectedly succeeded"
        ));
    }

    let error = expect_melt_failure(
        ctx.client.post_melt(&PaymentMethod::BOLT11, request).await,
        "melt P2PK SIG_ALL unsigned",
    )?;
    Ok(format!(
        "unsigned SIG_ALL melt rejected as expected: {error}"
    ))
}

async fn scenario_melt_p2pk_sigall_sig_inputs_fail(mint_url: String) -> Result<String> {
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
    let (ctx, quote, mut proofs) = prepare_locked_melt_proofs(&mint_url, &conditions).await?;

    sign_all_inputs(&mut proofs, &[alice.secret])?;
    let request = melt_request_from_proofs(quote.quote_id.clone(), proofs);
    if request.verify_spending_conditions().is_ok() {
        return Err(anyhow!(
            "melt P2PK SIG_ALL sig-inputs: local verification unexpectedly succeeded"
        ));
    }

    let error = expect_melt_failure(
        ctx.client.post_melt(&PaymentMethod::BOLT11, request).await,
        "melt P2PK SIG_ALL sig-inputs",
    )?;
    Ok(format!(
        "SIG_INPUTS melt rejected for SIG_ALL as expected: {error}"
    ))
}

async fn scenario_melt_p2pk_sigall_transaction_signature_succeeds(
    mint_url: String,
) -> Result<String> {
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
    let (ctx, quote, proofs) = prepare_locked_melt_proofs(&mint_url, &conditions).await?;

    let mut request = melt_request_from_proofs(quote.quote_id.clone(), proofs);
    sign_sig_all_request(&mut request, alice.secret)?;
    request.verify_spending_conditions()?;

    let response =
        post_melt_and_wait_for_success(&ctx.client, request, "melt P2PK SIG_ALL valid").await?;
    Ok(format!("melt succeeded with state {}", response.state()))
}

async fn scenario_melt_htlc_sigall_preimage_only_fails(mint_url: String) -> Result<String> {
    let alice = create_test_keypair();
    let fixture = create_test_hash_and_preimage();
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
    let (ctx, quote, mut proofs) = prepare_locked_melt_proofs(&mint_url, &conditions).await?;

    proofs[0].add_preimage(fixture.preimage.clone());
    let request = melt_request_from_proofs(quote.quote_id.clone(), proofs);
    if request.verify_spending_conditions().is_ok() {
        return Err(anyhow!(
            "melt HTLC SIG_ALL preimage-only: local verification unexpectedly succeeded"
        ));
    }

    let error = expect_melt_failure(
        ctx.client.post_melt(&PaymentMethod::BOLT11, request).await,
        "melt HTLC SIG_ALL preimage-only",
    )?;
    Ok(format!(
        "preimage-only SIG_ALL melt rejected as expected: {error}"
    ))
}

async fn scenario_melt_htlc_sigall_sig_inputs_fail(mint_url: String) -> Result<String> {
    let alice = create_test_keypair();
    let fixture = create_test_hash_and_preimage();
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
    let (ctx, quote, mut proofs) = prepare_locked_melt_proofs(&mint_url, &conditions).await?;

    proofs[0].add_preimage(fixture.preimage.clone());
    sign_all_inputs(&mut proofs, &[alice.secret])?;
    let request = melt_request_from_proofs(quote.quote_id.clone(), proofs);
    if request.verify_spending_conditions().is_ok() {
        return Err(anyhow!(
            "melt HTLC SIG_ALL sig-inputs: local verification unexpectedly succeeded"
        ));
    }

    let error = expect_melt_failure(
        ctx.client.post_melt(&PaymentMethod::BOLT11, request).await,
        "melt HTLC SIG_ALL sig-inputs",
    )?;
    Ok(format!(
        "SIG_INPUTS melt rejected for HTLC SIG_ALL as expected: {error}"
    ))
}

async fn scenario_melt_htlc_sigall_preimage_and_transaction_signature_succeeds(
    mint_url: String,
) -> Result<String> {
    let alice = create_test_keypair();
    let fixture = create_test_hash_and_preimage();
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
    let (ctx, quote, proofs) = prepare_locked_melt_proofs(&mint_url, &conditions).await?;

    let mut request = melt_request_from_proofs(quote.quote_id.clone(), proofs);
    request.inputs_mut()[0].add_preimage(fixture.preimage.clone());
    sign_sig_all_request(&mut request, alice.secret)?;
    request.verify_spending_conditions()?;

    let response =
        post_melt_and_wait_for_success(&ctx.client, request, "melt HTLC SIG_ALL valid").await?;
    Ok(format!("melt succeeded with state {}", response.state()))
}

async fn scenario_melt_p2pk_post_locktime_anyone_can_spend(mint_url: String) -> Result<String> {
    let alice = create_test_keypair();
    let bob = create_test_keypair();
    let conditions = SpendingConditions::new_p2pk(
        alice.public,
        Some(Conditions {
            locktime: Some(cdk::util::unix_time() - 3600),
            pubkeys: None,
            refund_keys: None,
            num_sigs: None,
            sig_flag: SigFlag::SigInputs,
            num_sigs_refund: None,
        }),
    );
    let (ctx, quote, mut proofs) = prepare_locked_melt_proofs(&mint_url, &conditions).await?;

    sign_all_inputs(&mut proofs, &[bob.secret])?;
    let request = melt_request_from_proofs(quote.quote_id.clone(), proofs);
    request.verify_spending_conditions()?;

    let response = post_melt_and_wait_for_success(
        &ctx.client,
        request,
        "melt anyone-can-spend after locktime",
    )
    .await?;
    Ok(format!("melt succeeded with state {}", response.state()))
}

async fn scenario_melt_p2pk_before_locktime_wrong_key_fails(mint_url: String) -> Result<String> {
    let alice = create_test_keypair();
    let bob = create_test_keypair();
    let conditions = SpendingConditions::new_p2pk(
        alice.public,
        Some(Conditions::new(
            Some(cdk::util::unix_time() + 365 * 24 * 60 * 60),
            None,
            None,
            None,
            Some(SigFlag::SigInputs),
            None,
        )?),
    );
    let (ctx, quote, mut proofs) = prepare_locked_melt_proofs(&mint_url, &conditions).await?;

    sign_all_inputs(&mut proofs, &[bob.secret])?;
    let request = melt_request_from_proofs(quote.quote_id.clone(), proofs);
    if request.verify_spending_conditions().is_ok() {
        return Err(anyhow!(
            "melt P2PK wrong key before locktime: local verification unexpectedly succeeded"
        ));
    }

    let error = expect_melt_failure(
        ctx.client.post_melt(&PaymentMethod::BOLT11, request).await,
        "melt P2PK wrong key before locktime",
    )?;
    Ok(format!(
        "wrong-key melt rejected before locktime as expected: {error}"
    ))
}

async fn scenario_melt_p2pk_before_locktime_correct_key_succeeds(
    mint_url: String,
) -> Result<String> {
    let alice = create_test_keypair();
    let conditions = SpendingConditions::new_p2pk(
        alice.public,
        Some(Conditions::new(
            Some(cdk::util::unix_time() + 365 * 24 * 60 * 60),
            None,
            None,
            None,
            Some(SigFlag::SigInputs),
            None,
        )?),
    );
    let (ctx, quote, mut proofs) = prepare_locked_melt_proofs(&mint_url, &conditions).await?;

    sign_all_inputs(&mut proofs, &[alice.secret])?;
    let request = melt_request_from_proofs(quote.quote_id.clone(), proofs);
    request.verify_spending_conditions()?;

    let response = post_melt_and_wait_for_success(
        &ctx.client,
        request,
        "melt P2PK correct key before locktime",
    )
    .await?;
    Ok(format!("melt succeeded with state {}", response.state()))
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
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(path, json).await?;
    Ok(())
}

fn print_results_header() {
    println!(
        "{:<w0$}  {:<w1$}  {:<w2$}  Note",
        "Scenario",
        "Status",
        "Duration",
        w0 = SCENARIO_COLUMN_WIDTH,
        w1 = STATUS_COLUMN_WIDTH,
        w2 = DURATION_COLUMN_WIDTH,
    );
    println!(
        "{:-<w0$}  {:-<w1$}  {:-<w2$}  {:-<w3$}",
        "",
        "",
        "",
        "",
        w0 = SCENARIO_COLUMN_WIDTH,
        w1 = STATUS_COLUMN_WIDTH,
        w2 = DURATION_COLUMN_WIDTH,
        w3 = 4,
    );
}

fn print_result_row(result: &ScenarioResult) {
    let status = match result.status {
        ScenarioStatus::Pass => "PASS",
        ScenarioStatus::Fail => "FAIL",
    };
    let duration = format!("{} ms", result.duration_ms);

    println!(
        "{:<w0$}  {:<w1$}  {:<w2$}  {}",
        result.name,
        status,
        duration,
        result.note,
        w0 = SCENARIO_COLUMN_WIDTH,
        w1 = STATUS_COLUMN_WIDTH,
        w2 = DURATION_COLUMN_WIDTH,
    );
}

impl fmt::Debug for LocalMintHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalMintHandle")
            .field("mint_url", &self.mint_url)
            .field("work_dir", &self.work_dir)
            .finish_non_exhaustive()
    }
}
