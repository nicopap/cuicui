//! Declare from format string what resource and components to read

struct Path<'a>(Vec<&'a str>);
impl<'a> Path<'a> {
    fn parse(input: &'a str) -> Self {
        Path(input.split('.').collect())
    }
}
