//! HTTP/3 server implementation.

use std::{
    future::Future,
    io::{Error as IoError, Result as IoResult},
    net::SocketAddr,
    panic::AssertUnwindSafe,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use bytes::Bytes;
use futures_util::FutureExt;
use http::uri::Scheme;
use tokio::{sync::Notify, time::Duration};
use tokio_util::sync::CancellationToken;

use super::{is_graceful_h3_close, read_h3_body, send_h3_body};
use crate::{
    Addr, Body, Endpoint, EndpointExt, IntoEndpoint, Request, Response,
    endpoint::{DynEndpoint, ToDynEndpoint},
    listener::quinn::QuinnListener,
    web::{LocalAddr, RemoteAddr},
};

/// An HTTP/3 Server.
///
/// This server handles HTTP/3 connections over QUIC using the Quinn library.
/// Unlike the standard `Server`, this directly manages QUIC connections and
/// HTTP/3 streams.
///
/// # Example
///
/// ```no_run
/// use poem::{
///     Route, get, handler,
///     h3::Http3Server,
///     listener::quinn::{QuinnListener, QuinnConfig},
/// };
///
/// #[handler]
/// fn hello() -> &'static str {
///     "Hello HTTP/3!"
/// }
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = QuinnConfig::new()
///     .cert(include_bytes!("cert.pem").to_vec())
///     .key(include_bytes!("key.pem").to_vec());
///
/// let app = Route::new().at("/", get(hello));
///
/// Http3Server::new(QuinnListener::bind("0.0.0.0:443", config)?)
///     .run(app)
///     .await?;
/// # Ok(())
/// # }
/// ```
#[cfg_attr(docsrs, doc(cfg(feature = "http3")))]
pub struct Http3Server {
    listener: QuinnListener,
    name: Option<String>,
}

impl Http3Server {
    /// Create a new HTTP/3 server with the given Quinn listener.
    pub fn new(listener: QuinnListener) -> Self {
        Self {
            listener,
            name: None,
        }
    }

    /// Specify the name of the server (used for logging).
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Run this HTTP/3 server.
    pub async fn run<E>(self, ep: E) -> IoResult<()>
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.run_with_graceful_shutdown(ep, futures_util::future::pending(), None)
            .await
    }

    /// Run this HTTP/3 server with graceful shutdown support.
    pub async fn run_with_graceful_shutdown<E>(
        self,
        ep: E,
        signal: impl Future<Output = ()>,
        timeout: Option<Duration>,
    ) -> IoResult<()>
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        let ep = Arc::new(ToDynEndpoint(ep.into_endpoint().map_to_response()));
        let name = self.name.as_deref();
        let local_addr = self.listener.bind_addr();
        let alive_connections = Arc::new(AtomicUsize::new(0));
        let notify = Arc::new(Notify::new());
        let timeout_token = CancellationToken::new();
        let server_graceful_shutdown_token = CancellationToken::new();

        // Create the QUIC endpoint
        let endpoint = self.listener.into_endpoint()?;

        tokio::pin!(signal);

        tracing::info!(
            name = name,
            addr = %local_addr,
            "HTTP/3 server listening"
        );

        loop {
            tokio::select! {
                _ = &mut signal => {
                    server_graceful_shutdown_token.cancel();
                    if let Some(timeout) = timeout {
                        tracing::info!(
                            name = name,
                            timeout_in_seconds = timeout.as_secs_f32(),
                            "initiating graceful shutdown",
                        );

                        let timeout_token = timeout_token.clone();
                        tokio::spawn(async move {
                            tokio::time::sleep(timeout).await;
                            timeout_token.cancel();
                        });
                    } else {
                        tracing::info!(name = name, "initiating graceful shutdown");
                    }
                    break;
                }
                incoming = endpoint.accept() => {
                    let Some(incoming) = incoming else {
                        tracing::debug!(name = name, "endpoint closed");
                        break;
                    };

                    let remote_addr = incoming.remote_address();
                    alive_connections.fetch_add(1, Ordering::Release);

                    let ep = ep.clone();
                    let alive_connections = alive_connections.clone();
                    let notify = notify.clone();
                    let timeout_token = timeout_token.clone();
                    let server_graceful_shutdown_token = server_graceful_shutdown_token.clone();
                    let server_graceful_shutdown_token_clone = server_graceful_shutdown_token.clone();

                    let spawn_fut = AssertUnwindSafe(async move {
                        match serve_h3_connection(
                            incoming,
                            local_addr,
                            remote_addr,
                            ep,
                            server_graceful_shutdown_token.clone(),
                        )
                        .await
                        {
                            Ok(()) => {}
                            Err(e) => {
                                tracing::error!(error = %e, "H3 connection error");
                            }
                        }
                    });

                    tokio::spawn(async move {
                        let result = if timeout.is_some() {
                            tokio::select! {
                                res = spawn_fut.catch_unwind() => res,
                                _ = timeout_token.cancelled() => Ok(()),
                            }
                        } else {
                            spawn_fut.catch_unwind().await
                        };

                        if alive_connections.fetch_sub(1, Ordering::Acquire) == 1 {
                            if server_graceful_shutdown_token_clone.is_cancelled() {
                                notify.notify_one();
                            }
                        }

                        if let Err(err) = result {
                            std::panic::resume_unwind(err);
                        }
                    });
                }
            }
        }

        // Wait for endpoint to finish
        endpoint.wait_idle().await;

        if alive_connections.load(Ordering::Acquire) > 0 {
            tracing::info!(name = name, "waiting for all connections to close");
            notify.notified().await;
        }

        tracing::info!(name = name, "HTTP/3 server stopped");
        Ok(())
    }
}

