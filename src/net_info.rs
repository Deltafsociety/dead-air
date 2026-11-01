use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};

pub async fn detect_local_interface_ip() -> Option<String> {
    let socket = UdpSocket::bind("0.0.0.0:0").await.ok()?;
    socket.connect("10.255.255.255:80").await.ok()?;
    let addr = socket.local_addr().ok()?;
    let ip = addr.ip().to_string();
    if ip != "0.0.0.0" && !ip.starts_with("127.") {
        Some(ip)
    } else {
        None
    }
}

pub async fn detect_public_ip() -> Option<String> {
    let endpoints = [
        ("api.ipify.org", "api.ipify.org:80", "GET / HTTP/1.1\r\nHost: api.ipify.org\r\nConnection: close\r\n\r\n"),
        ("icanhazip.com", "icanhazip.com:80", "GET / HTTP/1.1\r\nHost: icanhazip.com\r\nConnection: close\r\n\r\n"),
        ("ifconfig.me", "ifconfig.me:80", "GET /ip HTTP/1.1\r\nHost: ifconfig.me\r\nConnection: close\r\n\r\n"),
    ];

    for (_host, addr, request) in endpoints {
        if let Ok(ip) = fetch_ip_from_endpoint(addr, request).await {
            return Some(ip);
        }
    }

    None
}

async fn fetch_ip_from_endpoint(addr: &str, request: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let timeout = Duration::from_secs(3);
    let stream_res = tokio::time::timeout(timeout, TcpStream::connect(addr)).await?;
    let mut stream = stream_res?;

    stream.write_all(request.as_bytes()).await?;

    let mut buf = vec![0u8; 1024];
    let read_res = tokio::time::timeout(timeout, stream.read(&mut buf)).await?;
    let n = read_res?;

    let response = String::from_utf8_lossy(&buf[..n]);
    if let Some(body) = response.split("\r\n\r\n").nth(1) {
        let ip = body.trim().to_string();
        if ip.parse::<SocketAddr>().is_ok() || ip.parse::<std::net::IpAddr>().is_ok() {
            return Ok(ip);
        }
    }

    Err("invalid ip response".into())
}

pub async fn resolve_best_address(relay_port: u16) -> (String, String) {
    if let Some(public_ip) = detect_public_ip().await {
        let url = format!("ws://{}:{}", public_ip, relay_port);
        return (public_ip, url);
    }

    if let Some(local_ip) = detect_local_interface_ip().await {
        let url = format!("ws://{}:{}", local_ip, relay_port);
        return (local_ip, url);
    }

    ("127.0.0.1".to_string(), format!("ws://127.0.0.1:{}", relay_port))
}
