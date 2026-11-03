use eos_plugin_protocol::{MarketAccess, SearchRequest};
use eos_plugin_runtime::MarketClient;
use std::{
    io::{Read, Write},
    net::TcpListener,
};

#[test]
fn anonymous_discovery_sends_no_credentials_and_cannot_obtain_runtime_authority() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut bytes = [0; 8192];
        let length = stream.read(&mut bytes).unwrap();
        let request = String::from_utf8_lossy(&bytes[..length]).to_lowercase();
        assert!(request.starts_with("post /api/v1/plugin-market/search "));
        assert!(!request.contains("authorization:"));
        assert!(!request.contains("x-plugin-market-session:"));
        let body = r#"{"data":{"results":[],"total":0}}"#;
        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
    });
    let access: MarketAccess = serde_json::from_value(serde_json::json!({"origin":origin,"subjectId":"","accountToken":"","marketToken":"","applicationId":"fixture-app","environmentId":"fixture","hostVersion":"1.0.0"})).unwrap();
    let client = MarketClient::new(&access, vec![1]).unwrap();
    let request: SearchRequest = serde_json::from_value(serde_json::json!({"query":"","host":{"applicationId":"fixture-app","hostVersion":"1.0.0","platformApiVersion":"1.0.0","environmentId":"fixture","hostInstanceId":"instance","platform":{"os":"macos","arch":"aarch64","surface":"desktop"},"executors":[],"capabilities":[],"policyRevision":1}})).unwrap();
    assert_eq!(client.search(&request).unwrap().total, 0);
    let cache = tempfile::tempdir().unwrap();
    assert_eq!(
        client.verify(cache.path(), None).err().unwrap().code,
        "UNAUTHENTICATED"
    );
    server.join().unwrap();
}
