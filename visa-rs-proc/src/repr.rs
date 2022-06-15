use proc_macro2::{Delimiter, Ident, Span, TokenStream as TokenStream2, TokenTree};
use quote::{quote_spanned, ToTokens};
use syn::{parse::Parse, Path, Result, Token};

use crate::rusty_ident::NestedMacros;

pub struct Input {
    macs: Vec<Path>,
    exc: Token![!],
    inner: AttrProcessed,
}

impl Parse for Input {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let nest_macros: NestedMacros = input.parse()?;
        Ok(Self {
            macs: nest_macros.macs,
            exc: nest_macros.exc,
            inner: syn::parse2(nest_macros.body.content)?,
        })
    }
}

impl ToTokens for Input {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self { macs, exc, inner } = self;
        let mut ret = inner.to_token_stream();
        for mac in macs.iter().rev() {
            let mut m = mac.to_token_stream();
            exc.to_tokens(&mut m);
            proc_macro2::Group::new(Delimiter::Brace, ret).to_tokens(&mut m);
            ret = m;
        }
        ret.to_tokens(tokens)
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
    if fork.peek(Token![#]) {
        if let Ok(attr) = fork.call(syn::Attribute::parse_outer) {
            for a in attr {
                if a.path.is_ident("repr") {
                    input.advance_to(&fork);
                    let group: proc_macro2::Group = syn::parse2(a.tokens)?;

                    return Ok(Some(syn::parse2(group.stream())?));
                } else {
                    a.to_tokens(tokens);
                }
            }
        }
    }
    input.advance_to(&fork);
    Ok(None)
}

fn map_to_repr(ty: Ident) -> TokenStream2 {
    use visa_sys as vs;
    let align = if ty == "ViEventType" {
        unsigned_ty_token::<vs::ViEventType>(ty.span())
    } else if ty == "ViUInt16" {
        unsigned_ty_token::<vs::ViUInt16>(ty.span())
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
    } else {
        unimplemented!("{}", ty.to_string())
    };
    quote_spanned!(ty.span()=>#[repr(#align)])
}

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
