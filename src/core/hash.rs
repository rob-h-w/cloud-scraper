use hmac::Hmac;
use pbkdf2::pbkdf2;
use sha2::Sha256;

// Number of rounds to hash the password. Use a lower number for tests.
#[cfg(test)]
static ROUNDS: u32 = 2;
#[cfg(not(test))]
static ROUNDS: u32 = 600_000;

pub fn hash_sha256(message: &str, salt: &str) -> [u8; 20] {
    let mut hash = [0u8; 20];
    pbkdf2::<Hmac<Sha256>>(message.as_bytes(), salt.as_bytes(), ROUNDS, &mut hash)
        .expect("Hashing failed");
    hash
}
