use visa_rs::prelude::*;

fn main() {
    println!("Custom repr config example");
    println!("This example demonstrates using a custom visa_repr_config.toml file");
    println!("to override the default platform-specific type mappings.");
    println!();
    println!("The custom config in this example forces all types to 32-bit");
    println!("representations regardless of the target platform.");
    println!();
    println!("Build with: cargo build --manifest-path examples/custom_repr/Cargo.toml");

    // Try to create a resource manager to verify types work correctly
    match DefaultRM::new() {
        Ok(_rm) => {
            println!("Successfully created DefaultRM");
            println!("Resource manager created - custom repr config is working!");
        }
        Err(e) => {
            println!(
                "Note: Could not create DefaultRM (VISA library not installed): {:?}",
                e
            );
            println!("This is expected if VISA is not installed, but the build succeeded!");
            println!("The important part is that the code compiled with custom repr config.");
        }
    }
}
