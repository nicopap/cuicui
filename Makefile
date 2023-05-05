check:
	cargo clippy --examples --workspace --all-targets --all-features
run:
	# cargo run --package cuicui_richtext --example richtext
	cargo run --package cuicui_richtext --example breakout
	# cargo test --workspace --features winnow/debug
	# cargo test --workspace
