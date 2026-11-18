//! Reruns on any change under `tests/fixtures`, which the `#[files]`
//! case lists expand over at compile time.

fn main() {
    println!("cargo::rerun-if-changed=tests/fixtures");
}
