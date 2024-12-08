use chrono::{DateTime, TimeDelta, Utc};
use derive_getters::Getters;
use log::debug;
use oauth2::basic::{BasicTokenResponse, BasicTokenType};
use oauth2::{AccessToken, RefreshToken, TokenResponse};
use serde::{Deserialize, Serialize};
use std::ops::Add;

#[derive(Clone, Debug, Deserialize, Getters, Serialize)]
pub(crate) struct Token {
    access_token: AccessToken,
    token_type: BasicTokenType,
    expires_at: Option<DateTime<Utc>>,
    refresh_token: Option<RefreshToken>,
}

impl Token {
    fn from_response(response: &BasicTokenResponse) -> Self {
        debug!("Token::from_response {:?}", response);
        let max = TimeDelta::days(365);
        Token {
            access_token: response.access_token().clone(),
            token_type: response.token_type().clone(),
            expires_at: response.expires_in().map(|duration| {
                Utc::now()
                    .checked_add_signed(TimeDelta::from_std(duration).unwrap_or(max))
                    .unwrap_or(Utc::now().add(max))
            }),
            refresh_token: response.refresh_token().cloned(),
        }
    }

    pub(crate) fn get_status(&self) -> TokenStatus {
        if let Some(expires_at) = self.expires_at {
            if expires_at < Utc::now() {
                if let Some(refresh_token) = &self.refresh_token {
                    TokenStatus::Expired(refresh_token.clone())
                } else {
                    TokenStatus::Absent
                }
            } else {
                TokenStatus::Ok(self.clone())
            }
        } else {
            TokenStatus::Absent
        }
    }
}

#[derive(Debug)]
pub(crate) enum TokenStatus {
    Ok(Token),
    Expired(RefreshToken),
    Absent,
}

impl TokenStatus {
    pub(crate) fn with_refresh_token(self, refresh_token: &RefreshToken) -> Self {
        match self {
            TokenStatus::Ok(mut token) => {
                token.refresh_token = Some(refresh_token.clone());
                TokenStatus::Ok(token)
            }
            _ => self,
        }
    }
}

pub(crate) trait TokenExt {
    fn get_status(&self) -> TokenStatus;
}

impl TokenExt for Option<Token> {
    fn get_status(&self) -> TokenStatus {
        match self {
            Some(token) => token.get_status(),
            None => TokenStatus::Absent,
        }
    }
}

pub(crate) trait BasicTokenResponseExt {
    fn to_token_status(&self) -> TokenStatus;
}

impl BasicTokenResponseExt for Option<BasicTokenResponse> {
    fn to_token_status(&self) -> TokenStatus {
        match self {
            Some(token) => token.to_token_status(),
            None => TokenStatus::Absent,
        }
    }
}

impl BasicTokenResponseExt for BasicTokenResponse {
    fn to_token_status(&self) -> TokenStatus {
        Token::from_response(self).get_status()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use chrono::Duration;

    mod with_refresh_token {
        use super::*;
        use lazy_static::lazy_static;

        lazy_static! {
            static ref REFRESH_TOKEN: RefreshToken = RefreshToken::new("REFRESH_TOKEN".to_string());
        }

        #[test]
        fn test_retains_the_token() {
            let token = Token {
                access_token: AccessToken::new("access_token".to_string()),
                token_type: BasicTokenType::Bearer,
                expires_at: Some(Utc::now() + Duration::days(1)),
                refresh_token: None,
            };
            let status = TokenStatus::Ok(token.clone()).with_refresh_token(&REFRESH_TOKEN);

            if let TokenStatus::Ok(refreshed_token) = status {
                assert_eq!(
                    refreshed_token.refresh_token.unwrap().secret(),
                    REFRESH_TOKEN.secret()
                );
            } else {
                panic!("Expected Ok, got {:?}", status);
            }
        }

        #[test]
        fn test_does_not_modify_other_states() {
            let token = Token {
                access_token: AccessToken::new("access_token".to_string()),
                token_type: BasicTokenType::Bearer,
                expires_at: Some(Utc::now() + Duration::days(1)),
                refresh_token: None,
            };

            assert!(matches!(
                TokenStatus::Expired(REFRESH_TOKEN.clone()).with_refresh_token(&REFRESH_TOKEN),
                TokenStatus::Expired(_)
            ));
            assert!(matches!(
                TokenStatus::Absent.with_refresh_token(&REFRESH_TOKEN),
                TokenStatus::Absent
            ));
        }
    }
}
