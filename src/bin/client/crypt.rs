use ed25519_dalek::{Keypair, SecretKey, PublicKey, Signer, SIGNATURE_LENGTH};
use sha3::{Sha3_256, Digest};
use aes::{Aes256, cipher::{KeyIvInit, StreamCipher}};
use ctr::Ctr128BE;

use publichat::constants::*;

pub fn hash(data: &[u8]) -> Hash {
    let mut hash = [0; HASH_SIZE];

    // hash once
    let mut hasher = Sha3_256::new();
    hasher.update(data);
    hash.copy_from_slice(&hasher.finalize());

    hash
}

pub fn hash_twice(title: &[u8]) -> (Hash, Hash) {
    // TODO: rewrite to use `hash()` twice?
    let mut once = [0; HASH_SIZE];
    let mut twice = [0; HASH_SIZE];

    // hash once
    let mut hasher = Sha3_256::new();
    hasher.update(title);
    once.copy_from_slice(&hasher.finalize());

    // hash twice
    let mut hasher = Sha3_256::new();
    hasher.update(once);
    twice.copy_from_slice(&hasher.finalize());

    // TODO: do hashes in a loop? zip source and dest? Might be overkill...
    (once, twice)
}

type AesCtr = Ctr128BE<Aes256>;
const IV: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];

pub fn apply_aes(key: &Hash, buf: &mut Cypher) {
    let mut cypher = AesCtr::new(key.into(), &IV.into());
    cypher.apply_keystream(buf);
}

pub fn make_keypair(input: &[u8]) -> Result<Keypair, &'static str> {
    // hash input data to get a neat 32 bytes
    let hash = hash(input);

    // make privates & publics
    let private = SecretKey::from_bytes(&hash)
        .map_err(|_| "Failed to make private key")?;
    let public = PublicKey::from(&private);

    // copy into a pair (why isn't there just a Keypair::from::<SecretKey>??)
    let mut all_bytes = [0; ed25519_dalek::KEYPAIR_LENGTH];
    all_bytes[..ed25519_dalek::SECRET_KEY_LENGTH].copy_from_slice(&private.to_bytes());
    all_bytes[ed25519_dalek::SECRET_KEY_LENGTH..].copy_from_slice(&public.to_bytes());

    Keypair::from_bytes(&all_bytes).map_err(|_| "Failed to make keypair")
}

pub fn sign(cypher: &Cypher, keypair: &Keypair) -> [u8; SIGNATURE_LENGTH] {
    let hash = hash(cypher);
    let sig = keypair.sign(&hash);
    sig.to_bytes()
}
