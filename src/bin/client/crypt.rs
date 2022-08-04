use ed25519_dalek::{Keypair, SecretKey, PublicKey, Signer, SIGNATURE_LENGTH};
use sha3::{Sha3_256, Digest};
use aes::{Aes256, cipher::{KeyIvInit, StreamCipher}};
use ctr::Ctr128BE;

use publichat::constants::*;

pub fn hash(data: &[u8]) -> Hash {
    let mut res = [0; HASH_SIZE];

    let mut hasher = Sha3_256::new();
    hasher.update(data);
    res.copy_from_slice(&hasher.finalize());

    res
}

pub fn apply_aes(key: &Hash, buf: &mut Cypher) {
    // applies AES in-place on buf as side-effect
    const IV: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];

    let mut cypher = Ctr128BE::<Aes256>::new(key.into(), &IV.into());
    cypher.apply_keystream(buf);
}

pub fn make_keypair(input: &[u8]) -> Result<Keypair, &'static str> {
    // hash input data to get a neat 32 bytes
    let hash = hash(input);

    let secret = SecretKey::from_bytes(&hash)
        .map_err(|_| "Failed to make private key")?;
    let public = PublicKey::from(&secret);

    Ok(Keypair{secret, public})
}

pub fn sign(cypher: &Cypher, keypair: &Keypair) -> [u8; SIGNATURE_LENGTH] {
    let hash = hash(cypher);
    let sig = keypair.sign(&hash);
    sig.to_bytes()
}
