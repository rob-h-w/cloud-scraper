use derive_getters::Getters;

macro_rules! extra_parameters {
    ($($key: expr => $value: expr),* $(,)?) => {
        {
            use crate::domain::oauth2::extra_parameters::{ExtraParameter, ExtraParameters};
            let mut extra_parameters: ExtraParameters = Vec::new();
            $(
                extra_parameters.push(ExtraParameter::new($key.to_string(), $value.to_string()));
            )*
            extra_parameters
        }
    };
}

pub(crate) use extra_parameters;

pub(crate) type ExtraParameters = Vec<ExtraParameter>;

#[derive(Clone, Debug, Getters)]
pub(crate) struct ExtraParameter {
    key: String,
    value: String,
}

impl ExtraParameter {
    pub(crate) fn new(key: String, value: String) -> Self {
        assert_allowed_key(&key);
        Self { key, value }
    }
}

const DISALLOWED_KEYS: [&str; 19] = [
    // https://datatracker.ietf.org/doc/html/rfc6749
    "client_id",
    "client_secret",
    "response_type",
    "scope",
    "state",
    "redirect_uri",
    "error",
    "error_description",
    "error_uri",
    "grant_type",
    "code",
    "access_token",
    "token_type",
    "expires_in",
    "username",
    "password",
    "refresh_token",
    // https://datatracker.ietf.org/doc/html/rfc7636
    "code_verifier",
    "code_challenge",
];

fn assert_allowed_key(key: &str) {
    if DISALLOWED_KEYS.contains(&key) {
        panic!("Key '{}' is not allowed", key);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[should_panic]
    fn disallowed_keys() {
        ExtraParameter::new("client_id".to_string(), "value".to_string());
    }

    #[test]
    fn extra_parameters() {
        let parameters = extra_parameters!("key1" => "value1", "key2" => "value2");

        assert_eq!(parameters.len(), 2);
        assert_eq!(parameters[0].key(), "key1");
        assert_eq!(parameters[0].value(), "value1");
        assert_eq!(parameters[1].key(), "key2");
        assert_eq!(parameters[1].value(), "value2");
    }
}
