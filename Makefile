all: run

run:
	cargo fmt
	RUST_LOG=DEBUG cargo run
