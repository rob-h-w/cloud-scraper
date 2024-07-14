use chrono::{DateTime, TimeDelta, Utc};
use once_cell::sync::Lazy;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::Add;
use std::sync::Mutex;
use std::time::Duration;
use warp::reject::Reject;
use warp::Filter;

const KEY_BYTES: usize = 16;
const MAX_TOKEN_AGE_SECONDS: u64 = 24 * 60 * 60;

static TOKEN_MANAGER: Lazy<Mutex<TokenManager>> = Lazy::new(|| Mutex::new(TokenManager::new()));

pub fn gen_token_for_path(path: &str) -> Token {
    TOKEN_MANAGER
        .lock()
        .expect("Token manager mutex poisoned.")
        .put_token(Token::new(
            path,
            &Duration::from_secs(MAX_TOKEN_AGE_SECONDS),
        ))
}

pub fn token_is_valid(token: &str) -> bool {
    TOKEN_MANAGER
        .lock()
        .expect("Token manager mutex poisoned.")
        .token_is_valid(token)
}

#[derive(Debug)]
pub struct Unauthorized;

impl Unauthorized {
    pub fn rejection() -> warp::Rejection {
        warp::reject::custom(Unauthorized)
    }
}

impl Reject for Unauthorized {}

pub fn auth_validation() -> impl Filter<Extract = (), Error = warp::Rejection> + Copy {
    warp::cookie::<String>("token")
        .and_then(|cookie: String| async move {
            if token_is_valid(&cookie) {
                Ok(())
            } else {
                Err(Unauthorized::rejection())
            }
        })
        .untuple_one()
}

#[derive(Clone, Debug)]
pub struct Token {
    max_age: DateTime<Utc>,
    path: String,
    value: String,
}

impl Token {
    pub fn new(path: &str, lifespan: &Duration) -> Self {
        Token {
            max_age: Utc::now().add(TimeDelta::from_std(*lifespan).expect("Invalid duration.")),
            path: path.to_string(),
            value: thread_rng()
                .sample_iter(&Alphanumeric)
                .take(KEY_BYTES)
                .map(char::from)
                .collect(),
        }
    }

    pub fn to_cookie_string(&self) -> String {
        format!(
            "token={}; Path={}; HttpOnly; Max-Age={}; Secure",
            self.value,
            self.path,
            self.max_age.signed_duration_since(Utc::now()).num_seconds()
        )
    }
}

struct TokenManager {
    tokens_by_value: HashMap<String, Token>,
}

impl TokenManager {
    fn new() -> Self {
        TokenManager {
            tokens_by_value: HashMap::new(),
        }
    }

    fn put_token(&mut self, token: Token) -> Token {
        self.tokens_by_value
            .insert(token.value.clone(), token.clone());
        token
    }

    fn token_is_valid(&self, token: &str) -> bool {
        match self.tokens_by_value.get(token) {
            Some(stored_token) => Utc::now() < stored_token.max_age,
            None => false,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    mod token_manager {
        use super::*;
        use std::time;

        #[test]
        fn test_token_manager() {
            let mut token_manager = TokenManager::new();
            let token = token_manager.put_token(Token::new("/", &Duration::from_secs(1)));
            assert!(token_manager.token_is_valid(&token.value));
        }

        #[test]
        fn test_token_manager_expired() {
            let mut token_manager = TokenManager::new();
            let token = token_manager.put_token(Token::new("/", &Duration::from_millis(1)));
            std::thread::sleep(time::Duration::from_millis(2));
            assert!(!token_manager.token_is_valid(&token.value));
        }
    }
}
