check:
	cargo clippy --examples
run:
	cargo run --example richtext
	cargo run --example breakout
	# cargo test --features winnow/debug
	cargo test
