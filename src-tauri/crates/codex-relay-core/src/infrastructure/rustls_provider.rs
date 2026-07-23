use rustls::crypto::CryptoProvider;

pub(crate) fn ensure_ring_crypto_provider() -> Result<(), &'static str> {
    if CryptoProvider::get_default().is_some() {
        return Ok(());
    }

    match rustls::crypto::ring::default_provider().install_default() {
        Ok(()) => Ok(()),
        Err(_) if CryptoProvider::get_default().is_some() => Ok(()),
        Err(_) => Err("failed to install the rustls ring crypto provider"),
    }
}
