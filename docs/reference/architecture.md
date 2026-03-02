# UBL CORE — Architecture & Engineering Specification

**Status**: Active normative architecture document
**Date**: February 20, 2026 (rev 4)
**Implementation status source**: `TASKLIST.md`
**Documentation index**: `docs/index.md`

> **Universal Business Leverage** leverages the best of determinism with the best of the stochasticism of LLMs — both comfortable and at maximum potential, with limits expressed by clear rules.

The machine layer (NRF-1, BLAKE3, RB-VM) is **deterministic at the content level**: given the same canonical input and the same version of rules, it produces the same bytes and the same chip CID. Receipt CIDs remain event-specific by design (see PF-02). The LLM layer operates above it with full creative latitude — but grounded by the Universal Envelope (`@type`, `@id`, `@ver`, `@world`) and bounded by policies compiled into bytecode. Neither side is constrained to be the other. Determinism doesn't try to be creative. LLMs don't try to be precise. The system is the interface where both do what they're best at.

> **PF-01 — Determinism Contract.** Given the same canonical input (after UNC-1 normalization + Universal Envelope), the same version of rules (NRF-1 / `@ver`), and the same declared configuration, UBL produces exactly the same bytes and the same CID. This is **content determinism** — it is absolute and tested by KATs. **Binary determinism** (same source → same executable hash) is NOT guaranteed by default; Rust does not produce reproducible binaries without explicit toolchain controls (`--remap-path-prefix`, locked `codegen-units`, Nix/srtool). The `binary_hash` field in receipts is **observability for forensic auditing**, not a trust anchor. Trust comes from the Ed25519 signature chain via `ubl_kms`. Binary reproducibility is a future hardening phase (Nix/srtool pipeline).

### Engineering Principles

- **All Rust, always.** Native Rust solutions only. No shelling out, no FFI unless absolutely unavoidable. The ecosystem is rich enough.
- **Research before implementing.** Before writing any component, search the web. This industry evolves by the minute. Someone may have solved it better, and their solution may fit the pipeline. The question is always: *does this fit in canon JSON → canon bytecode → pipeline → gate?* If yes, adopt it. If no, build it.
- **Everything through the pipeline.** No side channels. Every action is a chip, every chip goes through KNOCK→WA→CHECK→TR→WF, every output is a receipt. If it can't be expressed as a chip flowing through the gate, it doesn't belong in the system.
- **The gate is the only entry point.** Nothing bypasses `ubl_gate`. Not admin tools, not debug endpoints, not migrations. If it mutates state, it's a chip.
- **Auth is the pipeline.** There is no separate auth system. Registration = `ubl/user` chip. Login = `ubl/token` chip. Permission = policy evaluation at CHECK. Blocking or permitting people is exactly what the pipeline does — it's just a policy on a chip type.
- **One pipeline, many services.** Attestation, witnessing, notarization, proofing, documentation — all are the same pipeline with different `@type`s and different policies. One copy of the gate served over HTTPS handles all of them. The commercial surface is configuration, not code.
- **The LLM is an Accountable Advisor.** The LLM judges, sorts, suggests, routes, narrates — but the pipeline decides. The LLM *signs its work* via `ubl/advisory` receipts. Judged wrong? That LLM. Write it down, move on. It has rights (advise, read context, suggest) and duties (sign, be traceable, be accountable). The first app is **AI Passport** (`ubl/ai.passport`) — an LLM's identity, rights, and duties as a chip.
- **Leverage = Pipeline × Engine.** `UBL = Deterministic Pipeline × LLM Engine`. Determinism provides proof, enforcement, verification. The LLM Engine provides understanding, advice, judgment. Neither alone is sufficient. The product is greater than the sum.
- **Software as Story.** The receipt chain is a narrative. Each receipt is a sentence, the chain is a paragraph, the chip's lifecycle is a chapter. Design principle: *if you can tell the story, you can build the chip. If you can't tell the story, you don't understand the feature yet.*
- **Ghost is an open question.** Three interpretations exist: (a) any DENY at any stage, (b) allowed in but failed during execution, (c) something else. The architecture doesn't depend on settling this now. Every stage receipts. Every receipt is evidence. The `ghost` flag is metadata whose precise semantics will be refined as we build.

---

## 0. The UBL Protocol Stack

UBL is not a product. It is a **protocol stack** — eight layers that turn any domain into a deterministic, auditable, LLM-augmented system.

| Layer | What | Examples |
|---|---|---|
| **Chips** | The atomic unit — every action, every fact, every intent | `ubl/user`, `ubl/payment`, `vcx/manifest`, `ubl/advisory` |
| **Pipeline** | The deterministic engine that processes chips | KNOCK→WA→CHECK→TR→WF, fuel metering |
| **Policy Gates** | Governance — what's allowed, who decides | Genesis, P0→P1, quorum, dependency chains, autonomia matrix |
| **Runtime** | The Certified Runtime — executor, arbiter, notary | Deterministic execution, sandbox, fuel, `runtime_hash`, self-attestation |
| **Receipts** | Proof of everything that happened | UnifiedReceipt, stage evolution, HMAC-BLAKE3 auth chain, policy trace |
| **Registry** | Identity + state + history | CID/DID, ChipStore, append-only ledger, Rich URLs |
| **Protocols** | Domain-specific chip vocabularies | Auth, Money, Media (VCX), Advisory, Documents |
| **Products** | Configuration on top of protocols | AI Passport, Notarization, Video editor, Payment gateway |

Chips are the input *and* the output. A receipt is a chip. A policy is a chip. An advisory is a chip. A payment is a chip. A video manifest is a chip. It's chips all the way down.

**You never write a new system.** You write a new `@type`, a new policy, and maybe a new WASM adapter. The pipeline, the gate, the receipts, the registry — they're already there. That's the leverage.

### What the client sees

1. **You send a Chip** — your intent, your action, your fact.
2. **The system validates it** — is it well-formed? who are you? what are the rules?
3. **The system executes it** — deterministically, under policy, with fuel limits.
4. **You get a Receipt** — signed proof of what happened, why, under which rules.
5. **It's in the Registry** — permanent, verifiable, auditable, with a URL you can share.

Five steps. No acronyms. Works for auth, payments, video, documents, AI advisories — any domain.

### Vision Handoff

Strategic vision, long-horizon protocol narrative, and roadmap content now live in:

- `docs/visao/MANIFESTO_DA_REINVENCAO.md`
- `docs/visao/VCX-Core.md`

This architecture document stays implementation-oriented and normative.

---

## 1. Origin and Evolution

This system descends from the **UBL Master Blueprint v2.0** (Chip-as-Code & Registry-First). The Blueprint established four invariant laws (Canon, Determinism, Identity/Scope, Receipt-is-State), the Chip-as-Code model, the fractal RB→Circuit→Policy hierarchy, and the WA→TR→WF pipeline.

