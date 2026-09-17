pub const AUTOSTART_FLAG: &str = "--autostart";

pub fn is_autostart() -> bool {
    std::env::args().any(|a| a == AUTOSTART_FLAG)
}
