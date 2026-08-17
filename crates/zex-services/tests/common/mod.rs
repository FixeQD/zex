//! Shared helpers for zex-services integration tests

use std::io::BufRead;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// An isolated D-Bus daemon used to run real bus interactions in tests
pub struct TestBus {
    child: Child,
    pub conn: zbus::Connection,
}

impl TestBus {
    /// Spawn a private `dbus-daemon` and connect to it
    pub async fn spawn() -> Option<Self> {
        let mut child = Command::new("dbus-daemon")
            .args(["--session", "--print-address=1", "--nofork"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let mut stdout = child.stdout.take()?;
        let (tx, rx) = flume::bounded::<String>(1);
        std::thread::spawn(move || {
            let mut line = String::new();
            if std::io::BufReader::new(&mut stdout)
                .read_line(&mut line)
                .is_ok()
            {
                let _ = tx.send(line);
            }
        });
        let address = match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(address) => address.trim().to_string(),
            Err(_) => {
                let _ = child.kill();
                return None;
            }
        };
        let builder = match zbus::connection::Builder::address(address.as_str()) {
            Ok(builder) => builder,
            Err(_) => {
                let _ = child.kill();
                return None;
            }
        };
        let conn = match builder.build().await {
            Ok(conn) => conn,
            Err(_) => {
                let _ = child.kill();
                return None;
            }
        };
        Some(Self { child, conn })
    }
}

impl Drop for TestBus {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}
