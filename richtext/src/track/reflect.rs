use crate::show::{Show, ShowBox};

use super::{some_content, Target, Tracker};

pub(crate) fn make_tracker(name: String, target: &str, show: ShowBox) -> Option<Tracker> {
    let target = Target::statik(target.to_string())?;
    Some(Tracker {
        binding_name: name,
        fetch: Box::new(move |world| {
            let reflect = target.get(world)?;
            let show: &dyn Show = show.as_ref();
            some_content(show.display(reflect))
        }),
    })
}
