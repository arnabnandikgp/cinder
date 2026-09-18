//! Native account subscriptions are wake hints, never finality or risk proof.
//! No account payload, endpoint or credential crosses into the mutation queue.
use crate::rpc::{Result, RuntimeError};
use std::{
    net::{TcpStream, ToSocketAddrs},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex,
    },
    time::Duration,
};
use tungstenite::{protocol::WebSocketConfig, Message};

pub(crate) struct NativeFeed {
    stop: Arc<AtomicBool>,
    wake: Arc<(Mutex<bool>, Condvar)>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl NativeFeed {
    pub fn start(env: &str, keys: Vec<String>) -> Result<Self> {
        let endpoint = std::env::var(env).map_err(|_| RuntimeError::Configuration)?;
        Self::connect(endpoint, keys)
    }
    fn connect(endpoint: String, keys: Vec<String>) -> Result<Self> {
        let url = reqwest::Url::parse(&endpoint).map_err(|_| RuntimeError::Configuration)?;
        let loopback = matches!(
            url.host_str(),
            Some("localhost" | "127.0.0.1" | "::1" | "[::1]")
        );
        if !(url.scheme() == "wss" || url.scheme() == "ws" && loopback)
            || !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
            || url.query_pairs().any(|(k, _)| k == "token")
            || keys.is_empty()
            || keys.len() > 66
            || keys.iter().collect::<std::collections::BTreeSet<_>>().len() != keys.len()
            || keys.iter().any(|k| crate::transaction::bytes(k).is_err())
        {
            return Err(RuntimeError::Configuration);
        }
        let stop = Arc::new(AtomicBool::new(false));
        let wake = Arc::new((Mutex::new(false), Condvar::new()));
        let worker = {
            let stopping = stop.clone();
            let waking = wake.clone();
            std::thread::spawn(move || {
                while !stopping.load(Ordering::Relaxed) {
                    let attempt = (|| -> std::result::Result<(), ()> {
                        let host = url.host_str().ok_or(())?.trim_matches(['[', ']']);
                        let port =
                            url.port()
                                .unwrap_or(if url.scheme() == "wss" { 443 } else { 80 });
                        let addresses = (host, port).to_socket_addrs().map_err(|_| ())?;
                        let mut socket = None;
                        for address in addresses.take(8) {
                            if stopping.load(Ordering::Relaxed) {
                                return Ok(());
                            }
                            if let Ok(s) =
                                TcpStream::connect_timeout(&address, Duration::from_secs(1))
                            {
                                socket = Some(s);
                                break;
                            }
                        }
                        let socket = socket.ok_or(())?;
                        socket.set_nodelay(true).map_err(|_| ())?;
                        socket
                            .set_read_timeout(Some(Duration::from_secs(1)))
                            .map_err(|_| ())?;
                        socket
                            .set_write_timeout(Some(Duration::from_secs(1)))
                            .map_err(|_| ())?;
                        let cfg = WebSocketConfig::default()
                            .max_message_size(Some(16 * 1024 * 1024))
                            .max_frame_size(Some(16 * 1024 * 1024));
                        let (mut ws, _) = tungstenite::client_tls_with_config(
                            endpoint.as_str(),
                            socket,
                            Some(cfg),
                            None,
                        )
                        .map_err(|_| ())?;
                        for (i, key) in keys.iter().enumerate() {
                            let msg=serde_json::json!({"jsonrpc":"2.0","id":i+1,"method":"accountSubscribe","params":[key,{"encoding":"base64","commitment":"confirmed"}]}).to_string();
                            ws.send(Message::Text(msg.into())).map_err(|_| ())?;
                        }
                        while !stopping.load(Ordering::Relaxed) {
                            match ws.read() {
                                Ok(Message::Text(t)) => {
                                    // Subscription errors force reconnect/replay. No
                                    // WS result is ever treated as financial evidence.
                                    let v: serde_json::Value =
                                        serde_json::from_str(&t).map_err(|_| ())?;
                                    if v.get("error").is_some() {
                                        return Err(());
                                    }
                                    if v["method"] == "accountNotification" {
                                        if let Ok(mut pending) = waking.0.lock() {
                                            *pending = true;
                                            waking.1.notify_one();
                                        }
                                    }
                                }
                                Ok(Message::Close(_)) => return Err(()),
                                Ok(_) => {}
                                Err(tungstenite::Error::Io(e))
                                    if matches!(
                                        e.kind(),
                                        std::io::ErrorKind::TimedOut
                                            | std::io::ErrorKind::WouldBlock
                                    ) => {}
                                Err(_) => return Err(()),
                            }
                        }
                        let _ = ws.close(None);
                        Ok(())
                    })();
                    if attempt.is_err() {
                        // Bounded reconnect delay; shutdown is checked every 50ms.
                        for _ in 0..20 {
                            if stopping.load(Ordering::Relaxed) {
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(50));
                        }
                    }
                }
            })
        };
        Ok(Self {
            stop,
            wake,
            worker: Some(worker),
        })
    }
    pub fn wait(&self, stop: &AtomicBool, duration: Duration) {
        if duration.is_zero() || stop.load(Ordering::Relaxed) {
            return;
        }
        if let Ok(pending) = self.wake.0.lock() {
            if let Ok((mut pending, _)) = self
                .wake
                .1
                .wait_timeout_while(pending, duration, |p| !*p && !stop.load(Ordering::Relaxed))
            {
                *pending = false;
            }
        }
    }
}

impl Drop for NativeFeed {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.wake.1.notify_all();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_public_plaintext_credentials_and_invalid_subscriptions() {
        let keys = vec!["11111111111111111111111111111111".to_owned()];
        for url in [
            "ws://example.com",
            "wss://user:secret@example.com",
            "wss://example.com?token=secret",
            "http://127.0.0.1",
        ] {
            assert!(NativeFeed::connect(url.to_owned(), keys.clone()).is_err());
        }
        assert!(NativeFeed::connect("ws://127.0.0.1:1".to_owned(), vec![]).is_err());
        assert!(
            NativeFeed::connect("ws://127.0.0.1:1".to_owned(), vec![keys[0].clone(); 2]).is_err()
        );
        assert!(
            NativeFeed::connect("ws://127.0.0.1:1".to_owned(), vec!["bad".to_owned()]).is_err()
        );
    }
    #[test]
    fn disconnect_reconnect_wakes_authoritative_replay_and_shutdown_is_bounded() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_secs(5);
            for generation in 0..2 {
                let socket = loop {
                    match listener.accept() {
                        Ok((socket, _)) => break socket,
                        Err(e)
                            if e.kind() == std::io::ErrorKind::WouldBlock
                                && std::time::Instant::now() < deadline =>
                        {
                            std::thread::sleep(Duration::from_millis(10))
                        }
                        Err(e) => panic!("local subscription did not reconnect: {e}"),
                    }
                };
                socket.set_nonblocking(false).unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                socket.set_nodelay(true).unwrap();
                let mut ws = tungstenite::accept(socket).unwrap();
                ws.flush().unwrap();
                let Message::Text(t) = ws.read().unwrap() else {
                    panic!("missing subscription");
                };
                let v: serde_json::Value = serde_json::from_str(&t).unwrap();
                assert_eq!(v["method"], "accountSubscribe");
                if generation == 1 {
                    ws.send(Message::Text("{\"method\":\"accountNotification\",\"params\":{\"result\":\"untrusted payload\"}}".into())).unwrap();
                }
                // EOF is a hint to reconnect, never proof of a financial failure.
            }
        });
        let feed = NativeFeed::connect(
            format!("ws://{address}"),
            vec!["11111111111111111111111111111111".to_owned()],
        )
        .unwrap();
        let stop = AtomicBool::new(false);
        let started = std::time::Instant::now();
        feed.wait(&stop, Duration::from_secs(4));
        assert!(started.elapsed() < Duration::from_secs(4));
        let stopped = std::time::Instant::now();
        drop(feed);
        assert!(stopped.elapsed() < Duration::from_secs(2));
        server.join().unwrap();
    }
}
