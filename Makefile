all: test

test:
	cargo fmt
	cargo test


test-with-stdout:
	cargo fmt
	cargo test -- --nocapture

run:
	cargo fmt
	cargo run
