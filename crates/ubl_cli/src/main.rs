//! ublx - UBL Chip-as-Code CLI

use clap::{Parser, Subcommand};
use serde_json::{json, Value};
use std::sync::Arc;
use ubl_ai_nrf1::{compute_cid, to_nrf1_bytes, ChipFile};

#[derive(Parser)]
#[command(name = "ublx")]
#[command(about = "UBL Chip-as-Code CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Verify chip integrity and recompute CID
    Verify {
        #[arg(short, long)]
        chip_file: String,
    },
    /// Build .chip file to binary
    Build {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Compute and print the canonical CID (BLAKE3) of a JSON file
    Cid {
        /// Path to a JSON file
        file: String,
    },
    /// Submit a chip JSON file to a running UBL gate
    Submit {
        /// Path to chip JSON file
        #[arg(short, long)]
        input: String,
        /// Base URL of the gate (e.g. http://127.0.0.1:4000)
        #[arg(long, default_value = "http://127.0.0.1:4000")]
        gate: String,
        /// Optional path to write raw gate response JSON
        #[arg(short, long)]
        output: Option<String>,
        /// Optional API key sent as X-API-Key for write-protected lanes
        /// (fallback envs: SOURCE_GATE_API_KEY, UBL_GATE_API_KEY, UBL_API_KEY)
        #[arg(long)]
        api_key: Option<String>,
        /// HTTP timeout in seconds
        #[arg(long, default_value = "30")]
        timeout_secs: u64,
    },
    /// Explain a WF receipt: print RB tree with PASS/DENY per node
    Explain {
        /// CID of the receipt, or path to a receipt JSON file
        target: String,
    },
    /// Search ChipStore by type, tag, or date range
    Search {
        /// Filter by chip type (e.g. "ubl/user")
        #[arg(short = 't', long)]
        chip_type: Option<String>,
        /// Filter by tag
        #[arg(long)]
        tag: Vec<String>,
        /// Filter: created after (RFC-3339)
        #[arg(long)]
        after: Option<String>,
        /// Filter: created before (RFC-3339)
        #[arg(long)]
        before: Option<String>,
        /// Max results
        #[arg(short, long, default_value = "20")]
        limit: u64,
    },
    /// Generate receipt fixtures for integration testing
    Fixture {
        /// Output directory for fixtures
        #[arg(short, long, default_value = "fixtures")]
        output_dir: String,
        /// Number of fixtures to generate
        #[arg(short, long, default_value = "5")]
        count: usize,
    },
    /// Generate a Rich URL for a receipt
    Url {
        /// Receipt CID
        receipt_cid: String,
        /// Host for the URL
        #[arg(long, default_value = "https://ubl.example.com")]
        host: String,
    },
    /// Disassemble RB-VM bytecode to human-readable listing
    Disasm {
        /// Path to bytecode file (binary) or hex string
        input: String,
        /// Treat input as hex string instead of file path
        #[arg(long)]
        hex: bool,
    },
    /// DID key utilities
    Did {
        #[command(subcommand)]
        command: DidCommands,
    },
    /// Capability signing utilities
    Cap {
        #[command(subcommand)]
        command: CapCommands,
    },
    /// Silicon chip compiler and disassembler
    Silicon {
        #[command(subcommand)]
        command: SiliconCommands,
    },
}

#[derive(Subcommand)]
enum DidCommands {
    /// Generate a new Ed25519 keypair and print DID material
    Generate {
        /// Write JSON output to file
        #[arg(short, long)]
        output: Option<String>,
        /// Use strict did:key multicodec format (0xED01)
        #[arg(long, default_value_t = false)]
        strict: bool,
    },
    /// Derive DID material from an existing Ed25519 signing key hex
    FromKey {
        /// 64-char Ed25519 private seed hex
        #[arg(long)]
        signing_key_hex: String,
        /// Write JSON output to file
        #[arg(short, long)]
        output: Option<String>,
        /// Use strict did:key multicodec format (0xED01)
        #[arg(long, default_value_t = false)]
        strict: bool,
    },
}

