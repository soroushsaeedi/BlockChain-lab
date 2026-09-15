mod rpc;

use rayon::prelude::*;
use rpc::Rpc;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::time::Instant;

const RPC_ADDR: &str = "127.0.0.1:18443";
/// Override with the BITCOIN_COOKIE environment variable.
const COOKIE_PATH: &str =
    r"E:\Personal Project\BlockChain Network\Tools\regset-data\regtest\.cookie";

#[derive(Clone, Copy)]
struct BlockHeader {
    version: u32,
    prev_hash: [u8; 32],
    merkle_root: [u8; 32],
    timestamp: u32,
    nbits: u32,
    nonce: u32,
}

impl BlockHeader {
    fn serialize(&self) -> [u8; 80] {
        let mut buf = [0u8; 80];
        buf[0..4].copy_from_slice(&self.version.to_le_bytes());
        buf[4..36].copy_from_slice(&self.prev_hash);
        buf[36..68].copy_from_slice(&self.merkle_root);
        buf[68..72].copy_from_slice(&self.timestamp.to_le_bytes());
        buf[72..76].copy_from_slice(&self.nbits.to_le_bytes());
        buf[76..80].copy_from_slice(&self.nonce.to_le_bytes());
        buf
    }

    fn block_hash(&self) -> [u8; 32] {
        let digest = Sha256::digest(Sha256::digest(self.serialize()));
        let mut out = [0u8; 32];
        out.copy_from_slice(&digest);
        out.reverse();
        out
    }

    /// Expand the compact `nbits` encoding into the full 256-bit target,
    /// big-endian. target = mantissa * 256^(exponent - 3).
    fn target(&self) -> [u8; 32] {
        let exponent = (self.nbits >> 24) as usize;
        let mantissa = self.nbits & 0x00ff_ffff;
        let mut t = [0u8; 32];
        // Big-endian bytes of the mantissa; [0] is always 0 since mantissa <= 0xffffff.
        let bytes = mantissa.to_be_bytes();
        let start = 32 - exponent;
        t[start..start + 3].copy_from_slice(&bytes[1..4]);
        t
    }

    /// Proof of work: the block hash, read as a 256-bit number, must be below
    /// the target. Both are big-endian, so a plain byte compare is a numeric one.
    fn meets_target(&self) -> bool {
        self.block_hash() < self.target()
    }

    fn parse(bytes: &[u8]) -> Result<BlockHeader, ParseError> {
        Ok(BlockHeader {
            version: read_u32_le(bytes, 0)?,
            prev_hash: read_hash(bytes, 4)?,
            merkle_root: read_hash(bytes, 36)?,
            timestamp: read_u32_le(bytes, 68)?,
            nbits: read_u32_le(bytes, 72)?,
            nonce: read_u32_le(bytes, 76)?,
        })
    }
}

/// Render bytes as lowercase hex.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn double_sha256(data: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(Sha256::digest(data));
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

/// Parse 64 hex chars into 32 bytes, reversing byte order.
fn unhex32(s: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    for i in 0..32 {
        let pair = &s[i * 2..i * 2 + 2];
        out[31 - i] = u8::from_str_radix(pair, 16).unwrap();
    }
    out
}

#[derive(Debug, PartialEq)]
enum ParseError {
    UnexpectedEnd,
    TrailingBytes,
    NoTransactions,
    InsufficientWork,
    MerkleMismatch,
}

fn read_u32_le(bytes: &[u8], at: usize) -> Result<u32, ParseError> {
    match bytes.get(at..at + 4) {
        Some(slice) => Ok(u32::from_le_bytes(slice.try_into().unwrap())),
        None => Err(ParseError::UnexpectedEnd),
    }
}

fn read_u16_le(bytes: &[u8], at: usize) -> Result<u16, ParseError> {
    match bytes.get(at..at + 2) {
        Some(slice) => Ok(u16::from_le_bytes(slice.try_into().unwrap())),
        None => Err(ParseError::UnexpectedEnd),
    }
}