The Rust codebase implemented these ideas but **evolved significantly** from the original spec:

| Blueprint Concept | What the Code Actually Does | Status |
|---|---|---|
| **BLAKE3 everywhere** | Canon/CID and receipt paths use BLAKE3 consistently across runtime crates. | ✅ Resolved |
| **5 stages**: KNOCK→WA→TR→EXECUTE→WF | **5 stages**: KNOCK→WA→CHECK→TR→WF. KNOCK is explicit (`knock.rs`, 11 tests). CHECK is the policy stage. | ✅ Resolved |
| **RB-VM opcodes** | **19 TLV opcodes**: JSON-oriented, linear, receipt-native. No JMP. | ✅ Locked — deliberate redesign |
| **Policy as bytecode chip** | PolicyBit/Circuit/ReasoningBit in Rust structs with Expression DSL. Fractal policy wiring with per-RB vote traces. | ✅ Working — bytecode compilation deferred |
| **S3 key layout** | `FsCas` with hash-sharded paths; `NdjsonLedger` for filesystem audit log | ✅ Ledger working — S3 backend future |
| **Double-Read** (cache + canonical path) | Single path only — no caching layer | Future optimization |
| **Newtype pattern** (Cid, ChipBody, UserId) | `ubl_types` newtypes (`Cid`, `Did`, `Kid`, `World`, etc.) are integrated in critical runtime and receipt paths. | ✅ Resolved |
| **Parse, Don't Validate** | Core pipeline paths parse and anchor once, then execute via typed context. | ✅ Implemented on critical paths |
| **Structured logging** (tracing) | Runtime + gate use structured `tracing` spans for operational visibility. | ✅ Resolved (PS4) |
| **LLM Advisory at KNOCK** (Gate σ) | LLM Observer consumes events post-pipeline, never at KNOCK | ✅ Correct — advisory stays off critical path |
| **Receipt as nested JSON** | `UnifiedReceipt` evolves through stages, CID recomputed per append, HMAC-BLAKE3 auth chain | ✅ Resolved S3.1 — 11 unit + 4 integration tests |
| **Proptest for canon** | Property-based suites are active in core canon/numeric/vm paths. | ✅ Resolved |

The four Laws remain **inviolable**. Everything else is implementation detail that evolved.

---

## 2. Crate Map (as-built)

| Crate | Role | Status |
|---|---|---|
| `ubl_ai_nrf1` | NRF-1.1 canonical encoding, CID (BLAKE3), Universal Envelope, chip format | ✅ Working (108 tests) |
| `rb_vm` | Deterministic stack VM, TLV bytecode, fuel metering | ✅ Working (79 tests) |
| `ubl_runtime` | Full pipeline: KNOCK→WA→CHECK→TR→WF, auth, onboarding, genesis, advisory, event bus | ✅ Working (352 tests) |
| `ubl_receipt` | UnifiedReceipt with stage evolution, HMAC-BLAKE3 auth chain, Decision enum | ✅ Working (22 tests) |
| `ubl_chipstore` | CAS storage, InMemory + Sled backends, indexing, query builder | ✅ Wired into pipeline WF stage |
| `ubl_ledger` | `NdjsonLedger` (filesystem), `InMemoryLedger` (testing) | ✅ Working (6 tests) |
| `ubl_did` | DID document generation, `did:cid:` resolution | ✅ Minimal but functional |
| `ubl_config` | `BASE_URL` from env | ✅ Trivial |
| `ubl_cli` | `ublx verify` / `ublx build` for .chip files | ✅ Working |
| `ubl_gate` | Axum HTTP gateway — `POST /v1/chips`, `GET /v1/chips/:cid`, `GET /v1/receipts/:cid/trace`, advisory endpoints | ✅ Fully rewritten with real ChipStore |
| `logline` | Structured text parser/serializer (from TDLN) — Layer 2 renderer | ✅ Working (full roundtrip, tokenizer, AST, builder) |

### Key modules inside `ubl_runtime`

| Module | Role |
|---|---|
| `pipeline/mod.rs` + `pipeline/processing.rs` + `pipeline/stages/*` | Modular KNOCK→WA→CHECK→TR→WF orchestration, real rb_vm execution at TR |
| `auth.rs` | 8 onboarding chip types (`ubl/app`, `ubl/user`, `ubl/tenant`, `ubl/membership`, `ubl/token`, `ubl/revoke`, `ubl/worldscope`, `ubl/role`), body validation, dependency chain enforcement (34 unit + 10 integration tests) |
| `genesis.rs` | Bootstrap genesis chip in ChipStore at startup, idempotent, self-signed |
| `knock.rs` | Input validation: size ≤1MB, depth ≤32, array ≤10K, no dup keys, valid UTF-8, required `@type`/`@world` (11 tests) |
| `error_response.rs` | Canonical `UblError` with Universal Envelope format, stable error codes, and HTTP/JSON-RPC mappings |
| `advisory.rs` | Advisory engine for post-CHECK and post-WF LLM hooks |
| `ai_passport.rs` | AI Passport chip type — LLM identity, rights, duties |
| `ledger.rs` | `LedgerWriter` trait, `NdjsonLedger`, `InMemoryLedger` (6 tests) |
| `event_bus.rs` | In-process event bus for receipt events |
| `durable_store.rs` | SQLite durability boundary: atomic commit (`receipts + idempotency + outbox`) |
| `outbox_dispatcher.rs` | Durable outbox claim/ack/nack + retry/backoff worker |
| `transition_registry.rs` | Deterministic TR bytecode resolution by `@type`/profile/override |

### 2.1 What Works End-to-End Today

```text
POST /v1/chips → KNOCK (validate) → WA (seal intent) → CHECK (policy + onboarding) → TR (rb_vm) → WF (receipt + store)
```

**Working**: Full 5-stage pipeline with real rb_vm execution. Chips stored in ChipStore. UnifiedReceipt evolves through stages. Genesis bootstrap at startup. Onboarding dependency chain enforced (app→user→tenant→membership→token→revoke). Canonical error responses. Advisory engine. AI Passport. Event bus. Gate serves real ChipStore lookups and receipt traces.

**Hardening in progress**: reproducible-build hardening remains. Runtime self-attestation, structured tracing, and durability boundary are implemented.

---

## 3. Canon & NRF-1 (Law I)

### 3.1 Decisions — LOCKED

