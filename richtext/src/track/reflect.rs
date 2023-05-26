use crate::show::{Show, ShowBox};

use super::{some_content, Target, Tracker};

pub(crate) fn make_tracker(name: String, target: &str, show: ShowBox) -> Option<Tracker> {
    let target = Target::statik(target)?;
    Some(Tracker {
        binding_name: name,
        fetch: Box::new(move |cache, world| {
            let reflect = target.get(cache, world)?;
            let show: &dyn Show = show.as_ref();
            some_content(show.display(reflect))
        }),
    })
}
