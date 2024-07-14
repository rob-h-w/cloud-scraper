use crate::core::hash::hash_sha256;
use hex;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Password {
    hash: String,
    salt: String,
}

impl Password {
    pub fn new(password: &str, salt_length: u16) -> Self {
        let salt = make_salt(salt_length);
        let hash = hash_password(password, &salt);
        Self { hash, salt }
    }

    pub fn verify(&self, password: &str) -> bool {
        self.hash == hash_password(password, &self.salt)
    }
}

fn hash_password(password: &str, salt: &str) -> String {
    hex::encode(hash_sha256(password, salt))
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
        let password = "password";
        let salt_length = 16;
        let password = Password::new(password, salt_length);
        assert!(password.verify("password"));
        assert!(!password.verify("wrong_password"));
    }
}
