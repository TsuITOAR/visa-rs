use proc_macro2::{Delimiter, Ident, TokenStream as TokenStream2, TokenTree};
use quote::{quote_spanned, ToTokens};
use syn::{parse::Parse, Path, Result, Token};

use crate::rusty_ident::NestedMacros;

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
                 Check the fixed-width mapping and the visa-sys bindings.",
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

// Default mappings follow VISA's fixed-width typedefs, not the proc-macro host ABI.
fn map_to_repr(ty: Ident) -> Result<TokenStream2> {
    let repr = match ty.to_string().as_str() {
        "ViUInt8" => "u8",
        "ViInt8" => "i8",
        "ViUInt16" => "u16",
        "ViInt16" => "i16",
        "ViUInt32" | "ViEvent" | "ViEventType" | "ViEventFilter" | "ViAttr" => "u32",
        "ViInt32" | "ViStatus" => "i32",
        "ViUInt64" => "u64",
        "ViInt64" => "i64",
        _ => return Ok(quote_spanned!(ty.span()=>#[repr(#ty)])),
    };
    let repr = Ident::new(repr, ty.span());
    Ok(quote_spanned!(ty.span()=>#[repr(#repr)]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visa_reprs_are_fixed_width() {
        for (ty, expected) in [
            ("ViUInt8", "u8"),
            ("ViInt8", "i8"),
            ("ViUInt16", "u16"),
            ("ViInt16", "i16"),
            ("ViUInt32", "u32"),
            ("ViInt32", "i32"),
            ("ViUInt64", "u64"),
            ("ViInt64", "i64"),
            ("ViEvent", "u32"),
            ("ViEventType", "u32"),
            ("ViEventFilter", "u32"),
            ("ViAttr", "u32"),
            ("ViStatus", "i32"),
        ] {
            let span = proc_macro2::Span::call_site();
            let actual = map_to_repr(Ident::new(ty, span)).unwrap();
            let expected = Ident::new(expected, span);
            assert_eq!(
                actual.to_string(),
                quote_spanned!(span=>#[repr(#expected)]).to_string(),
                "{ty}"
            );
        }
    }

    #[test]
    fn primitive_reprs_pass_through() {
        let span = proc_macro2::Span::call_site();
        let actual = map_to_repr(Ident::new("usize", span)).unwrap();
        assert_eq!(
            actual.to_string(),
            quote_spanned!(span=>#[repr(usize)]).to_string()
        );
    }
}
