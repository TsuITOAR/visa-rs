#![feature(is_some_with)]
use std::str::FromStr;

use proc_macro::TokenStream;
use proc_macro2::{Delimiter, TokenStream as TokenStream2, TokenTree};
use quote::{quote, ToTokens};
use syn::{
    bracketed, parenthesized,
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
    fn parse_to_u64(s: &str) -> std::result::Result<u64, std::num::ParseIntError> {
        u64::from_str_radix(s, 16)
    }
    fn u64_to_lit(n: u64) -> String {
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
            if let Some(Ok(d)) = id.to_string().strip_suffix('h').map(parse_to_u64) {
                let mut ret = proc_macro2::Literal::from_str(&u64_to_lit(d)).unwrap();
                ret.set_span(id.span());
                TokenTree::Literal(ret)
            } else {
                TokenTree::Ident(id)
            }
        }
        TokenTree::Literal(lit) => {
            if let Some(Ok(d)) = lit.to_string().strip_suffix('h').map(parse_to_u64) {
                let mut ret = proc_macro2::Literal::from_str(&u64_to_lit(d)).unwrap();
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

fn match_tokens(input: ParseStream, str: &str) -> bool {
    let stream: TokenStream2 = syn::parse_str(str).unwrap();
    let fork = input.fork();
    for token in stream {
        if token.to_string() != fork.parse::<TokenTree>().unwrap().to_string() {
            return false;
        }
    }
    use syn::parse::discouraged::Speculative;
    input.advance_to(&fork);
    return true;
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
    ty: Type,
    range: Range,
}

struct Type {
    attr_name: Option<Ident>,
    core: TypeCore,
}

impl Parse for Type {
    fn parse(input: ParseStream) -> Result<Self> {
        let attr_name;
        if input.peek2(Token![:]) {
            attr_name = Some(input.parse()?);
            input.parse::<Token![:]>()?;
        } else {
            attr_name = None;
        }
        Ok(Type {
            attr_name,
            core: input.parse()?,
        })
    }
}

enum TypeCore {
    Arch(Vec<ArchType>),
    UnArch(Ident),
}

impl Parse for TypeCore {
    fn parse(input: ParseStream) -> Result<Self> {
        let core = input.parse()?;
        if input.peek(Token![for]) {
            input.parse::<Token![for]>()?;
            let arch: LitInt = input.parse()?;
            if !match_tokens(input, "-bit applications") {
                return Err(input.error("expected '-bit applications' after architecture"));
            }
            let mut ret = vec![ArchType { arch, core: core }];
            while !input.is_empty() {
                let core = input.parse()?;
                input.parse::<Token![for]>()?;
                ret.push(ArchType {
                    core,
                    arch: input.parse()?,
                });
                if !match_tokens(input, "-bit applications") {
                    return Err(input.error("expected '-bit applications' after architecture"));
                }
            }
            return Ok(TypeCore::Arch(ret));
        } else {
            return Ok(TypeCore::UnArch(core));
        }
    }
}

struct ArchType {
    arch: LitInt,
    core: Ident,
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
        let range;
        bracketed!(range in input);
        let range = range.parse()?;
        Ok(Self {
            id,
            desc,
            vis,
            ty,
            range,
        })
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
