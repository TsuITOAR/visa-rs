#![feature(is_some_with)]
use std::str::FromStr;

use proc_macro::TokenStream;
use proc_macro2::{Delimiter, TokenStream as TokenStream2, TokenTree};
use quote::{quote, ToTokens};
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input,
    spanned::Spanned,
    Error, Ident, LitInt, LitStr, Result, Token,
};

mod range;
use range::Range;
struct Macros {
    inner: Vec<MacroInside>,
}

impl Parse for Macros {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut inner = Vec::new();
        while !input.is_empty() {
            inner.push(input.parse()?);
        }
        Ok(Self { inner })
    }
}

impl ToTokens for Macros {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        self.inner.iter().for_each(|x| x.to_tokens(tokens))
    }
}

struct MacroInside {
    mac: Ident,
    exc: Token![!],
    body: Body,
}

impl Parse for MacroInside {
    fn parse(input: ParseStream) -> Result<Self> {
        let mac: Ident = input.parse()?;
        let exc = input.parse::<Token![!]>()?;
        let body: Body = input.parse()?;
        Ok(Self { mac, exc, body })
    }
}

impl ToTokens for MacroInside {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self { mac, exc, body } = self;
        mac.to_tokens(tokens);
        exc.to_tokens(tokens);
        body.to_tokens(tokens);
    }
}

struct Body {
    delim: Delimiter,
    content: TokenStream2,
}

impl ToTokens for Body {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self { delim, content } = self;
        let content: TokenStream2 = content.clone().into_iter().map(subst_ident).collect();
        proc_macro2::Group::new(delim.clone(), content).to_tokens(tokens);
    }
}

impl Parse for Body {
    fn parse(input: ParseStream) -> Result<Self> {
        let g: proc_macro2::Group = input.parse()?;
        Ok(Self {
            delim: g.delimiter(),
            content: g.stream(),
        })
    }
}

fn screaming_snake_case_to_pascal_case(input: &str) -> String {
    input
        .split('_')
        .map(|x| x[..1].to_owned() + &x[1..].to_ascii_lowercase())
        .collect()
}

fn subst_ident(t: TokenTree) -> TokenStream2 {
    let mut stream = TokenStream2::new();
    if let TokenTree::Ident(ref id) = t {
        let id = id.to_string();
        if let Some(new_id) = id.strip_prefix("VI_") {
            TokenTree::Ident(Ident::new(
                &screaming_snake_case_to_pascal_case(new_id),
                id.span(),
            ))
            .to_tokens(&mut stream);
            return stream;
        }
    } else if let TokenTree::Group(ref g) = t {
        let content = g.stream();
        let content: TokenStream2 = content.into_iter().map(subst_ident).collect();
        proc_macro2::Group::new(g.delimiter(), content).to_tokens(&mut stream);
        return stream;
    }
    t.to_tokens(&mut stream);
    return stream;
}

