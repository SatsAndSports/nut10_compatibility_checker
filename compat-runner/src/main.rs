use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use bip39::Mnemonic;
use cdk::amount::SplitTarget;
use cdk::dhke::construct_proofs;
use cdk::mint_url::MintUrl;
use cdk::nuts::{
    Conditions, CurrencyUnit, PaymentMethod, PreMintSecrets, SecretKey, SigFlag,
    SpendingConditions, SwapRequest, ProofsMethods,
};
use cdk::wallet::{HttpClient, MintConnector, Wallet, WalletBuilder};
use cdk::{Amount, StreamExt};
use cdk_mintd::config::{Database, DatabaseEngine, FakeWallet, Info, Ln, LnBackend, Settings};
use serde::Serialize;
use tokio_util::sync::CancellationToken;

const DEFAULT_JSON_REPORT_PATH: &str = "compat-report.json";
const DEFAULT_MINT_HOST: &str = "127.0.0.1";

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

#[derive(Clone)]
struct TestContext {
    wallet: Wallet,
    client: HttpClient,
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
        let client = HttpClient::new(mint_url.clone(), None);

        Ok(Self {
            wallet,
            client,
        })
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

    async fn active_keyset_id(&self) -> Result<cdk::nuts::Id> {
        Ok(self.wallet.fetch_active_keyset().await?.id)
    }

