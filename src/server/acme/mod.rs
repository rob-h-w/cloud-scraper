mod challenge_token_server;
mod types;

use crate::domain::config::Config;
use crate::server::acme::types::CertAndPrivateKey;
use crate::server::site_state::SiteState;
use acme2::{
    gen_ec_p256_private_key, AccountBuilder, AuthorizationStatus, ChallengeStatus, Csr,
    DirectoryBuilder, Error, OrderBuilder, OrderStatus,
};
use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use tokio::{fs, join};
use x509_parser::parse_x509_certificate;
use x509_parser::pem::parse_x509_pem;

const LETS_ENCRYPT_URL: &str = "https://acme-v02.api.letsencrypt.org/directory";

pub struct Acme {
    config: Arc<Config>,
    site_state: SiteState,
}

impl Acme {
    pub fn new(config: &Arc<Config>) -> Self {
        Self {
            config: config.clone(),
            site_state: SiteState::new(config.as_ref()),
        }
    }

    async fn cert_is_valid(&self) -> bool {
        let path = self.site_state.cert_path();
        fs::metadata(path).await.is_ok() && cert_is_not_expired(path).await
    }

    pub(crate) fn cert_path(&self) -> &str {
        self.site_state.cert_path()
    }

    pub(crate) async fn ensure_certs(&self) -> Result<(), String> {
        if self.cert_is_valid().await {
            return Ok(());
        }

        let cert_and_private_key = self
            .get_cert()
            .await
            .map_err(|e| format!("Failed to get certificate: {}", e))?;

        self.write_key_and_cert_to_files(cert_and_private_key)
            .await
            .map_err(|e| format!("Failed to write key and certificate to files: {}", e))?;

        Ok(())
    }

    async fn get_cert(&self) -> Result<CertAndPrivateKey, Error> {
        let domain_config = self.config.domain_config();

        // Create a new ACMEv2 directory for Let's Encrypt.
        let dir = DirectoryBuilder::new(LETS_ENCRYPT_URL.to_string())
            .build()
            .await?;

        // Create an ACME account to use for the order. For production
        // purposes, you should keep the account (and private key), so
        // you can renew your certificate easily.
        let mut builder = AccountBuilder::new(dir.clone());
        builder.contact(
            domain_config
                .builder_contacts()
                .iter()
                .map(|it| format!("mailto:{}", it).to_string())
                .collect(),
        );
        builder.terms_of_service_agreed(true);
        let account = builder.build().await?;
        log::debug!("Account builder finished");

        // Create a new order for a specific domain name.
        let mut builder = OrderBuilder::new(account);
        let domain = domain_config
            .url_in_use()
            .domain()
            .unwrap_or_else(|| panic!("Could not get domain from {}", domain_config.url_in_use()))
            .to_string();
        builder.add_dns_identifier(domain.to_string());
        let order = builder.build().await?;
        log::debug!("Order builder finished");

        // Get the list of needed authorizations for this order.
        let authorizations = order.authorizations().await?;
        log::debug!("Authorizations retrieved");
        for auth in authorizations {
            log::debug!("Authorization: {:?}", auth);
            // Get an http-01 challenge for this authorization (or panic
            // if it doesn't exist).
            let challenge = auth
                .get_challenge("http-01")
                .expect("Could not get ACME challenge for http-01");

            // At this point in time, you must configure your webserver to serve
            // a file at `http://example.com/.well-known/${challenge.token}` or
            // `https://example.com/.well-known/${challenge.token}`
            // with the content of `challenge.key_authorization()??`.
            let key_authorization = challenge.key_authorization()?;
            let challenge_token = challenge.token.clone();

            if key_authorization.is_none() || challenge_token.is_none() {
                log::error!("Error getting ACME challenge key authorization or token");
                break;
            }

            let challenge_token_server = challenge_token_server::ChallengeTokenServer::new(
                key_authorization.expect("Could not get ACME key authorization."),
                domain.to_string(),
                challenge_token.expect("Could not get ACME challenge token."),
            );
            log::debug!("Challenge token server created");

            let challenge_token_server_wait_handle = challenge_token_server.serve();

            // Start the validation of the challenge.
            let challenge = challenge.validate().await?;
            let challenge_until_result = async {
                // challenge every interval seconds until it is in either the
                // `valid` or `invalid` state.
                let challenge = challenge
                    .wait_done(
                        Duration::from_secs(domain_config.poll_interval_seconds()),
                        domain_config.poll_attempts(),
                    )
                    .await?;
                log::debug!(
                    "Stopped waiting for challenge completion. Challenge is: {:?}",
                    challenge
                );
                assert_eq!(challenge.status, ChallengeStatus::Valid);

                // Stop the challenge token server.
                challenge_token_server.stop();
                Ok::<(), Error>(())
            };

            let (_challenge_result, _challenge_token_server_result) =
                join!(challenge_until_result, challenge_token_server_wait_handle,);

            // Poll the authorization every interval seconds until it is in either the
            // `valid` or `invalid` state.
            let authorization = auth
                .wait_done(
                    Duration::from_secs(self.config.domain_config().poll_interval_seconds()),
                    self.config.domain_config().poll_attempts(),
                )
                .await?;
            assert_eq!(authorization.status, AuthorizationStatus::Valid)
        }

        // Poll the order every interval seconds until it is in either the
        // `ready` or `invalid` state. Ready means that it is now ready
        // for finalization (certificate creation).
        let order = order
            .wait_ready(
                Duration::from_secs(self.config.domain_config().poll_interval_seconds()),
                self.config.domain_config().poll_attempts(),
            )
            .await?;
        log::debug!("Stopped waiting for order ready");

        assert_eq!(order.status, OrderStatus::Ready);

        // Generate an elliptic curve private key for the certificate.
        let pkey = gen_ec_p256_private_key()?;
        log::debug!("Private key generated");

        // Create a certificate signing request for the order, and request
        // the certificate.
        let order = order.finalize(Csr::Automatic(pkey.clone())).await?;
        log::debug!("Signing Request created");

        // Poll the order every interval seconds until it is in either the
        // `valid` or `invalid` state. Valid means that the certificate
        // has been provisioned, and is now ready for download.
        let order = order
            .wait_done(
                Duration::from_secs(self.config.domain_config().poll_interval_seconds()),
                self.config.domain_config().poll_attempts(),
            )
            .await?;
        log::debug!("Stopped waiting for order completion");

        assert_eq!(order.status, OrderStatus::Valid);

        // Download the certificate, and panic if it doesn't exist.
        let cert = order
            .certificate()
            .await?
            .expect("Could not get X509 certificate.");
        log::debug!("Certificate downloaded");
        assert!(cert.len() > 1);

        if cert.len() > 1 {
            log::warn!("Certificate has more than one element. Using the first one.");
        }

        Ok(CertAndPrivateKey {
            cert: cert[0].clone(),
            private_key: pkey,
        })
    }

