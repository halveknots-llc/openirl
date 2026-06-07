cargo fmt --all
cargo test --workspace
cargo run --package openirl-agent -- serve --bind 127.0.0.1:7707
