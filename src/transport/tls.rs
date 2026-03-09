use anyhow::Result;
use rcgen::{CertificateParams, DistinguishedName, SanType};
use rustls::ServerConfig;
use std::sync::Arc;

/// Build a `quinn::ServerConfig` backed by an ephemeral self-signed certificate.
///
/// The certificate is valid for `localhost` plus the optional `san_addresses` list
/// (e.g. the relay's advertised public IP string).
pub fn make_server_config(san_addresses: &[&str]) -> Result<quinn::ServerConfig> {
    let mut params = CertificateParams::default();
    params.distinguished_name = DistinguishedName::new();
    params.subject_alt_names = vec![SanType::DnsName("localhost".try_into()?)];

    for addr in san_addresses {
        // Try to parse as an IP first; fall back to DNS name.
        if let Ok(ip) = addr.parse::<std::net::IpAddr>() {
            params.subject_alt_names.push(SanType::IpAddress(ip));
        } else if let Ok(name) = (*addr).try_into() {
            params.subject_alt_names.push(SanType::DnsName(name));
        }
    }

    let key_pair = rcgen::KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;
    let cert_der = rustls::pki_types::CertificateDer::from(cert.der().to_vec());
    let key_der = rustls::pki_types::PrivateKeyDer::from(
        rustls::pki_types::PrivatePkcs8KeyDer::from(key_pair.serialize_der()),
    );

    let mut tls_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key_der)?;

    // Advertise the same ALPN token the C# client sends (QuicConnector.AlpnToken = "relay").
    // Without this, quinn rejects the handshake with "peer doesn't support any known protocol".
    tls_config.alpn_protocols = vec![b"relay".to_vec()];

    let quic_server_config =
        quinn::crypto::rustls::QuicServerConfig::try_from(Arc::new(tls_config))?;

    // Enable QUIC datagrams for broadcasts
    let mut transport = quinn::TransportConfig::default();
    transport.datagram_receive_buffer_size(Some(65536));  // 64KB buffer for datagrams
    transport.datagram_send_buffer_size(65536);
    
    let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(quic_server_config));
    server_config.transport_config(Arc::new(transport));

    Ok(server_config)
}