#[derive(Subcommand)]
enum CapCommands {
    /// Issue a signed @cap payload using an Ed25519 signing key
    Issue {
        /// Capability action (e.g. registry:init, membership:grant)
        #[arg(long)]
        action: String,
        /// Capability audience world scope (e.g. a/chip-registry or a/chip-registry/t/logline)
        #[arg(long)]
        audience: String,
        /// 64-char Ed25519 private seed hex
        #[arg(long)]
        signing_key_hex: String,
        /// Optional issuer DID override (default derives from signing key)
        #[arg(long)]
        issued_by: Option<String>,
        /// Optional issued_at timestamp (RFC3339; default now UTC)
        #[arg(long)]
        issued_at: Option<String>,
        /// Optional expires_at timestamp (RFC3339; default now+365d)
        #[arg(long)]
        expires_at: Option<String>,
        /// Write JSON output to file
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Verify a capability JSON against required action/world
    Verify {
        /// Path to capability JSON file (either raw cap object or chip body containing @cap)
        #[arg(long)]
        input: String,
        /// Required action (e.g. registry:init)
        #[arg(long)]
        action: String,
        /// World to validate against (e.g. a/chip-registry or a/chip-registry/t/logline)
        #[arg(long)]
        world: String,
    },
}

#[derive(Subcommand)]
enum SiliconCommands {
    /// Compile a silicon chip JSON to rb_vm TLV bytecode.
    ///
    /// Reads a self-contained silicon chip bundle (a JSON file with embedded
    /// bit/circuit/chip definitions) and outputs:
    ///   - the chip CID (content address of the chip body)
    ///   - the bytecode CID (content address of the compiled TLV bytes)
    ///   - the hex-encoded TLV bytecode
    ///
    /// Bundle format (single JSON file):
    ///   {
    ///     "chip":    { <ubl/silicon.chip body> },
    ///     "circuits": [ { "cid": "b3:...", "body": { <ubl/silicon.circuit body> } }, ... ],
    ///     "bits":    [ { "cid": "b3:...", "body": { <ubl/silicon.bit body> } }, ... ]
    ///   }
    Compile {
        /// Path to silicon bundle JSON file.
        /// Mutually exclusive with --from-store.
        #[arg(conflicts_with = "from_store")]
        bundle: Option<String>,
        /// Compile a chip already in the ChipStore by CID.
        /// Opens the Sled store at --store-path (default: ./data/chips).
        #[arg(long, value_name = "CHIP_CID")]
        from_store: Option<String>,
        /// Path to the Sled ChipStore directory (used with --from-store).
        #[arg(long, default_value = "./data/chips")]
        store_path: String,
        /// Print only the bytecode hex (machine-readable, no labels)
        #[arg(long)]
        hex_only: bool,
    },
    /// Disassemble silicon-compiled rb_vm TLV bytecode to human-readable listing.
    ///
    /// Accepts either a hex string or a binary bytecode file.
    Disasm {
        /// Hex string of bytecode, or path to a binary bytecode file
        input: String,
        /// Treat input as a binary file path (default: treat as hex string)
        #[arg(long)]
        file: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Verify { chip_file } => cmd_verify(&chip_file)?,
        Commands::Build { input, output } => cmd_build(&input, output)?,
        Commands::Cid { file } => cmd_cid(&file)?,
        Commands::Submit {
            input,
            gate,
            output,
            api_key,
            timeout_secs,
        } => {
            let resolved_api_key = api_key
                .or_else(|| std::env::var("SOURCE_GATE_API_KEY").ok())
                .or_else(|| std::env::var("UBL_GATE_API_KEY").ok())
                .or_else(|| std::env::var("UBL_API_KEY").ok());
            cmd_submit(
                &input,
                &gate,
                output,
                resolved_api_key.as_deref(),
                timeout_secs,
            )
            .await?
        }
        Commands::Explain { target } => cmd_explain(&target)?,
        Commands::Search {
            chip_type,
            tag,
            after,
            before,
            limit,
        } => {
            cmd_search(chip_type, tag, after, before, limit).await?;
        }
        Commands::Fixture { output_dir, count } => cmd_fixture(&output_dir, count)?,
        Commands::Url { receipt_cid, host } => cmd_url(&receipt_cid, &host)?,
        Commands::Disasm { input, hex } => cmd_disasm(&input, hex)?,
        Commands::Did { command } => match command {
            DidCommands::Generate { output, strict } => {
                cmd_did_generate(output.as_deref(), strict)?
            }
            DidCommands::FromKey {
                signing_key_hex,
                output,
                strict,
            } => cmd_did_from_key(&signing_key_hex, output.as_deref(), strict)?,
        },
        Commands::Cap { command } => match command {
            CapCommands::Issue {
                action,
                audience,
                signing_key_hex,
                issued_by,
                issued_at,
                expires_at,
                output,
            } => cmd_cap_issue(
                &action,
                &audience,
                &signing_key_hex,
                issued_by.as_deref(),
                issued_at.as_deref(),
                expires_at.as_deref(),
                output.as_deref(),
            )?,
            CapCommands::Verify {
                input,
                action,
                world,
            } => cmd_cap_verify(&input, &action, &world)?,
        },
        Commands::Silicon { command } => match command {
            SiliconCommands::Compile {
                bundle,
                from_store,
                store_path,
                hex_only,
            } => {
                cmd_silicon_compile(
                    bundle.as_deref(),
                    from_store.as_deref(),
                    &store_path,
                    hex_only,
                )
                .await?
            }
            SiliconCommands::Disasm { input, file } => cmd_silicon_disasm(&input, file)?,
        },
    }

