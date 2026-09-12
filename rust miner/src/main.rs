use sha2::{Digest, Sha256};
use std::time::Instant;

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
}

/// Render bytes as lowercase hex.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
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

fn main() {
    let mut h = BlockHeader {
        version: 1,
        prev_hash: unhex32("00000000000008a3a41b85b8b29ad444def299fee21793cd8b9e567eab02cd81"),
        merkle_root: unhex32("2b12fcf1b09288fcaff797d71e950e71ae42b91e8bdb2304758dfcffc2b620e3"),
        timestamp: 1305998791,
        nbits: 0x1e001fff,
        nonce: 0,
    };

    println!("target: {}", hex(&h.target()));

    let start = Instant::now();
    let mut found = None;

    for nonce in 0..=u32::MAX {
        h.nonce = nonce;
        if h.meets_target() {
            found = Some(nonce);
            break;
        }
    }

    let elapsed = start.elapsed();

    match found {
        Some(nonce) => {
            let hashes = nonce as f64 + 1.0;
            println!("found nonce: {nonce}");
            println!("hash:        {}", hex(&h.block_hash()));
            println!("elapsed:     {elapsed:.2?}");
            println!("hashrate:    {:.0} hashes/sec", hashes / elapsed.as_secs_f64());
        }
        None => println!("exhausted the nonce space in {elapsed:.2?}"),
    }
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
}