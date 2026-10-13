# Changelog

## 0.6.2 line - 2026-10-13

- Move to adapter 0.6.2, core 0.3.2 and one bsv-rs 0.6.0; keep other locked dependencies unchanged.
- Reject a non-canonical varint in a BEEF at payment verification, before the nonce claim and internalization. Return payer-side 400 ERR_PAYMENT_INVALID naming the reason; keep the quote for a corrected payment.
- Declare the Rust 1.88 floor required by bsv-rs 0.6.0 and published by both the adapter and core.

## 0.6.1 line - 2026-10-12

- Move to adapter 0.6.1, core 0.3.1 and one bsv-rs 0.5.0; keep other locked dependencies unchanged.
- Split authenticated multipart bodies and verify payment byte sources with the optional PAYMENT_WORK_LIMIT per-input budget. The example selects 1000000 script work units per input; unset means no bound.
- Keep the exhaustive verification error match and add BadWorkLimit as signed 500 ERR_SERVER_MISCONFIGURED. SDK invalid-byte and spend refusals are payer-side 400; missing mining context is server-side 503.
- Decode the multipart encoded_transaction accessor for internalization. A held nonce claim, including after a refunded storage failure, requires a fresh 402 quote.
- Add public /health build provenance with exact bsv-worker-build-info 0.1.0: lib_rev, tracked dirty state, hourly built_at and locked crate versions.
