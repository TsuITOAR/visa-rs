use std::env;
use std::path::PathBuf;

fn main() {
    let custom_repr_enabled = env::var_os("CARGO_FEATURE_CUSTOM_REPR").is_some();
    if custom_repr_enabled {
        println!("cargo:rerun-if-env-changed=VISA_REPR_CONFIG_PATH");
        for var in [
            "VISA_REPR_VIUINT16",
            "VISA_REPR_VIINT16",
            "VISA_REPR_VIUINT32",
            "VISA_REPR_VIEVENT",
            "VISA_REPR_VIEVENTTYPE",
            "VISA_REPR_VIEVENTFILTER",
            "VISA_REPR_VIATTR",
            "VISA_REPR_VISTATUS",
            "VISA_REPR_VIINT32",
        ] {
            println!("cargo:rerun-if-env-changed={}", var);
        }
        if let Ok(path) = env::var("VISA_REPR_CONFIG_PATH") {
            if !path.trim().is_empty() {
                let path_buf = PathBuf::from(&path);
                if !path_buf.is_absolute() {
                    println!(
                        "cargo:warning=VISA_REPR_CONFIG_PATH must be an absolute path: {}",
                        path
                    );
                    std::process::exit(1);
                } else if !path_buf.exists() {
                    println!(
                        "cargo:warning=VISA_REPR_CONFIG_PATH does not exist: {}",
                        path
                    );
                    std::process::exit(1);
                }
                println!("cargo:rerun-if-changed={}", path);
            }
        }
    }
}