fn read_u64_le(bytes: &[u8], at: usize) -> Result<u64, ParseError> {
    match bytes.get(at..at + 8) {
        Some(slice) => Ok(u64::from_le_bytes(slice.try_into().unwrap())),
        None => Err(ParseError::UnexpectedEnd),
    }
}

fn read_varint(bytes: &[u8], at: usize) -> Result<(u64, usize), ParseError> {
    let first = *bytes.get(at).ok_or(ParseError::UnexpectedEnd)?;
    match first {
        0xfd => Ok((read_u16_le(bytes, at + 1)? as u64, 3)),
        0xfe => Ok((read_u32_le(bytes, at + 1)? as u64, 5)),
        0xff => Ok((read_u64_le(bytes, at + 1)?, 9)),
        n => Ok((n as u64, 1)),
    }
}

fn read_hash(bytes: &[u8], at: usize) -> Result<[u8; 32], ParseError> {
    match bytes.get(at..at + 32) {
        Some(slice) => Ok(slice.try_into().unwrap()),
        None => Err(ParseError::UnexpectedEnd),
    }
}

fn tx_length(bytes: &[u8], start: usize) -> Result<usize, ParseError> {
    let mut at = start;
    at += 4; // version

    let (n_inputs, len) = read_varint(bytes, at)?;
    at += len;
    for _ in 0..n_inputs {
        at += 32 + 4; // previous txid + previous index
        let (script_len, len) = read_varint(bytes, at)?;
        at += len;
        at += script_len as usize; // skip the script
        at += 4; // sequence
    }

    let (n_outputs, len) = read_varint(bytes, at)?;
    at += len;
    for _ in 0..n_outputs {
        at += 8; // value
        let (script_len, len) = read_varint(bytes, at)?;
        at += len;
        at += script_len as usize; // skip the script
    }

    at += 4; // locktime
    if at > bytes.len() {
        return Err(ParseError::UnexpectedEnd);
    }
    Ok(at - start)
}

fn split_transactions(bytes: &[u8]) -> Result<Vec<&[u8]>, ParseError> {
    let (count, len) = read_varint(bytes, 80)?;
    let mut at = 80 + len;
    let mut txs = Vec::new();
    for _ in 0..count {
        let tx_len = tx_length(bytes, at)?;
        txs.push(&bytes[at..at + tx_len]);
        at += tx_len;
    }
    if at != bytes.len() {
        return Err(ParseError::TrailingBytes);
    }
    Ok(txs)
}

fn merkle_root(txids: &[[u8; 32]]) -> [u8; 32] {
    let mut level: Vec<[u8; 32]> = txids.to_vec();
    while level.len() > 1 {
        if level.len() % 2 == 1 {
            level.push(*level.last().unwrap()); // odd: duplicate the last
        }
        let mut next = Vec::new();
        for pair in level.chunks(2) {
            let mut joined = [0u8; 64];
            joined[..32].copy_from_slice(&pair[0]);
            joined[32..].copy_from_slice(&pair[1]);
            next.push(double_sha256(&joined));
        }
        level = next;
    }
    level[0]
}

fn verify_block(bytes: &[u8]) -> Result<(), ParseError> {
    let header = BlockHeader::parse(bytes)?;
    if !header.meets_target() {
        return Err(ParseError::InsufficientWork);
    }
    let txs = split_transactions(bytes)?;
    if txs.is_empty() {
        return Err(ParseError::NoTransactions);
    }
    let txids: Vec<[u8; 32]> = txs.iter().map(|tx| double_sha256(tx)).collect();
    if merkle_root(&txids) != header.merkle_root {
        return Err(ParseError::MerkleMismatch);
    }
    Ok(())
}

fn height_push(height: u32) -> Vec<u8> {
    if (1..=16).contains(&height) {
        return vec![0x50 + height as u8];
    }
    let mut num = Vec::new();
    let mut h = height;
    while h > 0 {
        num.push((h & 0xff) as u8); // little-endian, lowest byte first
        h >>= 8;
    }
    if num.last().map_or(false, |&b| b & 0x80 != 0) {
        num.push(0); // top bit set would mean negative; add a zero byte
    }
    let mut out = vec![num.len() as u8]; // "push N bytes"
    out.extend_from_slice(&num);
    out
}

