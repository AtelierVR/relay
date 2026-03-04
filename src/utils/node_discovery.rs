use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::net::IpAddr;
use std::time::Duration;

/// Default node HTTP API port
const DEFAULT_PORT: u16 = 3042;
/// Timeout for HTTP requests
const TIMEOUT: Duration = Duration::from_secs(5);

/// Automatic node gateway discovery
///
/// This module implements the Nox node discovery protocol:
/// 1. If the address is an IP or localhost, tries HTTP directly
/// 2. For domain names, tries /.well-known/nox endpoint (HTTPS then HTTP)
/// 3. Falls back to DNS TXT record lookup (_nox.{domain})
///
/// # Examples
/// ```no_run
/// use noxrelay::utils::node_discovery::find_node_gateway;
///
/// # async fn example() -> anyhow::Result<()> {
/// // Discover from domain
/// let gateway = find_node_gateway("example.com").await?;
///
/// // With explicit port
/// let gateway = find_node_gateway("example.com:3042").await?;
///
/// // IP address
/// let gateway = find_node_gateway("192.168.1.100").await?;
/// # Ok(())
/// # }
/// ```
/// DNS TXT record response from Google DNS API
#[derive(Debug, Deserialize)]
struct DnsResponse {
    #[serde(rename = "Status")]
    status: i32,
    #[serde(rename = "Answer", default)]
    answer: Vec<DnsAnswer>,
}

#[derive(Debug, Deserialize)]
struct DnsAnswer {
    data: String,
}

impl DnsAnswer {
    /// Parse TXT record data like "mg=https://example.com:3042"
    fn parse_gateway(&self) -> Option<String> {
        // Remove quotes from TXT record
        let data = self.data.trim_matches('"');

        // Split by semicolon and find mg= entry
        for part in data.split(';') {
            let kv: Vec<&str> = part.split('=').collect();
            if kv.len() == 2 && kv[0].trim() == "mg" {
                return Some(kv[1].trim().to_string());
            }
        }
        None
    }
}

/// Discover node gateway URL from domain name
pub async fn find_node_gateway(address: &str) -> Result<String> {
    if address.is_empty() {
        return Err(anyhow!("Empty address"));
    }

    // Parse address to extract host and port
    let (host, port) = parse_address(address)?;

    // Check if it's an IP address or localhost
    if host.parse::<IpAddr>().is_ok() || host == "localhost" {
        let gateway = format!("http://{}:{}", host, port);
        if test_well_known(&gateway).await {
            return Ok(gateway);
        }
        return Err(anyhow!("Node not found at {}", gateway));
    }

    // Try direct connection (HTTPS then HTTP)
    for protocol in ["https", "http"] {
        let gateway = format!("{}://{}:{}", protocol, host, port);
        if test_well_known(&gateway).await {
            return Ok(gateway);
        }
    }

    // Try DNS TXT record resolution
    if let Ok(gateways) = resolve_dns_txt(&host).await {
        if let Some(gateway) = gateways.first() {
            return Ok(gateway.clone());
        }
    }

    Err(anyhow!("Could not discover node gateway for {}", address))
}

/// Parse address into (host, port)
fn parse_address(address: &str) -> Result<(String, u16)> {
    // Handle "host:port" format
    if let Some((host, port_str)) = address.rsplit_once(':') {
        if let Ok(port) = port_str.parse::<u16>() {
            return Ok((host.to_string(), port));
        }
    }

    // No port specified, use default
    Ok((address.to_string(), DEFAULT_PORT))
}

/// Test if /.well-known/nox endpoint responds
async fn test_well_known(base_url: &str) -> bool {
    let url = format!("{}/.well-known/nox", base_url);

    match reqwest::Client::builder().timeout(TIMEOUT).build() {
        Ok(client) => match client.get(&url).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        },
        Err(_) => false,
    }
}

/// Resolve node gateway from DNS TXT record (_nox.{domain})
async fn resolve_dns_txt(domain: &str) -> Result<Vec<String>> {
    let url = format!("https://dns.google/resolve?name=_nox.{}&type=TXT", domain);

    let client = reqwest::Client::builder().timeout(TIMEOUT).build()?;

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(anyhow!("DNS query failed"));
    }

    let dns_response: DnsResponse = response.json().await?;

    if dns_response.status != 0 {
        return Err(anyhow!("DNS query returned error status"));
    }

    let gateways: Vec<String> = dns_response
        .answer
        .iter()
        .filter_map(|a| a.parse_gateway())
        .collect();

    if gateways.is_empty() {
        return Err(anyhow!("No gateway found in DNS TXT records"));
    }

    Ok(gateways)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_address() {
        assert_eq!(
            parse_address("example.com:3042").unwrap(),
            ("example.com".to_string(), 3042)
        );
        assert_eq!(
            parse_address("example.com").unwrap(),
            ("example.com".to_string(), DEFAULT_PORT)
        );
        assert_eq!(
            parse_address("192.168.1.1:8080").unwrap(),
            ("192.168.1.1".to_string(), 8080)
        );
    }

    #[test]
    fn test_dns_answer_parse() {
        let answer = DnsAnswer {
            data: "\"mg=https://example.com:3042\"".to_string(),
        };
        assert_eq!(
            answer.parse_gateway(),
            Some("https://example.com:3042".to_string())
        );

        let answer = DnsAnswer {
            data: "\"version=1;mg=http://node.example.com\"".to_string(),
        };
        assert_eq!(
            answer.parse_gateway(),
            Some("http://node.example.com".to_string())
        );
    }
}
