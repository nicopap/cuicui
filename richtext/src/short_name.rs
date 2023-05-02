use std::str::{self, Utf8Error};

/// `std::slice::get`, but runs in const contexts on stable.
///
/// # Panics
///
/// If `at >= slice.len()`
const fn slice_tail_at<T>(at: usize, slice: &[T]) -> &[T] {
    if at == 0 {
        slice
    } else {
        let Some(tail) = slice.split_first() else {
            panic!("slice_tail_at requests a size larger than provided slice")
        };
        slice_tail_at(at - 1, tail.1)
    }
}

/// `Result::unwrap`, but valid in `const` context on stable.
const fn unwrap_utf(result: Result<&'static str, Utf8Error>) -> &'static str {
    match result {
        Ok(v) => v,
        Err(_) => panic!(
            "Somehow managed to find invalid UTF in short_name. probs have some non-ASCII \
            character with a trailing byte equal to `:`"
        ),
    }
}
/// A hackish const rust type shortener.
///
/// Unlike bevy's `get_short_name`, this only removes the path prefix of the
/// type name. It doesn't handle at all generic parameters, it returns `None`
/// in this case.
pub(crate) const fn short_name(full_name: &'static str) -> &'static str {
    let full_name = full_name.as_bytes();
    let mut index = full_name.len() - 1;

    loop {
        match full_name[index] {
            b'>' => panic!("short_name doesn't handle generic types"),
            b':' => return unwrap_utf(str::from_utf8(slice_tail_at(index + 1, full_name))),
            _ => index = index.wrapping_sub(1),
        }
        if index == usize::MAX {
            return unwrap_utf(str::from_utf8(full_name));
        }
    }
}
