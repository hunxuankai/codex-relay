pub mod error;
pub mod infrastructure;
pub mod models;
pub mod services;

pub fn initialize_tls_provider() -> Result<(), &'static str> {
    infrastructure::rustls_provider::ensure_ring_crypto_provider()
}
