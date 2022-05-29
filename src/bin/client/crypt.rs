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

pub fn apply_aes(key: &Hash, buf: &mut Contents) {
    let mut cypher = AesCtr::new(key.into(), &IV.into());
    cypher.apply_keystream(buf);
}
