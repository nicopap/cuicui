//! Declare from format string what resource and components to read

use bevy::reflect::TypeRegistryInternal as TypeRegistry;

struct Path<'a>(Vec<&'a str>);
impl<'a> Path<'a> {
    fn parse(input: &'a str) -> Self {
        Path(input.split('.').collect())
    }
    fn resource(&self, registry: &TypeRegistry, world: &World) -> 
}
