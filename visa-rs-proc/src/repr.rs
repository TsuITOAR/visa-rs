use proc_macro2::{Delimiter, Ident, Span, TokenStream as TokenStream2, TokenTree};
use quote::{quote_spanned, ToTokens};
use syn::{parse::Parse, Path, Result, Token};

use crate::rusty_ident::NestedMacros;

// Configuration structures for parsing the TOML config file (only when cross-compile feature is enabled)
// Note: When both cross-compile and custom-repr features are enabled, custom-repr takes precedence
// and this code is not used (hence the allow(dead_code) attributes).
#[cfg(feature = "cross-compile")]
mod config {
    use serde::Deserialize;
    use std::collections::HashMap;

    #[allow(dead_code)]
    #[derive(Debug, Deserialize)]
    pub struct ReprConfig {
        pub invariant: HashMap<String, String>,
        pub platform_dependent: PlatformDependent,
    }

    #[allow(dead_code)]
    #[derive(Debug, Deserialize)]
    pub struct PlatformDependent {
        pub unsigned: TypeMappings,
        pub signed: TypeMappings,
    }

    #[allow(dead_code)]
    #[derive(Debug, Deserialize)]
    pub struct TypeMappings {
        pub types: Vec<String>,
        pub mappings: Vec<Mapping>,
    }

    #[allow(dead_code)]
    #[derive(Debug, Deserialize)]
    pub struct Mapping {
        pub condition: String,
        pub repr: String,
    }

    // Load configuration at compile time
    #[allow(dead_code)]
    pub fn load_config() -> ReprConfig {
        // Try to load from current directory first (allows user customization)
        // Fall back to bundled config if not found
        let config_str = if let Ok(current_dir) = std::env::var("CARGO_MANIFEST_DIR") {
            let custom_path = std::path::Path::new(&current_dir).join("visa_repr_config.toml");
            if custom_path.exists() {
                std::fs::read_to_string(&custom_path)
                    .unwrap_or_else(|e| {
                        panic!("Failed to read custom config file {:?}: {}", custom_path, e)
                    })
            } else {
                // Use bundled config
                include_str!("../repr_config.toml").to_string()
            }
        } else {
            // Use bundled config
            include_str!("../repr_config.toml").to_string()
        };
        
        toml::from_str(&config_str).unwrap_or_else(|e| {
            panic!("Failed to parse repr config: {}", e)
        })
    }

    // Lazy static config loaded once
    #[allow(dead_code)]
    static CONFIG: std::sync::OnceLock<ReprConfig> = std::sync::OnceLock::new();

    #[allow(dead_code)]
    pub fn get_config() -> &'static ReprConfig {
        CONFIG.get_or_init(load_config)
    }
}

pub struct Input {
    macs: Option<Vec<Path>>,
    exc: Option<Token![!]>,
    inner: AttrProcessed,
}

impl Parse for Input {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let fork = input.fork();
        if fork.parse::<NestedMacros>().is_ok() {
            let nest_macros: NestedMacros = input.parse()?;
            Ok(Self {
                macs: nest_macros.macs.into(),
                exc: nest_macros.exc.into(),
                inner: syn::parse2(nest_macros.body.content)?,
            })
        } else {
            Ok(Self {
                macs: None,
                exc: None,
                inner: input.parse()?,
            })
        }
    }
}

impl ToTokens for Input {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self { macs, exc, inner } = self;
        match (macs, exc) {
            (Some(macs), Some(exc)) => {
                let mut ret = inner.to_token_stream();
                for mac in macs.iter().rev() {
                    let mut m = mac.to_token_stream();
                    exc.to_tokens(&mut m);
                    proc_macro2::Group::new(Delimiter::Brace, ret).to_tokens(&mut m);
                    ret = m;
                }
                ret.to_tokens(tokens)
            }
            _ => inner.to_tokens(tokens),
        }
    }
}

pub struct AttrProcessed(TokenStream2);

impl ToTokens for AttrProcessed {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        self.0.to_tokens(tokens)
    }
}

impl Parse for AttrProcessed {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let mut inner = TokenStream2::new();
        while !input.is_empty() {
            if let Some(ty) = extract_repr_attribute(input, &mut inner)? {
                map_to_repr(ty).to_tokens(&mut inner);
            } else {
                input.parse::<TokenTree>()?.to_tokens(&mut inner);
            }
        }
        Ok(Self(inner))
    }
}

