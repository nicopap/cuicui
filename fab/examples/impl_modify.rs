use cuicui_fab::{impl_modify, modify::Modify};

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
    chief: Person,
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
    type Context<'a> = ();
    type Item = City;
    type Items = Vec<City>;

    /// This adds to the chief's age to that of `additional_year` plus the city's name
    #[modify(read(admin_name = .admin.chief.surname), read(.name), write(.admin.chief.age))]
    fn add_chief_age(additional_years: usize, admin_name: &str, name: &str) -> usize {
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

    /// Always set name of the first street to that of the city secretary.
    ///
    /// If no street exists, adds one.
    #[modify(read(.admin.secretary.name), read_write(.streets))]
    fn name_street_after_secretary(name: &'static str, streets: &mut Vec<Street>) {
        if let Some(street) = streets.first_mut() {
            street.name = name;
        } else {
            streets.push(Street { no: 0, name, people: 1 })
        }
    }

    #[modify(dynamic_read_write(reads, writes))]
    fn arbitrary_changes(inner: usize, item: &mut City) {
        item.streets[inner].no = inner;
    }
}

fn main() {
    todo!()
}