/// Serve a single HTTP/3 connection.
async fn serve_h3_connection(
    incoming: quinn::Incoming,
    local_addr: SocketAddr,
    remote_addr: SocketAddr,
    ep: Arc<dyn DynEndpoint<Output = Response>>,
    graceful_shutdown_token: CancellationToken,
) -> IoResult<()> {
    // Complete the QUIC handshake
    let quic_conn = incoming
        .await
        .map_err(|e| IoError::other(format!("QUIC handshake failed: {e}")))?;

    tracing::debug!(
        remote_addr = %remote_addr,
        "H3 connection established"
    );

    // Build the H3 connection
    let mut h3_conn = h3::server::builder()
        .build(h3_quinn::Connection::new(quic_conn))
        .await
        .map_err(|e| IoError::other(format!("H3 connection setup failed: {e}")))?;

    // Process requests on this connection
    loop {
        if graceful_shutdown_token.is_cancelled() {
            tracing::debug!("graceful shutdown requested, closing H3 connection");
            break;
        }

        match h3_conn.accept().await {
            Ok(Some(resolver)) => {
                let ep = ep.clone();
                let local_addr = local_addr;
                let remote_addr = remote_addr;

                tokio::spawn(async move {
                    if let Err(e) =
                        serve_h3_request(resolver, local_addr, remote_addr, ep).await
                    {
                        tracing::error!(error = %e, "H3 request error");
                    }
                });
            }
            Ok(None) => {
                // Connection closed gracefully
                tracing::debug!(remote_addr = %remote_addr, "H3 connection closed");
                break;
            }
            Err(e) => {
                if is_graceful_h3_close(&e) {
                    tracing::debug!(remote_addr = %remote_addr, "H3 connection closed gracefully");
                } else {
                    tracing::error!(error = %e, "H3 connection error");
                }
                break;
            }
        }
    }

    Ok(())
}

/// Serve a single HTTP/3 request.
async fn serve_h3_request(
    resolver: h3::server::RequestResolver<h3_quinn::Connection, Bytes>,
    local_addr: SocketAddr,
    remote_addr: SocketAddr,
    ep: Arc<dyn DynEndpoint<Output = Response>>,
) -> IoResult<()> {
    // Resolve the H3 request (get headers)
    let (request_head, mut stream) = resolver
        .resolve_request()
        .await
        .map_err(|e| IoError::other(format!("failed to resolve H3 request: {e}")))?;

    tracing::debug!(
        method = %request_head.method(),
        uri = %request_head.uri(),
        "H3 request received"
    );

    // Read the request body
    let body_bytes = read_h3_body(&mut stream).await?;

    // Build the poem Request using the builder
    let request = Request::builder()
        .method(request_head.method().clone())
        .uri(request_head.uri().clone())
        .version(request_head.version())
        .body(Body::from(body_bytes));

    // We need to set headers and state properly
    // Use a mutable request to add headers
    let mut request = request;
    for (name, value) in request_head.headers().iter() {
        request.headers_mut().insert(name.clone(), value.clone());
    }

    // Set the local and remote addresses in the request state
    request.state_mut().local_addr = LocalAddr(Addr::SocketAddr(local_addr));
    request.state_mut().remote_addr = RemoteAddr(Addr::SocketAddr(remote_addr));
    request.state_mut().scheme = Scheme::HTTPS;

    // Call the endpoint
    let response = ep.get_response(request).await;

    // Send the response
    let (parts, body) = response.into_parts();

    // Build HTTP response for h3
    let mut head_only = http::Response::builder()
        .status(parts.status)
        .version(parts.version)
        .body(())
        .map_err(|e| IoError::other(format!("failed to build H3 response: {e}")))?;

    *head_only.headers_mut() = parts.headers;

    stream
        .send_response(head_only)
        .await
        .map_err(|e| IoError::other(format!("failed to send H3 response headers: {e}")))?;

    // Stream the response body
    send_h3_body(&mut stream, body).await?;

    // Finish the stream
    stream
        .finish()
        .await
        .map_err(|e| IoError::other(format!("failed to finish H3 stream: {e}")))?;

    Ok(())
}