    async fn active_keyset_keys(&self) -> Result<cdk::nuts::Keys> {
        let keyset_id = self.active_keyset_id().await?;
        Ok(self.client.get_mint_keyset(keyset_id).await?.keys)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
        let mint = LocalMintHandle::start().await?;
        let target = "cdk".to_string();
        let mint_url = mint.mint_url.clone();

        let mut results = Vec::new();
        results.push(run_named_scenario(
            "p2pk_swap_unsigned_fails",
            &target,
            &mint_url,
            scenario_p2pk_swap_unsigned_fails,
        )
        .await);
        results.push(run_named_scenario(
            "p2pk_swap_signed_succeeds",
            &target,
            &mint_url,
            scenario_p2pk_swap_signed_succeeds,
        )
        .await);
        results.push(run_named_scenario(
            "htlc_swap_preimage_and_signature_succeeds",
            &target,
            &mint_url,
            scenario_htlc_swap_preimage_and_signature_succeeds,
        )
        .await);

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

async fn run_named_scenario<F, Fut>(
    name: &str,
    target: &str,
    mint_url: &str,
    scenario: F,
) -> ScenarioResult
where
    F: FnOnce(String) -> Fut,
    Fut: std::future::Future<Output = Result<String>>,
{
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

async fn scenario_p2pk_swap_unsigned_fails(mint_url: String) -> Result<String> {
    let ctx = TestContext::new(&mint_url).await?;
    let input_amount = Amount::from(10);
    ctx.fund_wallet(input_amount).await?;

    let input_proofs = ctx.wallet.get_unspent_proofs().await?;
    let keyset_id = ctx.active_keyset_id().await?;
    let keyset_keys = ctx.active_keyset_keys().await?;
    let (_secret_key, pubkey) = create_test_keypair();

    let spending_conditions = SpendingConditions::new_p2pk(pubkey, None);
    let pre_swap = PreMintSecrets::with_conditions(
        keyset_id,
        input_amount,
        &SplitTarget::default(),
        &spending_conditions,
        &standard_fee_and_amounts(),
    )?;

    let p2pk_swap = SwapRequest::new(
        input_proofs.clone(),
        pre_swap.blinded_messages().to_vec(),
    );
    let swap_response = ctx.client.post_swap(p2pk_swap).await?;
    let locked_proofs = construct_proofs(
        swap_response.signatures,
        pre_swap.rs(),
        pre_swap.secrets(),
        &keyset_keys,
    )?;

    let unlock_outputs = PreMintSecrets::random(
        keyset_id,
        input_amount,
        &SplitTarget::default(),
        &standard_fee_and_amounts(),
    )?;
    let unsigned_swap = SwapRequest::new(locked_proofs, unlock_outputs.blinded_messages().to_vec());

    match ctx.client.post_swap(unsigned_swap).await {
        Ok(_) => Err(anyhow!("unsigned P2PK spend unexpectedly succeeded")),
        Err(err) => Ok(format!("swap rejected as expected: {err}")),
    }
}

async fn scenario_p2pk_swap_signed_succeeds(mint_url: String) -> Result<String> {
    let ctx = TestContext::new(&mint_url).await?;
    let input_amount = Amount::from(10);
    ctx.fund_wallet(input_amount).await?;

    let input_proofs = ctx.wallet.get_unspent_proofs().await?;
    let keyset_id = ctx.active_keyset_id().await?;
    let keyset_keys = ctx.active_keyset_keys().await?;
    let (secret_key, pubkey) = create_test_keypair();

    let spending_conditions = SpendingConditions::new_p2pk(pubkey, None);
    let pre_swap = PreMintSecrets::with_conditions(
        keyset_id,
        input_amount,
        &SplitTarget::default(),
        &spending_conditions,
        &standard_fee_and_amounts(),
    )?;

    let p2pk_swap = SwapRequest::new(
        input_proofs.clone(),
        pre_swap.blinded_messages().to_vec(),
    );
    let swap_response = ctx.client.post_swap(p2pk_swap).await?;
    let mut locked_proofs = construct_proofs(
        swap_response.signatures,
        pre_swap.rs(),
        pre_swap.secrets(),
        &keyset_keys,
    )?;

    for proof in &mut locked_proofs {
        proof.sign_p2pk(secret_key.clone())?;
    }

    let unlock_outputs = PreMintSecrets::random(
        keyset_id,
        input_amount,
        &SplitTarget::default(),
        &standard_fee_and_amounts(),
    )?;
    let signed_swap = SwapRequest::new(locked_proofs, unlock_outputs.blinded_messages().to_vec());
    let response = ctx.client.post_swap(signed_swap).await?;

    Ok(format!(
        "swap succeeded with {} output signature(s)",
        response.signatures.len()
    ))
}

async fn scenario_htlc_swap_preimage_and_signature_succeeds(mint_url: String) -> Result<String> {
    let ctx = TestContext::new(&mint_url).await?;
    let input_amount = Amount::from(10);
    ctx.fund_wallet(input_amount).await?;

    let input_proofs = ctx.wallet.get_unspent_proofs().await?;
    let keyset_id = ctx.active_keyset_id().await?;
    let keyset_keys = ctx.active_keyset_keys().await?;
    let (secret_key, pubkey) = create_test_keypair();
    let (hash, preimage) = create_test_hash_and_preimage()?;

    let spending_conditions = SpendingConditions::new_htlc_hash(
        &hash,
        Some(Conditions {
            locktime: None,
            pubkeys: Some(vec![pubkey]),
            refund_keys: None,
            num_sigs: None,
            sig_flag: SigFlag::default(),
            num_sigs_refund: None,
        }),
    )?;

    let pre_swap = PreMintSecrets::with_conditions(
        keyset_id,
        input_amount,
        &SplitTarget::default(),
        &spending_conditions,
        &standard_fee_and_amounts(),
    )?;

    let htlc_swap = SwapRequest::new(
        input_proofs.clone(),
        pre_swap.blinded_messages().to_vec(),
    );
    let swap_response = ctx.client.post_swap(htlc_swap).await?;
    let mut locked_proofs = construct_proofs(
        swap_response.signatures,
        pre_swap.rs(),
        pre_swap.secrets(),
        &keyset_keys,
    )?;

    for proof in &mut locked_proofs {
        proof.add_preimage(preimage.clone());
        proof.sign_p2pk(secret_key.clone())?;
    }

    let unlock_outputs = PreMintSecrets::random(
        keyset_id,
        input_amount,
        &SplitTarget::default(),
        &standard_fee_and_amounts(),
    )?;
    let signed_swap = SwapRequest::new(locked_proofs, unlock_outputs.blinded_messages().to_vec());
    let response = ctx.client.post_swap(signed_swap).await?;

    Ok(format!(
        "HTLC swap succeeded with {} output signature(s)",
        response.signatures.len()
    ))
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

fn create_test_keypair() -> (SecretKey, cdk::nuts::PublicKey) {
    let secret = SecretKey::generate();
    let pubkey = secret.public_key();
    (secret, pubkey)
}

fn create_test_hash_and_preimage() -> Result<(String, String)> {
    use cdk::secp256k1::hashes::sha256::Hash as Sha256Hash;
    use cdk::secp256k1::hashes::Hash;

    let preimage_bytes = [0x42u8; 32];
    let hash = Sha256Hash::hash(&preimage_bytes);
    Ok((hash.to_string(), cdk::util::hex::encode(preimage_bytes)))
}

fn standard_fee_and_amounts() -> cdk::amount::FeeAndAmounts {
    (0, (0..32).map(|power| 2u64.pow(power)).collect::<Vec<_>>()).into()
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
