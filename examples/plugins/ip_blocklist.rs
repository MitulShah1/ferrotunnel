//! Example: IP Blocklist Plugin
#![allow(clippy::print_stdout)]
use async_trait::async_trait;
use ferrotunnel_plugin::{Plugin, PluginAction, PluginRegistry, RequestContext};
use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Plugin that blocks requests from specific IP addresses
pub struct IpBlocklistPlugin {
    blocked_ips: HashSet<IpAddr>,
}

impl IpBlocklistPlugin {
    pub fn new(ips: Vec<IpAddr>) -> Self {
        Self {
            blocked_ips: ips.into_iter().collect(),
        }
    }
}

#[async_trait]
impl Plugin for IpBlocklistPlugin {
    fn name(&self) -> &'static str {
        "ip-blocklist"
    }

    async fn on_request(
        &self,
        _req: &mut http::Request<()>,
        ctx: &RequestContext,
    ) -> Result<PluginAction, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let client_ip = ctx.remote_addr.ip();

        if self.blocked_ips.contains(&client_ip) {
            println!("Blocking request from denied IP: {client_ip}");
            return Ok(PluginAction::Reject {
                status: 403,
                reason: "IP Address Blocked".to_string(),
            });
        }

        Ok(PluginAction::Continue)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt().init();

    let blocked_ip = "192.168.1.100".parse()?;
    let mut registry = PluginRegistry::new();
    registry.register(Arc::new(RwLock::new(IpBlocklistPlugin::new(vec![
        blocked_ip,
    ]))));
    registry.init_all().await?;

    // Test Allowed
    println!("Testing allowed IP...");
    let mut req = http::Request::builder().body(())?;
    let allowed_ctx = RequestContext {
        tunnel_id: "test".into(),
        session_id: "test".into(),
        remote_addr: "127.0.0.1:1234".parse()?,
        timestamp: std::time::SystemTime::now(),
    };

    let action = registry
        .execute_request_hooks(&mut req, &allowed_ctx)
        .await?;
    assert_eq!(action, PluginAction::Continue);
    println!("Allowed IP passed.");

    // Test Blocked
    println!("Testing blocked IP...");
    let blocked_ctx = RequestContext {
        tunnel_id: "test".into(),
        session_id: "test".into(),
        remote_addr: format!("{blocked_ip}:1234").parse()?,
        timestamp: std::time::SystemTime::now(),
    };

    let action = registry
        .execute_request_hooks(&mut req, &blocked_ctx)
        .await?;
    match action {
        PluginAction::Reject { status, reason } => {
            println!("Blocked IP rejected as expected: {status} ({reason})");
            assert_eq!(status, 403);
        }
        _ => panic!("Expected rejection!"),
    }

    Ok(())
}
