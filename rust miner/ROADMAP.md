# Rust Bitcoin Miner — Roadmap

A tiny solar-powered lottery miner on an ESP32, written from scratch in Rust.

## Goals

1. **Learn Rust properly** — this is the primary goal.
2. **Learn Bitcoin's real data structures** by building them, not reading about them.
3. **End with a physical gadget** that runs real mining code on a desk.

## Non-goals

- **Profit.** Settled with arithmetic on 2026-09-05. An ESP32 earns ~1e-11 BTC/year;
  odds of a block are ~1 in 240 billion per year. Self-generated solar makes it worse,
  not better, because any kWh with a buyer is worth more sold than mined. The gadget is
  a $20 lottery ticket. That is the whole point and it is a fine point.
- Anything requiring a mainnet pool account, payouts, or moving money.

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

### Stage 0 — Toolchain
Install `rustup`, create the cargo project, get `cargo build` working.

**Known blocker:** `static.rust-lang.org` and `crates.io` are behind CDNs that are
unreliable from Iranian IPs. Solve this first — configure a crates.io mirror in
`.cargo/config.toml` if needed. Do not discover this mid-build.

### Stage 1 — Toy miner
Grind arbitrary bytes against a "N leading zero bits" rule. ~50 lines.

*Teaches:* cargo, ownership basics, slices, `[u8; 32]`, iterators, a hashing crate,
CLI args. No Bitcoin structure yet — all difficulty is Rust.

*Milestone:* watch it find a hash. Compare its speed to the Python version (~500 KH/s
vs 2–5 MH/s). This is where Rust justifies itself.

### Stage 2 — Real block header
Serialize the actual 80 bytes: version, prev hash, Merkle root, timestamp, nBits, nonce.
Decode `nBits` into a 256-bit target. Double-SHA256.

*Teaches:* structs, `Result` and `?`, error handling, and the endianness fight. Expect
to get the byte order wrong three or four times — that is the stage working correctly.

*Milestone:* compute a known block's hash and match it exactly.

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

### Stage 7 — The gadget
Port the core to ESP32 (`esp-hal`, `no_std`), using the chip's hardware SHA-256
accelerator (~78 KH/s). Add an OLED showing hashrate and uptime. Solo-mine to your own
address. Run it off a small solar panel — 1.5W, so a $10 panel is plenty.

*Teaches:* embedded Rust. **Biggest difficulty spike in the project** — `no_std`, HAL,
flashing, debugging without a debugger. Do not attempt before stage 3.

*Optional easier step first:* run stages 1–5 on a Raspberry Pi. Ordinary Linux, normal
Rust, no cross-compilation.

---

## After

Two directions, both from the 2026-09-04 session:

- **Own chain from scratch** — reuse every primitive above, add fork resolution
  (the other concept never covered).
- **Postgres indexer** — plays to the ERP/data background. Async, DB-driven,
  network-bound; a good *third* project, a bad first one.

## Hardware

| Item | Cost | Needed at |
|---|---|---|
| Nothing — laptop only | $0 | Stages 0–6 |
| ESP32 + display (LilyGO T-Display S3 or similar) | $15–25 | Stage 7 |
| Small solar panel + battery | ~$10–20 | Stage 7, optional |