| Decision | Value | Rationale |
|---|---|---|
| **Hash** | BLAKE3, 32 bytes | Fast, parallel, no length-extension. `rb_vm` already uses it. |
| **CID format** | `b3:` + lowercase hex, 64 chars | `b3:a1b2c3...` (32 bytes = 64 hex chars) |
| **Strings** | NFC normalized, BOM rejected | Already enforced in `ubl_ai_nrf1::nrf.rs` |
| **Prohibited chars** | `\u0000`–`\u001F` in source YAML | Escape required in NRF string encoding |
| **Surrogates** | Reject unpaired surrogates | Invalid UTF-8 → DENY at KNOCK |
| **Numbers** | `i64` for simple integers; UNC-1 `@num` objects for all other numerics | `json_to_nrf` rejects raw floats. See §3.3 UNC-1. |
| **Decimals** | UNC-1 `dec/1` (`mantissa × 10^−scale`, bigint strings) | Replaces planned `NrfValue::Decimal(i128, u8)`. See [docs/canon/UNC-1.md](./docs/canon/UNC-1.md). |
| **Null vs absence** | Null values REMOVED from maps | Absence ≠ null; `{"a": null}` canonicalizes to `{}` |
| **Map key order** | Strict Unicode code point ascending, post-NFC | Already uses `BTreeMap` in `nrf.rs` |
| **Duplicate keys** | Reject (DENY) | Must fail at parse, not silently deduplicate |

### 3.2 UNC-1 — Numeric Canon (new)

All non-integer numbers use **UNC-1** (`@num` tagged objects). No IEEE-754 in canon.

| Kind | Tag | Fields | Use Case |
|---|---|---|---|
| **INT** | `int/1` | `v` (string bigint) | Counts, IDs |
| **DEC** | `dec/1` | `m` (mantissa), `s` (scale) | Finance, measurement |
| **RAT** | `rat/1` | `p`, `q` (strings) | Exact fractions |
| **BND** | `bnd/1` | `lo`, `hi` (Num) | IEEE-754 imports, uncertainty |

- Optional `u` field for units (`"USD"`, `"kg"`, etc.)
- Rounding mode is explicit at the operation site, never in the data
- `f64` imports become `bnd/1` (minimal interval) — imprecision is always visible
- NRF-1 encodes UNC-1 atoms as MAP — zero changes to encoding layer
- Full spec: [docs/canon/UNC-1.md](./docs/canon/UNC-1.md)
- Crate: `crates/ubl_unc1/`
- Schema: `schemas/unc-1.schema.json`
- KATs: `kats/unc1/unc1_kats.v1.json`

### 3.3 Two-Layer Canonical Representation

Every chip exists in two canonical forms. Both are deterministic; one is for machines, the other is for LLMs and humans.

**Layer 0 — Machine Canon (NRF-1 bytecode)**

The deterministic binary encoding. This is what gets hashed → CID, signed, and stored. One byte per type tag. No ambiguity. The CID is derived exclusively from this layer.

**Layer 1 — LLM Canon (Anchored JSON)**

A minimal, flat JSON derived deterministically from the bytecode. Designed for LLM consumption without requiring computation. Mandatory anchor fields prevent drift:

```json
{"@id":"b3:a1b2...","@type":"ubl/user","@ver":"1.0","@world":"a/acme/t/prod","body":{"email":"bob@acme.com","theme":"dark"}}
```

Rules:
- **Flat**: Minimal nesting (max 2 levels in body). No deep trees.
- **Anchored**: `@id` (CID), `@type`, `@ver`, `@world` (app/tenant scope) are always present at the top. These ground the LLM — it always knows *what* it's reading, *which version*, and *where* it lives.
- **Low overhead**: No pretty-printing, no comments, no trailing commas. One line per chip.
- **Deterministic**: `LLM_Canon(chip) = json_from_nrf1(nrf1_bytes)`. Same bytecode → same JSON → always.
- **Read-only contract**: The LLM reads this form. To write, it produces this form, which gets compiled to NRF-1 bytecode and verified.

The three-layer canonical stack:

```
Layer 0: NRF-1 bytecode     → Machine (hash, sign, store)
Layer 1: Anchored JSON       → LLM (read, write, reason)
Layer 2: LogLine             → Human (debug, audit, observe) — future
```

All three are deterministic derivations of the same data. Layer 0 is truth. Layer 1 is derived. Layer 2 is rendered. The `logline` crate (from TDLN, already built) is the renderer for Layer 2 — but human-facing representation is ultimately UI, not text. Layer 2 is deferred.

Key rule for Layer 1: `@type` is always the **first key**, `@id` always **second**. LLMs read left-to-right; first token = grounding.

**Universal Envelope Rule**: The anchored JSON is not just for chips — it is the base format for **everything** in the system. Chips, receipts, events, API responses, error payloads, policy traces — all share the same minimum fields:

```json
{"@id":"...","@type":"...","@ver":"...","@world":"..."}
```

Different types add fields on top (`body`, `stages`, `decision`, `error`, `trace`, etc.) but **no message may have fewer than these four anchors**. This means:

- A **receipt** is `{"@id":"b3:...","@type":"ubl/wf","@ver":"1.0","@world":"a/acme/t/prod","stages":[...],"decision":"allow",...}`
- An **event** is `{"@id":"b3:...","@type":"ubl/event","@ver":"1.0","@world":"a/acme/t/prod","event_type":"receipt.created",...}`
- An **error** is `{"@id":"b3:...","@type":"ubl/error","@ver":"1.0","@world":"a/acme/t/prod","code":"POLICY_DENIED",...}`
- A **policy** is `{"@id":"b3:...","@type":"ubl/policy","@ver":"1.0","@world":"a/acme/t/prod","rules":[...],...}`

One format. Always anchored. Always parseable by the same code. An LLM reading any UBL artifact always sees the same four fields first and immediately knows what it is, what version, and where it belongs.

### 3.3 CID Contract (BLAKE3)

**Current state**: `ubl_ai_nrf1::compute_cid` and `rb_vm` both use BLAKE3. CID is derived from NRF bytes and encoded as `b3:<hex>`.

**Contract**: one hash function everywhere in the trust path.

```
cid = "b3:" + hex::encode(blake3::hash(nrf1_bytes).as_bytes())
```

### 3.4 NRF-1 Header Type Codes

| Code | Type | Stage |
|---|---|---|
| `0x10` | Chip (generic) | — |
| `0x11` | WA Receipt | Stage 1 |
| `0x12` | TR Receipt | Stage 3 |
| `0x13` | WF Receipt | Stage 4 |
| `0x14` | Policy | — |
| `0x15` | Advisory | — |
| `0x16` | Knock | Stage 0 |
| `0x17` | Ghost | WBE |
| `0x18` | Unified Receipt | Future |

Flags byte (reserved): bit 0 = ghost, bit 1 = signed, bits 2-7 = reserved.

---

## 4. RB-VM (Law II)

### 4.1 Current State

`rb_vm` is the most mature crate. It implements a deterministic stack VM with:
- **19 opcodes** in TLV (Type-Length-Value) bytecode format
- **Fuel metering** — 1 unit per opcode, configurable limit
- **No-IO by construction** — only `CasProvider` and `SignProvider` traits
- **Ghost mode** — same execution, flagged in receipt
- **10 Laws** verified by 633 lines of tests with golden CIDs

### 4.2 Opcode Table — LOCKED

