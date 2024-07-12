extern crate bevy_translation_table;
use std::path::Path;

use bevy_ecs::prelude::*;
use bevy_translation_table::*;

/// A minimal usage example built with bevy_ecs.
/// refer to the [bevy_ecs example](https://docs.rs/bevy_ecs/latest/bevy_ecs/#using-bevy-ecs) to better understand how those systems are working.
fn main() {
    let mut world = World::new();
    world.insert_resource(
        // initialize as default
        Translations::default()
            // select a CSV file and a default locale
            .csv_file(&Path::new("assets/lang.csv"), &"en".into())
            // optionally switch the current locale
            .use_locale("es")
            // Strips mutability to easily finish inserting into the world.
            .build(),
    );

    let mut schedule = Schedule::default();
    schedule.add_systems(system_use_translation);

    schedule.run(&mut world);
}

// A system that uses a read-only reference to the translations table.
// This allows better parallelization of translation lookups when not needing to change the tables
fn system_use_translation(trans: Res<Translations>) {
    for key in ["hello", "green", "invalid"] {
        // using Res<Translations>.tr('key') to perform the key-value lookup.
        // Without the feature `catch-missing-values` enabled, this will simply provide the key again when failing to find a matching value for the current locale.
        println!("{} = {}", key, trans.tr(key))
    }
}
