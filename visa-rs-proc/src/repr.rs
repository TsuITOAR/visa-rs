use proc_macro2::{Delimiter, Ident, TokenStream as TokenStream2, TokenTree};
use quote::{quote_spanned, ToTokens};
use syn::{parse::Parse, Path, Result, Token};

use crate::rusty_ident::NestedMacros;

// Configuration structures for parsing the TOML config file (only when cross-compile feature is enabled)
// Note: When both cross-compile and custom-repr features are enabled, custom-repr takes precedence
// and this code is not used (hence the allow(dead_code) attributes).

#[cfg(any(feature = "cross-compile", feature = "custom-repr"))]
mod config {
    use crate::TokenStream2;
    use quote::{quote_spanned, ToTokens};
    use serde::Deserialize;
    use std::collections::HashMap;
    use syn::{Ident, Result};
    const REPR_CONFIG_ENV: &str = "VISA_REPR_CONFIG_PATH";

    #[allow(dead_code)]
    #[derive(Debug, Deserialize)]
    pub struct ReprConfig {
        pub platforms: Vec<PlatformMapping>,
    }

    #[derive(Debug, Deserialize)]
    pub struct PlatformMapping {
        pub condition: String,
        pub types: HashMap<String, String>,
    }

    type ConfigResult<T> = std::result::Result<T, String>;

    fn try_load_from_env_path() -> ConfigResult<Option<String>> {
        let path = match std::env::var(REPR_CONFIG_ENV) {
            Ok(path) => path,
            Err(std::env::VarError::NotPresent) => return Ok(None),
            Err(std::env::VarError::NotUnicode(_)) => {
                return Err(format!(
                    "{} contains non-UTF-8 data. Please use a valid UTF-8 path.",
                    REPR_CONFIG_ENV
                ));
            }
        };
        if path.trim().is_empty() {
            #[cfg(not(feature = "custom-repr"))]
            return Ok(None);
            #[cfg(feature = "custom-repr")]
            return Err(format!(
                "{} is set but empty. Please provide a valid path to the config file.",
                REPR_CONFIG_ENV
            ));
        }
        let custom_path = std::path::Path::new(&path);
        if !custom_path.exists() {
            #[cfg(not(feature = "custom-repr"))]
            return Ok(None);
            #[cfg(feature = "custom-repr")]
            return Err(format!(
                "Custom repr config path '{}' does not exist (from {}).",
                custom_path.display(),
                REPR_CONFIG_ENV
            ));
        }
        let config_str = std::fs::read_to_string(custom_path).map_err(|e| {
            format!(
                "Failed to read custom config file {} (from {}): {}",
                custom_path.display(),
                REPR_CONFIG_ENV,
                e
            )
        })?;
        Ok(Some(config_str))
    }
    // Load configuration at compile time
    pub fn load_config() -> ConfigResult<ReprConfig> {
        // Try to load from current directory first (allows user customization)
        // Fall back to bundled config if not found if custom-repr feature not set only if custom-repr feature not set
        let env_cfg = try_load_from_env_path()?;
        let repr_cfg = match env_cfg.as_deref() {
            Some(cfg) => cfg,
            None => {
                #[cfg(feature = "custom-repr")]
                return Err(format!(
                    "custom-repr feature is enabled but no custom repr config was found. \
                    Provide mappings via VISA_REPR_* environment variables, or set {} to an absolute \
                    config path, or place 'visa_repr_config.toml' in the crate root directory. \
                    Otherwise disable the 'custom-repr' feature.",
                    REPR_CONFIG_ENV
                ));
                #[cfg(not(feature = "custom-repr"))]
                {
                    include_str!("../default_repr_config.toml")
                }
            }
        };

        let config: ReprConfig = toml::from_str(repr_cfg)
            .map_err(|e| format!("Failed to parse repr config TOML: {}", e))?;

        Ok(config)
    }

    // Lazy static config loaded once
    static CONFIG: std::sync::OnceLock<ReprConfig> = std::sync::OnceLock::new();

