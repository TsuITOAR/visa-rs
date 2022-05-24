use proc_macro::TokenStream;
use proc_macro2::{Delimiter, TokenStream as TokenStream2, TokenTree};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    spanned::Spanned,
    Ident, Result, Token,
};

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
