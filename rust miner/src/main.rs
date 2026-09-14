use sha2::{Digest, Sha256};
use std::time::Instant;
use rayon::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};

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

/// Split a raw block into its transactions, each a slice of the original bytes.
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

fn main() {
    let mut h = BlockHeader {
        version: 1,
        prev_hash: unhex32("000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f"),
        merkle_root: unhex32("0e3e2357e806b6cdb1f70b54c3a3a17b6714ee1f0e68bebb44a74b1efd512098"),
        timestamp: 1231469665,
        nbits: 0x1d00ffff,
        nonce: 0,
    };

    println!("target: {}", hex(&h.target()));

    let hashes = AtomicU64::new(0);
    let start = Instant::now();
    let found = (0..=u32::MAX).into_par_iter().find_any(|&nonce| {
        hashes.fetch_add(1, Ordering::Relaxed);
        let mut candidate = h;
        candidate.nonce = nonce;
        candidate.meets_target()
    });

    let elapsed = start.elapsed();
    let total = hashes.load(Ordering::Relaxed);

    match found {
        Some(nonce) => {
            h.nonce = nonce;
            println!("found nonce: {nonce}");
            println!("hashes:      {total}");
            println!("hash:        {}", hex(&h.block_hash()));
            println!("header:      {}", hex(&h.serialize()));
            println!("hashrate:    {:.0} hashes/sec", total as f64 / elapsed.as_secs_f64());
            println!("elapsed:     {elapsed:.2?}");
        }
        None => println!("exhausted the nonce space in {elapsed:.2?}"),
    }

    let bench_h = BlockHeader { nbits: 0x03000001, ..h };
    let n: u32 = 100_000_000;
    let t = Instant::now();
    let hits = (0..n).into_par_iter().filter(|&nonce| {
        let mut c = bench_h;
        c.nonce = nonce;
        c.meets_target()
    }).count();
    let e = t.elapsed();
    println!("bench: {hits} hits, {:.0} hashes/sec", n as f64 / e.as_secs_f64());

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
}