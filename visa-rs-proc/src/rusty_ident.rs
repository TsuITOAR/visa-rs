use proc_macro2::{Delimiter, TokenStream as TokenStream2, TokenTree};
use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    Path, Result, Token,
};

use crate::{Body, OneLayer};

pub struct Macros {
    inner: Vec<NestedMacros>,
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

pub struct NestedMacros {
    pub(crate) macs: Vec<Path>,
    pub(crate) exc: Token![!],
    pub(crate) body: Body,
}

impl Parse for NestedMacros {
    fn parse(input: ParseStream) -> Result<Self> {
        let mac: Path = input.call(Path::parse_mod_style)?;
        let exc = input.parse::<Token![!]>()?;
        let mut body: Body = input.parse()?;
        let mut macs = vec![mac];
        while let Ok(s) = syn::parse2::<OneLayer>(body.content.clone()) {
            body = s.body;
            macs.push(s.mac);
        }
        Ok(Self { macs, exc, body })
    }
}

impl ToTokens for NestedMacros {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self { macs, exc, body } = self;
        let body = body
            .content
            .clone()
            .into_iter()
            .map(subst_token_tree)
            .collect();
        let mut ret = body;
        for mac in macs.iter().rev() {
            let mut m = mac.to_token_stream();
            exc.to_tokens(&mut m);
            proc_macro2::Group::new(Delimiter::Brace, ret).to_tokens(&mut m);
            ret = m;
        }
        ret.to_tokens(tokens)
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
    stream
}
