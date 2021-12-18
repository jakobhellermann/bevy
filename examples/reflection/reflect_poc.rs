use bevy::prelude::*;
use bevy::reflect::{GetTypeRegistration, TypeRegistration};

#[derive(Reflect, Component, Default)]
struct A {}

fn main() {
    let registration: TypeRegistration = A::get_type_registration();
    assert!(registration.data::<ReflectComponent>().is_some());
}