| Byte | Opcode | Fuel | Stack Effect | Payload |
|---|---|---|---|---|
| `0x01` | `ConstI64` | 1 | → i64 | 8 bytes BE |
| `0x02` | `ConstBytes` | 1 | → bytes | N bytes |
| `0x03` | `JsonNormalize` | 1 | bytes → json | — |
| `0x04` | `JsonValidate` | 1 | json → json | — |
| `0x05` | `AddI64` | 1 | i64, i64 → i64 | — |
| `0x06` | `SubI64` | 1 | i64, i64 → i64 | — |
| `0x07` | `MulI64` | 1 | i64, i64 → i64 | — |
| `0x08` | `CmpI64` | 1 | i64, i64 → bool | 1 byte op |
| `0x09` | `AssertTrue` | 1 | bool → ∅ | — |
| `0x0A` | `HashBlake3` | 1 | bytes → bytes | — |
| `0x0B` | `CasPut` | 1 | bytes → cid | — |
| `0x0C` | `CasGet` | 1 | cid → bytes | — |
| `0x0D` | `SetRcBody` | 1 | json → ∅ | — |
| `0x0E` | `AttachProof` | 1 | cid → ∅ | — |
| `0x0F` | `SignDefault` | 1 | (no-op, signing at EmitRc) | — |
| `0x10` | `EmitRc` | 1 | → (terminates) | — |
| `0x11` | `Drop` | 1 | a → ∅ | — |
| `0x12` | `PushInput` | 1 | → cid | 2 bytes BE index |
| `0x13` | `JsonGetKey` | 1 | json → i64 | UTF-8 key |

### 4.3 Decisions — LOCKED

| Decision | Value | Rationale |
|---|---|---|
| **Fuel ceiling per TR** | 1,000,000 units | Prevents runaway; DENY if exceeded |
| **Cost model** | 1 unit/opcode (MVP) | Future: weighted by opcode class |
| **Types** | `i64`, `bool`, `bytes`, `cid`, `json`, `unit` | No implicit conversions — type mismatch = DENY |
| **Halting** | No JMP/LOOP opcodes | Fuel-bounded linear execution only |
| **Signature domain** | `"ubl-rb-vm/v1"` context string | Prevents cross-domain replay |
| **Arithmetic overflow** | Saturating (`saturating_add/sub/mul`) | Already implemented |

### 4.4 Implemented Opcodes (S2.6) + Planned

| Byte | Opcode | Status |
|---|---|---|
| `0x14` | `Dup` | ✅ Implemented S2.6 |
| `0x15` | `Swap` | ✅ Implemented S2.6 |
| `0x16` | `VerifySig` | ✅ Implemented S2.6 — Ed25519 verify with domain separation |
| `0x17` | `DecimalAdd` | Planned — depends on `NrfValue::Decimal` |
| `0x18` | `DecimalCmp` | Planned — depends on `NrfValue::Decimal` |

---

## 5. Pipeline (WA→TR→WF)

### 5.1 Stage Flow

```
KNOCK → WA (ghost) → CHECK (policy) → TR (rb_vm) → WF (final receipt)
  │         │              │               │              │
  │         │              │               │              └─ Store in ChipStore
  │         │              │               └─ Execute bytecode, emit RC
  │         │              └─ Evaluate policy chain (genesis→app→tenant→chip)
  │         └─ Create ghost record, freeze time, assign policy_cid
  └─ Validate input size/depth, rate limit, assign nonce
```

### 5.2 Unified Receipt (✅ Implemented — S3.1)

Single `UnifiedReceipt` that evolves through stages. CID recomputed after each stage append. HMAC-BLAKE3 auth chain links stages. Its JSON form follows the Universal Envelope — the receipt is just another chip that an LLM can read without special-casing. 11 unit tests + 4 integration tests.

Gate read paths (`GET /v1/receipts/:cid`, `GET /v1/receipts/:cid/trace`, `GET /v1/chips/:cid/verify`) verify the receipt auth-chain before returning success. Broken chains return `TAMPER_DETECTED` (HTTP 422).

```rust
struct UnifiedReceipt {
    v: u32,                           // Schema version
    t: String,                        // RFC-3339 UTC
    did: String,                      // Issuer DID
    subject: Option<String>,          // Subject DID
    kid: String,                      // Key ID: did:key:z...#ed25519
    nonce: String,                    // Anti-replay (see §6.2)
    stages: Vec<StageExecution>,      // Append-only
    decision: Decision,               // Current decision state
    effects: serde_json::Value,       // Side-effects record
    rt: RuntimeInfo,                  // binary_sha256, env, certs
    prev_receipt_cid: Option<String>, // Chain linkage
    receipt_cid: String,              // b3:hash(NRF(self_without_sig))
    sig: String,                      // Ed25519 JWS detached
}

struct StageExecution {
    stage: PipelineStage,             // WA, CHECK, TR, WF
    timestamp: String,                // RFC-3339 UTC
    input_cid: String,                // What entered this stage
    output_cid: Option<String>,       // What this stage produced
    fuel_used: Option<u64>,           // For TR stage
    policy_trace: Vec<PolicyTraceEntry>, // For CHECK stage
    signature: String,                // Stage executor signature
    auth_token: String,               // HMAC proving stage N authorizes stage N+1
}
```

**Auth chain**: Each stage computes `auth_token = HMAC-BLAKE3(stage_secret, prev_stage_cid || stage_name)`. Next stage verifies before executing.

**CID evolution**: `receipt_cid` recomputed after each stage append. The WF `receipt_cid` is the final canonical CID.

### 5.3 rb_vm Pipeline Integration (✅ Implemented — S2.1 + P1 registry wiring)

TR stage creates a `Vm` instance and executes TLV bytecode selected by `TransitionRegistry` (not a fixed passthrough blob). `PipelineCas`, `PipelineSigner`, and `PipelineCanon` implement rb_vm traits. Fuel usage, bytecode provenance, and adapter metadata are recorded in TR `vm_state`.

Resolution order:
1. chip override `@tr.bytecode_hex`
2. chip override `@tr.profile`
3. env map `UBL_TR_BYTECODE_MAP_JSON`
4. env map `UBL_TR_PROFILE_MAP_JSON`
5. built-in default profile by `@type`

```rust
let resolution = transition_registry.resolve(&request.chip_type, &request.body)?;
let instructions = tlv::decode_stream(&resolution.bytecode)?;
let outcome = vm.run(&instructions)?;
// outcome.rc_cid, outcome.fuel_used, outcome.steps + resolution metadata -> TR receipt
```

### 5.4 Input Validation (KNOCK stage)

| Check | Limit | Action |
|---|---|---|
| Max chip size | 1 MB | DENY at KNOCK |
| Max receipt size | 1 MB | DENY at WF |
| Max JSON depth | 32 levels | DENY at KNOCK |
| Max array length | 10,000 elements | DENY at KNOCK |
| Duplicate keys | 0 allowed | DENY at KNOCK |
| Input normalization (ρ) | NFD→NFC, BOM strip, map null-strip, timestamp/set normalization | Normalize at KNOCK; reject collisions/control chars |
| Cost per byte | 1 fuel unit per 1KB | Added to TR fuel budget |