fn build_coinbase(height: u32, value: u64) -> Vec<u8> {
    let mut script = height_push(height);
    let tag = b"soroush";
    script.push(tag.len() as u8); // push 7 bytes
    script.extend_from_slice(tag);

    let mut tx = Vec::new();
    tx.extend_from_slice(&1u32.to_le_bytes()); // version
    tx.push(1); // input count
    tx.extend_from_slice(&[0u8; 32]); // prev txid: none
    tx.extend_from_slice(&0xffff_ffffu32.to_le_bytes()); // prev index: none
    tx.push(script.len() as u8); // script length
    tx.extend_from_slice(&script);
    tx.extend_from_slice(&0xffff_ffffu32.to_le_bytes()); // sequence
    tx.push(1); // output count
    tx.extend_from_slice(&value.to_le_bytes()); // value
    tx.push(1); // script length
    tx.push(0x51); // OP_TRUE
    tx.extend_from_slice(&0u32.to_le_bytes()); // locktime
    tx
}

/// Append `n` as a CompactSize varint — the inverse of `read_varint`.
fn write_varint(out: &mut Vec<u8>, n: u64) {
    match n {
        0..=0xfc => out.push(n as u8),
        0xfd..=0xffff => {
            out.push(0xfd);
            out.extend_from_slice(&(n as u16).to_le_bytes());
        }
        0x1_0000..=0xffff_ffff => {
            out.push(0xfe);
            out.extend_from_slice(&(n as u32).to_le_bytes());
        }
        _ => {
            out.push(0xff);
            out.extend_from_slice(&n.to_le_bytes());
        }
    }
}

/// Assemble a full block: 80-byte header, tx count, transactions.
fn serialize_block(header: &BlockHeader, txs: &[Vec<u8>]) -> Vec<u8> {
    let mut block = Vec::new();
    block.extend_from_slice(&header.serialize());
    write_varint(&mut block, txs.len() as u64);
    for tx in txs {
        block.extend_from_slice(tx);
    }
    block
}

/// The parts of a `getblocktemplate` reply this miner uses.
struct Template {
    version: u32,
    prev_hash: [u8; 32],
    timestamp: u32,
    nbits: u32,
    height: u32,
    /// Reward the coinbase may claim. The template's transactions are left
    /// out (empty blocks), so their fees are subtracted: claiming fees for
    /// transactions not in the block is rejected as `bad-cb-amount`.
    reward: u64,
}

fn fetch_template(rpc: &Rpc) -> Result<Template, Box<dyn Error>> {
    let t = rpc.call("getblocktemplate", json!([{"rules": ["segwit"]}]))?;
    let field = |name: &str| t[name].as_u64().ok_or(format!("template: missing {name}"));

    let prev = t["previousblockhash"].as_str().ok_or("template: missing previousblockhash")?;
    if prev.len() != 64 {
        return Err("template: previousblockhash is not 64 hex chars".into());
    }
    let bits = t["bits"].as_str().ok_or("template: missing bits")?;
    let fees: u64 = t["transactions"]
        .as_array()
        .ok_or("template: missing transactions")?
        .iter()
        .map(|tx| tx["fee"].as_u64().unwrap_or(0))
        .sum();

    Ok(Template {
        version: field("version")? as u32,
        prev_hash: unhex32(prev), // display order -> internal order
        timestamp: field("curtime")? as u32,
        nbits: u32::from_str_radix(bits, 16)?,
        height: field("height")? as u32,
        reward: field("coinbasevalue")? - fees,
    })
}

/// Search the whole nonce range in parallel for a header meeting its target.
fn mine(header: BlockHeader) -> Option<BlockHeader> {
    (0..=u32::MAX)
        .into_par_iter()
        .map(|nonce| BlockHeader { nonce, ..header })
        .find_any(|candidate| candidate.meets_target())
}

