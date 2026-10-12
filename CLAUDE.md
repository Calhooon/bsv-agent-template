# bsv-agent-template

Rust Cloudflare Worker template with BRC-31 mutual authentication and BRC-29 micropayments. Fork and add your own paid endpoints.

## Project Structure

```
bsv-agent-template/
├── src/
│   └── lib.rs              # All worker logic: routing, auth, payment, handlers
├── Cargo.toml              # Rust deps (bsv-middleware-cloudflare, bsv-rs, worker 0.8)
├── wrangler.toml            # CF Worker config (placeholder account_id, KV id, CHAINTRACKS_URL)
├── package.json             # npm run dev / npm run build
├── .dev.vars.example        # Template for SERVER_PRIVATE_KEY
├── .dev.vars                # Actual secrets (gitignored)
└── .gitignore
```

## How to Run

```bash
# Local development
npm run dev
# → http://localhost:8787

# Build only (no server)
npm run build

# Compile check
cargo check --target wasm32-unknown-unknown

# Deploy to Cloudflare
npx wrangler deploy
```

## Dependencies

- **bsv-sdk** (crates.io `bsv-rs` 0.4.5, `package = "bsv-rs"`) - BSV primitives: `PrivateKey`, `ProtoWallet`, `create_nonce`/`verify_nonce`, `from_base64`, `VerifiedPayment`.
- **worker** 0.8 (crates.io) - Cloudflare Workers Rust runtime. Build with the global `worker-build` 0.8.x.
- **serde** / **serde_json** (crates.io) - JSON serialization.

## Architecture

### Endpoints

| Route | Method | Auth | Payment | Handler |
|:------|:------:|:----:|:-------:|:--------|
| `/` | GET | No | No | Inline in `main()` |
| `/.well-known/x402-info` | GET | No | No | `handle_x402_info()` |
| `/.well-known/auth` | POST | - | - | `process_auth()` (middleware) |
| `/free` | POST | Yes | No | `handle_free()` |
| `/paid` | POST | Yes | 10 sats | `handle_paid()` |

### Request Flow

1. `main()` → CORS preflight check → public endpoints → auth middleware → routing
2. Auth middleware (`process_auth`) handles BRC-31 handshake (returns `AuthResult::Response`) or verifies signatures (returns `AuthResult::Authenticated`)
3. Authenticated requests are routed by path to handler functions
4. All authenticated responses are signed via `sign_json_response()`

### Payment Flow (`handle_paid`)

1. Check `x-bsv-payment` header
2. **No header** → `create_nonce()` → bind nonce→price in KV (5-min TTL) → return 402 with payment headers (version, satoshis-required, derivation-prefix)
3. **Header present** → parse `BsvPayment` JSON → `verify_nonce()` → verify nonce exists in KV → `from_base64()` decode tx → `verify_brc29_payment_verified()` with required CHAINTRACKS_URL and obtain txid → claim D1 nonce → `WorkerStorageClient` internalize with sender label via storage.babbage.systems → return 200 with receipt

### Key Constants

```rust
const AGENT_NAME: &str = "bsv-agent-template";  // Manifest name
const ORIGINATOR: &str = "bsv-agent-template";   // Nonce originator (must match between create/verify)
const PAID_ENDPOINT_PRICE: u64 = 10;             // Satoshis for /paid endpoint
```

## Adding New Endpoints

1. Write handler fn - follow `handle_free` (no payment) or `handle_paid` (with payment)
2. Add match arm in `main()` routing section
3. Add entry in `handle_x402_info()` manifest
4. Set price via constant at top of file

## Testing

Requires [MetaNet Client](https://projectbabbage.com) running locally (wallet at `localhost:3321`).

```bash
# Using x402-client (https://github.com/Calgooon/x402)
python3 brc31_helpers.py discover "http://localhost:8787"
python3 brc31_helpers.py auth POST "http://localhost:8787/free"
python3 brc31_helpers.py pay POST "http://localhost:8787/paid"  # spends 10 real sats
```

Or basic health check:

```bash
curl http://localhost:8787/
# {"status":"ok","service":"bsv-agent-template"}
```

## Configuration

- **SERVER_PRIVATE_KEY** - 64-char hex private key. Set in `.dev.vars` (local) or `npx wrangler secret put` (production).
- **AUTH_SESSIONS** - KV namespace binding for session storage. Create with `npx wrangler kv namespace create AUTH_SESSIONS`.
- **account_id** - Your Cloudflare account ID in `wrangler.toml`.
- **CHAINTRACKS_URL** - `[vars]` in `wrangler.toml`; your block-header (ChainTracks) service. Placeholder `https://chaintracks.invalid` until you set it.

## Critical Notes

- Payment internalization goes through `storage.babbage.systems` (mainnet). Test payments spend real sats.
- The nonce originator (`ORIGINATOR`) must match between `create_nonce` and `verify_nonce` - changing it after issuing 402s will break in-flight payments.
- `.dev.vars` is gitignored. Only `.dev.vars.example` is committed. Never commit real keys.
- The KV id in `wrangler.toml` is a placeholder - must be replaced before `wrangler dev` will work with KV persistence.

Payment verification uses bsv-middleware-cloudflare 0.5.1, bsv-rs 0.4.5 and worker 0.8.
CHAINTRACKS_URL is required. The shipped .invalid URL is a placeholder and fails closed.
Verification runs before the D1 nonce claim. Storage failures release the claim and keep the quote.
Only accepted payments consume the quote. No second BEEF parse is needed for the receipt txid.

| Refusal | Signed status/code | Quote |
| --- | --- | --- |
| KeyDerivation, NoHeaderService, Source | 500 ERR_SERVER_MISCONFIGURED | Kept |
| Server-side Unverifiable | 503 ERR_HEADER_SERVICE_UNAVAILABLE | Kept; retry same payment |
| Payer-side Unverifiable, wrong script, underpayment, root mismatch | 400 ERR_PAYMENT_INVALID, reason Display | Kept; correct payment |

The CLAIMS D1 binding requires your own database and the middleware payment_claims schema.
All account, namespace, database and key configuration in this template is placeholders only.
