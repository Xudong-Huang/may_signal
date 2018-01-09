extern crate may_signal;

fn main() {
    let s = may_signal::ctrl_c();
    for _ in s.iter().take(3) {
        println!("CTRL_C pressed!");
    }
}
