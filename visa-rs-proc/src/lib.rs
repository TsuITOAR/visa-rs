//#![feature(proc_macro_diagnostic)]
use std::str::FromStr;

use proc_macro::TokenStream;
use proc_macro2::{TokenStream as TokenStream2, TokenTree};
use quote::quote;
use syn::{parse::ParseStream, parse_macro_input, Ident};

mod rusty_ident;

fn screaming_snake_case_to_pascal_case(input: &str) -> String {
    input
        .split('_')
        .map(|x| x[..1].to_owned() + &x[1..].to_ascii_lowercase())
        .collect()
}

fn subst_ident(t: Ident) -> Ident {
    if let Some(new_id) = t.to_string().strip_prefix("VI_") {
        Ident::new(&screaming_snake_case_to_pascal_case(new_id), t.span())
    } else {
        t
    }
}
#[proc_macro]
pub fn rusty_ident(input: TokenStream) -> TokenStream {
    let macros = parse_macro_input!(input as rusty_ident::Macros);
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

fn match_tokens(input: ParseStream, str: &str) -> Option<proc_macro2::Span> {
    let stream: TokenStream2 = syn::parse_str(str).unwrap();
    let fork = input.fork();
    for token in stream {
        if token.to_string() != fork.parse::<TokenTree>().unwrap().to_string() {
            return None;
        }
    }
    use syn::parse::discouraged::Speculative;
    let start = input.span();
    input.advance_to(&fork);
    let end = input.span();

    return Some(start.join(end).unwrap_or(start));
}
mod attrs;

#[proc_macro]
pub fn visa_attrs(input: TokenStream) -> TokenStream {
    let input: TokenStream2 = input.into();
    let input: TokenStream2 = input.into_iter().map(get_visa_num).collect();
    let input: TokenStream = input.into();
    let attrs = parse_macro_input!(input as attrs::Attributes);
    quote! {#attrs}.into()
}