    Ok(())
}

// ── verify ──────────────────────────────────────────────────────

fn cmd_verify(chip_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let chip_yaml = std::fs::read_to_string(chip_file)?;
    let chip: ChipFile = serde_yaml::from_str(&chip_yaml)?;
    let compiled = chip.compile()?;

    println!("Chip verified successfully");
    println!("  Type: {}", compiled.chip_type);
    println!("  ID:   {}", compiled.logical_id);
    println!("  CID:  {}", compiled.cid);
    println!("  Size: {} bytes", compiled.nrf1_bytes.len());
    Ok(())
}

// ── build ───────────────────────────────────────────────────────

fn cmd_build(input: &str, output: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let chip_yaml = std::fs::read_to_string(input)?;
    let chip: ChipFile = serde_yaml::from_str(&chip_yaml)?;
    let compiled = chip.compile()?;

    let output_path = output.unwrap_or_else(|| format!("{}.bin", compiled.cid));
    std::fs::write(&output_path, &compiled.nrf1_bytes)?;

    println!("Compiled: {} -> {}", input, output_path);
    println!("  CID: {}", compiled.cid);
    Ok(())
}

// ── cid ─────────────────────────────────────────────────────────

fn cmd_cid(file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(file)?;
    let json: Value = serde_json::from_str(&content)?;
    let nrf_bytes = to_nrf1_bytes(&json)?;
    let cid = compute_cid(&nrf_bytes)?;
    println!("{}", cid);
    Ok(())
}

// ── did / cap helpers ──────────────────────────────────────────

fn did_material_json(
    sk: &ubl_kms::Ed25519SigningKey,
    strict: bool,
) -> Result<Value, Box<dyn std::error::Error>> {
    let vk = ubl_kms::verifying_key(sk);
    let did = if strict {
        ubl_kms::did_from_verifying_key_strict(&vk)
    } else {
        ubl_kms::did_from_verifying_key(&vk)
    };
    let kid = format!("{}#ed25519", did);
    let signing_key_hex = hex::encode(sk.to_bytes());
    let public_key_hex = hex::encode(vk.to_bytes());
    let key_cid = ubl_kms::key_cid(&vk);

    Ok(json!({
        "did": did,
        "kid": kid,
        "signing_key_hex": signing_key_hex,
        "public_key_hex": public_key_hex,
        "key_cid": key_cid,
    }))
}

fn write_or_print_json(
    value: &Value,
    output: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let text = serde_json::to_string_pretty(value)?;
    if let Some(path) = output {
        std::fs::write(path, text.as_bytes())?;
        println!("wrote {}", path);
    } else {
        println!("{}", text);
    }
    Ok(())
}

fn cmd_did_generate(output: Option<&str>, strict: bool) -> Result<(), Box<dyn std::error::Error>> {
    let sk = ubl_kms::generate_signing_key();
    let out = did_material_json(&sk, strict)?;
    write_or_print_json(&out, output)
}

fn cmd_did_from_key(
    signing_key_hex: &str,
    output: Option<&str>,
    strict: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let sk = ubl_kms::signing_key_from_hex(signing_key_hex)?;
    let out = did_material_json(&sk, strict)?;
    write_or_print_json(&out, output)
}

fn cmd_cap_issue(
    action: &str,
    audience: &str,
    signing_key_hex: &str,
    issued_by: Option<&str>,
    issued_at: Option<&str>,
    expires_at: Option<&str>,
    output: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let sk = ubl_kms::signing_key_from_hex(signing_key_hex)?;
    let vk = ubl_kms::verifying_key(&sk);
    let derived_issuer = ubl_kms::did_from_verifying_key_strict(&vk);
    let issuer = issued_by.unwrap_or(derived_issuer.as_str());
    if issuer != derived_issuer {
        return Err(format!(
            "issued_by '{}' does not match signing key DID '{}'",
            issuer, derived_issuer
        )
        .into());
    }

    let issued_at_ts = issued_at
        .map(ToString::to_string)
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    let expires_at_ts = expires_at.map(ToString::to_string).unwrap_or_else(|| {
        (chrono::Utc::now() + chrono::Duration::days(365))
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    });

    let payload = json!({
        "action": action,
        "audience": audience,
        "issued_by": issuer,
        "issued_at": issued_at_ts,
        "expires_at": expires_at_ts,
    });
    let signature = ubl_kms::sign_canonical(&sk, &payload, ubl_kms::domain::CAPABILITY)?;

    let cap = json!({
        "action": action,
        "audience": audience,
        "issued_by": issuer,
        "issued_at": issued_at_ts,
        "expires_at": expires_at_ts,
        "signature": signature,
    });
    write_or_print_json(&cap, output)
}

fn cmd_cap_verify(
    input: &str,
    required_action: &str,
    world: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(input)?;
    let value: Value = serde_json::from_str(&raw)?;
    let cap = if value.get("@cap").is_some() {
        ubl_runtime::capability::extract_cap(&value)?
    } else {
        serde_json::from_value::<ubl_runtime::capability::Capability>(value)?
    };
    ubl_runtime::capability::validate_cap(&cap, required_action, world)?;
    println!(
        "capability ok action='{}' audience='{}' world='{}'",
        cap.action, cap.audience, world
    );
    Ok(())
}

// ── submit ──────────────────────────────────────────────────────

async fn cmd_submit(
    input: &str,
    gate: &str,
    output: Option<String>,
    api_key: Option<&str>,
    timeout_secs: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let payload = std::fs::read(input)?;
    let endpoint = format!("{}/v1/chips", gate.trim_end_matches('/'));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()?;

    let mut req = client
        .post(&endpoint)
        .header("content-type", "application/json");
    if let Some(key) = api_key.map(str::trim).filter(|k| !k.is_empty()) {
        req = req.header("X-API-Key", key);
    }
    let resp = req.body(payload).send().await?;

    let status = resp.status();
    let body_text = resp.text().await?;
    if !status.is_success() {
        return Err(format!("gate submit failed: {} {}", status, body_text).into());
    }

    let response_json: Value = serde_json::from_str(&body_text)?;
    if let Some(out) = output {
        std::fs::write(out, serde_json::to_vec_pretty(&response_json)?)?;
    }

    if let Some(receipt_cid) = response_json.get("receipt_cid").and_then(|v| v.as_str()) {
        println!("receipt_cid={}", receipt_cid);
    }
    if let Some(receipt_url) = response_json.get("receipt_url").and_then(|v| v.as_str()) {
        println!("receipt_url={}", receipt_url);
    }
    println!("{}", serde_json::to_string_pretty(&response_json)?);
    Ok(())
}

// ── explain ─────────────────────────────────────────────────────

fn cmd_explain(target: &str) -> Result<(), Box<dyn std::error::Error>> {
    // If target is a file path, read it; otherwise treat as inline JSON or CID
    let receipt_json: Value = if std::path::Path::new(target).exists() {
        let content = std::fs::read_to_string(target)?;
        serde_json::from_str(&content)?
    } else if target.starts_with('{') {
        serde_json::from_str(target)?
    } else {
        // CID-only mode: print what we know
        println!("Receipt CID: {}", target);
        println!("  (Pass a receipt JSON file for full explanation)");
        return Ok(());
    };

    // Print envelope
    println!("=== Receipt Explanation ===");
    if let Some(t) = receipt_json.get("@type").and_then(|v| v.as_str()) {
        println!("  @type: {}", t);
    }
    if let Some(d) = receipt_json.get("decision").and_then(|v| v.as_str()) {
        let marker = if d == "allow" { "ALLOW" } else { "DENY" };
        println!("  Decision: {}", marker);
    }
    if let Some(r) = receipt_json.get("reason").and_then(|v| v.as_str()) {
        println!("  Reason: {}", r);
    }

    // Print policy trace as RB tree
    if let Some(trace) = receipt_json.get("policy_trace").and_then(|v| v.as_array()) {
        println!("\n--- Policy Trace ({} policies) ---", trace.len());
        for (i, entry) in trace.iter().enumerate() {
            let policy_id = entry
                .get("policy_id")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let decision = entry
                .get("decision")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let marker = match decision {
                "allow" => "PASS",
                "deny" => "DENY",
                "require" => "REQUIRE",
                _ => decision,
            };
            println!("  [{}] {} -> {}", i + 1, policy_id, marker);

            // Print individual RB results
            if let Some(rbs) = entry.get("rb_results").and_then(|v| v.as_array()) {
                for rb in rbs {
                    let rb_id = rb.get("rb_id").and_then(|v| v.as_str()).unwrap_or("?");
                    let rb_dec = rb.get("decision").and_then(|v| v.as_str()).unwrap_or("?");
                    let rb_marker = match rb_dec {
                        "allow" => "PASS",
                        "deny" => "DENY",
                        _ => rb_dec,
                    };
                    println!("      RB {} -> {}", rb_id, rb_marker);
                }
            }
        }
    }

    // Print VM state if present
    if let Some(vm) = receipt_json.get("vm_state") {
        println!("\n--- VM State ---");
        if let Some(fuel) = vm.get("fuel_used").and_then(|v| v.as_u64()) {
            println!("  Fuel used: {}", fuel);
        }
        if let Some(steps) = vm.get("steps").and_then(|v| v.as_u64()) {
            println!("  Steps: {}", steps);
        }
    }

    // Recompute CID for verification
    let nrf_bytes = to_nrf1_bytes(&receipt_json)?;
    let cid = compute_cid(&nrf_bytes)?;
    println!("\n  Computed CID: {}", cid);

    Ok(())
}

// ── search ──────────────────────────────────────────────────────

async fn cmd_search(
    chip_type: Option<String>,
    tags: Vec<String>,
    after: Option<String>,
    before: Option<String>,
    limit: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    use ubl_chipstore::{ChipQuery, ChipStore, InMemoryBackend};

    // In a real deployment, this would connect to the running ChipStore.
    // For now, demonstrate the query API with an in-memory store.
    let backend = Arc::new(InMemoryBackend::new());
    let store = ChipStore::new(backend);

    let query = ChipQuery {
        chip_type,
        tags,
        created_after: after,
        created_before: before,
        executor_did: None,
        limit: Some(limit as usize),
        offset: None,
    };

    println!("Searching ChipStore...");
    println!("  Query: {}", serde_json::to_string_pretty(&query)?);

    let results = store.query(&query).await?;
    println!(
        "\n  Found: {} chips (total: {})",
        results.chips.len(),
        results.total_count
    );

    for chip in &results.chips {
        println!("  ---");
        println!("    CID:  {}", chip.cid);
        println!("    Type: {}", chip.chip_type);
        println!("    Receipt: {}", chip.receipt_cid);
    }

    if results.total_count == 0 {
        println!("  (No chips found. In production, connect to a running ChipStore.)");
    }

    Ok(())
}

// ── fixture ─────────────────────────────────────────────────────

fn cmd_fixture(output_dir: &str, count: usize) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(output_dir)?;

