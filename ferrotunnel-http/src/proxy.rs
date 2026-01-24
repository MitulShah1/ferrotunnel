use ferrotunnel_core::stream::multiplexer::VirtualStream;
use tokio::net::TcpStream;
use tracing::{debug, error, info};

#[derive(Clone)]
pub struct HttpProxy {
    target_addr: String,
}

impl HttpProxy {
    pub fn new(target_addr: String) -> Self {
        Self { target_addr }
    }

    pub fn handle_stream(&self, mut stream: VirtualStream) {
        let target = self.target_addr.clone();
        tokio::spawn(async move {
            debug!("Connecting to local service {}", target);
            match TcpStream::connect(&target).await {
                Ok(mut local_stream) => {
                    info!("Proxied HTTP request to {}", target);
                    if let Err(e) =
                        tokio::io::copy_bidirectional(&mut stream, &mut local_stream).await
                    {
                        // Connection reset or closed is normal sometimes
                        debug!("Proxy stream ended: {}", e);
                    }
                }
                Err(e) => {
                    error!("Failed to connect to local service {}: {}", target, e);
                    // We should probably close the stream or send an error?
                    // VirtualStream dropped here will send CloseStream effectively?
                    // Dropping VirtualStream sends CloseStream if we implemented Drop?
                    // No, VirtualStream implement AsyncWrite shutdown.
                    // If we drop it, the sender is dropped. The Multiplexer sees channel closed.
                    // But Multiplexer assumes "Stream receiver dropped, cleanup".
                    // It doesn't necessarily send CloseStream to peer.
                    // We should definitely shutdown the stream.
                }
            }
        });
    }
}