    fn get_config() -> ConfigResult<&'static ReprConfig> {
        if let Some(config) = CONFIG.get() {
            return Ok(config);
        }
        let config = load_config()?;
        CONFIG
            .set(config)
            .map_err(|_| "Failed to initialize repr config".to_string())?;
        Ok(CONFIG
            .get()
            .expect("repr config initialized but not available"))
    }

    pub fn map_to_repr_from_toml(ty: Ident) -> Result<TokenStream2> {
        let config = get_config().map_err(|e| syn::Error::new(ty.span(), e))?;
        let ty_str = ty.to_string();
        let mut attrs = TokenStream2::new();
        let mut missing_conditions = Vec::new();
        let mut found_any = false;

        for platform in &config.platforms {
            if let Some(repr_type) = platform.types.get(&ty_str) {
                found_any = true;
                let condition: TokenStream2 = platform.condition.parse().map_err(|_| {
                    syn::Error::new(
                        ty.span(),
                        format!(
                            "Invalid condition '{}' in repr_config.toml",
                            platform.condition
                        ),
                    )
                })?;
                let repr_type = Ident::new(repr_type, ty.span());
                let attr = quote_spanned!(ty.span()=>
                    #[cfg_attr(#condition, repr(#repr_type))]
                );
                attr.to_tokens(&mut attrs);
            } else {
                missing_conditions.push(platform.condition.clone());
            }
        }

        if !found_any {
            return Err(syn::Error::new(
                ty.span(),
                format!(
                    "Type '{}' not found in repr_config.toml. \
                    Please add it under each [[platforms]] entry.",
                    ty_str
                ),
            ));
        }

        // if !missing_conditions.is_empty() {
        //     return Err(syn::Error::new(
        //         ty.span(),
        //         format!(
        //             "Type '{}' missing in repr_config.toml for platform condition(s): {}",
        //             ty_str,
        //             missing_conditions.join(", ")
        //         ),
        //     ));
        // }

        Ok(attrs)
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
                ret.to_tokens(tokens);
                // Add assertions after the macro expansion
                for assertion in &inner.assertions {
                    assertion.to_tokens(tokens);
                }
            }
            _ => {
                inner.to_tokens(tokens);
            }
        }
    }
}

pub struct AttrProcessed {
    tokens: TokenStream2,
    assertions: Vec<TokenStream2>,
}

impl ToTokens for AttrProcessed {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        self.tokens.to_tokens(tokens);
        // Assertions are now added in Input::to_tokens after macro expansion
    }
}

impl Parse for AttrProcessed {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let mut inner = TokenStream2::new();
        let mut assertions = Vec::new();
        let mut enum_assert_pairs: Vec<(Ident, Ident)> = Vec::new();
        let mut last_visa_repr: Option<Ident> = None;

