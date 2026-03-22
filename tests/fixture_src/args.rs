fn main() {
    let args: Vec<String> = std::env::args().collect();
    for arg in &args[1..] {
        println!("{arg}");
    }
}
