use bevy_ecs::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new();

    unsafe {
        let lib = libloading::Library::new("../build/libbasic.so")?;
        let setup: libloading::Symbol<unsafe extern "C" fn(&mut World)> = lib.get(b"setup")?;
        setup(&mut world);
    }

    println!("{:#?}", world.components());

    Ok(())
}
