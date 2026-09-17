fn main() {
    let html = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    let rows = murating_core::player::parse_history(&html);
    println!("history rows: {}", rows.len());
    for r in rows.iter().take(40) {
        println!("{:?}", r);
    }
}
