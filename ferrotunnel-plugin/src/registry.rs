use crate::traits::*;
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
        req: &mut http::Request<Vec<u8>>,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct RejectPlugin;
    #[async_trait]
    impl Plugin for RejectPlugin {
        fn name(&self) -> &str {
            "rejector"
        }

        async fn on_request(
            &self,
            _req: &mut http::Request<Vec<u8>>,
            _ctx: &RequestContext,
        ) -> Result<PluginAction, Box<dyn std::error::Error + Send + Sync + 'static>> {
            Ok(PluginAction::Reject {
                status: 403,
                reason: "Blocked".into(),
            })
        }
    }

    #[tokio::test]
    async fn test_registry_executes_plugins() {
        let plugin = Arc::new(RwLock::new(RejectPlugin));
        let mut registry = PluginRegistry::new();
        registry.register(plugin);

        let mut req = http::Request::builder().body(vec![]).unwrap();
        let ctx = RequestContext {
            tunnel_id: "test".into(),
            session_id: "sess".into(),
            remote_addr: "127.0.0.1:80".parse().unwrap(),
            timestamp: std::time::SystemTime::now(),
        };

        let action = registry
            .execute_request_hooks(&mut req, &ctx)
            .await
            .unwrap();
        match action {
            PluginAction::Reject { status, .. } => assert_eq!(status, 403),
            _ => panic!("Expected reject"),
        }
    }
}
