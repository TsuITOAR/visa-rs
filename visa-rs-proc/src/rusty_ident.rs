use proc_macro2::{Delimiter, TokenStream as TokenStream2, TokenTree};
use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Ident, Result, Token,
};

pub struct Macros {
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
        let content: TokenStream2 = content.clone().into_iter().map(subst_token_tree).collect();
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

fn subst_token_tree(t: TokenTree) -> TokenStream2 {
    let mut stream = TokenStream2::new();
    if let TokenTree::Ident(id) = t {
        crate::subst_ident(id).to_tokens(&mut stream);
        return stream;
    } else if let TokenTree::Group(ref g) = t {
        let content = g.stream();
        let content: TokenStream2 = content.into_iter().map(subst_token_tree).collect();
        proc_macro2::Group::new(g.delimiter(), content).to_tokens(&mut stream);
        return stream;
    }
    t.to_tokens(&mut stream);
    return stream;
}
