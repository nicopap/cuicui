use crate::show::{Show, ShowBox};

use super::{some_content, Target, Tracker};

pub(crate) fn make_tracker(name: &'static str, target: Target<'static>, show: ShowBox) -> Tracker {
    Tracker {
        binding_name: name,
        fetch: Box::new(move |world| {
            let reflect = target.get(world)?;
            let show: &dyn Show = show.as_ref();
            some_content(show.display(reflect))
        }),
    }
}