ρ validation/normalization failures include a JSON path (for example `body.name` or `body.profile.email`) to make rejection causes actionable.

---

## 6. Policy Model

### 6.1 Composition Hierarchy

```
Genesis Policy (immutable, self-signed)
  └─ App Policy (per application)
       └─ Tenant Policy (per tenant within app)
            └─ Chip Policy (per chip type)
```

Evaluation order: genesis first (most general), chip-specific last. First DENY short-circuits.

### 6.2 Policy ROM

- `policy_cid` is **immutable** once written into a WA receipt
- Policy migration: deploy new policy chip → update app/tenant config to reference new `policy_cid` → new chips use new policy; old receipts remain valid under old policy
- **No retroactive policy changes** — a receipt's `policy_cid` is its law forever

### 6.3 Policy Imports & Lockfile

**Current**: Policies resolved at runtime via `PolicyLoader.load_policy_chain()`.

**Target**: Compile-time resolution with lockfile:

```yaml
# policy.lock
genesis: b3:abc123...
app/acme: b3:def456...
tenant/acme-prod: b3:789abc...
```

TR stage verifies lockfile CIDs match loaded policies. Divergence = DENY.

### 6.4 RB → Circuit → PolicyBit (the fractal)

The policy model is fractal — the same pattern at every scale:

```
Layer 0:  Reasoning Bit (RB)     — atomic decision: ALLOW/DENY/REQUIRE
Layer 1:  Circuit                 — graph of RBs wired together
Layer 2:  PolicyBit               — composition of Circuits into governance
Layer 3+: PolicyBits compose further (fractal)
```

A Reasoning Bit is a transistor. A Circuit is an integrated circuit. A PolicyBit is a board. Boards compose into systems. Same pattern, every level.

- **ReasoningBit**: Atomic decision unit with `Expression` condition language. Evaluates to Allow/Deny/Require. Every RB produces a receipt proving its decision.
- **Circuit**: Composes RBs with `CompositionMode` (Sequential/Parallel/Conditional) and `AggregationMode` (All/Any/Majority/KofN/FirstDecisive). A Circuit produces a composed receipt.
- **PolicyBit**: Groups circuits with a `PolicyScope` (chip types, operations, level). The PolicyBit produces the final governance receipt.

K-of-N: The policy trace must expose individual RB votes. `SEAL` markers identify which RBs are audit anchors.

The genesis chip is the root PolicyBit — the first board in the system. Every other policy inherits from it.

---

## 7. Identity, Scope & Replay Prevention

### 7.1 `@world` — The Scope Anchor

Every chip lives in a world. The `@world` field in the Universal Envelope is the logical address:

```
@world = "a/{app}/t/{tenant}"
```

- `a/acme/t/prod` — the production tenant of the Acme app
- `a/acme/t/dev` — the dev tenant of the same app
- `a/public/t/dev` — public staging's dev environment

Rules:
- A chip **cannot reference** chips in a different `@world` unless the policy explicitly allows cross-world reads.
- The gate resolves `@world` from the authenticated DID's membership. No world in the request = DENY at KNOCK.
- `@world` is frozen into the WA receipt and cannot change after that point.
- The genesis policy lives at `@world = "a/_system/t/_genesis"` — the root world.

### 7.2 DID & Key Management

| Decision | Value |
|---|---|
| DID method | `did:key:z...` Ed25519 with strict multicodec (`0xED01`) support + compat fallback |
| Key ID format | `did:key:z...#ed25519` |
| Key rotation | New `kid` published as `ubl/key.rotate` chip; old kid valid for verification of past receipts |

`UBL_STAGE_SECRET` fallback is derived from signing key material using domain-separated BLAKE3 (`ubl.stage_secret.v1`), never by reusing raw Ed25519 private key bytes.
| Signing curve | Ed25519 (RFC 8032) |

### 7.3 Anti-Replay (✅ Implemented — S2.3)

Each WA receipt includes a `nonce` field (16-byte random hex). Pipeline checks against `seen_nonces` set.

```
nonce = BLAKE3(did || tenant_id || monotonic_counter)
```

- Counter is per-key, per-tenant, monotonically increasing
- Gate rejects WA with nonce ≤ last-seen nonce for that (did, tenant) pair
- Anti-replay window: 5 minutes for clock skew tolerance

### 7.4 Signature Domain Separation

All signatures include a context prefix to prevent cross-domain replay:

| Context | Domain String |
|---|---|
| Receipt signing | `"ubl/receipt/v1"` |
| RB-VM signing | `"ubl-rb-vm/v1"` |
| Policy signing | `"ubl-policy/v1"` |
| URL signing | `"ubl/rich-url/v1"` |

Format: `sig = Ed25519.sign(key, domain_string || BLAKE3(payload))`

---

## 8. Storage

### 8.1 ChipStore (✅ Wired — S3.3)

`ubl_chipstore` provides:
- `ChipStoreBackend` trait with `InMemoryBackend` and `SledBackend`
- `ChipIndexer` with in-memory indexes (type, tag, executor) rebuilt via backend `scan_all()`
- `ChipQueryBuilder` with sorting, pagination, filtering
- `CommonQueries` for customers, payments, audit trails

**Integration**: `UblPipeline` accepts `Arc<ChipStore>` and calls `store_executed_chip()` in the WF stage. Wired since S3.3.

### 8.2 Ledger Key Layout (S3/Garage)

```
{root}/{prefix[0:2]}/{prefix[2:4]}/{full_cid}
```

Example: `cas/a1/b2/b3:a1b2c3d4e5f6...`

- GET is O(1) by CID
- Idempotent writes (content-addressed)
- `FsCas` in `rb_vm` already implements this with BLAKE3

### 8.3 Ledger (✅ Implemented — S3.4)

`ubl_ledger` provides `LedgerWriter` trait with `NdjsonLedger` (filesystem) and `InMemoryLedger` (testing). 6 tests.
- Append-only NDJSON audit log per (app, tenant)
- Receipt and ghost lifecycle events
- Failures warn-logged, never block pipeline
- Future: S3-compatible object storage (Garage/MinIO for self-hosted)

---

## 9. WASM Adapters

### 9.1 Execution Model

WASM adapters run in the TR stage for chips that require external effects (email, payment, etc.).

| Constraint | Value |
|---|---|
| No filesystem | WASI FS disabled |
| No clock | `clock_time_get` returns frozen WA timestamp |
| No network | All I/O via injected CAS artifacts |
| Memory limit | 64 MB per execution |
| Fuel limit | Shared with RB-VM fuel budget |
| Module pinning | `sha256(wasm_module)` recorded in receipt `rt` field |

