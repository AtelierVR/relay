/// Common test fixtures and helpers used across all handler test modules.
use std::sync::Arc;

use bytes::Bytes;
use parking_lot::Mutex;
use quinn::Connection;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, Error as TlsError, SignatureScheme};

use crate::{
    client::{client::new_client, ClientManager},
    config::{Config, LoadBalancingConfig},
    constants::DEFAULT_NODE_ADDRESS,
    handlers::context::AppState,
    instance::InstanceManager,
    master::MasterClient,
    proto::{buffer::PacketReader, packet_type::PacketType},
    utils::log_buffer::LogBuffer,
};

/// A no-op TLS verifier for test QUIC connections.
#[derive(Debug)]
struct NoVerifier;

impl ServerCertVerifier for NoVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, TlsError> {
        Ok(ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }
    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA1,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
            SignatureScheme::ED448,
        ]
    }
}

/// Create a real loopback QUIC connection for use in unit tests.
fn make_test_connection() -> Arc<Connection> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let server_config =
            crate::transport::tls::make_server_config(&[], 30, 5).expect("test server TLS");
        let server_ep =
            quinn::Endpoint::server(server_config, "127.0.0.1:0".parse().unwrap()).unwrap();
        let server_addr = server_ep.local_addr().unwrap();

        let mut client_tls = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoVerifier))
            .with_no_client_auth();
        client_tls.alpn_protocols = vec![b"relay".to_vec()];

        let mut transport = quinn::TransportConfig::default();
        transport.datagram_receive_buffer_size(Some(65536));

        let quic_client =
            quinn::crypto::rustls::QuicClientConfig::try_from(Arc::new(client_tls)).unwrap();
        let mut client_cfg = quinn::ClientConfig::new(Arc::new(quic_client));
        client_cfg.transport_config(Arc::new(transport));

        let mut client_ep = quinn::Endpoint::client("127.0.0.1:0".parse().unwrap()).unwrap();
        client_ep.set_default_client_config(client_cfg);

        let connecting = client_ep.connect(server_addr, "localhost").unwrap();
        let (conn, _) = tokio::join!(connecting, async {
            if let Some(inc) = server_ep.accept().await {
                let _ = inc.await;
            }
        });
        Arc::new(conn.unwrap())
    })
}

/// Build a minimal `AppState` for unit tests.
/// - `node_address` is set to `DEFAULT_NODE_ADDRESS` so `has_master = true`.
pub fn make_state() -> Arc<AppState> {
    let config = Arc::new(Config {
        port: 23032,
        node_gateway: String::new(),
        node_address: DEFAULT_NODE_ADDRESS.to_owned(),
        use_address: None,
        token: String::new(),
        max_instances: 3,
        connection_timeout: 15,
        keep_alive_interval: 5,
        property_resend_interval: 0,
        debug: false,
        load_balancing: LoadBalancingConfig::default(),
    });
    let clients = Arc::new(ClientManager::new());
    let instances = Arc::new(InstanceManager::new());
    let log_buf = Arc::new(Mutex::new(LogBuffer::new(16)));
    let log_forwarder = Arc::new(Mutex::new(None));
    Arc::new(AppState::new(
        clients.clone(),
        instances.clone(),
        Arc::new(MasterClient::new(
            config.clone(),
            clients,
            instances,
            log_buf.clone(),
            log_forwarder,
        )),
        config,
        log_buf,
    ))
}

/// Register a fresh client in `state` and return the push channel receiver.
pub fn register_client(state: &AppState, client_id: u16) -> tokio::sync::mpsc::Receiver<Bytes> {
    let (arc, rx) = new_client(client_id, make_test_connection());
    state.clients.add(arc);
    rx
}

/// Mark a registered client as handshaked.
pub fn set_handshaked(state: &AppState, client_id: u16) {
    if let Some(arc) = state.clients.get(client_id) {
        arc.write().auth_state = crate::client::client::AuthState::Handshaked;
    }
}

/// Decode a fully-framed stream packet returned by a handler.
///
/// Wire: `[total: u16 BE][uid: u16 BE][type: u8][payload...]`
/// where `total` is the inclusive total length (= `data.len()`).
///
/// Returns `(uid, PacketType, payload_bytes)`.
pub fn decode_stream(data: Bytes) -> (u16, PacketType, Bytes) {
    assert!(!data.is_empty(), "expected a non-empty response packet");
    let mut r = PacketReader::new(data.clone());
    let total = r.read_u16() as usize;
    assert_eq!(
        total,
        data.len(),
        "stream packet: total-length field mismatch"
    );
    let uid = r.read_u16();
    let type_byte = r.read_u8();
    let ptype = PacketType::try_from(type_byte)
        .unwrap_or_else(|b| panic!("unknown packet type byte 0x{b:02X}"));
    let payload = r.read_remaining();
    (uid, ptype, payload)
}
