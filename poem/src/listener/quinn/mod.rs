//! QUIC/HTTP3 listener implementation using Quinn.
//!
//! This module provides HTTP/3 support for poem using the Quinn QUIC implementation.
//!
//! # Example
//!
//! ```no_run
//! use poem::{
//!     Route, get, handler,
//!     listener::quinn::{QuinnListener, QuinnConfig},
//! };
//!
//! #[handler]
//! fn hello() -> &'static str {
//!     "Hello HTTP/3!"
//! }
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = QuinnConfig::new()
//!     .cert(include_bytes!("cert.pem").to_vec())
//!     .key(include_bytes!("key.pem").to_vec());
//!
//! let app = Route::new().at("/", get(hello));
//!
//! // Use Http3Server instead of Server for HTTP/3
//! // Http3Server::new(QuinnListener::bind("0.0.0.0:443", config))
//! //     .run(app)
//! //     .await?;
//! # Ok(())
//! # }
//! ```

use std::{
    io::{Error as IoError, ErrorKind, Result as IoResult},
    net::{SocketAddr, ToSocketAddrs},
    sync::Arc,
};

use rustls_pemfile::Item;
use tokio_rustls::rustls::{
    self,
    crypto::{aws_lc_rs, aws_lc_rs::sign::any_supported_type, CryptoProvider},
    pki_types::{CertificateDer, PrivateKeyDer},
    ConfigBuilder, ServerConfig, WantsVerifier, DEFAULT_VERSIONS,
};

/// Configuration for QUIC/HTTP3 server.
///
/// This configuration is used to create a Quinn endpoint with the appropriate
/// TLS settings for HTTP/3.
#[derive(Clone)]
pub struct QuinnConfig {
    cert: Vec<u8>,
    key: Vec<u8>,
    alpn_protocols: Vec<Vec<u8>>,
}

impl QuinnConfig {
    /// Create a new QuinnConfig.
    pub fn new() -> Self {
        Self {
            cert: Vec::new(),
            key: Vec::new(),
            alpn_protocols: vec![b"h3".to_vec()],
        }
    }

    /// Set the certificate chain in PEM format.
    #[must_use]
    pub fn cert(mut self, cert: impl Into<Vec<u8>>) -> Self {
        self.cert = cert.into();
        self
    }

    /// Set the private key in PEM format.
    #[must_use]
    pub fn key(mut self, key: impl Into<Vec<u8>>) -> Self {
        self.key = key.into();
        self
    }

    /// Set custom ALPN protocols.
    ///
    /// Default is `["h3"]`.
    #[must_use]
    pub fn alpn_protocols(mut self, protocols: Vec<Vec<u8>>) -> Self {
        self.alpn_protocols = protocols;
        self
    }

    /// Build the rustls ServerConfig from this configuration.
    pub fn build_server_config(&self) -> IoResult<ServerConfig> {
        // Parse certificates
        let certs: Vec<CertificateDer<'static>> =
            rustls_pemfile::certs(&mut self.cert.as_slice())
                .collect::<Result<_, _>>()
                .map_err(|e| IoError::new(ErrorKind::InvalidData, e))?;

        if certs.is_empty() {
            return Err(IoError::new(
                ErrorKind::InvalidData,
                "no certificates found in PEM data",
            ));
        }

        // Parse private key
        let mut key_reader = self.key.as_slice();
        let priv_key: PrivateKeyDer<'static> = loop {
            match rustls_pemfile::read_one(&mut key_reader)
                .map_err(|e| IoError::new(ErrorKind::InvalidData, e))?
            {
                Some(Item::Pkcs1Key(key)) => break key.into(),
                Some(Item::Pkcs8Key(key)) => break key.into(),
                Some(Item::Sec1Key(key)) => break key.into(),
                None => {
                    return Err(IoError::new(
                        ErrorKind::InvalidData,
                        "no private key found in PEM data",
                    ));
                }
                _ => continue,
            }
        };

        // Verify the key is valid
        let _ = any_supported_type(&priv_key)
            .map_err(|e| IoError::new(ErrorKind::InvalidData, format!("invalid private key: {e}")))?;

        // Build server config
        let builder = make_server_config_builder();
        let mut server_config = builder
            .with_no_client_auth()
            .with_single_cert(certs, rustls::pki_types::PrivateKeyDer::try_from(priv_key).unwrap())
            .map_err(|e| IoError::new(ErrorKind::InvalidData, format!("TLS config error: {e}")))?;

        server_config.alpn_protocols = self.alpn_protocols.clone();
        server_config.max_early_data_size = u32::MAX;

        Ok(server_config)
    }

    /// Build a Quinn ServerConfig from this configuration.
    pub fn build_quinn_config(&self) -> IoResult<quinn::ServerConfig> {
        let rustls_config = self.build_server_config()?;
        let mut quinn_config =
            quinn::ServerConfig::with_crypto(Arc::new(quinn::crypto::rustls::QuicServerConfig::try_from(rustls_config)
                .map_err(|e| IoError::new(ErrorKind::InvalidData, format!("QUIC config error: {e}")))?));

        // Configure transport parameters for HTTP/3
        let mut transport_config = quinn::TransportConfig::default();
        transport_config.max_idle_timeout(Some(
            std::time::Duration::from_secs(30)
                .try_into()
                .expect("valid idle timeout"),
        ));
        quinn_config.transport_config(Arc::new(transport_config));

        Ok(quinn_config)
    }
}

impl Default for QuinnConfig {
    fn default() -> Self {
        Self::new()
    }
}

// Helper to create a rustls ServerConfig builder
fn make_server_config_builder() -> ConfigBuilder<ServerConfig, WantsVerifier> {
    if CryptoProvider::get_default().is_none() {
        let provider = aws_lc_rs::default_provider();
        let _ = provider.install_default();
    }

    let provider = CryptoProvider::get_default().expect("crypto provider must be set");

    ServerConfig::builder_with_provider(provider.clone())
        .with_protocol_versions(DEFAULT_VERSIONS)
        .expect("valid protocol versions")
}

/// A QUIC listener that accepts HTTP/3 connections.
///
/// Unlike TCP listeners, QUIC connections are multiplexed, so this listener
/// works differently from the standard poem `Listener` trait.
pub struct QuinnListener {
    bind_addr: SocketAddr,
    config: QuinnConfig,
}

impl QuinnListener {
    /// Create a new QuinnListener bound to the specified address.
    pub fn bind<A: ToSocketAddrs>(addr: A, config: QuinnConfig) -> IoResult<Self> {
        let bind_addr = addr
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| IoError::new(ErrorKind::InvalidInput, "no address provided"))?;

        Ok(Self { bind_addr, config })
    }

    /// Get the bind address.
    pub fn bind_addr(&self) -> SocketAddr {
        self.bind_addr
    }

    /// Create a Quinn endpoint from this listener configuration.
    pub fn into_endpoint(self) -> IoResult<quinn::Endpoint> {
        let quinn_config = self.config.build_quinn_config()?;
        quinn::Endpoint::server(quinn_config, self.bind_addr)
            .map_err(|e| IoError::new(ErrorKind::Other, format!("failed to create QUIC endpoint: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quinn_config_builder() {
        let config = QuinnConfig::new()
            .cert(b"cert".to_vec())
            .key(b"key".to_vec())
            .alpn_protocols(vec![b"h3".to_vec()]);

        assert_eq!(config.cert, b"cert".to_vec());
        assert_eq!(config.key, b"key".to_vec());
        assert_eq!(config.alpn_protocols, vec![b"h3".to_vec()]);
    }
}
