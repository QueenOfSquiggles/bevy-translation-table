# Bevy Translation Table

This crate allows simple translation to be done within [Bevy Engine](https://github.com/bevyengine/bevy) using a global `Resource` (`Translations`). Support for different file formats is entirely optional allowing stripping unneeded implementations and dependencies.

## Design Goals

The goal of this plugin was to allow loading string translation data from table style files such as CSV and Open Document Spreadsheet (ODS) files. Additionally supporting injection of raw key-value data for more customization if needed.

The features of this crate are to be:
- simple to use
- globally accessible from bevy systems
- support easily edited and open document types to allow ease of translation for any team members or third party translators without much technical experience.

### Non-Goals
- Automatic translation using external tooling
- Authoring systems for generating translation tables
- Anything more than simple key-value mapping based on table columns.

## Quick Start

Code from `examples/minimal.rs`

**Initialize Resource**
```rust
// Here you can replace 'world' with 'app' if using bevy instead of just bevy-ecs
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
```

**Poll Translated String(s)**
```rust
// A system that uses a read-only reference to the translations table.
// This allows better parallelization of translation lookups when not needing to change the tables
fn system_use_translation(trans: Res<Translations>) {
    for key in ["hello", "green", "invalid"] {
        // using Res<Translations>.tr('key') to perform the key-value lookup.
        // Without the feature `catch-missing-values` enabled, this will simply provide the key again when failing to find a matching value for the current locale.
        println!("{} = {}", key, trans.tr(key))
    }
}
```

# License

Following the precedent set by [bevy itself](https://github.com/bevyengine/bevy?tab=readme-ov-file#license), this crate is dual licensed under either MIT or Apache-2.0