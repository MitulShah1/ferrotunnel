//! # FerroTunnel Plugin System
//!
//! This crate provides an extensible plugin system for FerroTunnel, allowing you to
//! inspect, modify, or reject HTTP requests and responses as they flow through the tunnel.
//!
//! ## Features
//!
//! - **Async-First** - All plugin hooks are async for non-blocking I/O
//! - **Type-Safe** - Trait-based design with compile-time guarantees
//! - **Lifecycle Management** - Plugin initialization and graceful shutdown
//! - **Built-in Plugins** - Logger, token auth, and rate limiting included
//!
//! ## Quick Start
//!
//! ### Creating a Custom Plugin
//!
//! ```rust
//! use ferrotunnel_plugin::{Plugin, PluginAction, RequestContext};
//! use async_trait::async_trait;
//!
//! struct MyPlugin;
//!
//! #[async_trait]
//! impl Plugin for MyPlugin {
//!     fn name(&self) -> &str {
//!         "my-plugin"
//!     }
//!
//!     async fn on_request(
//!         &self,
//!         req: &mut http::Request<()>,
//!         ctx: &RequestContext,
//!     ) -> Result<PluginAction, Box<dyn std::error::Error + Send + Sync + 'static>> {
//!         // Inspect or modify the request
//!         if req.uri().path() == "/admin" {
//!             return Ok(PluginAction::Reject {
//!                 status: 403,
//!                 reason: "Access denied".to_string(),
//!             });
//!         }
//!         Ok(PluginAction::Continue)
//!     }
//! }
//! ```
//!
//! ### Registering Plugins
//!
//! ```rust,no_run
//! use ferrotunnel_plugin::PluginRegistry;
//! use std::sync::Arc;
//! use tokio::sync::RwLock;
//!
//! # struct MyPlugin;
//! # #[async_trait::async_trait]
//! # impl ferrotunnel_plugin::Plugin for MyPlugin {
//! #     fn name(&self) -> &str { "my-plugin" }
//! # }
//! #
//! #[tokio::main]
//! async fn main() {
//!     let mut registry = PluginRegistry::new();
//!     
//!     // Register your custom plugin
//!     registry.register(Arc::new(RwLock::new(MyPlugin)));
//!     
//!     // Initialize all plugins
//!     let _ = registry.init_all().await;
//! }
//! ```
//!
//! ## Built-in Plugins
//!
//! ### Logger Plugin
//!
//! Logs all HTTP requests and responses:
//!
//! ```rust
//! use ferrotunnel_plugin::builtin::LoggerPlugin;
//! # use ferrotunnel_plugin::PluginRegistry;
//! # use std::sync::Arc;
//! # use tokio::sync::RwLock;
//!
//! let logger = LoggerPlugin::new().with_body_logging();
//! # let mut registry = PluginRegistry::new();
//! registry.register(Arc::new(RwLock::new(logger)));
//! ```
//!
//! ### Token Auth Plugin
//!
//! Validates authentication tokens:
//!
//! ```rust
//! use ferrotunnel_plugin::builtin::TokenAuthPlugin;
//! # use ferrotunnel_plugin::PluginRegistry;
//! # use std::sync::Arc;
//! # use tokio::sync::RwLock;
//!
//! let auth = TokenAuthPlugin::new(vec!["secret-token".to_string()]);
//! # let mut registry = PluginRegistry::new();
//! registry.register(Arc::new(RwLock::new(auth)));
//! ```
//!
//! ### Rate Limit Plugin
//!
//! Limits requests per tunnel:
//!
//! ```rust
//! use ferrotunnel_plugin::builtin::RateLimitPlugin;
//! # use ferrotunnel_plugin::PluginRegistry;
//! # use std::sync::Arc;
//! # use tokio::sync::RwLock;
//! # use std::num::NonZero;
//!
//! let rate_limiter = RateLimitPlugin::new(NonZero::new(100).unwrap()); // 100 requests/second
//! # let mut registry = PluginRegistry::new();
//! registry.register(Arc::new(RwLock::new(rate_limiter)));
//! ```
//!
//! ## Plugin Actions
//!
//! Plugins can return different actions:
//!
//! - `PluginAction::Continue` - Allow request, continue to next plugin
//! - `PluginAction::Reject { status, reason }` - Reject with HTTP status
//! - `PluginAction::Respond { status, headers, body }` - Send custom response
//!
//! ## See Also
//!
//! - [`Plugin`] - Core plugin trait
//! - [`PluginRegistry`] - Plugin management and execution
//! - [`PluginAction`] - Actions that plugins can return

pub mod builtin;
pub mod registry;
pub mod traits;

pub use registry::*;
pub use traits::*;