### 9.2 ABI

```
Input:  NRF-1 bytes (chip body + context)
Output: NRF-1 bytes (result + effects)
```

The adapter receives a single NRF-1 encoded input and must return a single NRF-1 encoded output. No other I/O.

### 9.3 Adapter Registry

Each adapter is a chip of type `ubl/adapter`:
```yaml
ubl_chip: "1.0"
metadata:
  type: "ubl/adapter"
  id: "email-sendgrid-v1"
body:
  wasm_cid: "b3:..."        # CID of the WASM module
  wasm_sha256: "..."         # SHA-256 of the WASM binary
  abi_version: "1.0"
  fuel_budget: 100000
  capabilities: ["email.send"]
```

---

## 10. EventBus & Observability

### 10.1 Event Schema

```json
{
  "schema_version": "1.0",
  "event_type": "ubl.receipt.created",
  "receipt_cid": "b3:...",
  "receipt_type": "ubl/wf",
  "decision": "allow",
  "duration_ms": 42,
  "timestamp": "2026-02-15T12:00:00Z",
  "pipeline_stage": "wf",
  "idempotency_key": "b3:...",
  "metadata": { ... }
}
```

- **Idempotency key** = `receipt_cid` (exactly-once by CID)
- **Schema version** field for forward compatibility
- Topic: `ubl.receipts` on Iggy message broker

### 10.2 Observability Fields in Receipts

Every WF receipt must include:
- `fuel_used`: Total fuel consumed in TR
- `rb_count`: Number of reasoning bits evaluated
- `artifact_cids`: List of CIDs produced during execution
- `policy_trace`: Full RB vote breakdown

### 10.3 LLM Observer (as-built)

Consumes events from Iggy, performs mock AI analysis. Stays **outside the critical path** — advisory only, never blocks pipeline.

---

## 11. LLM Engine

The LLM operates beside the pipeline, not inside it. It is an **Accountable Advisor** — it acts in the world and signs what it did.

### 11.1 Hook Points

| Stage | LLM Role | Binding? |
|---|---|---|
| Pre-KNOCK | Semantic triage: "does this look like what the user intended?" | No — advisory only |
| Post-CHECK | Explain denial: "policy X rejected because..." | No — narration |
| Post-TR | Summarize execution: "this chip did X, consumed Y fuel" | No — narration |
| Post-WF | Route/classify: "this receipt belongs in category Z" | No — suggestion |
| On-demand | Audit storytelling: "here's what happened in this receipt chain" | No — narration |

The LLM never overrides an RB decision. It never produces a CID. It never touches the receipt chain directly.

### 11.2 `ubl/advisory` Receipt

Every LLM action produces a `ubl/advisory` chip — signed by the LLM's AI Passport key, following the Universal Envelope:

```json
{"@type":"ubl/advisory","@id":"b3:...","@ver":"1.0","@world":"a/acme/t/prod","passport_cid":"b3:...","action":"classify","input_cid":"b3:...","output":{"category":"compliance","confidence":0.92},"model":"gpt-4","seed":0}
```

Advisory receipts are stored, indexed, and auditable — but never block the pipeline.

### 11.3 AI Passport (`ubl/ai.passport`)

The first app. An LLM's identity, rights, and duties as a chip:

```json
{"@type":"ubl/ai.passport","@id":"b3:...","@ver":"1.0","@world":"a/acme/t/prod","model":"gpt-4","provider":"openai","rights":["advise","classify","narrate"],"duties":["sign","trace","account"],"scope":["a/acme/*"],"fuel_limit":100000,"signing_key":"did:key:z..."}
```

The passport enters the registry through the same door as everything else — POST /v1/chips.

---

## 12. Error Model

### 12.1 Canonical Error Response

```json
{
  "error": true,
  "code": "POLICY_DENIED",
  "message": "Genesis policy: chip body exceeds 1MB limit",
  "receipt_cid": "b3:...",
  "link": "/v1/receipts/b3:.../trace",
  "details": {
    "policy_id": "genesis",
    "rb_id": "size_limit",
    "limit": 1048576,
    "actual": 2097152
  }
}
```

### 12.2 Error Code Enum — LOCKED

| Code | Meaning | Stage |
|---|---|---|
| `INVALID_INPUT` | Malformed JSON, size exceeded, depth exceeded | KNOCK |
| `CANON_ERROR` | NRF-1 encoding failure, BOM, invalid Unicode | KNOCK/WA |
| `POLICY_DENIED` | Policy evaluation returned DENY | CHECK |
| `FUEL_EXHAUSTED` | TR execution exceeded fuel limit | TR |
| `TYPE_MISMATCH` | RB-VM type error | TR |
| `STACK_UNDERFLOW` | RB-VM stack underflow | TR |
| `CAS_NOT_FOUND` | CasGet on missing CID | TR |
| `SIGN_ERROR` | Signature generation/verification failure | WF |
| `STORAGE_ERROR` | ChipStore/Ledger write failure | WF |
| `invalid_signature` | Rich URL / receipt signature invalid (strict mode) | Verify |
| `runtime_hash_mismatch` | Rich URL `rt` differs from receipt runtime hash | Verify |
| `idempotency_conflict` | Replay key already committed in durable idempotency store | WF |
| `durable_commit_failed` | Atomic SQLite commit failed (`receipts + idempotency + outbox`) | WF |
| `INTERNAL_ERROR` | Unexpected system error | Any |
| `REPLAY_DETECTED` | Nonce reuse detected | WA |

### 12.3 Error → Receipt Mapping

Every error that reaches WF produces a DENY receipt with full `policy_trace`. Errors before WA (KNOCK failures) return HTTP 400 without a receipt.

---

## 13. Rich URLs

### 13.1 Format

```
https://{host}/{app}/{tenant}/receipts/{receipt_id}.json
  #cid={receipt_cid}
  &did={issuer_did}
  &rt={binary_sha256}
  &sig={url_signature}
```

### 13.2 Offline Verification

A rich URL contains enough information to:
1. Fetch the receipt JSON from the URL path
2. Recompute `b3:hash(NRF(receipt_body))` and verify it matches `cid`
3. Verify `sig` against `did:key` public key with domain `"ubl/rich-url/v1"`
4. Verify `rt` matches the expected runtime binary hash

Rollout mode is controlled by `UBL_RICHURL_VERIFY_MODE`:
- `shadow`: verify and log, do not fail request
- `strict`: fail-closed on any verification mismatch

### 13.3 Self-Contained URLs (for QR/mobile)

For offline use, the chip data can be embedded:

```
ubl://{base64url(compressed_chip)}?cid={cid}&sig={sig}
```

Max URL length: 2KB (QR code limit). Larger chips use the hosted URL format.

---

## 14. Security & DoS

### 14.1 Size Limits

