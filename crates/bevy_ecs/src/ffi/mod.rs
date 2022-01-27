#[macro_use]
mod macros;

use std::{alloc::Layout, ffi::CStr, os::raw::c_char};

use crate::{
    component::{ComponentDescriptor, ComponentId, StorageType},
    prelude::*,
};

ffi_fn! {
    fn bevy_ecs_world_new() -> *mut World {
        Box::into_raw(Box::new(World::new()))
    } ?= std::ptr::null_mut()
}

ffi_fn! {
    fn bevy_ecs_debug_world(world: &World) {
        println!("{:?}", world);
        println!("{:#?}", world.components());
    }
}

ffi_fn! {
    fn bevy_ecs_world_free(world: *mut World) {
        drop(unsafe { Box::from_raw(world) });
    }
}

ffi_fn! {
    fn bevy_ecs_register_component(
        world: &mut World,
        name: *const c_char,
        storage_type: StorageType,
        is_send_and_sync: bool,
        size: usize,
        alignment: usize,
    ) -> ComponentId {
        let name = unsafe { CStr::from_ptr(name.cast()) };
        let name = name.to_str().unwrap().to_string();

        let descriptor = unsafe {
            ComponentDescriptor::new_dynamic(
                name,
                storage_type,
                is_send_and_sync,
                None,
                Layout::from_size_align_unchecked(size, alignment),
                |_: *mut u8| {},
            )
        };
        world.init_component_dynamic(descriptor)
    }
}

ffi_fn! {
    fn bevy_ecs_spawn_entity(world: &mut World) -> Entity {
        world.spawn().id()
    }
}

ffi_fn! {
    fn bevy_ecs_get(world: &mut World, entity:Entity, component_id: ComponentId) -> *const std::ffi::c_void {
        let entity = match world.get_entity(entity) {
            Some(entity) => entity,
            None => return std::ptr::null(),
        };

        entity.get_dynamic(component_id).unwrap_or_else(std::ptr::null).cast()
    }
}

ffi_fn! {
    fn bevy_ecs_get_mut(world: &mut World, entity: Entity, component_id: ComponentId) -> *mut std::ffi::c_void  {
        let mut entity = match world.get_entity_mut(entity) {
            Some(entity) => entity,
            None => return std::ptr::null_mut(),
        };

        entity.get_mut_dynamic(component_id).unwrap_or_else(std::ptr::null_mut).cast()
    }
}

ffi_fn! {
    fn bevy_ecs_world_get_component_id_by_name(world: &World, name: *const c_char) -> ComponentId {
        let name = unsafe { CStr::from_ptr(name) };
        let name = match name.to_str() {
            Ok(name) => name,
            Err(_) => return ComponentId::INVALID,
        };

        world
            .components()
            .get_id_by_name(name)
            .unwrap_or(ComponentId::INVALID)
    } ?= ComponentId::INVALID
}