fn extract_repr_attribute(
    input: syn::parse::ParseStream,
    tokens: &mut TokenStream2,
) -> Result<Option<Ident>> {
    let fork = input.fork();
    use syn::parse::discouraged::Speculative;
    let mut ret = None;
    if fork.peek(Token![#]) {
        if let Ok(attr) = fork.call(syn::Attribute::parse_outer) {
            let mut iter = attr.into_iter();
            for a in &mut iter {
                if a.path.is_ident("repr") {
                    input.advance_to(&fork);
                    let group: proc_macro2::Group = syn::parse2(a.tokens)?;
                    ret = Some(syn::parse2(group.stream())?);
                    break;
                } else {
                    a.to_tokens(tokens);
                }
            }
            iter.for_each(|a| a.to_tokens(tokens));
            input.advance_to(&fork);
        }
    }
    Ok(ret)
}

// Feature: custom-repr - Read repr types from environment variables
// When both custom-repr and cross-compile features are enabled, this implementation
// is used, ensuring custom-repr takes precedence as required.
#[cfg(feature = "custom-repr")]
fn map_to_repr(ty: Ident) -> TokenStream2 {
    let ty_str = ty.to_string();
    let env_var = format!("VISA_REPR_{}", ty_str.to_uppercase());
    
    // Try to read from environment variable
    if let Ok(custom_repr) = std::env::var(&env_var) {
        // Parse the custom repr which should be in format: "condition1:type1,condition2:type2,..."
        // Or just "type" for unconditional repr
        if custom_repr.contains(':') {
            // Platform-dependent with conditions
            let mut attrs = TokenStream2::new();
            for part in custom_repr.split(',') {
                let parts: Vec<&str> = part.splitn(2, ':').collect();
                if parts.len() == 2 {
                    let condition: TokenStream2 = parts[0].trim().parse()
                        .unwrap_or_else(|_| {
                            panic!("Invalid condition '{}' in {}={}", parts[0], env_var, custom_repr)
                        });
                    let repr_type = Ident::new(parts[1].trim(), ty.span());
                    let attr = quote_spanned!(ty.span()=>
                        #[cfg_attr(#condition, repr(#repr_type))]
                    );
                    attr.to_tokens(&mut attrs);
                }
            }
            return attrs;
        } else {
            // Simple unconditional repr
            let repr_ident = Ident::new(custom_repr.trim(), ty.span());
            return quote_spanned!(ty.span()=>#[repr(#repr_ident)]);
        }
    }
    
    // ERROR: Environment variable not set
    panic!(
        "custom-repr feature is enabled but environment variable '{}' is not set. \
         Please set the environment variable with the repr type for '{}', \
         or use the cross-compile feature instead for predefined platform configurations.",
        env_var, ty_str
    );
}

// Feature: cross-compile - Use predefined config from TOML file
// Note: This is only used when cross-compile is enabled but custom-repr is NOT enabled.
// When both features are active, custom-repr takes precedence (as required).
#[cfg(all(feature = "cross-compile", not(feature = "custom-repr")))]
fn map_to_repr(ty: Ident) -> TokenStream2 {
    use config::get_config;
    
    let config = get_config();
    let ty_str = ty.to_string();
    
    // Check if it's a platform-invariant type
    if let Some(repr_type) = config.invariant.get(&ty_str) {
        let repr_ident = Ident::new(repr_type, ty.span());
        return quote_spanned!(ty.span()=>#[repr(#repr_ident)]);
    }
    
    // Check if it's a platform-dependent unsigned type
    if config.platform_dependent.unsigned.types.contains(&ty_str) {
        return generate_platform_dependent_repr(&ty, &config.platform_dependent.unsigned.mappings);
    }
    
    // Check if it's a platform-dependent signed type
    if config.platform_dependent.signed.types.contains(&ty_str) {
        return generate_platform_dependent_repr(&ty, &config.platform_dependent.signed.mappings);
    }
    
    // ERROR: Type not found in configuration
    panic!(
        "Type '{}' not found in repr_config.toml. \
         Please add it to either [invariant] or [platform_dependent] section.",
        ty_str
    );
}

#[cfg(feature = "cross-compile")]
#[allow(dead_code)] // May be unused when custom-repr is also enabled
fn generate_platform_dependent_repr(ty: &Ident, mappings: &[config::Mapping]) -> TokenStream2 {
    let mut attrs = TokenStream2::new();
    
    for mapping in mappings {
        let condition: TokenStream2 = mapping.condition.parse()
            .unwrap_or_else(|_| {
                panic!("Invalid condition '{}' in repr_config.toml", mapping.condition)
            });
        let repr_type = Ident::new(&mapping.repr, ty.span());
        
        let attr = quote_spanned!(ty.span()=>
            #[cfg_attr(#condition, repr(#repr_type))]
        );
        attr.to_tokens(&mut attrs);
    }
    
    attrs
}

// Default behavior: Use size_of to determine repr (original implementation)
#[cfg(all(not(feature = "cross-compile"), not(feature = "custom-repr")))]
fn map_to_repr(ty: Ident) -> TokenStream2 {
    map_to_repr_default(ty)
}

// Default implementation using size_of (original approach)
#[allow(dead_code)]
fn map_to_repr_default(ty: Ident) -> TokenStream2 {
    use visa_sys as vs;
    let align = if ty == "ViUInt16" {
        unsigned_ty_token::<vs::ViUInt16>(ty.span())
    } else if ty == "ViUInt32" {
        unsigned_ty_token::<vs::ViUInt32>(ty.span())
    } else if ty == "ViEvent" {
        unsigned_ty_token::<vs::ViEvent>(ty.span())
    } else if ty == "ViEventType" {
        unsigned_ty_token::<vs::ViEventType>(ty.span())
    } else if ty == "ViEventFilter" {
        unsigned_ty_token::<vs::ViEventFilter>(ty.span())
    } else if ty == "ViAttr" {
        unsigned_ty_token::<vs::ViAttr>(ty.span())
    } else if ty == "ViStatus" {
        signed_ty_token::<vs::ViStatus>(ty.span())
    } else if ty == "ViInt16" {
        signed_ty_token::<vs::ViInt16>(ty.span())
    } else if ty == "ViInt32" {
        signed_ty_token::<vs::ViInt32>(ty.span())
    } else {
        //ty.span().unwrap().warning("unknown repr value");
        ty.clone()
    };
    quote_spanned!(ty.span()=>#[repr(#align)])
}

#[allow(dead_code)]
fn unsigned_ty_token<T: Sized>(span: Span) -> Ident {
    use std::mem::size_of;
    let t = size_of::<T>();
    if t == size_of::<u8>() {
        Ident::new("u8", span)
    } else if t == size_of::<u16>() {
        Ident::new("u16", span)
    } else if t == size_of::<u32>() {
        Ident::new("u32", span)
    } else if t == size_of::<u64>() {
        Ident::new("u64", span)
    } else if t == size_of::<u128>() {
        Ident::new("u128", span)
    } else {
        unimplemented!()
    }
}

#[allow(dead_code)]
fn signed_ty_token<T: Sized>(span: Span) -> Ident {
    use std::mem::size_of;
    let t = size_of::<T>();
    if t == size_of::<i8>() {
        Ident::new("i8", span)
    } else if t == size_of::<i16>() {
        Ident::new("i16", span)
    } else if t == size_of::<i32>() {
        Ident::new("i32", span)
    } else if t == size_of::<i64>() {
        Ident::new("i64", span)
    } else if t == size_of::<i128>() {
        Ident::new("i128", span)
    } else {
        unimplemented!()
    }
}

// solved by add conditional link flag #[cfg(not(docsrs))]
/*
/// copied from visa-sys. If add visa-sys as a dependency,
/// would failed linking when running macros in visa-rs
mod visa_sys {
    #![allow(non_camel_case_types)]
    #![allow(unused)]

    /// A UInt that is the same size as the target's pointer width
    #[cfg(target_pointer_width = "64")]
    pub type ViUIntPtrSize = ViUInt64;
    #[cfg(not(target_pointer_width = "64"))]
    pub type ViUIntPtrSize = ViUInt32;

    pub type __builtin_va_list = *mut ::std::os::raw::c_char;

    pub type va_list = __builtin_va_list;
    pub type __gnuc_va_list = __builtin_va_list;
    pub type ViUInt64 = ::std::os::raw::c_ulonglong;
    pub type ViInt64 = ::std::os::raw::c_longlong;
    pub type ViPUInt64 = *mut ViUInt64;
    pub type ViAUInt64 = *mut ViUInt64;
    pub type ViPInt64 = *mut ViInt64;
    pub type ViAInt64 = *mut ViInt64;
    pub type ViUInt32 = ::std::os::raw::c_ulong;
    pub type ViPUInt32 = *mut ViUInt32;
    pub type ViAUInt32 = *mut ViUInt32;
    pub type ViInt32 = ::std::os::raw::c_long;
    pub type ViPInt32 = *mut ViInt32;
    pub type ViAInt32 = *mut ViInt32;
    pub type ViUInt16 = ::std::os::raw::c_ushort;
    pub type ViPUInt16 = *mut ViUInt16;
    pub type ViAUInt16 = *mut ViUInt16;
    pub type ViInt16 = ::std::os::raw::c_short;
    pub type ViPInt16 = *mut ViInt16;
    pub type ViAInt16 = *mut ViInt16;
    pub type ViUInt8 = ::std::os::raw::c_uchar;
    pub type ViPUInt8 = *mut ViUInt8;
    pub type ViAUInt8 = *mut ViUInt8;
    pub type ViInt8 = ::std::os::raw::c_schar;
    pub type ViPInt8 = *mut ViInt8;
    pub type ViAInt8 = *mut ViInt8;
    pub type ViChar = ::std::os::raw::c_char;
    pub type ViPChar = *mut ViChar;
    pub type ViAChar = *mut ViChar;
    pub type ViByte = ::std::os::raw::c_uchar;
    pub type ViPByte = *mut ViByte;
    pub type ViAByte = *mut ViByte;
    pub type ViAddr = *mut ::std::os::raw::c_void;
    pub type ViPAddr = *mut ViAddr;
    pub type ViAAddr = *mut ViAddr;
    pub type ViReal32 = f32;
    pub type ViPReal32 = *mut ViReal32;
    pub type ViAReal32 = *mut ViReal32;
    pub type ViReal64 = f64;
    pub type ViPReal64 = *mut ViReal64;
    pub type ViAReal64 = *mut ViReal64;
    pub type ViBuf = ViPByte;
    pub type ViConstBuf = *const ViByte;
    pub type ViPBuf = ViPByte;
    pub type ViABuf = *mut ViPByte;
    pub type ViString = ViPChar;
    pub type ViConstString = *const ViChar;
    pub type ViPString = ViPChar;
    pub type ViAString = *mut ViPChar;
    pub type ViRsrc = ViString;
    pub type ViConstRsrc = ViConstString;
    pub type ViPRsrc = ViString;
    pub type ViARsrc = *mut ViString;
    pub type ViBoolean = ViUInt16;
    pub type ViPBoolean = *mut ViBoolean;
    pub type ViABoolean = *mut ViBoolean;
    pub type ViStatus = ViInt32;
    pub type ViPStatus = *mut ViStatus;
    pub type ViAStatus = *mut ViStatus;
    pub type ViVersion = ViUInt32;
    pub type ViPVersion = *mut ViVersion;
    pub type ViAVersion = *mut ViVersion;
    pub type ViObject = ViUInt32;
    pub type ViPObject = *mut ViObject;
    pub type ViAObject = *mut ViObject;
    pub type ViSession = ViObject;
    pub type ViPSession = *mut ViSession;
    pub type ViASession = *mut ViSession;
    pub type ViAttr = ViUInt32;
    pub type ViEvent = ViObject;
    pub type ViPEvent = *mut ViEvent;
    pub type ViFindList = ViObject;
    pub type ViPFindList = *mut ViFindList;
    pub type ViBusAddress = ViUIntPtrSize;
    pub type ViBusSize = ViUIntPtrSize;
    pub type ViAttrState = ViUIntPtrSize;
    pub type ViBusAddress64 = ViUInt64;
    pub type ViPBusAddress64 = *mut ViBusAddress64;
    pub type ViEventType = ViUInt32;
    pub type ViPEventType = *mut ViEventType;
    pub type ViAEventType = *mut ViEventType;
    pub type ViPAttrState = *mut ::std::os::raw::c_void;
    pub type ViPAttr = *mut ViAttr;
    pub type ViAAttr = *mut ViAttr;
    pub type ViKeyId = ViString;
    pub type ViConstKeyId = ViConstString;
    pub type ViPKeyId = ViPString;
    pub type ViJobId = ViUInt32;
    pub type ViPJobId = *mut ViJobId;
    pub type ViAccessMode = ViUInt32;
    pub type ViPAccessMode = *mut ViAccessMode;
    pub type ViPBusAddress = *mut ViBusAddress;
    pub type ViEventFilter = ViUInt32;
    pub type ViVAList = va_list;
    pub type ViHndlr = ::std::option::Option<
        unsafe extern "system" fn(
            vi: ViSession,
            eventType: ViEventType,
            event: ViEvent,
            userHandle: ViAddr,
        ) -> ViStatus,
    >;
}
 */