#[proc_macro]
pub fn rusty_ident(input: TokenStream) -> TokenStream {
    let macros = parse_macro_input!(input as Macros);
    quote! {#macros}.into()
}

fn get_visa_num(input: TokenTree) -> TokenTree {
    fn parse_to_u32(s: &str) -> std::result::Result<u32, std::num::ParseIntError> {
        u32::from_str_radix(s, 16)
    }
    fn u32_to_lit(n: u32) -> String {
        format!("{:#010X}", n)
    }
    match input {
        TokenTree::Group(g) => {
            let mut ret = proc_macro2::Group::new(
                g.delimiter(),
                g.stream().into_iter().map(get_visa_num).collect(),
            );
            ret.set_span(g.span());
            TokenTree::Group(ret)
        }
        TokenTree::Ident(id) => {
            if let Some(Ok(d)) = id.to_string().strip_suffix('h').map(parse_to_u32) {
                let mut ret = proc_macro2::Literal::from_str(&u32_to_lit(d)).unwrap();
                ret.set_span(id.span());
                TokenTree::Literal(ret)
            } else {
                TokenTree::Ident(id)
            }
        }
        TokenTree::Literal(lit) => {
            if let Some(Ok(d)) = lit.to_string().strip_suffix('h').map(parse_to_u32) {
                let mut ret = proc_macro2::Literal::from_str(&u32_to_lit(d)).unwrap();
                ret.set_span(lit.span());
                TokenTree::Literal(ret)
            } else {
                TokenTree::Literal(lit)
            }
        }
        _ => input,
    }
}

#[test]
fn test_visa_num() {
    let stream = TokenStream2::from_str(r#"FFFFFFFFh,{Ah,0h,-1},0000001h"#).unwrap();
    let stream: TokenStream2 = stream.into_iter().map(get_visa_num).collect();
    assert_eq!(
        stream.to_string(),
        r#"0xFFFFFFFF , { 0x0000000A , 0x00000000 ,- 1 } , 0x00000001"#
    );
}

struct Attributes {
    attrs: Vec<Attr>,
}

impl Parse for Attributes {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut attrs = Vec::new();
        while !input.is_empty() {
            attrs.push(input.parse()?);
        }
        Ok(Attributes { attrs })
    }
}

struct Attr {
    id: Ident,
    desc: LitStr,
    vis: TokenStream2,
    ty: Ident,
    range: Range,
    default: DefaultValue,
}

impl Parse for Attr {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![const]>()?;
        let id = input.parse()?;
        input.parse::<Token![:]>()?;
        let desc = input.parse()?;
        let vis;
        parenthesized!(vis in input);
        let vis = vis.parse()?;
        let ty;
        parenthesized!(ty in input);
        let ty = ty.parse()?;
        input.parse::<Token![::]>()?;
        input.parse::<Token![<]>()?;
        let range = input.parse()?;
        input.parse::<Token![>]>()?;
        input.parse::<Token![=]>()?;
        let default = input.parse()?;
        Ok(Self {
            id,
            desc,
            vis,
            ty,
            range,
            default,
        })
    }
}

enum DefaultValue {
    Num(LitInt),
    Ident(Ident),
    Key {
        key_name: Vec<TokenTree>,
        char: LitInt,
    },
    NA,
}

fn is_na(input: ParseStream) -> bool {
    let fork = input.fork();
    if !fork.parse::<Ident>().is_ok_and(|n| n == "N") {
        return false;
    }
    if !fork.parse::<Token![/]>().is_ok() {
        return false;
    }
    if !fork.parse::<Ident>().is_ok_and(|a| a == "A") {
        return false;
    }
    input.parse::<Ident>().unwrap();
    input.parse::<Token![/]>().unwrap();
    input.parse::<Ident>().unwrap();
    true
}

impl Parse for DefaultValue {
    fn parse(input: ParseStream) -> Result<Self> {
        if is_na(input) {
            Ok(Self::NA)
        } else {
            let look = input.lookahead1();
            if look.peek(LitInt) {
                Ok(Self::Num(input.parse()?))
            } else if look.peek(Ident) {
                Ok(Self::Ident(input.parse()?))
            } else if look.peek(Token![<]) {
                input.parse::<Token![<]>()?;
                let mut key_name = Vec::new();
                while !input.peek(Token![>]) {
                    key_name.push(input.parse()?);
                }
                input.parse::<Token![>]>()?;
                let char;
                parenthesized!(char in input);
                Ok(Self::Key {
                    key_name,
                    char: char.parse()?,
                })
            } else {
                Err(look.error())
            }
        }
    }
}

#[proc_macro]
pub fn visa_attrs(input: TokenStream) -> TokenStream {
    let input: TokenStream2 = input.into();
    let input: TokenStream2 = input.into_iter().map(get_visa_num).collect();
    let input: TokenStream = input.into();
    let macros = parse_macro_input!(input as Attributes);
    quote! {}.into()
}
