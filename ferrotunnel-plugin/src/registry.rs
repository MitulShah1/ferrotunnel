use crate::traits::{Plugin, PluginAction, RequestContext, ResponseContext};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Registry manages all loaded plugins
#[derive(Default)]
pub struct PluginRegistry {
    plugins: Vec<Arc<RwLock<dyn Plugin>>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Register a plugin
    pub fn register(&mut self, plugin: Arc<RwLock<dyn Plugin>>) {
        self.plugins.push(plugin);
    }

    /// Initialize all plugins
    pub async fn init_all(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        for plugin_lock in &self.plugins {
            let mut plugin = plugin_lock.write().await;
            tracing::info!("Initializing plugin: {}", plugin.name());
            plugin.init().await?;
        }
        Ok(())
    }

    /// Execute request hooks on all plugins
    pub async fn execute_request_hooks(
        &self,
        req: &mut http::Request<()>,
        ctx: &RequestContext,
    ) -> Result<PluginAction, Box<dyn std::error::Error + Send + Sync + 'static>> {
        for plugin_lock in &self.plugins {
            let plugin = plugin_lock.read().await;
            match plugin.on_request(req, ctx).await? {
                PluginAction::Continue => continue,
                action => return Ok(action), // Short-circuit on non-Continue
            }
        }
        Ok(PluginAction::Continue)
    }

    /// Execute response hooks on all plugins
    pub async fn execute_response_hooks(
        &self,
        res: &mut http::Response<Vec<u8>>,
        ctx: &ResponseContext,
    ) -> Result<PluginAction, Box<dyn std::error::Error + Send + Sync + 'static>> {
        for plugin_lock in &self.plugins {
            let plugin = plugin_lock.read().await;
            match plugin.on_response(res, ctx).await? {
                PluginAction::Continue => continue,
                action => return Ok(action),
            }
        }
        Ok(PluginAction::Continue)
    }

    /// Shutdown all plugins
    pub async fn shutdown_all(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        for plugin_lock in &self.plugins {
            let mut plugin = plugin_lock.write().await;
            tracing::info!("Shutting down plugin: {}", plugin.name());
            plugin.shutdown().await?;
        }
        Ok(())
    }

    /// Returns true if any plugin needs to inspect/modify response bodies.
    /// When false, responses can be streamed without buffering for better performance.
    pub async fn needs_response_buffering(&self) -> bool {
        for plugin_lock in &self.plugins {
            let plugin = plugin_lock.read().await;
            if plugin.needs_response_body() {
                return true;
            }
        }
        false
    }

    /// Returns true if no plugins are registered
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    // Test plugin that always rejects
    struct RejectPlugin;
    #[async_trait]
    impl Plugin for RejectPlugin {
        fn name(&self) -> &str {
            "rejector"
        }

        async fn on_request(
            &self,
            _req: &mut http::Request<()>,
            _ctx: &RequestContext,
        ) -> Result<PluginAction, Box<dyn std::error::Error + Send + Sync + 'static>> {
            Ok(PluginAction::Reject {
                status: 403,
                reason: "Blocked".into(),
            })
        }
    }

    // Test plugin that always continues
    struct PassthroughPlugin;
    #[async_trait]
    impl Plugin for PassthroughPlugin {
        fn name(&self) -> &str {
            "passthrough"
        }
    }

    // Test plugin that needs response body
    struct BodyInspectorPlugin;
    #[async_trait]
    impl Plugin for BodyInspectorPlugin {
        fn name(&self) -> &str {
            "body-inspector"
        }

        fn needs_response_body(&self) -> bool {
            true
        }
    }

    fn make_request_ctx() -> RequestContext {
        RequestContext {
            tunnel_id: "test".into(),
            session_id: "sess".into(),
            remote_addr: "127.0.0.1:80".parse().unwrap(),
            timestamp: std::time::SystemTime::now(),
        }
    }

    #[test]
    fn test_registry_new_is_empty() {
        let registry = PluginRegistry::new();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_registry_not_empty_after_register() {
        let mut registry = PluginRegistry::new();
        registry.register(Arc::new(RwLock::new(PassthroughPlugin)));
        assert!(!registry.is_empty());
    }

    #[tokio::test]
    async fn test_registry_executes_plugins() {
        let plugin = Arc::new(RwLock::new(RejectPlugin));
        let mut registry = PluginRegistry::new();
        registry.register(plugin);

        let mut req = http::Request::builder().body(()).unwrap();
        let ctx = make_request_ctx();

        let action = registry
            .execute_request_hooks(&mut req, &ctx)
            .await
            .unwrap();
        match action {
            PluginAction::Reject { status, .. } => assert_eq!(status, 403),
            _ => panic!("Expected reject"),
        }
    }

    #[tokio::test]
    async fn test_registry_empty_returns_continue() {
        let registry = PluginRegistry::new();
        let mut req = http::Request::builder().body(()).unwrap();
        let ctx = make_request_ctx();

        let action = registry
            .execute_request_hooks(&mut req, &ctx)
            .await
            .unwrap();
        assert_eq!(action, PluginAction::Continue);
    }

    #[tokio::test]
    async fn test_registry_passthrough_returns_continue() {
        let mut registry = PluginRegistry::new();
        registry.register(Arc::new(RwLock::new(PassthroughPlugin)));

        let mut req = http::Request::builder().body(()).unwrap();
        let ctx = make_request_ctx();

        let action = registry
            .execute_request_hooks(&mut req, &ctx)
            .await
            .unwrap();
        assert_eq!(action, PluginAction::Continue);
    }

    #[tokio::test]
    async fn test_registry_init_all_success() {
        let mut registry = PluginRegistry::new();
        registry.register(Arc::new(RwLock::new(PassthroughPlugin)));

        let result = registry.init_all().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_registry_shutdown_all_success() {
        let mut registry = PluginRegistry::new();
        registry.register(Arc::new(RwLock::new(PassthroughPlugin)));

        let result = registry.shutdown_all().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_registry_needs_response_buffering_false_when_empty() {
        let registry = PluginRegistry::new();
        assert!(!registry.needs_response_buffering().await);
    }

    #[tokio::test]
    async fn test_registry_needs_response_buffering_detects_plugin() {
        let mut registry = PluginRegistry::new();
        registry.register(Arc::new(RwLock::new(BodyInspectorPlugin)));
        assert!(registry.needs_response_buffering().await);
    }

    #[tokio::test]
    async fn test_registry_short_circuits_on_reject() {
        let mut registry = PluginRegistry::new();
        // RejectPlugin first, then PassthroughPlugin
        registry.register(Arc::new(RwLock::new(RejectPlugin)));
        registry.register(Arc::new(RwLock::new(PassthroughPlugin)));

        let mut req = http::Request::builder().body(()).unwrap();
        let ctx = make_request_ctx();

        let action = registry
            .execute_request_hooks(&mut req, &ctx)
            .await
            .unwrap();
        // Should get Reject (short-circuited), not Continue
        match action {
            PluginAction::Reject { status, .. } => assert_eq!(status, 403),
            _ => panic!("Expected reject - should short-circuit"),
        }
    }
}
