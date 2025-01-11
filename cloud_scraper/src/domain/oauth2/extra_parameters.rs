use derive_getters::Getters;
use oauth2::{AuthorizationRequest, RefreshTokenRequest};

#[macro_export]
macro_rules! extra_parameters {
    ($($key: expr => $value: expr),* $(,)?) => {
        {
            use crate::domain::oauth2::extra_parameters::ExtraParameter;
            vec![
            $(
                ExtraParameter::new($key.to_string(), $value.to_string()),
            )*
            ]
        }
    };
}

pub use extra_parameters;

#[derive(Clone, Debug, Getters)]
pub struct ExtraParameter {
    key: String,
    value: String,
}

impl ExtraParameter {
    pub fn new(key: String, value: String) -> Self {
        assert_allowed_key(&key);
        Self { key, value }
    }
}

pub type ExtraParameters = Vec<ExtraParameter>;

macro_rules! apply_extra_parameters {
    ($self: expr, $extra_parameters: expr) => {{
        let mut it = $self;
        for extra_parameter in $extra_parameters.iter() {
            it = it.add_extra_param(
                extra_parameter.key().clone(),
                extra_parameter.value().clone(),
            );
        }

        it
    }};
}

pub(crate) trait WithExtraParametersExt<'a, 'b> {
    fn with_extra_parameters(self, extra_parameters: &'b ExtraParameters) -> Self;
}

impl<'a, 'b> WithExtraParametersExt<'a, 'b> for AuthorizationRequest<'a> {
    fn with_extra_parameters(
        self,
        extra_parameters: &'b ExtraParameters,
    ) -> AuthorizationRequest<'a> {
        apply_extra_parameters!(self, extra_parameters)
    }
}

impl<'a, 'b, TE, TR, TT> WithExtraParametersExt<'a, 'b> for RefreshTokenRequest<'a, TE, TR, TT>
where
    TE: oauth2::ErrorResponse + 'static,
    TR: oauth2::TokenResponse<TT>,
    TT: oauth2::TokenType,
{
    fn with_extra_parameters(
        self,
        extra_parameters: &'b ExtraParameters,
    ) -> RefreshTokenRequest<'a, TE, TR, TT> {
        apply_extra_parameters!(self, extra_parameters)
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
mod tests {
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

    mod with_extra_parameters {
        use super::*;
        use crate::domain::oauth2::extra_parameters::ExtraParameter;
        use std::borrow::Cow;

        struct Call {
            key: String,
            value: String,
        }

        struct TestSubject<'a> {
            calls: Vec<Call>,
            _phantom: std::marker::PhantomData<&'a ()>,
        }

        impl<'a> TestSubject<'a> {
            pub fn new() -> Self {
                Self {
                    calls: Vec::new(),
                    _phantom: std::marker::PhantomData,
                }
            }

            pub fn add_extra_param<N, V>(mut self, name: N, value: V) -> Self
            where
                N: Into<Cow<'a, str>>,
                V: Into<Cow<'a, str>>,
            {
                self.calls.push(Call {
                    key: name.into().to_string(),
                    value: value.into().to_string(),
                });
                self
            }
        }

        impl<'a, 'b> WithExtraParametersExt<'a, 'b> for TestSubject<'a> {
            fn with_extra_parameters(self, extra_parameters: &'b ExtraParameters) -> Self {
                apply_extra_parameters!(self, extra_parameters)
            }
        }

        #[test]
        fn to_authorization_request() {
            let extra_parameters = vec![
                ExtraParameter::new("key1".to_string(), "value1".to_string()),
                ExtraParameter::new("key2".to_string(), "value2".to_string()),
            ];

            let mut test_subject = TestSubject::new();
            test_subject = test_subject.with_extra_parameters(&extra_parameters);

            assert_eq!(test_subject.calls.len(), 2);
            assert_eq!(test_subject.calls[0].key, "key1");
            assert_eq!(test_subject.calls[0].value, "value1");
            assert_eq!(test_subject.calls[1].key, "key2");
            assert_eq!(test_subject.calls[1].value, "value2");
        }
    }
}