| Resource | Limit |
|---|---|
| Chip body (WA input) | 1 MB |
| Receipt (WF output) | 1 MB |
| URL (self-contained) | 2 KB |
| JSON depth | 32 levels |
| Array length | 10,000 elements |
| Map keys | 1,000 per object |
| String length | 1 MB |

### 14.2 Rate Limiting

- Per-DID: 100 chips/minute
- Per-tenant: 1,000 chips/minute
- Per-IP (unauthenticated): 10 chips/minute
- Fuel cost per byte: 1 unit per 1KB of input

### 14.3 Cold Path Rejection

KNOCK stage rejects early (before WA) on:
- Oversized body
- Excessive nesting depth
- Duplicate JSON keys
- Invalid UTF-8
- Missing required fields (`@type`)

---

## 15. Acceptance Criteria

### 15.1 Determinism Boundary (PF-02)

Two distinct determinism levels exist in the pipeline:

**Chip CID — fully deterministic.**
Same canonical content → same NRF-1 bytes → same BLAKE3 hash → same `b3:` CID.
This holds across machines, runs, and time. Verified by `rb_vm` golden CID tests.

**Receipt CID — contextually unique.**
Receipts include `frozen_time` (WA), `nonce` (anti-replay), `timestamp` per stage,
and `RuntimeInfo` (binary hash, build meta). These fields are *intentionally* non-reproducible:
the receipt is proof that *this specific execution happened at this moment on this binary*.
Same chip processed twice → same chip CID, different receipt CIDs.

**Consequence:** never compare receipt CIDs for content equality. Compare chip CIDs.
Receipt CIDs are identifiers of *events*, not *content*. The auth chain
(`HMAC-BLAKE3` per stage) proves ordering and integrity within a single execution,
not reproducibility across executions.

Verified by: `rb_vm` golden CID tests + pipeline integration tests + `receipt_cid_is_deterministic` test (same inputs including forced timestamp → same CID).

### 15.2 Opcode Cost Stability

> Changing opcode costs = new VM version. Old receipts remain valid under old cost table.

### 15.3 Offline Reconstruction

> Given only `chips/` and `receipts/` directories, `ublx verify` reconstructs and verifies every receipt bit-for-bit.

### 15.4 Policy Immutability

> A receipt's `policy_cid` is its law forever. New policy = new CID = new chips only.

---

## 16. Build History & Current State

This section is intentionally evidence-based. It records what is implemented and measured; it does not define dated milestones or fixed-duration windows.

### Completed — Foundation Sprints

_Note: sprint-phase test numbers in this section are historical snapshots at delivery time. Use the measured table in this section for current totals._

| Sprint | Goal | Key Deliverables | Tests |
|---|---|---|---|
| **S1** — Canon + CID | Lock canonical encoding, Universal Envelope | NRF-1.1 encoding, CID computation, `ublx` CLI, type code table | 64 (ubl_ai_nrf1) |
| **S2** — RB-VM + Policy | Wire rb_vm into pipeline, lock policy resolution | Real TR stage execution, fuel ceiling, unified `Decision` enum, nonce/anti-replay, policy lockfile | 33 (rb_vm) |
| **S3** — Receipts + Storage + Gate | Unified receipt, persistent storage, end-to-end flow | `UnifiedReceipt` with HMAC chain, ChipStore in pipeline, `NdjsonLedger`, KNOCK stage, canonical errors, gate rewrite, genesis bootstrap | 22 (receipt) + 290 (runtime) |
| **S4** — WASM + URLs + EventBus | External effects, observability, portable URLs | WASM adapter ABI, adapter registry, Rich URL generation, event bus with idempotency, `ublx explain` | — |

### Completed — Post-Sprint

| Phase | Goal | Key Deliverables | Tests |
|---|---|---|---|
| **PS1** — AI Passport | First product on the pipeline | AI Passport chip type, advisory wiring, gate endpoints | — |
| **PS2** — Auth as Pipeline | Auth IS the pipeline — no separate auth system | `auth.rs` with 8 chip types, onboarding dependency chain, `validate_onboarding_chip` at CHECK, drift endpoints removed | 34 + 10 integration |
| **Onboarding** | Full lifecycle | `ubl/app` → `ubl/user` → `ubl/tenant` → `ubl/membership` → `ubl/token` → `ubl/revoke`. Dependency chain enforced. `DependencyMissing` (409) error code. | Included in 352 total (`ubl_runtime`) |

### Completed — Hardening

| Item | Deliverables | Tests |
|---|---|---|
| **H1** Signing key from env | `ubl_kms` crate, `signing_key_from_env()`, domain separation | 16 |
| **H2** Real DID resolution | All placeholder DIDs replaced, `did:key:z...` derived from Ed25519 via `ubl_kms` | — |
| **H3** `NaiveCanon` → full ρ | `RhoCanon` in `rb_vm/src/canon.rs`, NFC, BOM rejection, null stripping, key sorting, idempotent | 19 |
| **H4** P0→P1 rollout automation | Chip-native governance flow (proposal/activation receipts + traces + witness), no external preflight script as source of truth | — |
| **H7** Signature domain separation | `domain::RECEIPT`, `RB_VM`, `CAPSULE`, `CHIP` in `ubl_kms` | — |
| **H8** Rate limiting | Sliding-window per-key, `GateRateLimiter` (IP/tenant/DID), `prune()` | 13 |
| **H9** UNC-1 core ops | `ubl_unc1` crate: add/sub/mul/div with promotion, `to_dec`, `to_rat`, `from_f64_bits`, BND intervals | 57 |
| **H10** Policy lockfile | `PolicyLock` with YAML parse/serialize, `pin()`, `verify()` | 11 |
| **H11** RuntimeInfo + BuildMeta | `RuntimeInfo::capture()`, BLAKE3 binary hash, `BuildMeta`, wired into every receipt | 7 |
| **H13** ρ test vectors | 14 JSON edge-case files in `kats/rho_vectors/`, 16 integration tests | 16 |
| **H14** `ubl_kms` crate | `sign_canonical`, `verify_canonical`, strict+compat DID/KID derivation | 16 |
| **H15** Prometheus `/metrics` | Counters + histogram on gate | — |

### Completed — PR-A/B/C (Security, Observability, API Surface)

