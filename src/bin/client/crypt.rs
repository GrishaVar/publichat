pub mod sha {
    use sha3::{Sha3_256, Digest};
    use publichat::buffers::Hash;

    pub fn hash(data: &[u8]) -> Hash::Buf {
        let mut res = Hash::DEFAULT;

        let mut hasher = Sha3_256::new();
        hasher.update(data);
        res.copy_from_slice(&hasher.finalize());

        res
    }
}

pub mod aes {
    use aes::{Aes256, cipher::{KeyIvInit, StreamCipher}};
    use ctr::Ctr128BE;
    use publichat::buffers::{Hash, Cypher};

    pub fn apply(key: &Hash::Buf, buf: &mut Cypher::Buf) {
        // applies AES in-place on buf as side-effect
        const IV: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];

        // TODO: key is always the same; generate decrypter once in main?
        let mut cypher = Ctr128BE::<Aes256>::new(key.into(), &IV.into());
        cypher.apply_keystream(buf);
    }
}

pub mod ed25519 {
    pub use ed25519_dalek::Keypair;  // allow use outside
    use ed25519_dalek::{
        SecretKey,
        PublicKey,
        Signature,
        SIGNATURE_LENGTH,
        Signer,
        Verifier,
    };
    use publichat::buffers::{Hash, Cypher};

    pub type SigBuffer = [u8; SIGNATURE_LENGTH];

    pub fn make_keypair(input: &[u8]) -> Result<Keypair, &'static str> {
        // hash input data to get a neat 32 bytes
        let hash = super::sha::hash(input);

        let secret = SecretKey::from_bytes(&hash)
            .map_err(|_| "Failed to make private key")?;
        let public = PublicKey::from(&secret);

        Ok(Keypair{secret, public})
    }

    pub fn sign(cypher: &Cypher::Buf, keypair: &Keypair) -> SigBuffer {
        let hash = super::sha::hash(cypher);
        keypair.sign(&hash).to_bytes()
    }

    pub fn verify(
        cypher_hash: &Hash::Buf,
        pub_key: &Hash::Buf,
        signature: &SigBuffer,
    ) -> Result<bool, &'static str> {
        let pub_key = PublicKey::from_bytes(pub_key)
            .map_err(|_| "Failed to make pub key")?;

        let signature = Signature::from_bytes(signature)
            .map_err(|_| "Failed to make signature")?;

        Ok(pub_key.verify(cypher_hash, &signature).is_ok())
    }
}
