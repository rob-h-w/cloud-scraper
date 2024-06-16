mod challenge_token_server;
mod types;

use crate::domain::config::Config;
use crate::server::acme::types::CertAndPrivateKey;
use crate::server::site_state::SiteState;
use acme2::{
    gen_ec_p256_private_key, AccountBuilder, AuthorizationStatus, ChallengeStatus, Csr,
    DirectoryBuilder, Error, OrderBuilder, OrderStatus,
};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::{fs, join};

const LETS_ENCRYPT_URL: &str = "https://acme-v02.api.letsencrypt.org/directory";

#[async_trait]
pub trait Acme: Send + Sync {
    fn cert_path(&self) -> &str;
    async fn ensure_certs(&self) -> Result<(), String>;
    fn key_path(&self) -> &str;
}

pub struct AcmeImpl<ConfigType> {
    config: Arc<ConfigType>,
    site_state: SiteState,
}

impl<ConfigType> AcmeImpl<ConfigType>
where
    ConfigType: Config,
{
    pub fn new(config: Arc<ConfigType>) -> Self {
        Self {
            config: config.clone(),
            site_state: SiteState::new(config.as_ref()),
        }
    }

    async fn cert_is_valid(&self) -> bool {
        fs::metadata(self.site_state.cert_path()).await.is_ok()
    }

    async fn get_cert(&self) -> Result<CertAndPrivateKey, Error> {
        let domain_config = self
            .config
            .domain_config()
            .expect("Domain config is not defined");

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
                .builder_contacts
                .iter()
                .map(|it| format!("mailto:{}", it).to_string())
                .collect(),
        );
        builder.terms_of_service_agreed(true);
        let account = builder.build().await?;
        log::debug!("Account builder finished");

        // Create a new order for a specific domain name.
        let mut builder = OrderBuilder::new(account);
        builder.add_dns_identifier(domain_config.domain_name.to_string());
        let order = builder.build().await?;
        log::debug!("Order builder finished");

        // Get the list of needed authorizations for this order.
        let authorizations = order.authorizations().await?;
        log::debug!("Authorizations retrieved");
        for auth in authorizations {
            log::debug!("Authorization: {:?}", auth);
            // Get an http-01 challenge for this authorization (or panic
            // if it doesn't exist).
            let challenge = auth.get_challenge("http-01").unwrap();

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
                key_authorization.unwrap(),
                domain_config.domain_name.clone(),
                challenge_token.unwrap(),
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
                        Duration::from_secs(
                            self.config.domain_config().unwrap().poll_interval_seconds,
                        ),
                        self.config.domain_config().unwrap().poll_attempts,
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
                    Duration::from_secs(self.config.domain_config().unwrap().poll_interval_seconds),
                    self.config.domain_config().unwrap().poll_attempts,
                )
                .await?;
            assert_eq!(authorization.status, AuthorizationStatus::Valid)
        }

        // Poll the order every interval seconds until it is in either the
        // `ready` or `invalid` state. Ready means that it is now ready
        // for finalization (certificate creation).
        let order = order
            .wait_ready(
                Duration::from_secs(self.config.domain_config().unwrap().poll_interval_seconds),
                self.config.domain_config().unwrap().poll_attempts,
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
                Duration::from_secs(self.config.domain_config().unwrap().poll_interval_seconds),
                self.config.domain_config().unwrap().poll_attempts,
            )
            .await?;
        log::debug!("Stopped waiting for order completion");

        assert_eq!(order.status, OrderStatus::Valid);

        // Download the certificate, and panic if it doesn't exist.
        let cert = order.certificate().await?.unwrap();
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

#[async_trait]
impl<ConfigType> Acme for AcmeImpl<ConfigType>
where
    ConfigType: Config,
{
    fn cert_path(&self) -> &str {
        self.site_state.cert_path()
    }

    async fn ensure_certs(&self) -> Result<(), String> {
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

    fn key_path(&self) -> &str {
        self.site_state.key_path()
    }
}
