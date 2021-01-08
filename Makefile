all: test

test:
	cargo fmt
	cargo test

run:
	cargo fmt
	cargo run
