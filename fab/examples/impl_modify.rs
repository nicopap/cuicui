use cuicui_fab::{impl_modify, modify::Modify};
use pretty_assertions::assert_eq;

pub struct MyContext {
    admin: usize,
    closest_city: &'static str,
}

#[derive(Clone, Debug)]
pub struct Person {
    name: &'static str,
    surname: &'static str,
    age: usize,
}
#[derive(Debug, Clone)]
struct Street {
    no: usize,
    name: &'static str,
    people: usize,
}
#[derive(Debug, Clone)]
struct Administration {
    mayor: Person,
    secretary: Person,
    lawyer: Person,
}
#[derive(Debug, Clone)]
pub struct City {
    streets: Vec<Street>,
    name: &'static str,
    admin: Administration,
}

/// Modify a [`City`].
#[impl_modify]
impl Modify for ModifyCity {
    type Context<'a> = MyContext;
    type Item = City;
    type Items = Vec<City>;

    /// This adds to the mayor's age to that of `additional_year` plus the city's name
    #[modify(read(admin_name = .admin.mayor.surname), read(.name), write(.admin.mayor.age))]
    fn add_mayor_age(additional_years: usize, admin_name: &str, name: &str) -> usize {
        admin_name.len() + name.len() + additional_years
    }

    /// This sets the name of the secretary to `set_name`
    #[modify(write_mut(.admin.secretary.name))]
    pub fn secretary_name(set_name: &'static str, name: &mut &'static str) {
        *name = set_name;
    }

    #[modify(write(.admin.secretary.age))]
    pub fn secretary_age(set_age: usize) -> usize {
        set_age
    }

    #[modify(write(.admin.lawyer))]
    pub fn lawyer(set_lawyer: &Person) -> Person {
        set_lawyer.clone()
    }

    #[modify(write(.admin.secretary))]
    pub fn set_secretary(to_set: &Person) -> Person {
        to_set.clone()
    }

    #[modify(context(ctx), write(.name))]
    pub fn read_context(ctx: &MyContext) -> &'static str {
        ctx.closest_city
    }

    /// Always set name of the third street to that of the city secretary.
    #[modify(read(.admin.secretary.name), write(street = .streets[3].name))]
    fn name_street_after_secretary(name: &'static str) -> &'static str {
        name
    }

    #[modify(read_write(.admin.mayor.age))]
    fn make_mayor_older(by_age: usize, age: &mut usize) {
        *age += by_age;
    }

    #[modify(dynamic_read_write(reewd, uwurites))]
    fn arbitrary_changes(inner: usize, item: &mut City) {
        item.streets[inner].no = inner;
    }
}

fn main() {
    let secretary_fields = ModifyCity::set_secretary_changes();
    let age_fields = ModifyCity::secretary_age_changes();
    let name_fields = ModifyCity::name_street_after_secretary_depends();
    assert_eq!(age_fields | name_fields, secretary_fields);
}
