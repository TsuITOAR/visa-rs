use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2, TokenTree};
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned};
#[proc_macro]
pub fn rusty_ident(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as TokenStream2);
    let output = input.into_iter().map(|x| {
        if let TokenTree::Ident(ref id) = x {
            let id = id.to_string();
            if let Some(new_id) = id.strip_prefix("VI_") {
                return TokenTree::Ident(Ident::new(
                    &ident_case::RenameRule::SnakeCase.apply_to_variant(new_id),
                    id.span(),
                ));
            }
        }
        return x;
    });
    quote! {#(#output)*}.into()
}
