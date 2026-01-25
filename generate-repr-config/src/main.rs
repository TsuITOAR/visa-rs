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
//! ./target/x86_64-pc-windows-gnu/debug/generate_repr_config.exe --format toml > repr_config.toml
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
        println!("#!/bin/sh");
        println!("# Generated VISA repr configuration for custom-repr feature");
        println!("# Source this file or copy the exports to your environment");
        println!();
        println!("export VISA_REPR_VIUINT16=\"{}\"", self.vi_uint16);
        println!("export VISA_REPR_VIINT16=\"{}\"", self.vi_int16);
        println!("export VISA_REPR_VIUINT32=\"{}\"", self.vi_uint32);
        println!("export VISA_REPR_VIINT32=\"{}\"", self.vi_int32);
        println!("export VISA_REPR_VISTATUS=\"{}\"", self.vi_status);
        println!("export VISA_REPR_VIEVENT=\"{}\"", self.vi_event);
        println!("export VISA_REPR_VIEVENTTYPE=\"{}\"", self.vi_event_type);
        println!("export VISA_REPR_VIEVENTFILTER=\"{}\"", self.vi_event_filter);
        println!("export VISA_REPR_VIATTR=\"{}\"", self.vi_attr);
    }

    /// Output as Windows batch script
    fn output_batch(&self) {
        println!("@echo off");
        println!("REM Generated VISA repr configuration for custom-repr feature");
        println!("REM Run this file to set environment variables");
        println!();
        println!("set VISA_REPR_VIUINT16={}", self.vi_uint16);
        println!("set VISA_REPR_VIINT16={}", self.vi_int16);
        println!("set VISA_REPR_VIUINT32={}", self.vi_uint32);
        println!("set VISA_REPR_VIINT32={}", self.vi_int32);
        println!("set VISA_REPR_VISTATUS={}", self.vi_status);
        println!("set VISA_REPR_VIEVENT={}", self.vi_event);
        println!("set VISA_REPR_VIEVENTTYPE={}", self.vi_event_type);
        println!("set VISA_REPR_VIEVENTFILTER={}", self.vi_event_filter);
        println!("set VISA_REPR_VIATTR={}", self.vi_attr);
    }

    /// Output as TOML configuration file
    fn output_toml(&self) {
        println!("# Generated VISA repr configuration");
        println!("# This can be used as a reference for repr_config.toml");
        println!();
        println!("[invariant]");
        println!("ViUInt16 = \"{}\"", self.vi_uint16);
        println!("ViInt16 = \"{}\"", self.vi_int16);
        println!("ViUInt32 = \"{}\"", self.vi_uint32);
        println!("ViInt32 = \"{}\"", self.vi_int32);
        println!("ViStatus = \"{}\"", self.vi_status);
        println!("ViEvent = \"{}\"", self.vi_event);
        println!("ViEventType = \"{}\"", self.vi_event_type);
        println!("ViEventFilter = \"{}\"", self.vi_event_filter);
        println!("ViAttr = \"{}\"", self.vi_attr);
    }

    /// Output as JSON for programmatic use
    fn output_json(&self) {
        println!("{{");
        println!("  \"ViUInt16\": \"{}\",", self.vi_uint16);
        println!("  \"ViInt16\": \"{}\",", self.vi_int16);
        println!("  \"ViUInt32\": \"{}\",", self.vi_uint32);
        println!("  \"ViInt32\": \"{}\",", self.vi_int32);
        println!("  \"ViStatus\": \"{}\",", self.vi_status);
        println!("  \"ViEvent\": \"{}\",", self.vi_event);
        println!("  \"ViEventType\": \"{}\",", self.vi_event_type);
        println!("  \"ViEventFilter\": \"{}\",", self.vi_event_filter);
        println!("  \"ViAttr\": \"{}\"", self.vi_attr);
        println!("}}");
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
        "json" => config.output_json(),
        _ => {
            eprintln!("Error: Unknown format: {}", format);
            eprintln!("Supported formats: shell, batch, toml, json");
            std::process::exit(1);
        }
    }
}
