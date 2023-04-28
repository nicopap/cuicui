check:
	cargo clippy --examples
run:
	# cargo run --example richtext
	# cargo test
	cargo test --features winnow/debug richtext::parse::tests::balanced_text_complete
	cargo test --features winnow/debug richtext::parse::tests::balanced_text_incomplete
	cargo test --features winnow/debug richtext::parse::tests::closed_element_complete
	cargo test --features winnow/debug richtext::parse::tests::closed_element_incomplete
	cargo test --features winnow/debug richtext::parse::tests::bare_content_complete
	cargo test --features winnow/debug richtext::parse::tests::closed_complete
	cargo test --features winnow/debug richtext::parse::tests::closed_incomplete
	cargo test --features winnow/debug richtext::parse::tests::single_dynamic_shorthand
	cargo test --features winnow/debug richtext::parse::tests::plain_text
	cargo test --features winnow/debug richtext::parse::tests::closed_content
	cargo test --features winnow/debug richtext::parse::tests::closed_explicit_content_escape_comma
	cargo test --features winnow/debug richtext::parse::tests::outer_dynamic_shorthand
	cargo test --features winnow/debug richtext::parse::tests::outer_dynamic_content_implicit
	cargo test --features winnow/debug richtext::parse::tests::dynamic_content_implicit
	cargo test --features winnow/debug richtext::parse::tests::outer_color_mod
	cargo test --features winnow/debug richtext::parse::tests::nested_dynamic_shorthand
	cargo test --features winnow/debug richtext::parse::tests::deep_nesting
	cargo test --features winnow/debug richtext::parse::tests::multiple_mods
	cargo test --features winnow/debug richtext::parse::tests::fancy_color_multiple_mods
	cargo test --features winnow/debug richtext::parse::tests::escape_curlies_outer
	cargo test --features winnow/debug richtext::parse::tests::escape_curlies_inner
	cargo test --features winnow/debug richtext::parse::tests::named_dynamic_mod
	cargo test --features winnow/debug richtext::parse::tests::implicit_dynamic_mod
	cargo test --features winnow/debug richtext::parse::tests::implicit_dynamic_content_mod
	cargo test --features winnow/debug richtext::parse::tests::all_dynamic_content_declarations
