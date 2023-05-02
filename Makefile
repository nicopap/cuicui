check:
	cargo clippy --examples --workspace --all-targets --all-features
run:
	# cargo run --example richtext
	# cargo run --example breakout
	# cargo test --features winnow/debug
	cargo test --workspace