    let chip_types = [
        "ubl/user",
        "ubl/token",
        "ubl/policy",
        "ubl/app",
        "ubl/advisory",
    ];

    for i in 0..count {
        let chip_type = chip_types[i % chip_types.len()];
        let id = format!("fixture-{:04}", i);
        let world = "a/test/t/fixtures";

        // Generate chip body
        let chip_body = json!({
            "@type": chip_type,
            "@id": id,
            "@ver": "1.0",
            "@world": world,
            "fixture_index": i,
            "created_at": chrono::Utc::now().to_rfc3339(),
        });

        // Compute CID
        let nrf_bytes = to_nrf1_bytes(&chip_body)?;
        let cid = compute_cid(&nrf_bytes)?;

        // Generate a mock WF receipt
        let receipt = json!({
            "@type": "ubl/wf",
            "chip_cid": cid,
            "chip_type": chip_type,
            "decision": if i % 7 == 0 { "deny" } else { "allow" },
            "reason": if i % 7 == 0 { "Policy denied: fixture test" } else { "All policies passed" },
            "policy_trace": [
                {
                    "policy_id": "genesis-type-validation",
                    "decision": if i % 7 == 0 { "deny" } else { "allow" },
                    "rb_results": [
                        {
                            "rb_id": "type-allowed",
                            "decision": if i % 7 == 0 { "deny" } else { "allow" },
                            "expression": format!("TypeEquals(\"{}\")", chip_type)
                        }
                    ]
                }
            ],
            "vm_state": {
                "fuel_used": 1000 + (i as u64 * 100),
                "steps": 5 + i as u64,
                "rc_cid": format!("b3:rc-{:04}", i),
            },
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        // Write chip
        let chip_path = format!("{}/chip-{:04}.json", output_dir, i);
        std::fs::write(&chip_path, serde_json::to_string_pretty(&chip_body)?)?;

        // Write receipt
        let receipt_path = format!("{}/receipt-{:04}.json", output_dir, i);
        std::fs::write(&receipt_path, serde_json::to_string_pretty(&receipt)?)?;

        println!(
            "  [{}/{}] {} type={} cid={}",
            i + 1,
            count,
            id,
            chip_type,
            &cid[..20]
        );
    }

    println!(
        "\nGenerated {} chip + receipt fixture pairs in {}/",
        count, output_dir
    );
    Ok(())
}

// ── url ─────────────────────────────────────────────────────────

fn cmd_url(receipt_cid: &str, host: &str) -> Result<(), Box<dyn std::error::Error>> {
    use ubl_runtime::rich_url::HostedUrl;

    // Parse world from CID or use defaults
    let url = HostedUrl::new(
        host,
        "app",
        "tenant",
        receipt_cid,
        receipt_cid,
        "did:key:placeholder",
        "sha256:placeholder",
        "sig:placeholder",
    );

    println!("Hosted URL:");
    println!("  {}", url.to_url_string());
    println!("\nSigning payload ({} bytes):", url.signing_payload().len());
    println!("  {}", String::from_utf8_lossy(&url.signing_payload()));
    println!("\nNote: Replace placeholder DID, RT, and SIG with real values for production.");

    Ok(())
}

// ── disasm ──────────────────────────────────────────────────────

fn cmd_disasm(input: &str, is_hex: bool) -> Result<(), Box<dyn std::error::Error>> {
    let bytecode = if is_hex {
        let clean = input.replace([' ', '\n', '\t'], "");
        hex::decode(&clean)?
    } else {
        std::fs::read(input)?
    };

    println!("=== RB-VM Disassembly ({} bytes) ===\n", bytecode.len());
    match rb_vm::disassemble(&bytecode) {
        Ok(listing) => print!("{}", listing),
        Err(e) => eprintln!("Disassembly error: {}", e),
    }
    Ok(())
}

// ── silicon compile ─────────────────────────────────────────────
//
// Bundle format (self-contained JSON):
// {
//   "chip":     { <ubl/silicon.chip body> },
//   "circuits": [ { "cid": "b3:...", "body": { <ubl/silicon.circuit body> } }, ... ],
//   "bits":     [ { "cid": "b3:...", "body": { <ubl/silicon.bit body> } }, ... ]
// }
//
// The circuit body's "bits" array and the chip body's "circuits" array use the
// bundle CIDs ("b3:...") as symbolic references.  The command:
//   1. Stores all bits → records bundle_cid → stored_cid mapping.
//   2. Rewrites each circuit's "bits" array with stored CIDs, stores circuits.
//   3. Rewrites the chip's "circuits" array with stored CIDs, stores chip.
//   4. Resolves the chip graph and compiles to rb_vm TLV bytecode.
//   5. Prints chip CID, bytecode CID, hex bytecode, and disassembly.

async fn cmd_silicon_compile(
    bundle_path: Option<&str>,
    from_store: Option<&str>,
    store_path: &str,
    hex_only: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::HashMap;
    use std::sync::Arc;
    use ubl_chipstore::{ChipStore, ExecutionMetadata, InMemoryBackend, SledBackend};
    use ubl_runtime::silicon_chip::{
        compile_chip_to_rb_vm, parse_silicon, resolve_chip_graph, SiliconRequest, TYPE_SILICON_BIT,
        TYPE_SILICON_CHIP, TYPE_SILICON_CIRCUIT,
    };
    use ubl_types::Did as TypedDid;

    // ── from-store path: open live Sled ChipStore, compile chip by CID ──
    if let Some(chip_cid) = from_store {
        let backend = Arc::new(SledBackend::new(store_path)?);
        let store = ChipStore::new(backend);

        let chip_data = store
            .get_chip(chip_cid)
            .await?
            .ok_or_else(|| format!("chip '{}' not found in store at '{}'", chip_cid, store_path))?;

        if chip_data.chip_type != TYPE_SILICON_CHIP {
            return Err(format!(
                "chip '{}' has type '{}', expected '{}'",
                chip_cid, chip_data.chip_type, TYPE_SILICON_CHIP
            )
            .into());
        }

        let chip = match parse_silicon(TYPE_SILICON_CHIP, &chip_data.chip_data)? {
            SiliconRequest::Chip(c) => c,
            _ => return Err("chip body did not parse as ubl/silicon.chip".into()),
        };

        let circuits = resolve_chip_graph(&chip, &store).await?;
        let bytecode = compile_chip_to_rb_vm(&circuits)?;

        let bc_hash = blake3::hash(&bytecode);
        let bc_cid = format!("b3:{}", hex::encode(bc_hash.as_bytes()));
        let bc_hex = hex::encode(&bytecode);

        if hex_only {
            println!("{}", bc_hex);
        } else {
            println!("=== Silicon Compile (from store) ===");
            println!();
            println!("Chip CID:            {}", chip_cid);
            println!("Store path:          {}", store_path);
            println!("Bytecode CID:        {}", bc_cid);
            println!(
                "Bytecode size:       {} bytes ({} instructions)",
                bytecode.len(),
                count_tlv_instrs(&bytecode)
            );
            println!();
            println!("=== Bytecode (hex) ===");
            println!("{}", bc_hex);
            println!();
            println!("=== Disassembly ===");
            match rb_vm::disassemble(&bytecode) {
                Ok(listing) => print!("{}", listing),
                Err(e) => eprintln!("Disassembly error: {}", e),
            }
        }
        return Ok(());
    }

    // ── bundle path: self-contained JSON ─────────────────────────
    let bundle_path = bundle_path.ok_or("provide a bundle file path or --from-store <chip_cid>")?;

    // ── parse bundle ────────────────────────────────────────────
    let bundle_str = std::fs::read_to_string(bundle_path)?;
    let bundle: Value = serde_json::from_str(&bundle_str)?;

    let chip_body = bundle
        .get("chip")
        .ok_or("bundle missing 'chip' field")?
        .clone();
    let circuits_arr = bundle
        .get("circuits")
        .and_then(|v| v.as_array())
        .ok_or("bundle missing 'circuits' array")?
        .clone();
    let bits_arr = bundle
        .get("bits")
        .and_then(|v| v.as_array())
        .ok_or("bundle missing 'bits' array")?
        .clone();

    // ── in-memory store + shared metadata ───────────────────────
    let backend = Arc::new(InMemoryBackend::new());
    let store = ChipStore::new(backend);
    let meta = ExecutionMetadata {
        runtime_version: "ublx/0.1.0".to_string(),
        execution_time_ms: 0,
        fuel_consumed: 0,
        policies_applied: vec![],
        executor_did: TypedDid::new_unchecked("did:key:ublx"),
        reproducible: true,
    };

    // ── 1. Store bits: bundle_cid → stored_cid ──────────────────
    let mut cid_map: HashMap<String, String> = HashMap::new();
    for entry in &bits_arr {
        let bundle_cid = entry
            .get("cid")
            .and_then(|v| v.as_str())
            .ok_or("bits[] entry missing 'cid'")?
            .to_string();
        let body = entry
            .get("body")
            .ok_or("bits[] entry missing 'body'")?
            .clone();
        let mut chip_data = body;
        if let Some(obj) = chip_data.as_object_mut() {
            obj.insert(
                "@type".to_string(),
                Value::String(TYPE_SILICON_BIT.to_string()),
            );
            obj.entry("@world".to_string())
                .or_insert(Value::String("a/ublx/t/cli".to_string()));
        }
        let receipt_cid = format!(
            "b3:receipt-bit-{}",
            &bundle_cid[3..].chars().take(8).collect::<String>()
        );
        let stored_cid = store
            .store_executed_chip(chip_data, receipt_cid, meta.clone())
            .await?;
        cid_map.insert(bundle_cid, stored_cid);
    }

    // ── 2. Store circuits (rewrite bits[] with stored CIDs) ──────
    for entry in &circuits_arr {
        let bundle_cid = entry
            .get("cid")
            .and_then(|v| v.as_str())
            .ok_or("circuits[] entry missing 'cid'")?
            .to_string();
        let body = entry
            .get("body")
            .ok_or("circuits[] entry missing 'body'")?
            .clone();
        let mut chip_data = body;
        if let Some(obj) = chip_data.as_object_mut() {
            obj.insert(
                "@type".to_string(),
                Value::String(TYPE_SILICON_CIRCUIT.to_string()),
            );
            obj.entry("@world".to_string())
                .or_insert(Value::String("a/ublx/t/cli".to_string()));
            // Rewrite bits[] using cid_map (bundle CID → stored CID)
            if let Some(bits_val) = obj.get("bits").and_then(|v| v.as_array()).cloned() {
                let rewritten: Vec<Value> = bits_val
                    .iter()
                    .map(|b| {
                        let s = b.as_str().unwrap_or("");
                        Value::String(cid_map.get(s).cloned().unwrap_or_else(|| s.to_string()))
                    })
                    .collect();
                obj.insert("bits".to_string(), Value::Array(rewritten));
            }
        }
        let receipt_cid = format!(
            "b3:receipt-ckt-{}",
            &bundle_cid[3..].chars().take(8).collect::<String>()
        );
        let stored_cid = store
            .store_executed_chip(chip_data, receipt_cid, meta.clone())
            .await?;
        cid_map.insert(bundle_cid, stored_cid);
    }

    // ── 3. Store chip (rewrite circuits[] with stored CIDs) ──────
    let mut chip_data = chip_body.clone();
    if let Some(obj) = chip_data.as_object_mut() {
        obj.insert(
            "@type".to_string(),
            Value::String(TYPE_SILICON_CHIP.to_string()),
        );
        obj.entry("@world".to_string())
            .or_insert(Value::String("a/ublx/t/cli".to_string()));
        if let Some(circs_val) = obj.get("circuits").and_then(|v| v.as_array()).cloned() {
            let rewritten: Vec<Value> = circs_val
                .iter()
                .map(|c| {
                    let s = c.as_str().unwrap_or("");
                    Value::String(cid_map.get(s).cloned().unwrap_or_else(|| s.to_string()))
                })
                .collect();
            obj.insert("circuits".to_string(), Value::Array(rewritten));
        }
    }
    let chip_store_cid = store
        .store_executed_chip(
            chip_data.clone(),
            "b3:receipt-chip".to_string(),
            meta.clone(),
        )
        .await?;

    // ── chip body CID = BLAKE3 content address of the raw body ───
    let chip_nrf = ubl_ai_nrf1::to_nrf1_bytes(&chip_body)?;
    let chip_content_cid = ubl_ai_nrf1::compute_cid(&chip_nrf)?;

    // ── 4. Resolve + compile ─────────────────────────────────────
    let chip = match parse_silicon(TYPE_SILICON_CHIP, &chip_data)? {
        SiliconRequest::Chip(c) => c,
        _ => return Err("chip body did not parse as ubl/silicon.chip".into()),
    };
    let circuits = resolve_chip_graph(&chip, &store).await?;
    let bytecode = compile_chip_to_rb_vm(&circuits)?;

    // ── 5. Output ────────────────────────────────────────────────
    let bc_hash = blake3::hash(&bytecode);
    let bc_cid = format!("b3:{}", hex::encode(bc_hash.as_bytes()));
    let bc_hex = hex::encode(&bytecode);

    if hex_only {
        println!("{}", bc_hex);
    } else {
        println!("=== Silicon Compile ===");
        println!();
        println!("Chip CID (content):  {}", chip_content_cid);
        println!("Store CID:           {}", chip_store_cid);
        println!("Bytecode CID:        {}", bc_cid);
        println!(
            "Bytecode size:       {} bytes ({} instructions)",
            bytecode.len(),
            count_tlv_instrs(&bytecode)
        );
        println!();
        println!("=== Bytecode (hex) ===");
        println!("{}", bc_hex);
        println!();
        println!("=== Disassembly ===");
        match rb_vm::disassemble(&bytecode) {
            Ok(listing) => print!("{}", listing),
            Err(e) => eprintln!("Disassembly error: {}", e),
        }
    }

    Ok(())
}

/// Count TLV instructions in a bytecode buffer (each is 3-byte header + payload).
fn count_tlv_instrs(bytecode: &[u8]) -> usize {
    let mut count = 0;
    let mut i = 0;
    while i + 2 < bytecode.len() {
        let len = u16::from_be_bytes([bytecode[i + 1], bytecode[i + 2]]) as usize;
        i += 3 + len;
        count += 1;
    }
    count
}

// ── silicon disasm ───────────────────────────────────────────────

fn cmd_silicon_disasm(input: &str, is_file: bool) -> Result<(), Box<dyn std::error::Error>> {
    let bytecode = if is_file {
        std::fs::read(input)?
    } else {
        let clean = input.replace([' ', '\n', '\t'], "");
        hex::decode(&clean)?
    };

    println!(
        "=== Silicon Chip Disassembly ({} bytes, {} instructions) ===\n",
        bytecode.len(),
        count_tlv_instrs(&bytecode),
    );
    match rb_vm::disassemble(&bytecode) {
        Ok(listing) => print!("{}", listing),
        Err(e) => eprintln!("Disassembly error: {}", e),
    }
    Ok(())
}
