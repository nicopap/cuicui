check:
	cargo clippy --examples
run:
	# cargo run --example richtext
	# cargo test richtext::parse::tests::balanced_text_invalid
	cargo test
