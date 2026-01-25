use async_trait::async_trait;
use ferrotunnel_plugin::{Plugin, PluginAction, PluginRegistry, RequestContext};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Example plugin that adds a custom header to requests
pub struct HelloPlugin {
    greeting: String,
}

impl HelloPlugin {
    pub fn new(greeting: impl Into<String>) -> Self {
        Self {
            greeting: greeting.into(),
        }
    }
}

#[async_trait]
impl Plugin for HelloPlugin {
    fn name(&self) -> &str {
        "hello-plugin"
    }

    // Optional: Implement init to perform setup
    async fn init(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        println!(
            "Plug: HelloPlugin initialized with greeting: {}",
            self.greeting
        );
        Ok(())
    }

    async fn on_request(
        &self,
        req: &mut http::Request<Vec<u8>>,
        _ctx: &RequestContext,
    ) -> Result<PluginAction, Box<dyn std::error::Error + Send + Sync + 'static>> {
        // Add custom header to all requests
        req.headers_mut()
            .insert("X-Custom-Greeting", self.greeting.parse()?);

        Ok(PluginAction::Continue)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Setup logging
    tracing_subscriber::fmt().init();

    // Create registry
    let mut registry = PluginRegistry::new();

    println!("Registering HelloPlugin...");
    // Register the custom plugin
    registry.register(Arc::new(RwLock::new(HelloPlugin::new(
        "Hello from Example!",
    ))));

    // Initialize plugins
    registry.init_all().await?;

    println!("HelloPlugin Registered and Initialized successfully!");

    // In a real server, we would now start the ingress with this registry.
    // For this example, we just simulate a request to verify functionality.

    // Simulate request
    let mut req = http::Request::builder()
        .uri("http://example.com")
        .body(vec![])?;

    let ctx = RequestContext {
        tunnel_id: "example-tunnel".to_string(),
        session_id: "test-session".to_string(),
        remote_addr: "127.0.0.1:1234".parse()?,
        timestamp: std::time::SystemTime::now(),
    };

    println!("Simulating request processing...");
    registry.execute_request_hooks(&mut req, &ctx).await?;

    if let Some(greeting) = req.headers().get("X-Custom-Greeting") {
        println!("Success! Found header: {:?}", greeting);
    } else {
        println!("Error: Header not found!");
    }

    Ok(())
}