    pub(crate) fn key_path(&self) -> &str {
        self.site_state.key_path()
    }

    async fn write_key_and_cert_to_files(
        &self,
        cert_and_private_key: CertAndPrivateKey,
    ) -> Result<(), std::io::Error> {
        fs::create_dir_all(self.site_state.site_folder()).await?;

        let key = cert_and_private_key
            .private_key
            .private_key_to_pem_pkcs8()?;
        let cert = cert_and_private_key.cert.to_pem()?;

        // Write the key to the key file
        fs::write(self.key_path(), key).await?;

        // Write the certificate to the cert file
        fs::write(self.cert_path(), cert).await?;

        Ok(())
    }
}

async fn cert_is_not_expired(path: &str) -> bool {
    // Get the expiry timestamp of the certificate
    let validity = match get_cert_validity(path).await {
        Ok(t) => t,
        Err(e) => {
            log::error!("Failed to get certificate expiry timestamp: {}", e);
            return false;
        }
    };

    // Get the current time
    let now = Utc::now();

    // Check if the certificate has expired
    validity.is_valid_at(now.timestamp())
}

struct Validity {
    not_before: i64,
    not_after: i64,
}

impl Validity {
    fn is_valid_at(&self, now: i64) -> bool {
        now >= self.not_before && now <= self.not_after
    }
}

async fn get_cert_validity(path: &str) -> Result<Validity, String> {
    // Read the certificate file
    let cert_pem = fs::read(path)
        .await
        .map_err(|e| format!("Failed to read certificate file: {}", e))?;

    // Read the pem
    let (_, pem) = parse_x509_pem(&cert_pem).map_err(|e| format!("Failed to read pem: {}", e))?;

    if pem.label != "CERTIFICATE" {
        return Err(format!("Expected a certificate, got {:?}", pem.label));
    }

    // Parse the certificate
    let (_, cert) = parse_x509_certificate(&pem.contents)
        .map_err(|e| format!("Failed to parse certificate: {}", e))?;

    Ok(Validity {
        not_after: cert.tbs_certificate.validity.not_after.timestamp(),
        not_before: cert.tbs_certificate.validity.not_before.timestamp(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_cert_expiry_timestamp() {
        let cert = include_bytes!("../../../tests/fixtures/cert.pem");
        let path = "/tmp/cert.pem";
        fs::write(path, cert).await.unwrap();
        let validity = get_cert_validity(path)
            .await
            .expect("Could not get validity");

        // Saturday, June 29, 2024 19:04:36
        assert_eq!(validity.not_before, 1719687876);

        // Sunday, June 30, 2024 19:04:36
        assert_eq!(validity.not_after, 1719774276);
    }

    #[test]
    fn test_validity_is_valid_at() {
        let validity = Validity {
            not_before: 1,
            not_after: 10,
        };

        assert!(!validity.is_valid_at(0));
        assert!(validity.is_valid_at(1));
        assert!(validity.is_valid_at(5));
        assert!(validity.is_valid_at(10));
        assert!(!validity.is_valid_at(11));
    }
}
