use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::sync::OnceLock;
use std::time::Duration;

use tauri::AppHandle;

#[cfg(not(feature = "demo"))]
const PORT: u16 = 47863;
#[cfg(feature = "demo")]
const PORT: u16 = 47864;

static LISTENER: OnceLock<TcpListener> = OnceLock::new();

pub fn claim() -> bool {
    let deadline = if std::env::args().any(|a| a == crate::update::UPDATED_FLAG) {
        Some(std::time::Instant::now() + HANDOVER)
    } else {
        None
    };
    loop {
        match TcpListener::bind(("127.0.0.1", PORT)) {
            Ok(l) => {
                let _ = LISTENER.set(l);
                return true;
            }
            Err(_) => {
                if deadline.is_some_and(|d| std::time::Instant::now() < d) {
                    std::thread::sleep(Duration::from_millis(150));
                    continue;
                }
                let _ = TcpStream::connect_timeout(&([127, 0, 0, 1], PORT).into(), Duration::from_millis(500));
                return false;
            }
        }
    }
}

const HANDOVER: Duration = Duration::from_secs(15);

pub fn serve(app: &AppHandle) {
    let Some(listener) = LISTENER.get() else { return };
    let Ok(listener) = listener.try_clone() else { return };
    let app = app.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut stream = stream;
            let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
            let _ = stream.read(&mut [0u8; 8]);
            crate::tray::show_main(&app);
        }
    });
}