/// Usage: `cargo run -- [blocks]` — fetch work, mine, submit, repeat.
fn main() -> Result<(), Box<dyn Error>> {
    let blocks: u32 = match std::env::args().nth(1) {
        Some(arg) => arg.parse()?,
        None => 1,
    };
    let cookie = std::env::var("BITCOIN_COOKIE").unwrap_or(COOKIE_PATH.to_string());
    let rpc = Rpc::new(RPC_ADDR, &cookie);

    for _ in 0..blocks {
        let t = fetch_template(&rpc)?;
        let coinbase = build_coinbase(t.height, t.reward);
        let header = BlockHeader {
            version: t.version,
            prev_hash: t.prev_hash,
            merkle_root: merkle_root(&[double_sha256(&coinbase)]),
            timestamp: t.timestamp,
            nbits: t.nbits,
            nonce: 0,
        };

        let start = Instant::now();
        let header = mine(header).ok_or("nonce space exhausted (would need an extranonce)")?;
        let elapsed = start.elapsed();

        // Check our own work with the stage 4 verifier before the node sees it.
        let block = serialize_block(&header, &[coinbase]);
        if let Err(e) = verify_block(&block) {
            return Err(format!("built an invalid block: {e:?}").into());
        }

        // submitblock returns null on success, or a short rejection reason.
        let result = rpc.call("submitblock", json!([hex(&block)]))?;
        match result.as_str() {
            None => println!(
                "height {:>4}  nonce {:>10}  {}  accepted ({elapsed:.2?})",
                t.height,
                header.nonce,
                hex(&header.block_hash())
            ),
            Some(reason) => {
                return Err(format!("height {}: node rejected block: {reason}", t.height).into());
            }
        }
    }

    let count = rpc.call("getblockcount", json!([]))?;
    println!("node's chain height: {count}");
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_125552_hash_matches() {
        let h = BlockHeader {
            version: 1,
            prev_hash: unhex32("00000000000008a3a41b85b8b29ad444def299fee21793cd8b9e567eab02cd81"),
            merkle_root: unhex32("2b12fcf1b09288fcaff797d71e950e71ae42b91e8bdb2304758dfcffc2b620e3"),
            timestamp: 1305998791,
            nbits: 0x1a44b9f2,
            nonce: 2504433986,
        };
        assert_eq!(
            hex(&h.block_hash()),
            "00000000000000001e8d6829a8a21adc5d38d0a473b144b6765798e61f98bd1d"
        );
        assert_eq!(
            hex(&h.target()),
            "00000000000044b9f20000000000000000000000000000000000000000000000"
        );
        assert!(h.meets_target());
    }

    #[test]
    fn parse_version_from_raw_block() {
        let bytes = include_bytes!("../block125552.bin");
        let version = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        assert_eq!(version, 1);
    }

    #[test]
    fn read_u32_past_end_is_error() {
        let bytes = include_bytes!("../block125552.bin");
        assert_eq!(read_u32_le(bytes, 1494), Err(ParseError::UnexpectedEnd));
        assert_eq!(read_u32_le(bytes, 1492), Ok(0));
    }

    #[test]
    fn block_1_hash_matches() {
        let h = BlockHeader {
            version: 1,
            prev_hash: unhex32("000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f"),
            merkle_root: unhex32("0e3e2357e806b6cdb1f70b54c3a3a17b6714ee1f0e68bebb44a74b1efd512098"),
            timestamp: 1231469665,
            nbits: 0x1d00ffff,
            nonce: 2573394689,
        };
        assert_eq!(
            hex(&h.serialize()),
            "010000006fe28c0ab6f1b372c1a6a246ae63f74f931e8365e15a089c68d61900000000\
             00982051fd1e4ba744bbbe680e1fee14677ba1a3c3540bf7b1cdb606e857233e0e61bc\
             6649ffff001d01e36299"
        );
        assert_eq!(
            hex(&h.block_hash()),
            "00000000839a8e6886ab5951d76f411475428afc90947ee320161bbf18eb6048"
        );
        assert!(h.meets_target());
    }

    #[test]
    fn parse_header_from_raw_block() {
        let bytes = include_bytes!("../block125552.bin");
        let h = BlockHeader::parse(bytes).unwrap();
        assert_eq!(
            hex(&h.block_hash()),
            "00000000000000001e8d6829a8a21adc5d38d0a473b144b6765798e61f98bd1d"
        )
    }

    #[test]
    fn varint_reads() {
        let bytes = include_bytes!("../block125552.bin");
        assert_eq!(read_varint(bytes, 80), Ok((4, 1)));
        assert_eq!(read_varint(&[0xfd, 0x2c, 0x01], 0), Ok((300, 3)));
    }

    #[test]
    fn coinbase_length() {
        let bytes = include_bytes!("../block125552.bin");
        assert_eq!(tx_length(bytes, 81), Ok(135));
    }

    #[test]
    fn split_block_into_transactions() {
        let bytes = include_bytes!("../block125552.bin");
        let txs = split_transactions(bytes).unwrap();
        assert_eq!(txs.len(), 4);
        assert_eq!(txs[0].len(), 135);
        let total: usize = txs.iter().map(|tx| tx.len()).sum();
        assert_eq!(80 + 1 + total, 1496);
    }

    #[test]
    fn coinbase_txid() {
        let bytes = include_bytes!("../block125552.bin");
        let txs = split_transactions(bytes).unwrap();
        let mut id = double_sha256(txs[0]);
        id.reverse(); // display order
        assert_eq!(
            hex(&id),
            "51d37bdd871c9e1f4d5541be67a6ab625e32028744d7d4609d0c37747b40cd2d"
        );
    }

    #[test]
    fn merkle_root_matches_header() {
        let bytes = include_bytes!("../block125552.bin");
        let header = BlockHeader::parse(bytes).unwrap();
        let txids: Vec<[u8; 32]> = split_transactions(bytes)
            .unwrap()
            .iter()
            .map(|tx| double_sha256(tx))
            .collect();
        assert_eq!(merkle_root(&txids), header.merkle_root);
    }

    #[test]
    fn real_block_verifies() {
        let bytes = include_bytes!("../block125552.bin");
        assert_eq!(verify_block(bytes), Ok(()));
    }

    #[test]
    fn tampered_transaction_is_caught() {
        let mut bytes = include_bytes!("../block125552.bin").to_vec();
        bytes[136] ^= 0x01; // coinbase value 0x40 -> 0x41: one extra satoshi
        assert_eq!(verify_block(&bytes), Err(ParseError::MerkleMismatch));
    }

    #[test]
    fn height_encoding() {
        assert_eq!(height_push(1), vec![0x51]);
        assert_eq!(height_push(17), vec![0x01, 0x11]);
        assert_eq!(height_push(128), vec![0x02, 0x80, 0x00]);
        assert_eq!(height_push(255), vec![0x02, 0xff, 0x00]);
        assert_eq!(height_push(300), vec![0x02, 0x2c, 0x01]);
    }

    #[test]
    fn coinbase_parses_back() {
        let tx = build_coinbase(1, 5_000_000_000);
        assert_eq!(tx.len(), 70);
        assert_eq!(tx_length(&tx, 0), Ok(70));
    }

    #[test]
    fn varint_round_trips() {
        for n in [0, 0xfc, 0xfd, 0xffff, 0x1_0000, 0xffff_ffff, 0x1_0000_0000] {
            let mut buf = Vec::new();
            write_varint(&mut buf, n);
            assert_eq!(read_varint(&buf, 0), Ok((n, buf.len())));
        }
    }

    #[test]
    fn mined_regtest_block_verifies() {
        let coinbase = build_coinbase(1, 5_000_000_000);
        let header = BlockHeader {
            version: 0x2000_0000,
            prev_hash: unhex32("0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206"),
            merkle_root: merkle_root(&[double_sha256(&coinbase)]),
            timestamp: 1789457083,
            nbits: 0x207fffff,
            nonce: 0,
        };
        let header = mine(header).unwrap();
        let block = serialize_block(&header, &[coinbase]);
        assert_eq!(verify_block(&block), Ok(()));
    }
}