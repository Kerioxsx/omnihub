//! Vault file format and cryptography.
//!
//! ```text
//! "OHVAULT1" | flags (u8) | body
//! flags bit 0: body is wrapped with Windows DPAPI (bound to this Windows user)
//!
//! body:
//!   argon2id m_cost (u32 KiB) | t_cost (u32) | p_cost (u32) | salt (16)
//!   key nonce (12) | wrapped vault key (32 + 16 tag)     AES-256-GCM(KEK)
//!   data nonce (12) | data length (u32) | data (+16 tag)  AES-256-GCM(vault key)
//! ```
//!
//! KEK = Argon2id(master password, salt). The random vault key encrypts the
//! entries, so changing the master password only re-wraps 32 bytes. Both
//! ciphertexts authenticate the fixed header as associated data. With DPAPI
//! on, a copied vault file cannot even be attacked offline without also
//! having the Windows user's credentials.

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use rand::RngCore;
use zeroize::Zeroizing;

pub const MAGIC: &[u8; 8] = b"OHVAULT1";
pub const FLAG_DPAPI: u8 = 0x01;

const AAD_KEY: &[u8] = b"omnihub-vault-key-v1";
const AAD_DATA: &[u8] = b"omnihub-vault-data-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KdfParams {
    pub m_cost: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}

impl Default for KdfParams {
    /// 64 MiB, 3 passes, 4 lanes: about half a second on a typical desktop.
    fn default() -> Self {
        KdfParams { m_cost: 64 * 1024, t_cost: 3, p_cost: 4 }
    }
}

impl KdfParams {
    /// Fast parameters for tests only.
    #[cfg(test)]
    pub fn testing() -> Self {
        KdfParams { m_cost: 256, t_cost: 1, p_cost: 1 }
    }

    fn valid(&self) -> bool {
        (8..=4 * 1024 * 1024).contains(&self.m_cost) && (1..=64).contains(&self.t_cost) && (1..=64).contains(&self.p_cost)
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CryptoError {
    #[error("wrong master password")]
    WrongPassword,
    #[error("the vault file is damaged or not a vault")]
    Corrupt,
    #[error("the vault was created for another Windows user or PC")]
    Dpapi,
    #[error("key derivation failed")]
    Kdf,
}

pub fn random_bytes<const N: usize>() -> [u8; N] {
    let mut b = [0u8; N];
    rand::rngs::OsRng.fill_bytes(&mut b);
    b
}

pub fn derive_kek(password: &str, salt: &[u8; 16], p: KdfParams) -> Result<Zeroizing<[u8; 32]>, CryptoError> {
    let params = argon2::Params::new(p.m_cost, p.t_cost, p.p_cost, Some(32)).map_err(|_| CryptoError::Kdf)?;
    let argon = argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    let mut out = Zeroizing::new([0u8; 32]);
    argon.hash_password_into(password.as_bytes(), salt, out.as_mut()).map_err(|_| CryptoError::Kdf)?;
    Ok(out)
}

fn seal(key: &[u8; 32], nonce: &[u8; 12], plain: &[u8], aad: &[u8]) -> Vec<u8> {
    let cipher = Aes256Gcm::new_from_slice(key).expect("32-byte key");
    cipher.encrypt(Nonce::from_slice(nonce), Payload { msg: plain, aad }).expect("AES-GCM encryption cannot fail")
}

fn open(key: &[u8; 32], nonce: &[u8; 12], ct: &[u8], aad: &[u8]) -> Option<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key).ok()?;
    cipher.decrypt(Nonce::from_slice(nonce), Payload { msg: ct, aad }).ok()
}

/// The unencrypted pieces of a vault body plus its keys.
pub struct VaultKeys {
    pub kdf: KdfParams,
    pub salt: [u8; 16],
    pub key: Zeroizing<[u8; 32]>,
    pub key_nonce: [u8; 12],
    pub wrapped_key: Vec<u8>,
}

impl VaultKeys {
    /// New random vault key wrapped with a KEK from `password`.
    pub fn create(password: &str, kdf: KdfParams) -> Result<Self, CryptoError> {
        let salt = random_bytes::<16>();
        let key = Zeroizing::new(random_bytes::<32>());
        let kek = derive_kek(password, &salt, kdf)?;
        let key_nonce = random_bytes::<12>();
        let wrapped_key = seal(&kek, &key_nonce, key.as_ref(), AAD_KEY);
        Ok(VaultKeys { kdf, salt, key, key_nonce, wrapped_key })
    }

    /// Wrap the same vault key under a new password (new salt).
    pub fn rewrap(&self, new_password: &str) -> Result<Self, CryptoError> {
        let salt = random_bytes::<16>();
        let kek = derive_kek(new_password, &salt, self.kdf)?;
        let key_nonce = random_bytes::<12>();
        let wrapped_key = seal(&kek, &key_nonce, self.key.as_ref(), AAD_KEY);
        Ok(VaultKeys { kdf: self.kdf, salt, key: self.key.clone(), key_nonce, wrapped_key })
    }
}

/// Serialise the body (everything after magic + flags).
pub fn encode_body(keys: &VaultKeys, data: &[u8]) -> Vec<u8> {
    let data_nonce = random_bytes::<12>();
    let ct = seal(&keys.key, &data_nonce, data, AAD_DATA);
    let mut out = Vec::with_capacity(96 + ct.len());
    out.extend_from_slice(&keys.kdf.m_cost.to_le_bytes());
    out.extend_from_slice(&keys.kdf.t_cost.to_le_bytes());
    out.extend_from_slice(&keys.kdf.p_cost.to_le_bytes());
    out.extend_from_slice(&keys.salt);
    out.extend_from_slice(&keys.key_nonce);
    out.extend_from_slice(&keys.wrapped_key);
    out.extend_from_slice(&data_nonce);
    out.extend_from_slice(&(ct.len() as u32).to_le_bytes());
    out.extend_from_slice(&ct);
    out
}

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], CryptoError> {
        if self.0.len() < n {
            return Err(CryptoError::Corrupt);
        }
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        Ok(a)
    }

    fn u32(&mut self) -> Result<u32, CryptoError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
}

