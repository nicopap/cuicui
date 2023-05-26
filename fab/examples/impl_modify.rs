use cuicui_fab::{
    impl_modify,
    prefab::{Modify, Prefab},
};

struct Person {
    name: &'static str,
    surname: &'static str,
    age: usize,
    place_of_birth: Option<Box<City>>,
}
struct Street {
    no: usize,
    name: &'static str,
    people: usize,
}
struct Administration {
    chief: Person,
    secretary: Person,
}
struct City {
    streets: Vec<Street>,
    name: &'static str,
    admin: Administration,
}

struct CityPrefab;
impl Prefab for CityPrefab {
    type Item = City;
    type Modify = ModifyCity;
    type Items = Vec<City>;
}

/// Modify a [`City`].
#[impl_modify]
#[derive(Debug)]
impl Modify<City> for ModifyCity {
    type Context<'a> = ();

    #[modify(read(admin_name = .admin.chief.surname), read(.name), write(.admin.chief.age))]
    fn add_chief_age(additional_years: usize, admin_name: &str, name: &str) -> usize {
        admin_name.len() + name.len() + additional_years
    }

    #[modify(write_mut(.admin.secretary.name))]
    fn secretary_name(set_name: &'static str, name: &mut &'static str) {
        *name = set_name;
    }

    #[modify(write(.admin.secretary.age))]
    fn secretary_age(set_age: usize) -> usize {
        set_age
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
