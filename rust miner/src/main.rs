use sha2::{Digest, Sha256};
use std::time::Instant;

/// Render bytes as lowercase hex.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

const ATTEMPTS: u64 = 10_000_000;
fn main() {
    const ZERO_BYTES: usize = 4;
    let mut found = false;
    let start = Instant::now();

    for nonce in 0..ATTEMPTS {

        let input = format!("soroush{nonce}");
        let digest = Sha256::digest(Sha256::digest(input.as_bytes()));

        if digest[0] == 0 && digest[1] == 0 && digest[2] == 0 && digest[3] == 0 {
            println!("found it!");
            println!(" nonce = {nonce}");
            println!(" hash = {}", hex(&digest));
            found = true;
            break;
        }
    }

    let elapsed = start.elapsed();

    if !found {
        println!("gave up after {ATTEMPTS} attempts ({ZERO_BYTES} zero bytes)");
    }

    println!("elapsed: {elapsed:.2?}");
    println!("hashrate: {:.0} hashes/sec", ATTEMPTS as f64 / elapsed.as_secs_f64());

}