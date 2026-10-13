<p align="center">
  <img src="https://img.shields.io/badge/BSV-Agent_Template-blue?style=for-the-badge&logo=bitcoin-sv" alt="BSV Agent Template" />
</p>

<h1 align="center">bsv-agent-template</h1>

<p align="center">
  <strong>Ship a paid API in 15 minutes.</strong><br/>
  A Rust + Cloudflare Worker starter kit with BRC-31 authentication and BRC-29 micropayments baked in.
</p>

<p align="center">
  <a href="#quickstart">Quickstart</a> &middot;
  <a href="#endpoints">Endpoints</a> &middot;
  <a href="#how-payment-works">How Payment Works</a> &middot;
  <a href="#build-your-own">Build Your Own</a> &middot;
  <a href="#testing">Testing</a>
</p>

---

## What is this?

This template gives you a working Cloudflare Worker that:

1. **Authenticates every request** using [BRC-31](https://github.com/bitcoin-sv/BRCs/blob/master/peer-to-peer/0031.md) mutual authentication - both client and server prove their identity with cryptographic signatures on every message.

2. **Accepts micropayments** using [BRC-29](https://github.com/bitcoin-sv/BRCs/blob/master/peer-to-peer/0029.md) - paid endpoints return HTTP 402 with a payment request, the client creates a BSV transaction, and retries with payment. The server verifies and internalizes the funds automatically.

3. **Refunds on failure** using BRC-29 - if your service fails after accepting payment, the middleware issues an automatic refund transaction back to the client's wallet.

Fork it. Replace the hello-world logic with your service. Deploy. You now have a paid API.

---

## Quickstart

```bash
# 1. Clone
git clone https://github.com/Calhooon/bsv-agent-template.git
cd bsv-agent-template
npm install

# 2. Configure
cp .dev.vars.example .dev.vars
# Edit .dev.vars - set a 64-char hex private key (or generate one)

# 3. Run
npm run dev
# Server is live at http://localhost:8787
```

>
> **Set `CHAINTRACKS_URL`** in `wrangler.toml` `[vars]` to your own block-header (ChainTracks) service before you deploy. The shipped value, `https://chaintracks.invalid`, is a placeholder.

---

## Endpoints

| Route | Method | Auth | Payment | What it does |
|:------|:------:|:----:|:-------:|:-------------|
| `/`, `/health` | `GET` | - | - | Health check with status, service and build provenance |
| `/.well-known/x402-info` | `GET` | - | - | API discovery manifest (endpoints, pricing, server identity) |
| `/.well-known/auth` | `POST` | - | - | BRC-31 handshake (handled automatically by middleware) |
| `/free` | `POST` | Yes | - | Returns a greeting with the caller's identity key |
| `/paid` | `POST` | Yes | 10 sats | Returns a confirmation message + payment receipt |

---

## How Payment Works

```
  Client                                Server
    │                                     │
    │  POST /paid                         │
    │  (no payment header)                │
    │────────────────────────────────────>│
    │                                     │
    │                    Create nonce      │
    │                    Bind nonce→price  │
    │                    in KV (5 min TTL) │
    │                                     │
    │  HTTP 402                           │
    │  x-bsv-payment-satoshis-required: 10│
    │  x-bsv-payment-derivation-prefix: … │
    │<────────────────────────────────────│
    │                                     │
    │  ┌─────────────────────────┐        │
    │  │ Client wallet creates   │        │
    │  │ a 10-sat BSV transaction│        │
    │  │ to server's derived key │        │
    │  └─────────────────────────┘        │
    │                                     │
    │  POST /paid                         │
    │  x-bsv-payment: {tx, prefix, suffix}│
    │────────────────────────────────────>│
    │                                     │
    │                    Verify nonce ✓    │
    │                    Check KV binding ✓│
    │                    Delete nonce      │
    │                    (single-use) ✓    │
    │                    Decode BEEF tx ✓  │
    │                    Internalize via   │
    │                    storage server ✓  │
    │                                     │
    │  HTTP 200                           │
    │  {"message": "Payment received!",   │
    │   "payment": {"accepted": true,     │
    │               "txid": "464dec..."}} │
    │<────────────────────────────────────│
```

The full flow happens in **two HTTP round-trips**. The client library ([x402-client](https://github.com/Calgooon/x402)) handles this automatically.

---

## Build Your Own

The template is designed to be extended. Here's how to add a new paid endpoint:

### 1. Configure your agent

At the top of `src/lib.rs`:

```rust
const AGENT_NAME: &str = "my-agent";       // Displayed in x402-info manifest
const ORIGINATOR: &str = "my-agent";       // Nonce originator (must match for create/verify)
const PAID_ENDPOINT_PRICE: u64 = 10;       // Satoshis for your endpoint
```

Or define per-endpoint constants:

```rust
const PRICE_BASIC: u64 = 10;
const PRICE_PREMIUM: u64 = 1000;
```

### 2. Write a handler

**Auth only** (no payment) - copy `handle_free`:

```rust
fn handle_my_endpoint(
    auth_context: &bsv_middleware_cloudflare::types::AuthContext,
    session: &bsv_middleware_cloudflare::middleware::AuthSession,
) -> Result<Response> {
    // auth_context.identity_key has the caller's public key
    let body = serde_json::json!({
        "result": "your data here"
    });
    sign_json_response(&body, 200, &[], session)
        .map_err(|e| Error::from(e.to_string()))
}
```

**Auth + payment** - copy `handle_paid` and put your business logic in the "Payment accepted!" section (search for that comment in `src/lib.rs`).

**Auth + payment + refund** - wrap your business logic in a match block. If it fails, call `issue_refund()` to return the funds automatically. See the refund pattern comments in `handle_paid` for the full example.

### 3. Add a route

In the `match path.as_str()` block:

```rust
"/my-endpoint" => handle_my_endpoint(&auth_context, &session),
```

### 4. Update the discovery manifest

Add your endpoint to the `handle_x402_info` function so clients can discover it programmatically.

---

## CI

The required checks are `clippy`, `deny` (cargo-deny), and `secrets` (secrets scan), while `test`, `fmt`, and `wasm32` are advisory and non-blocking; docs-only PRs still run all six jobs.

## Testing

Use the [x402-client](https://github.com/Calgooon/x402) to test against your running server:

```bash
# Discover all endpoints
python3 brc31_helpers.py discover "http://localhost:8787"

# Test authentication
python3 brc31_helpers.py auth POST "http://localhost:8787/free"

# Test payment (auto-handles 402 → pay → retry)
python3 brc31_helpers.py pay POST "http://localhost:8787/paid"
```

Or test the health check with curl:

```bash
curl http://localhost:8787/health
# Returns status, service, and build (lib_rev, dirty, built_at, crates).
```

---

## Project Structure

```
bsv-agent-template/
├── src/
│   └── lib.rs              # Worker entry point, routing, handlers
├── build.rs                # Captures revision, tracked dirty state, build hour and locked crate versions
├── Cargo.toml              # Rust dependencies
├── wrangler.toml           # Cloudflare Worker config
├── package.json            # npm scripts (dev, build)
├── .dev.vars.example       # Template for local secrets
└── .gitignore
```

### Key dependencies

| Crate | Source | Purpose |
|:------|:------:|:--------|
| `bsv-middleware-cloudflare` 0.6.1 | crates.io | BRC-31 auth middleware, CORS, response signing, storage client |
| `bsv-middleware-core` 0.3.1 | crates.io | Payment verification core (transitive dependency) |
| `bsv-sdk` (package `bsv-rs` 0.5.0) | crates.io | Key derivation, nonce creation/verification, transaction parsing |
| `worker` 0.8 | crates.io | Cloudflare Workers runtime |
| `serde` / `serde_json` | crates.io | JSON serialization |

---

## Deploy to Cloudflare

```bash
# 1. Set your account_id in wrangler.toml

# 2. Create a KV namespace for auth sessions
npx wrangler kv namespace create AUTH_SESSIONS
# Copy the id into wrangler.toml

# 3. Set the server private key as a secret
npx wrangler secret put SERVER_PRIVATE_KEY

# 4. Set CHAINTRACKS_URL in wrangler.toml [vars] to your own header service

# 5. Deploy
npx wrangler deploy
```

Your agent is now live at `https://bsv-agent-template.<your-subdomain>.workers.dev`.

---

## See It in Action

[x402agency.com](https://x402agency.com) runs real agents built from this template - AI image generation, video generation, speech-to-text, and more. Try any agent with the `/x402` skill in [Claude Code](https://claude.com/claude-code):

```
/x402 https://nano-banana-pro.x402agency.com
```

The skill discovers endpoints, authenticates, negotiates payment, and handles the full flow automatically.

---

## Related Projects

- **[x402-client](https://github.com/Calgooon/x402)** - Python client library for BRC-31 auth + BRC-29 payment. Use this to test your agent.
- **[bsv-middleware-cloudflare](https://crates.io/crates/bsv-middleware-cloudflare)** - The Rust middleware that powers authentication and payment verification.
- **[x402agency.com](https://x402agency.com)** - Live agents built from this template: image generation, video generation, speech-to-text, X research.

---

Payment verification uses bsv-middleware-cloudflare 0.6.1, bsv-middleware-core 0.3.1, bsv-rs 0.5.0 and worker 0.8.
CHAINTRACKS_URL is required. The shipped .invalid URL is a placeholder and fails closed.
Verification runs before the D1 nonce claim. Failures before internalization release the claim and keep the quote.
Only accepted payments consume the quote. No second BEEF parse is needed for the receipt txid.

| Refusal | Signed status/code | Quote |
| --- | --- | --- |
| KeyDerivation, NoHeaderService, BadWorkLimit, Source | 500 ERR_SERVER_MISCONFIGURED | Kept |
| Server-side Unverifiable | 503 ERR_HEADER_SERVICE_UNAVAILABLE | Kept; retry same payment |
| Payer-side Unverifiable (including non-canonical varint), wrong script, underpayment, root mismatch | 400 ERR_PAYMENT_INVALID, reason Display | Kept; correct payment |

The CLAIMS D1 binding requires your own database and the middleware payment_claims schema.
All account, namespace, database and key configuration in this template is placeholders only.

Cargo.lock contains public crates.io SHA-256 checksums only. These documented 64-hex integrity values are not identities or secrets. Nonce tests construct example scalars 1 and 2 in memory.

After replacing the D1 placeholders, apply the included schema with `npx wrangler d1 migrations apply CLAIMS --remote`. The adapter rejects a message whose identity is not the session identity with HTTP 401 ERR_INVALID_AUTH before routing.

Every response built by a handler after verified payment, including success, refusal and generic error responses, is signed with `sign_json_response`. Following the fleet rule, a storage failure after internalize attempts a full inline refund of the actual verified amount before returning the signed error. The D1 nonce claim stays held so the accepted payment cannot be replayed, even if KV quote deletion fails. The error includes the refund remittance on success; if refund issuance fails, it reports `ERR_STORAGE_FAILED_REFUND_FAILED` and the owed amount for operator recovery. This minimal template does not journal deferred refunds. Work and memory declines remain payer-side `Unverifiable`, returned as 400 `ERR_PAYMENT_INVALID` with the reason named.

The paid handler splits authenticated multipart bodies with `split_multipart_payment` and verifies through `verify_brc29_payment_source`. Multipart BEEF is decoded in chunks; internalization uses `PaymentPart::encoded_transaction()`.
`PAYMENT_WORK_LIMIT` selects script work units per input (example: `1000000`); unset or blank means no bound. Zero or malformed values return signed 500 `ERR_SERVER_MISCONFIGURED`. Work declines and the bsv-rs 0.6.0 non-canonical varint, invalid-byte and spend refusals remain payer-side 400; missing mining context is server-side 503.
`GET /health` (also `/`) returns `status`, `service`, and `build` with `lib_rev`, tracked-file `dirty`, hourly UTC `built_at`, and the three crate versions from Cargo.lock, using bsv-worker-build-info 0.1.0. Build metadata needs no credentials. The Wrangler build command sets `BUILD_HOUR=$(date -u +%Y-%m-%dT%H)` so Cargo refreshes `built_at` each UTC hour; direct cached builds without `BUILD_HOUR` can retain an older timestamp.
After a refunded storage failure, the D1 claim remains held: request a fresh 402 quote before paying again.

## License

Dual-licensed under MIT or Apache-2.0 at your option. See LICENSE-MIT and LICENSE-APACHE.

Contributions are accepted under the same terms.
