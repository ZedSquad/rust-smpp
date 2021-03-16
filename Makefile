all: test

test:
	cargo fmt
	cargo test


test-with-stdout:
	cargo fmt
	cargo test -- --nocapture

watch:
	# cargo install cargo-watch
	cargo watch -s 'clear; cargo check --tests --color=always 2>&1 | head -40'

run:
	cargo fmt
	cargo run
