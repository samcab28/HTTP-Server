use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use http_server::{handlers::basics, ConfigBuilder, HttpServer, Router};
use http_server::{ServerResult};
use http_server::server::{HttpRequest, HttpResponse};

fn pick_free_port() -> u16 {
    std::net::TcpListener::bind(("127.0.0.1", 0))
        .and_then(|l| l.local_addr())
        .map(|addr| addr.port())
        .unwrap()
}

fn start_test_server(port: u16) -> (std::thread::JoinHandle<()>, u16, std::sync::Arc<std::sync::atomic::AtomicBool>) {
    // Build config
    let config = ConfigBuilder::new().port(port).build().unwrap();

    // Minimal router with a couple of endpoints
    let router = Router::new()
        .route("/help", basics::handle_help)
        .route("/timestamp", basics::handle_timestamp);

    let mut server = HttpServer::new(config, router);
    let running = server.running_flag();

    let handle = thread::spawn(move || {
        // Ignore error on purpose to allow test teardown
        let _ = server.start();
    });

    (handle, port, running)
}

fn start_server_with_router(
    port: u16,
    router: Router,
) -> (std::thread::JoinHandle<()>, u16, std::sync::Arc<std::sync::atomic::AtomicBool>) {
    let config = ConfigBuilder::new().port(port).build().unwrap();
    let mut server = HttpServer::new(config, router);
    let running = server.running_flag();
    let handle = thread::spawn(move || {
        let _ = server.start();
    });
    (handle, port, running)
}

fn http_get(port: u16, path: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .write_all(format!("GET {} HTTP/1.0\r\nHost: localhost\r\n\r\n", path).as_bytes())
        .expect("write");
    stream.flush().ok();

    let mut buf = String::new();
    stream.read_to_string(&mut buf).ok();
    let _ = stream.shutdown(Shutdown::Both);
    buf
}

#[test]
fn test_help_endpoint_responds() {
    let port = pick_free_port();
    let (handle, port, running) = start_test_server(port);

    // Allow server to start
    thread::sleep(Duration::from_millis(100));

    let resp = http_get(port, "/help");
    assert!(resp.starts_with("HTTP/1.0 200 OK"), "unexpected response: {}", resp);
    assert!(resp.contains("\"endpoints\""), "help should list endpoints");

    // Shutdown server
    running.store(false, Ordering::SeqCst);
    // Wake the accept loop
    let _ = TcpStream::connect(("127.0.0.1", port));
    let _ = handle.join();
}

#[test]
fn test_timestamp_endpoint_responds() {
    let port = pick_free_port();
    let (handle, port, running) = start_test_server(port);

    thread::sleep(Duration::from_millis(100));

    let resp = http_get(port, "/timestamp");
    assert!(resp.starts_with("HTTP/1.0 200 OK"));
    assert!(resp.contains("\"timestamp\""));

    running.store(false, Ordering::SeqCst);
    let _ = TcpStream::connect(("127.0.0.1", port));
    let _ = handle.join();
}

// A test handler that sleeps to simulate a slow operation
fn slow_handler(_req: &HttpRequest) -> ServerResult<HttpResponse> {
    thread::sleep(Duration::from_millis(250));
    Ok(HttpResponse::ok().with_text("slow-ok"))
}

#[test]
fn test_concurrent_requests_complete_faster_than_serial() {
    let port = pick_free_port();
    let router = Router::new().route("/slow", slow_handler);
    let (handle, port, running) = start_server_with_router(port, router);

    // Give server time to start
    thread::sleep(Duration::from_millis(100));

    // Fire N concurrent requests to /slow
    let n = 8;
    let start = std::time::Instant::now();

    let mut threads = Vec::new();
    for _ in 0..n {
        let port = port;
        threads.push(thread::spawn(move || {
            let _ = http_get(port, "/slow");
        }));
    }

    for t in threads {
        let _ = t.join();
    }

    let elapsed = start.elapsed();
    // If handled serially: ~ n * 250ms = ~2000ms
    // With thread-per-connection: should be close to ~250-600ms
    println!("Elapsed for {} concurrent requests: {:?}", n, elapsed);
    assert!(
        elapsed < Duration::from_millis(1500),
        "expected concurrent handling, elapsed = {:?}",
        elapsed
    );

    // Shutdown
    running.store(false, Ordering::SeqCst);
    let _ = TcpStream::connect(("127.0.0.1", port));
    let _ = handle.join();
}
