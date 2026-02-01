//! Binary to generate repr configuration for the target platform.
//! 
//! This tool uses `size_of` to detect actual VISA type sizes on the target platform
//! and generates configuration that can be used with the `custom-repr` feature.
//! 
//! Usage:
//! 1. Cross-compile this binary to the target platform
//! 2. Run it on the target to generate configuration
//! 3. Use the output to set environment variables or create config files
//! 
//! Example:
//! ```bash
//! # Cross-compile to Windows
//! cargo build --bin generate_repr_config --target x86_64-pc-windows-gnu
//! 
//! # Run on Windows target
//! ./target/x86_64-pc-windows-gnu/debug/generate_repr_config.exe --format shell > set_repr_vars.sh
//! 
//! # Or generate TOML config
//! ./target/x86_64-pc-windows-gnu/debug/generate_repr_config.exe --format toml > visa_repr_config.toml
//! ```

use std::env;
use std::mem::size_of;

// Import VISA types from visa-sys
use visa_sys as vs;

/// Map a size to the corresponding Rust repr type
fn size_to_unsigned_repr(size: usize) -> &'static str {
    match size {
        1 => "u8",
        2 => "u16",
        4 => "u32",
        8 => "u64",
        16 => "u128",
        _ => panic!("Unexpected type size: {}", size),
    }
}

/// Map a size to the corresponding Rust repr type for signed types
fn size_to_signed_repr(size: usize) -> &'static str {
    match size {
        1 => "i8",
        2 => "i16",
        4 => "i32",
        8 => "i64",
        16 => "i128",
        _ => panic!("Unexpected type size: {}", size),
    }
}

/// Detect repr for all VISA types
struct VisaReprConfig {
    vi_uint16: &'static str,
    vi_int16: &'static str,
    vi_uint32: &'static str,
    vi_int32: &'static str,
    vi_status: &'static str,
    vi_event: &'static str,
    vi_event_type: &'static str,
    vi_event_filter: &'static str,
    vi_attr: &'static str,
}

impl VisaReprConfig {
    fn detect() -> Self {
        Self {
            vi_uint16: size_to_unsigned_repr(size_of::<vs::ViUInt16>()),
            vi_int16: size_to_signed_repr(size_of::<vs::ViInt16>()),
            vi_uint32: size_to_unsigned_repr(size_of::<vs::ViUInt32>()),
            vi_int32: size_to_signed_repr(size_of::<vs::ViInt32>()),
            vi_status: size_to_signed_repr(size_of::<vs::ViStatus>()),
            vi_event: size_to_unsigned_repr(size_of::<vs::ViEvent>()),
            vi_event_type: size_to_unsigned_repr(size_of::<vs::ViEventType>()),
            vi_event_filter: size_to_unsigned_repr(size_of::<vs::ViEventFilter>()),
            vi_attr: size_to_unsigned_repr(size_of::<vs::ViAttr>()),
        }
    }

    /// Output as shell script for setting environment variables
    fn output_shell(&self) {
        use std::fmt::Write;
        let mut out = String::new();
        let _ = writeln!(out, "#!/bin/sh");
        let _ = writeln!(out, "# Generated VISA repr configuration for custom-repr feature");
        let _ = writeln!(out, "# Source this file or copy the exports to your environment");
        let _ = writeln!(out);
        let _ = writeln!(out, "export VISA_REPR_VIUINT16=\"{}\"", self.vi_uint16);
        let _ = writeln!(out, "export VISA_REPR_VIINT16=\"{}\"", self.vi_int16);
        let _ = writeln!(out, "export VISA_REPR_VIUINT32=\"{}\"", self.vi_uint32);
        let _ = writeln!(out, "export VISA_REPR_VIINT32=\"{}\"", self.vi_int32);
        let _ = writeln!(out, "export VISA_REPR_VISTATUS=\"{}\"", self.vi_status);
        let _ = writeln!(out, "export VISA_REPR_VIEVENT=\"{}\"", self.vi_event);
        let _ = writeln!(out, "export VISA_REPR_VIEVENTTYPE=\"{}\"", self.vi_event_type);
        let _ = writeln!(out, "export VISA_REPR_VIEVENTFILTER=\"{}\"", self.vi_event_filter);
        let _ = writeln!(out, "export VISA_REPR_VIATTR=\"{}\"", self.vi_attr);
        print!("{out}");
    }

    /// Output as Windows batch script
    fn output_batch(&self) {
        use std::fmt::Write;
        let mut out = String::new();
        let _ = writeln!(out, "@echo off");
        let _ = writeln!(out, "REM Generated VISA repr configuration for custom-repr feature");
        let _ = writeln!(out, "REM Run this file to set environment variables");
        let _ = writeln!(out);
        let _ = writeln!(out, "set VISA_REPR_VIUINT16={}", self.vi_uint16);
        let _ = writeln!(out, "set VISA_REPR_VIINT16={}", self.vi_int16);
        let _ = writeln!(out, "set VISA_REPR_VIUINT32={}", self.vi_uint32);
        let _ = writeln!(out, "set VISA_REPR_VIINT32={}", self.vi_int32);
        let _ = writeln!(out, "set VISA_REPR_VISTATUS={}", self.vi_status);
        let _ = writeln!(out, "set VISA_REPR_VIEVENT={}", self.vi_event);
        let _ = writeln!(out, "set VISA_REPR_VIEVENTTYPE={}", self.vi_event_type);
        let _ = writeln!(out, "set VISA_REPR_VIEVENTFILTER={}", self.vi_event_filter);
        let _ = writeln!(out, "set VISA_REPR_VIATTR={}", self.vi_attr);
        print!("{out}");
    }

