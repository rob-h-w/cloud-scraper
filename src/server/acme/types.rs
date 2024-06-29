use acme2::openssl::pkey::{PKey, Private};
use acme2::openssl::x509::X509;

pub struct CertAndPrivateKey {
    pub(crate) cert: X509,
    pub(crate) private_key: PKey<Private>,
}