/// Decrypt a body with the master password.
pub fn decode_body(body: &[u8], password: &str) -> Result<(VaultKeys, Zeroizing<Vec<u8>>), CryptoError> {
    let mut r = Reader(body);
    let kdf = KdfParams { m_cost: r.u32()?, t_cost: r.u32()?, p_cost: r.u32()? };
    if !kdf.valid() {
        return Err(CryptoError::Corrupt);
    }
    let salt: [u8; 16] = r.take(16)?.try_into().unwrap();
    let key_nonce: [u8; 12] = r.take(12)?.try_into().unwrap();
    let wrapped_key = r.take(48)?.to_vec();
    let kek = derive_kek(password, &salt, kdf)?;
    let key_bytes = Zeroizing::new(open(&kek, &key_nonce, &wrapped_key, AAD_KEY).ok_or(CryptoError::WrongPassword)?);
    let key: [u8; 32] = key_bytes.as_slice().try_into().map_err(|_| CryptoError::Corrupt)?;
    let keys = VaultKeys { kdf, salt, key: Zeroizing::new(key), key_nonce, wrapped_key };
    let data = decode_data(&mut r, &keys.key)?;
    Ok((keys, data))
}

/// Decrypt a body with an already known vault key (Windows Hello unlock).
pub fn decode_body_with_key(body: &[u8], key: &[u8; 32]) -> Result<(VaultKeys, Zeroizing<Vec<u8>>), CryptoError> {
    let mut r = Reader(body);
    let kdf = KdfParams { m_cost: r.u32()?, t_cost: r.u32()?, p_cost: r.u32()? };
    let salt: [u8; 16] = r.take(16)?.try_into().unwrap();
    let key_nonce: [u8; 12] = r.take(12)?.try_into().unwrap();
    let wrapped_key = r.take(48)?.to_vec();
    let keys = VaultKeys { kdf, salt, key: Zeroizing::new(*key), key_nonce, wrapped_key };
    let data = decode_data(&mut r, &keys.key)?;
    Ok((keys, data))
}

fn decode_data(r: &mut Reader, key: &[u8; 32]) -> Result<Zeroizing<Vec<u8>>, CryptoError> {
    let data_nonce: [u8; 12] = r.take(12)?.try_into().unwrap();
    let len = r.u32()? as usize;
    let ct = r.take(len)?;
    Ok(Zeroizing::new(open(key, &data_nonce, ct, AAD_DATA).ok_or(CryptoError::Corrupt)?))
}

/// Encrypt arbitrary bytes with a 32-byte key (used to wrap the vault key
/// for Windows Hello).
pub fn seal_with(key: &[u8; 32], plain: &[u8], aad: &[u8]) -> Vec<u8> {
    let nonce = random_bytes::<12>();
    let mut out = nonce.to_vec();
    out.extend(seal(key, &nonce, plain, aad));
    out
}

pub fn open_with(key: &[u8; 32], sealed: &[u8], aad: &[u8]) -> Option<Zeroizing<Vec<u8>>> {
    if sealed.len() < 12 {
        return None;
    }
    let (nonce, ct) = sealed.split_at(12);
    open(key, nonce.try_into().ok()?, ct, aad).map(Zeroizing::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_wrong_password() {
        let keys = VaultKeys::create("correct horse battery staple", KdfParams::testing()).unwrap();
        let body = encode_body(&keys, b"{\"entries\":[]}");
        let (k2, data) = decode_body(&body, "correct horse battery staple").unwrap();
        assert_eq!(&data[..], b"{\"entries\":[]}");
        assert_eq!(k2.key.as_ref(), keys.key.as_ref());
        assert_eq!(decode_body(&body, "wrong").err(), Some(CryptoError::WrongPassword));
    }

    #[test]
    fn tampering_is_detected() {
        let keys = VaultKeys::create("pw-pw-pw-pw", KdfParams::testing()).unwrap();
        let mut body = encode_body(&keys, b"secret data");
        let last = body.len() - 1;
        body[last] ^= 1;
        assert_eq!(decode_body(&body, "pw-pw-pw-pw").err(), Some(CryptoError::Corrupt));
        assert_eq!(decode_body(&body[..20], "pw-pw-pw-pw").err(), Some(CryptoError::Corrupt));
    }

    #[test]
    fn rewrap_keeps_data_key() {
        let keys = VaultKeys::create("old password", KdfParams::testing()).unwrap();
        let new = keys.rewrap("new password").unwrap();
        let body = encode_body(&new, b"x");
        assert!(decode_body(&body, "old password").is_err());
        let (k, _) = decode_body(&body, "new password").unwrap();
        assert_eq!(k.key.as_ref(), keys.key.as_ref());
        let (_, d) = decode_body_with_key(&body, &keys.key).unwrap();
        assert_eq!(&d[..], b"x");
    }

    #[test]
    fn sealing_helpers() {
        let key = random_bytes::<32>();
        let s = seal_with(&key, b"abc", b"aad");
        assert_eq!(&open_with(&key, &s, b"aad").unwrap()[..], b"abc");
        assert!(open_with(&key, &s, b"other").is_none());
    }
}