        while !input.is_empty() {
            if let Some(ty) = extract_repr_attribute(input, &mut inner)? {
                assert!(
                    last_visa_repr.is_none(),
                    "Multiple #[repr(...)] attributes found before enum declaration. \
                     Each enum can only have one #[repr(...)] attribute."
                );
                // Track VISA types so the next enum can consume them in order
                last_visa_repr = Some(ty.clone());
                map_to_repr(ty)?.to_tokens(&mut inner);
            } else {
                let token = input.parse::<TokenTree>()?;

                // Track when we enter/exit the enum body
                // this may not be a legal enum since there are other macros to handle generating enums
                match &token {
                    TokenTree::Ident(id) if id == "enum" && last_visa_repr.is_some() => {
                        // We're about to see the enum name
                        token.to_tokens(&mut inner);

                        // Next should be the enum name
                        let enum_name = input.parse::<Ident>()?;
                        enum_name.to_tokens(&mut inner);

                        let visa_type = last_visa_repr
                            .take()
                            .expect("last_visa_repr checked non-empty");
                        enum_assert_pairs.push((enum_name.clone(), visa_type));
                    }
                    _ => {
                        token.to_tokens(&mut inner);
                    }
                }
            }
        }
        for (enum_name, visa_type) in enum_assert_pairs {
            let assertion_name = Ident::new(
                &format!("_ASSERT_SIZE_EQ_{}", enum_name.to_string().to_uppercase()),
                enum_name.span(),
            );

            // Create a helpful error message
            let error_msg = format!(
                "Size mismatch: {} enum does not have the same size as visa_sys::{}. \
                 This likely means the #[repr(...)] attribute is incorrect for the target platform. \
                 If cross-compiling, ensure you're using the 'cross-compile' feature or have set \
                 the correct environment variables with 'custom-repr' feature.",
                enum_name, visa_type
            );

            let assertion = quote_spanned!(enum_name.span()=>
                #[allow(clippy::let_unit_value)]
                const #assertion_name: () = {
                    // Use a struct that implements a trait to provide better error messages
                    struct SizeAssert<const L: usize, const R: usize>;

                    #[allow(dead_code)]
                    impl<const L: usize, const R: usize> SizeAssert<L, R> {
                        const VALID: () = assert!(
                            L == R,
                            #error_msg
                        );
                    }

                    // Force evaluation of the assertion
                    let _ = SizeAssert::<
                        {::std::mem::size_of::<#enum_name>()},
                        {::std::mem::size_of::<visa_sys::#visa_type>()}
                    >::VALID;

                    // Also use transmute as a fallback for older Rust versions
                    const fn _assert_size_eq() {
                        let _ = ::std::mem::transmute::<#enum_name, visa_sys::#visa_type>;
                    }
                };
            );
            assertions.push(assertion);
        }

        Ok(Self {
            tokens: inner,
            assertions,
        })
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
                if a.path().is_ident("repr") {
                    input.advance_to(&fork);
                    let ident: Ident = a.parse_args()?;
                    ret = Some(ident);
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

#[cfg(feature = "custom-repr")]
fn map_to_repr(ty: Ident) -> Result<TokenStream2> {
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
                    let condition: TokenStream2 = parts[0].trim().parse().map_err(|_| {
                        syn::Error::new(
                            ty.span(),
                            format!(
                                "Invalid condition '{}' in {}={}",
                                parts[0], env_var, custom_repr
                            ),
                        )
                    })?;
                    let repr_type = Ident::new(parts[1].trim(), ty.span());
                    let attr = quote_spanned!(ty.span()=>
                        #[cfg_attr(#condition, repr(#repr_type))]
                    );
                    attr.to_tokens(&mut attrs);
                }
            }
            return Ok(attrs);
        } else {
            let repr_type = Ident::new(custom_repr.trim(), ty.span());
            return Ok(quote_spanned!(ty.span()=>#[repr(#repr_type)]));
        }
    } else {
        // if env var not set, try to get form config file
        config::map_to_repr_from_toml(ty)
    }
}

// Feature: cross-compile - Use predefined config from TOML file
// Note: This is only used when cross-compile is enabled but custom-repr is NOT enabled.
// When both features are active, custom-repr takes precedence (as required).
#[cfg(all(feature = "cross-compile", not(feature = "custom-repr")))]
fn map_to_repr(ty: Ident) -> Result<TokenStream2> {
    config::map_to_repr_from_toml(ty)
}

// Default behavior: Use size_of to determine repr (original implementation)
#[cfg(all(not(feature = "cross-compile"), not(feature = "custom-repr")))]
fn map_to_repr(ty: Ident) -> Result<TokenStream2> {
    Ok(map_to_repr_default(ty))
}

// Default implementation using size_of (original approach)
#[cfg(all(not(feature = "cross-compile"), not(feature = "custom-repr")))]
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

#[cfg(all(not(feature = "cross-compile"), not(feature = "custom-repr")))]
fn unsigned_ty_token<T: Sized>(span: proc_macro2::Span) -> Ident {
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

#[cfg(all(not(feature = "cross-compile"), not(feature = "custom-repr")))]
fn signed_ty_token<T: Sized>(span: proc_macro2::Span) -> Ident {
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
