check:
	cargo clippy --examples
run:
	# cargo run --example richtext
	# cargo test --features winnow/debug
	# cargo test
	cargo test richtext::parse::tests::balanced_text_complete
	cargo test richtext::parse::tests::closed_element_complete
	cargo test --features winnow/debug richtext::parse::tests::closed_complete
	cargo test richtext::parse::tests::bare_content_complete
	cargo test richtext::parse::tests::balanced_text_incomplete
	cargo test richtext::parse::tests::closed_element_incomplete
	cargo test richtext::parse::tests::closed_incomplete
	cargo test richtext::parse::tests::single_dynamic_shorthand
	cargo test richtext::parse::tests::plain_text
	cargo test richtext::parse::tests::closed_content
	cargo test --features winnow/debug richtext::parse::tests::single_no_escape_closed_content
	cargo test --features winnow/debug richtext::parse::tests::single_closed_content
	cargo test --features winnow/debug richtext::parse::tests::closed_explicit_content_escape_comma
	cargo test richtext::parse::tests::outer_dynamic_shorthand
	cargo test richtext::parse::tests::outer_dynamic_content_implicit
	cargo test richtext::parse::tests::dynamic_content_implicit
	cargo test richtext::parse::tests::outer_color_mod
	cargo test richtext::parse::tests::nested_dynamic_shorthand
	cargo test richtext::parse::tests::deep_nesting
	cargo test richtext::parse::tests::multiple_mods
	cargo test richtext::parse::tests::fancy_color_multiple_mods
	cargo test richtext::parse::tests::escape_curlies_outer
	cargo test richtext::parse::tests::escape_curlies_inner
	cargo test richtext::parse::tests::named_dynamic_mod
	cargo test richtext::parse::tests::implicit_dynamic_mod
	cargo test richtext::parse::tests::implicit_dynamic_content_mod
	cargo test richtext::parse::tests::all_dynamic_content_declarations
