use sha2::{Digest, Sha256};

/// Render bytes as lowercase hex.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn main() {
    // Stage 0 smoke test: prove the toolchain links and sha2 works.
    // Bitcoin hashes twice, so this is the primitive everything else builds on.
    let digest = Sha256::digest(Sha256::digest(b"soroush"));
    println!("double-sha256(\"soroush\") = {}", hex(&digest));
}
