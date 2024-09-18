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

pub(crate) enum TokenStatus {
    Ok(Token),
    Expired(RefreshToken),
    Absent,
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
