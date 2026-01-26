//! Embeddable tunnel server with builder pattern.
//!
//! # Example
//!
//! ```rust,no_run
//! use ferrotunnel::Server;
//!
//! # async fn example() -> ferrotunnel::Result<()> {
//! let mut server = Server::builder()
//!     .bind("0.0.0.0:7835".parse().unwrap())
//!     .http_bind("0.0.0.0:8080".parse().unwrap())
//!     .token("my-secret-token")
//!     .build()?;
//!
//! server.start().await?;
//! # Ok(())
//! # }
//! ```

use crate::config::ServerConfig;
use ferrotunnel_common::config::TlsConfig;
use ferrotunnel_common::{Result, TunnelError};
use ferrotunnel_core::transport::{tls::TlsTransportConfig, TransportConfig};
use ferrotunnel_core::TunnelServer;
use ferrotunnel_http::HttpIngress;
use ferrotunnel_plugin::PluginRegistry;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{watch, RwLock};
use tokio::task::JoinHandle;
use tracing::info;

/// A tunnel server that can be embedded in your application.
///
/// Use [`Server::builder()`] to create a new server with the builder pattern.
#[derive(Debug)]
pub struct Server {
    config: ServerConfig,
    transport_config: TransportConfig,
    shutdown_tx: Option<watch::Sender<bool>>,
    task: Option<JoinHandle<Result<()>>>,
}

/// Builder for constructing a [`Server`] with ergonomic configuration.
#[derive(Debug, Default)]
pub struct ServerBuilder {
    config: ServerConfig,
    transport_config: Option<TransportConfig>,
}

impl Server {
    /// Create a new server builder.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ferrotunnel::Server;
    ///
    /// let server = Server::builder()
    ///     .bind("0.0.0.0:7835".parse().unwrap())
    ///     .token("secret")
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn builder() -> ServerBuilder {
        ServerBuilder::default()
    }

    /// Start the tunnel server.
    ///
    /// This will bind to the configured addresses and start accepting connections.
    /// The server runs until [`shutdown()`](Self::shutdown) is called.
    ///
    /// # Errors
    ///
    /// Returns an error if the server is already running.
    pub async fn start(&mut self) -> Result<()> {
        if self.task.is_some() {
            return Err(TunnelError::InvalidState("server already started".into()));
        }

        let config = self.config.clone();
        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
        self.shutdown_tx = Some(shutdown_tx);

        info!("Starting `FerroTunnel` Server");
        info!("  Tunnel bind: {}", config.bind_addr);
        info!("  HTTP bind: {}", config.http_bind_addr);

        let tunnel_server = TunnelServer::new(config.bind_addr, config.token)
            .with_transport(self.transport_config.clone());

        // Initialize plugins
        let mut registry = PluginRegistry::new();
        // Add default plugins
        registry.register(Arc::new(RwLock::new(
            ferrotunnel_plugin::builtin::LoggerPlugin::new(),
        )));

        if let Err(e) = registry.init_all().await {
            tracing::error!("Failed to initialize plugins: {}", e);
            // We continue starting the server even if plugins fail, but we log it.
            // Alternatively, we could return the error: return Err(TunnelError::InvalidState(format!("Plugin init failed: {e}")));
        }

        let registry = Arc::new(registry);
        let sessions = tunnel_server.sessions();
        let ingress = HttpIngress::new(config.http_bind_addr, sessions, registry);

        // Spawn both services
        let tunnel_handle = tokio::spawn(async move { tunnel_server.run().await });
        let ingress_handle = tokio::spawn(async move { ingress.start().await });

        // Wait for shutdown or either service to exit
        tokio::select! {
            result = tunnel_handle => {
                match result {
                    Ok(inner) => inner?,
                    Err(e) => return Err(TunnelError::Connection(format!("Tunnel task panicked: {e}"))),
                }
            }
            result = ingress_handle => {
                match result {
                    Ok(inner) => inner?,
                    Err(e) => return Err(TunnelError::Connection(format!("Ingress task panicked: {e}"))),
                }
            }
            _ = shutdown_rx.changed() => {
                info!("Server shutdown requested");
            }
        }

        Ok(())
    }

    /// Shutdown the tunnel server and wait for cleanup.
    ///
    /// This will gracefully shut down the server and close all connections.
    pub async fn shutdown(&mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(true);
        }
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
        Ok(())
    }

    /// Signal the server to stop (non-blocking).
    ///
    /// Use [`shutdown()`](Self::shutdown) if you need to wait for cleanup.
    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(true);
        }
    }

    /// Check if the server is currently running.
    pub fn is_running(&self) -> bool {
        if let Some(task) = &self.task {
            !task.is_finished()
        } else {
            self.shutdown_tx.is_some()
        }
    }

    /// Get the current configuration.
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        // Best-effort signal shutdown on drop
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(true);
        }
    }
}

impl ServerBuilder {
    /// Set the address to bind the tunnel control plane.
    ///
    /// Default: `0.0.0.0:7835`
    #[must_use]
    pub fn bind(mut self, addr: SocketAddr) -> Self {
        self.config.bind_addr = addr;
        self
    }

    /// Set the address to bind the HTTP ingress.
    ///
    /// Default: `0.0.0.0:8080`
    #[must_use]
    pub fn http_bind(mut self, addr: SocketAddr) -> Self {
        self.config.http_bind_addr = addr;
        self
    }

    /// Set the authentication token.
    ///
    /// Clients must provide this token to connect.
    #[must_use]
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.config.token = token.into();
        self
    }

    /// Configure TLS for the server.
    ///
    /// When enabled, the server will use TLS for all connections.
    #[must_use]
    pub fn tls(mut self, config: TlsConfig) -> Self {
        if config.enabled {
            self.transport_config = Some(TransportConfig::Tls(TlsTransportConfig {
                ca_cert_path: config.ca_cert_path.map(|p| p.to_string_lossy().to_string()),
                cert_path: config
                    .cert_path
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default(),
                key_path: config
                    .key_path
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default(),
                server_name: config.server_name,
                client_auth: config.client_auth,
            }));
        }
        self
    }

    /// Build the server with the configured options.
    ///
    /// # Errors
    ///
    /// Returns an error if required configuration is missing:
    /// - `token` must be set
    pub fn build(self) -> Result<Server> {
        self.config.validate()?;
        Ok(Server {
            config: self.config,
            transport_config: self.transport_config.unwrap_or_default(),
            shutdown_tx: None,
            task: None,
        })
    }
}
