/// Installerer aws-lc-rs som rustls-provider for hele prosessen.
///
/// Avhengighetene aktiverer både `ring` og `aws-lc-rs`, så rustls kan ikke
/// velge selv og får panikk ved første TLS-tilkobling. Kall denne først i `main`.
pub fn install_crypto_provider() {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("rustls CryptoProvider er allerede installert");
}

#[cfg(test)]
mod tests {
    #[test]
    fn installerer_aws_lc_rs() {
        super::install_crypto_provider();
        assert!(rustls::crypto::CryptoProvider::get_default().is_some());
    }
}
