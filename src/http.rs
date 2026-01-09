//! HTTP client with custom DNS resolver for cross-platform compatibility
//!
//! Uses hickory-dns with Cloudflare DNS (1.1.1.1) to avoid relying on
//! system DNS configuration, which may not exist on some platforms (e.g., Termux/Android).

use hickory_resolver::{
    config::ResolverConfig,
    name_server::TokioConnectionProvider,
    Resolver,
};
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

type TokioResolver = Resolver<TokioConnectionProvider>;

/// Custom DNS resolver that uses Cloudflare DNS (1.1.1.1)
/// Does not depend on /etc/resolv.conf
struct HickoryDnsResolver {
    resolver: Arc<TokioResolver>,
}

impl HickoryDnsResolver {
    fn new() -> Self {
        // Use Cloudflare's public DNS - fast and privacy-focused, no system config needed
        let resolver = Resolver::builder_with_config(
            ResolverConfig::cloudflare(),
            TokioConnectionProvider::default(),
        )
        .build();
        Self {
            resolver: Arc::new(resolver),
        }
    }
}

impl Resolve for HickoryDnsResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let resolver = self.resolver.clone();
        Box::pin(async move {
            let lookup = resolver
                .lookup_ip(name.as_str())
                .await
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

            let addrs: Vec<SocketAddr> = lookup
                .iter()
                .map(|ip| SocketAddr::new(ip, 0))
                .collect();

            Ok(Box::new(addrs.into_iter()) as Addrs)
        })
    }
}

/// Create an HTTP client builder with custom DNS resolver
/// This works on all platforms including Termux/Android
pub fn create_client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder().dns_resolver(Arc::new(HickoryDnsResolver::new()))
}

/// Create an HTTP client with custom DNS resolver
/// This works on all platforms including Termux/Android
pub fn create_client() -> reqwest::Client {
    create_client_builder()
        .build()
        .expect("Failed to create HTTP client")
}