| Item | Deliverables | Tests |
|---|---|---|
| **PR-A P0.1** Rigid idempotency | `IdempotencyStore` keyed by `(@type,@ver,@world,@id)`, replay returns cached receipt, wired into `process_chip` | 10 |
| **PR-A P0.2** Canon-aware rate limit | `CanonFingerprint` (BLAKE3 of NRF-1 bytes) + `CanonRateLimiter`, cosmetic JSON variations hit same bucket | 7 |
| **PR-A P0.3** Secure bootstrap | `capability.rs` — `Capability` struct, `ubl/app` + first `ubl/user` require `cap.registry:init`, wired into CHECK | 15 |
| **PR-A P0.4** Receipts-as-AuthZ | `ubl/membership` requires `cap.membership:grant`, `ubl/revoke` requires `cap.revoke:execute`, audience/scope/expiration validation | — |
| **PR-B P1.5** Canonical stage events | `ReceiptEvent` extended with `input_cid`, `output_cid`, `binary_hash`, `build_meta`, `world`, `actor`, `latency_ms` | 1 |
| **PR-B P1.6** ETag/cache | `GET /v1/chips/:cid` returns `ETag`=CID, `Cache-Control: immutable`, `If-None-Match` → 304 | — |
| **PR-B P1.7** Unified error taxonomy | 4 new `ErrorCode` variants (401/404/429/503), `category()` → 8 categories, `mcp_code()` → JSON-RPC | 7 |
| **PR-C P2.8** Manifest generator | `GateManifest` → OpenAPI 3.1, MCP tool manifest, WebMCP manifest. Gate serves `/openapi.json`, `/mcp/manifest`, `/.well-known/webmcp.json` | 14 |
| **PR-C P2.9** MCP server proxy | `POST /mcp/rpc` — JSON-RPC 2.0 with `tools/list` + `tools/call` dispatching `ubl.deliver`, `ubl.query`, `ubl.verify`, `registry.listTypes` | — |
| **PR-C P2.10** Meta-chips | `ubl/meta.register` (mandatory KATs, reserved prefix check), `ubl/meta.describe`, `ubl/meta.deprecate` | 16 |

### Current Test Counts (measured on February 20, 2026)

Method: `cargo test -p <crate> -- --list` (unit + integration test harness totals)

| Crate | Tests |
|---|---|
| `rb_vm` | 79 |
| `ubl_receipt` | 22 |
| `ubl_runtime` | 352 |
| `ubl_ai_nrf1` | 108 |
| `ubl_kms` | 16 |
| `ubl_unc1` | 57 |
| `ubl_chipstore` | 10 |
| `ubl_types` | 24 |
| `ubl_gate` | 21 |
| **Total (measured set)** | **689** |

### Open — Hardening (0 critical remaining)

Current hardening baseline is closed for critical paths. Incremental type-safety expansions can continue opportunistically as refactoring work, not as a release gate.

### Vision References

Future-facing protocol horizons were moved out of this normative architecture file and consolidated in:

- `docs/visao/MANIFESTO_DA_REINVENCAO.md`
- `docs/visao/VCX-Core.md`

---

## 17. Known Technical Debt

| Item | Location | Severity | Status |
|---|---|---|---|
| ~~SHA2-256 used instead of BLAKE3~~ | ~~`ubl_ai_nrf1::compute_cid`~~ | ~~🔴 Critical — CID mismatch with rb_vm~~ | ✅ Fixed — BLAKE3 unified across runtime and VM |
| ~~Two `Decision` enums~~ | ~~`ubl_runtime` vs `ubl_receipt`~~ | ~~🟡 Confusing~~ | ✅ Fixed S2.2 — unified to `ubl_receipt::Decision` |
| ~~TR stage is placeholder~~ | ~~`pipeline.rs`~~ | ~~🔴 Critical~~ | ✅ Fixed S2.1 — real rb_vm execution |
| ~~Hardcoded signing key~~ | ~~`ubl_receipt::SIGNING_KEY`~~ | ~~🟡 Dev only~~ | ✅ Fixed H1/H14 — `ubl_kms`, `signing_key_from_env()` |
| ~~`ubl_ledger` is all no-ops~~ | ~~`ubl_ledger::lib.rs`~~ | ~~🔴 Critical~~ | ✅ Fixed S3.4 — `NdjsonLedger` + `InMemoryLedger` |
| ~~ChipStore not in pipeline~~ | ~~`UblPipeline`~~ | ~~🔴 Critical~~ | ✅ Fixed S3.3 — `Arc<ChipStore>` persists at WF |
| ~~No nonce/anti-replay~~ | ~~WA receipts~~ | ~~🟡 Replay possible~~ | ✅ Fixed S2.3 — 16-byte hex nonce + PR-A P0.1 rigid idempotency |
| ~~Placeholder DIDs~~ | ~~`"did:key:placeholder"` in WA stage~~ | ~~🟡 Must come from auth~~ | ✅ Fixed H2 — real `did:key:z...` from Ed25519 via `ubl_kms` |
| ~~`NaiveCanon` in rb_vm~~ | ~~Sorts keys but doesn't do full ρ~~ | ~~🟡 Must delegate~~ | ✅ Fixed H3 — `RhoCanon` with full ρ rules (19 tests) |
| ~~Hardcoded duration_ms~~ | ~~`50` in WF stage~~ | ~~🟢 Minor~~ | ✅ Fixed S3.7 — real `Instant::now()` timing |
| ~~Separate WA/WF receipts~~ | ~~`ubl_receipt`~~ | ~~🔴 Critical~~ | ✅ Fixed S3.1 — `UnifiedReceipt` with HMAC chain |
| ~~KNOCK implicit~~ | ~~`pipeline.rs`~~ | ~~🟡 Missing validation~~ | ✅ Fixed S3.5 — explicit `knock.rs` (11 tests) |
| ~~Gate GET stubs~~ | ~~`ubl_gate`~~ | ~~🟡 Non-functional~~ | ✅ Fixed S3.3 — real ChipStore lookups |
| ~~No canonical errors~~ | ~~HTTP responses~~ | ~~🟡 Inconsistent~~ | ✅ Fixed S3.6 + PR-B P1.7 — `UblError` with 8-category taxonomy |
| ~~4 pre-existing chip_format test failures~~ | ~~`ubl_ai_nrf1::chip_format`~~ | ~~🟡 Tests exist but fail~~ | ✅ Fixed C2 — tests were already passing |
| ~~No runtime self-attestation~~ | ~~`ubl_runtime`~~ | ~~🟡 Needed for PS3~~ | ✅ Fixed H11 — `RuntimeInfo::capture()`, BLAKE3 binary hash, `BuildMeta` |
| ~~No structured tracing~~ | ~~All crates~~ | ~~🟡 `eprintln!` only~~ | ✅ Fixed F2 — tracing spans and structured logs wired |
| ~~Newtype pattern needed~~ | ~~All crates~~ | ~~🟢 Minor~~ | ✅ Fixed H5 — `ubl_types` newtypes integrated |
| Parse, Don't Validate (beyond critical paths) | Pipeline + chip types | 🟢 Minor | ✅ Core paths migrated; incremental expansion remains optional hardening |

---

*This document is the engineering source of truth. Code that contradicts it is a bug. Decisions marked LOCKED require a new document version to change.*

*UBL is a protocol stack, not a product pitch. Eight layers — Chips, Pipeline, Policy Gates, Runtime, Receipts, Registry, Protocols, Products — that turn any domain into a deterministic, auditable, LLM-augmented system. You never write a new system. You write a new `@type`, a new policy, and the leverage is already there.*
