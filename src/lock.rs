//! Senha para exibir o WhatsApp. Guarda só sal + PBKDF2-HMAC-SHA256, nunca a senha.
//! É uma trava de tela: os dados do WebView2 em `%LOCALAPPDATA%` continuam sem criptografia.
use serde::{Deserialize, Serialize};

const ITERATIONS: u32 = 200_000;
pub const MIN_LEN: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasswordLock {
    pub salt: String,
    pub hash: String,
    pub iterations: u32,
}

impl PasswordLock {
    pub fn new(password: &str) -> Self {
        Self::with_iterations(password, ITERATIONS)
    }

    fn with_iterations(password: &str, iterations: u32) -> Self {
        let salt = random_salt();
        Self {
            hash: hex(&derive(password, &salt, iterations)),
            salt: hex(&salt),
            iterations,
        }
    }

    pub fn verify(&self, password: &str) -> bool {
        let (Some(salt), Some(want)) = (unhex(&self.salt), unhex(&self.hash)) else {
            return false;
        };
        let got = derive(password, &salt, self.iterations.max(1));
        // comparação em tempo constante
        got.len() == want.len() && got.iter().zip(&want).fold(0u8, |acc, (a, b)| acc | (a ^ b)) == 0
    }
}

fn derive(password: &str, salt: &[u8], iterations: u32) -> [u8; 32] {
    pbkdf2::pbkdf2_hmac_array::<sha2::Sha256, 32>(password.as_bytes(), salt, iterations)
}

#[cfg(target_os = "windows")]
fn random_salt() -> [u8; 16] {
    #[link(name = "bcrypt")]
    extern "system" {
        fn BCryptGenRandom(alg: *mut std::ffi::c_void, buf: *mut u8, len: u32, flags: u32) -> i32;
    }
    const BCRYPT_USE_SYSTEM_PREFERRED_RNG: u32 = 2;
    let mut salt = [0u8; 16];
    let status = unsafe { BCryptGenRandom(std::ptr::null_mut(), salt.as_mut_ptr(), 16, BCRYPT_USE_SYSTEM_PREFERRED_RNG) };
    assert_eq!(status, 0, "BCryptGenRandom falhou");
    salt
}

#[cfg(not(target_os = "windows"))]
fn random_salt() -> [u8; 16] {
    use std::io::Read;
    let mut salt = [0u8; 16];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut salt))
        .expect("sem /dev/urandom");
    salt
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifies_only_the_right_password() {
        let lock = PasswordLock::with_iterations("s3nha forte", 10);
        assert!(lock.verify("s3nha forte"));
        assert!(!lock.verify("s3nha fortE"));
        assert!(!lock.verify(""));
        assert!(!lock.hash.contains("s3nha"));
    }

    #[test]
    fn each_password_gets_its_own_salt() {
        let (a, b) = (PasswordLock::with_iterations("x", 10), PasswordLock::with_iterations("x", 10));
        assert_ne!(a.salt, b.salt);
        assert_ne!(a.hash, b.hash);
    }

    #[test]
    fn broken_stored_values_never_unlock() {
        let mut lock = PasswordLock::with_iterations("abcd", 10);
        lock.hash = "zz".into();
        assert!(!lock.verify("abcd"));
        assert_eq!(unhex("0a1"), None);
        assert_eq!(unhex("0aff"), Some(vec![0x0a, 0xff]));
    }
}
