//! HTTP/3 support for poem.
//!
//! This module provides HTTP/3 server functionality using Quinn (QUIC) and the h3 crate.
//!
//! # Overview
//!
//! HTTP/3 runs over QUIC instead of TCP, which provides improved performance through:
//! - Multiplexed streams without head-of-line blocking
//! - Faster connection establishment (0-RTT)
//! - Connection migration
//! - Built-in encryption (TLS 1.3)
//!
//! # Example
//!
//! ```no_run
//! use poem::{
//!     Route, get, handler,
//!     h3::Http3Server,
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
//! Http3Server::new(QuinnListener::bind("0.0.0.0:443", config)?)
//!     .run(app)
//!     .await?;
//! # Ok(())
//! # }
//! ```

mod server;

pub use server::Http3Server;

use std::io::Error as IoError;

use bytes::{Buf, Bytes, BytesMut};



/// Read the complete body from an H3 request stream.
///
/// This is a helper function used by the HTTP/3 server to read request bodies.
pub(crate) async fn read_h3_body<S>(stream: &mut h3::server::RequestStream<S, Bytes>) -> Result<Bytes, IoError>
where
    S: h3::quic::BidiStream<Bytes> + Send,
{
    let mut body_bytes = BytesMut::new();

    loop {
        match stream.recv_data().await {
            Ok(Some(mut chunk)) => {
                body_bytes.extend_from_slice(&chunk.copy_to_bytes(chunk.remaining()));
            }
            Ok(None) => break,
            Err(e) => {
                return Err(IoError::other(format!("failed to read H3 body: {e}")));
            }
        }
    }

    Ok(body_bytes.freeze())
}

/// Send the response body over an H3 stream.
///
/// This streams the body chunk by chunk for proper streaming support.
pub(crate) async fn send_h3_body<S>(
    stream: &mut h3::server::RequestStream<S, Bytes>,
    body: crate::Body,
) -> Result<(), IoError>
where
    S: h3::quic::BidiStream<Bytes> + Send,
{
    let mut body_stream = std::pin::pin!(body.into_bytes_stream());

    while let Some(chunk_result) = futures_util::StreamExt::next(&mut body_stream).await {
        match chunk_result {
            Ok(chunk) => {
                if !chunk.is_empty() {
                    stream
                        .send_data(chunk)
                        .await
                        .map_err(|e| IoError::other(format!("failed to send H3 body chunk: {e}")))?;
                }
            }
            Err(e) => {
                return Err(IoError::other(format!("error reading body: {e}")));
            }
        }
    }

    Ok(())
}

/// Check if an H3 connection error represents a graceful close.
///
/// HTTP/3 and QUIC have multiple ways to signal graceful connection closure.
/// This function identifies them to avoid logging benign closes as errors.
pub fn is_graceful_h3_close(err: &h3::error::ConnectionError) -> bool {
    let err_debug = format!("{:?}", err);

    if err_debug.contains("NO_ERROR")
        || err_debug.contains("ApplicationClose: 0x0")
        || err_debug.contains("ApplicationClose(0x0)")
        || err_debug.contains("ConnectionClosed")
    {
        return true;
    }

    // Walk error source chain for typed QUIC-level causes
    let mut cur: &(dyn std::error::Error + 'static) = err;
    while let Some(src) = std::error::Error::source(cur) {
        let src_debug = format!("{:?}", src);
        if src_debug.contains("NO_ERROR") || src_debug.contains("ApplicationClose") {
            return true;
        }
        cur = src;
    }

    false
}
