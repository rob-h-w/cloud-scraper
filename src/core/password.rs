use hex;
use hmac::Hmac;
use pbkdf2::pbkdf2;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

// Number of rounds to hash the password. Use a lower number for tests.
#[cfg(test)]
static ROUNDS: u32 = 2;
#[cfg(not(test))]
static ROUNDS: u32 = 600_000;

#[derive(Debug, Deserialize, Serialize)]
pub struct Password {
    hash: String,
    salt: String,
}

impl Password {
    pub fn new(password: String, salt_length: u16) -> Self {
        let salt = make_salt(salt_length);
        let hash = hash_password(&password, &salt);
        Self { hash, salt }
    }

    pub fn verify(&self, password: &str) -> bool {
        self.hash == hash_password(password, &self.salt)
    }
}

fn hash_password(password: &str, salt: &str) -> String {
    let mut hash = [0u8; 20];
    pbkdf2::<Hmac<Sha256>>(password.as_bytes(), salt.as_bytes(), ROUNDS, &mut hash)
        .expect("Hashing failed");
    hex::encode(hash)
}

fn make_salt(length: u16) -> String {
    let rng = thread_rng();
    rng.sample_iter(&Alphanumeric)
        .take(length as usize)
        .map(char::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password() {
        let password = "password".to_string();
        let salt_length = 16;
        let password = Password::new(password, salt_length);
        assert!(password.verify("password"));
        assert!(!password.verify("wrong_password"));
    }
}