    /// Output as TOML configuration file (visa_repr_config.toml)
    fn output_toml(&self) {
        use std::fmt::Write;
        let mut out = String::new();
        let _ = writeln!(out, "# Generated VISA repr configuration");
        let _ = writeln!(out, "# This can be used as a reference for visa_repr_config.toml");
        let _ = writeln!(out);
        let _ = writeln!(out, "[[platforms]]");
        let _ = writeln!(out, "condition = 'all()'");
        let _ = writeln!(out, "[platforms.types]");
        let _ = writeln!(out, "ViUInt16 = \"{}\"", self.vi_uint16);
        let _ = writeln!(out, "ViInt16 = \"{}\"", self.vi_int16);
        let _ = writeln!(out, "ViUInt32 = \"{}\"", self.vi_uint32);
        let _ = writeln!(out, "ViInt32 = \"{}\"", self.vi_int32);
        let _ = writeln!(out, "ViStatus = \"{}\"", self.vi_status);
        let _ = writeln!(out, "ViEvent = \"{}\"", self.vi_event);
        let _ = writeln!(out, "ViEventType = \"{}\"", self.vi_event_type);
        let _ = writeln!(out, "ViEventFilter = \"{}\"", self.vi_event_filter);
        let _ = writeln!(out, "ViAttr = \"{}\"", self.vi_attr);
        print!("{out}");
    }

    /// Output as Cargo config (for .cargo/config.toml)
    fn output_cargo_config(&self) {
        use std::fmt::Write;
        let mut out = String::new();
        let _ = writeln!(out, "# Generated VISA repr configuration for .cargo/config.toml");
        let _ = writeln!(out, "[env]");
        let _ = writeln!(out, "VISA_REPR_VIUINT16 = \"{}\"", self.vi_uint16);
        let _ = writeln!(out, "VISA_REPR_VIINT16 = \"{}\"", self.vi_int16);
        let _ = writeln!(out, "VISA_REPR_VIUINT32 = \"{}\"", self.vi_uint32);
        let _ = writeln!(out, "VISA_REPR_VIINT32 = \"{}\"", self.vi_int32);
        let _ = writeln!(out, "VISA_REPR_VISTATUS = \"{}\"", self.vi_status);
        let _ = writeln!(out, "VISA_REPR_VIEVENT = \"{}\"", self.vi_event);
        let _ = writeln!(out, "VISA_REPR_VIEVENTTYPE = \"{}\"", self.vi_event_type);
        let _ = writeln!(out, "VISA_REPR_VIEVENTFILTER = \"{}\"", self.vi_event_filter);
        let _ = writeln!(out, "VISA_REPR_VIATTR = \"{}\"", self.vi_attr);
        print!("{out}");
    }

    /// Output as JSON for programmatic use
    fn output_json(&self) {
        use std::fmt::Write;
        let mut out = String::new();
        let _ = writeln!(out, "{{");
        let _ = writeln!(out, "  \"ViUInt16\": \"{}\",", self.vi_uint16);
        let _ = writeln!(out, "  \"ViInt16\": \"{}\",", self.vi_int16);
        let _ = writeln!(out, "  \"ViUInt32\": \"{}\",", self.vi_uint32);
        let _ = writeln!(out, "  \"ViInt32\": \"{}\",", self.vi_int32);
        let _ = writeln!(out, "  \"ViStatus\": \"{}\",", self.vi_status);
        let _ = writeln!(out, "  \"ViEvent\": \"{}\",", self.vi_event);
        let _ = writeln!(out, "  \"ViEventType\": \"{}\",", self.vi_event_type);
        let _ = writeln!(out, "  \"ViEventFilter\": \"{}\",", self.vi_event_filter);
        let _ = writeln!(out, "  \"ViAttr\": \"{}\"", self.vi_attr);
        let _ = writeln!(out, "}}");
        print!("{out}");
    }
}

fn print_usage() {
    eprintln!("Usage: generate_repr_config [OPTIONS]");
    eprintln!();
    eprintln!("Generate VISA repr configuration for the target platform.");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --format <FORMAT>   Output format: shell, batch, toml, json (default: shell)");
    eprintln!("  --help              Show this help message");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  generate_repr_config --format shell > set_vars.sh");
    eprintln!("  generate_repr_config --format batch > set_vars.bat");
    eprintln!("  generate_repr_config --format toml > detected_config.toml");
    eprintln!("  generate_repr_config --format cargo-config > .cargo/config.toml");
    eprintln!("  generate_repr_config --format json");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    let mut format = "shell";
    let mut i = 1;
    
    while i < args.len() {
        match args[i].as_str() {
            "--format" => {
                if i + 1 < args.len() {
                    format = &args[i + 1];
                    i += 2;
                } else {
                    eprintln!("Error: --format requires an argument");
                    print_usage();
                    std::process::exit(1);
                }
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            _ => {
                eprintln!("Error: Unknown argument: {}", args[i]);
                print_usage();
                std::process::exit(1);
            }
        }
    }
    
    let config = VisaReprConfig::detect();
    
    match format {
        "shell" | "sh" => config.output_shell(),
        "batch" | "bat" | "cmd" => config.output_batch(),
        "toml" => config.output_toml(),
        "cargo-config" | "cargo" | "config" => config.output_cargo_config(),
        "json" => config.output_json(),
        _ => {
            eprintln!("Error: Unknown format: {}", format);
            eprintln!("Supported formats: shell, batch, toml, cargo-config, json");
            std::process::exit(1);
        }
    }
}
