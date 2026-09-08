# Rust Bitcoin Miner — Roadmap

A Bitcoin miner written from scratch in Rust, built to learn the language and the protocol.

## Goals

1. **Learn Rust properly** — this is the primary goal.
2. **Learn Bitcoin's real data structures** by building them, not reading about them.

## Non-goals

- **Profit.** Settled with arithmetic on 2026-09-05. At any hashrate reachable here the
  expected return is indistinguishable from zero, and mining is only economic on
  stranded energy at industrial scale. This is a learning artifact, not a business.
- Anything requiring a mainnet pool account, payouts, or moving money.
- **Hardware.** Everything runs on the laptop. No devices to buy.

## Two tracks, running in parallel

**Track A — this project.** Stages below.
**Track B — the book.** *Mastering Bitcoin* 3rd ed: ch. 4 (keys/addresses), then 6
(transactions), then 11 (blockchain/Merkle). Skip 1–2 (review) and most of 12 (covered).
The tracks are independent — don't block one on the other.

Run `rustlings` alongside track A, ~30 min at a time. Building one project teaches Rust
by immersion and leaves holes; rustlings covers the corners.

---

## Stages

Each stage ends with something that runs. Stop and commit at each one.

### Stage 0 — Toolchain ✅ done 2026-09-06
Rust 1.98.1 on the **GNU** host toolchain (no MSVC / Visual Studio needed — saves a 2–4 GB
download). Cargo project created, `sha2` added, double-SHA256 verified against Python.

*Anticipated blocker that did not materialise:* `static.rust-lang.org` and `crates.io` were
both fast and reachable. If that changes, configure a crates.io mirror in
`.cargo/config.toml`.

*Gotcha worth remembering:* PATH changes don't reach an already-running VS Code. Restart it,
or `$env:Path += ";$env:USERPROFILE\.cargo\bin"` for the current terminal.

### Stage 1 — Toy miner ✅ done 2026-09-08
Grind `"soroush" + nonce` until the hash has N leading zero bytes. ~30 lines.

*Taught:* `let` and immutability, `mut`, `format!` vs `println!`, macros vs functions,
ranges, `&str` vs `String`, references, `const`, casts with `as`, `Instant` timing,
byte indexing.

*Results — same algorithm throughout, 10,000,000 hashes each:*

| Version | Hashrate | vs start |
|---|---|---|
| debug, hex-string compare | 51,840/s | 1× |
| release, hex-string compare | 288,457/s | 5.6× |
| release, byte compare | 1,183,721/s | **22.8×** |

*The real lesson:* the bottleneck was never SHA-256. `hex()` called `format!` once per byte
— 33 heap allocations per iteration, ~340 million total. Measuring first, and only then
optimising, found that; guessing would not have.

*Left deliberately unoptimised:* `format!("soroush{nonce}")` still allocates once per
iteration. Stage 2 deletes it for free — a real header is a fixed 80-byte array with the
nonce written as raw bytes, so no text is ever built.

### Stage 2 — Real block header ✅ done 2026-09-08
Serialize the actual 80 bytes: version, prev hash, Merkle root, timestamp, nBits, nonce.
Decode `nBits` into a 256-bit target. Double-SHA256.

*Taught:* `struct` and `impl`, methods and `&self`, fixed-size arrays `[u8; 32]`,
slice ranges and `copy_from_slice`, `to_le_bytes` / `to_be_bytes`, tail-expression
returns, `#[cfg(test)]` modules and `assert_eq!`.

*Milestone hit:* block **125552** reproduced exactly — 80-byte header byte-for-byte,
hash `00000000000000001e8d6829a8a21adc5d38d0a473b144b6765798e61f98bd1d`, locked in
as a test.

*The endianness fight, resolved into three separate rules:*

1. **Integer fields** are little-endian on the wire — `to_le_bytes()`. Arbitrary;
   Satoshi picked it because x86 is LE. Matters only because everyone must agree,
   or the hashes differ and consensus breaks.
2. **Hash fields have no endianness.** They are 32-byte strings, copied as-is.
   The reversal people talk about is a *display* convention — explorers print
   hashes backwards. So `unhex32` reverses on the way in, `block_hash` reverses
   on the way out.
3. **The target is big-endian**, because it is a numeral being laid out
   most-significant-byte-first. Different job from serialization.

*Deferred:* `unhex32` still `.unwrap()`s. `Result` and `?` — listed as a stage 2
teaching goal — were not covered and are still owed.

*The one line that is proof-of-work:*

```rust
fn meets_target(&self) -> bool {
    self.block_hash() < self.target()
}
```

Everything in stage 3 is trying nonces until that returns `true`.

### Stage 3 — Parallelize
Split the nonce range across cores with `rayon`.

*Teaches:* Rust's actual selling point — safe concurrency, with a real motive.

*Milestone (the good one):* **re-mine an early difficulty-1 block.** ~4 billion hashes,
roughly half an hour single-threaded, a few minutes with rayon. You rediscover the exact
nonce Satoshi's machine found in 2009, and check it against the real block. Out of reach
in Python.

### Stage 4 — Flip it: verify instead of mine
Parse a real mainnet block from raw hex. Compute the Merkle root from the transaction
list and check it matches the header. Validate PoW.

*Teaches:* parsing, varints, tree construction — and **Merkle roots, which the 2026-09-04
session never covered.** Mostly reuses stage 2 code, backwards.

### Stage 5 — Talk to a real node
`bitcoind` on signet on the Ubuntu server. Fetch work via `getblocktemplate` over
JSON-RPC, mine it, submit the block.

*Teaches:* async Rust, `serde`, networking, RPC. **Difficulty spike — this is where
async and lifetimes arrive.** Budget for it.

*Milestone:* **actually find blocks.** On signet you control difficulty, so this works.
This is the first stage where the thing is a real miner rather than a simulation.

### Stage 6 — Stratum client (optional)
Connect to a mainnet solo pool. Real work units, real share submission.

*Caveat:* many pools geo-block Iranian IPs. Expect to test several. Skippable —
stage 5 already gave you the protocol experience without the friction.

---

## After

Two directions, both from the 2026-09-04 session:

- **Own chain from scratch** — reuse every primitive above, add fork resolution
  (the other concept never covered).
- **Postgres indexer** — plays to the ERP/data background. Async, DB-driven,
  network-bound; a good *third* project, a bad first one.
