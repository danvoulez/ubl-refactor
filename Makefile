.PHONY: all build test contract conformance wasm-conformance wasm-conformance-all quality-gate fmt lint kat gate gate-prod clean check load-validate docs-attest-keygen docs-attest-manifest docs-attest-sign docs-attest-verify bootstrap-core host-lockdown ops-maintenance forever-bootstrap workzone-cleanup

all: build

build:
	cargo build --workspace

test:
	cargo test --workspace

contract:
	bash scripts/contract_suite.sh --out-dir artifacts/contract

conformance:
	bash scripts/conformance_suite.sh --out-dir artifacts/conformance

wasm-conformance:
	bash scripts/wasm_conformance.sh --out-dir artifacts/wasm-conformance

wasm-conformance-all:
	bash scripts/wasm_conformance.sh --mode all --out-dir artifacts/wasm-conformance

quality-gate: fmt-check lint contract test conformance

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

lint:
	cargo clippy --workspace --all-targets -- -D warnings

check: fmt-check lint test

kat:
	cargo test --workspace -- --nocapture rho_ unc1_ golden_

gate:
	RUST_LOG=info cargo run -p ubl_gate

gate-prod:
	REQUIRE_UNC1_NUMERIC=true F64_IMPORT_MODE=reject RUST_LOG=info cargo run -p ubl_gate

load-validate:
	cargo test -p ubl_chipstore --test load_validation -- --ignored --nocapture

docs-attest-keygen:
	bash scripts/docs_attest.sh init-key \
		--key-out "$${KEY_OUT:-$$HOME/.ubl-core/keys/docs_attest_ed25519.pem}" \
		--pub-out "$${PUB_OUT:-$$HOME/.ubl-core/keys/docs_attest_ed25519.pub.pem}"

docs-attest-manifest:
	bash scripts/docs_attest.sh build-manifest \
		--out "$${MANIFEST_OUT:-./release-artifacts/docs/manifest.json}"

docs-attest-sign:
	@test -n "$${KEY_PATH:-}" || (echo "set KEY_PATH to private key path"; exit 1)
	bash scripts/docs_attest.sh sign \
		--manifest "$${MANIFEST_PATH:-./release-artifacts/docs/manifest.json}" \
		--key "$${KEY_PATH}" \
		$$( [ -n "$${PUB_PATH:-}" ] && printf '%s' "--pub $${PUB_PATH}" ) \
		--out "$${ATTEST_OUT:-./release-artifacts/docs/attestation.json}"

docs-attest-verify:
	bash scripts/docs_attest.sh verify \
		--manifest "$${MANIFEST_PATH:-./release-artifacts/docs/manifest.json}" \
		--attestation "$${ATTEST_PATH:-./release-artifacts/docs/attestation.json}"

bootstrap-core:
	bash scripts/ubl_ops.sh bootstrap-core --env "$${FOREVER_ENV_FILE:-./ops/forever_bootstrap.env}"

host-lockdown:
	sudo bash scripts/ubl_ops.sh host-lockdown --env "$${FOREVER_ENV_FILE:-./ops/forever_bootstrap.env}"

ops-maintenance:
	bash scripts/ubl_ops.sh ops-maintenance --env "$${FOREVER_ENV_FILE:-./ops/forever_bootstrap.env}"

forever-bootstrap: bootstrap-core

workzone-cleanup: ops-maintenance

clean:
	cargo clean
