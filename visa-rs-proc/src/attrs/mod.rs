use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use range::Range;
use syn::{
    braced, bracketed, parenthesized,
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Ident, LitInt, LitStr, Result, Token,
};

use crate::{match_tokens, subst_ident};
mod range;
pub struct Attributes {
    vis: Token![pub],
    struct_token: Token![struct],
    ident: Ident,
    attrs: Vec<Attr>,
}

impl Parse for Attributes {
    fn parse(input: ParseStream) -> Result<Self> {
        let vis = input.parse()?;
        let struct_token = input.parse()?;
        let ident = input.parse()?;
        let attrs_input;
        braced!(attrs_input in input);
        let mut attrs = Vec::new();
        while !attrs_input.is_empty() {
            attrs.push(attrs_input.parse()?);
        }
        Ok(Attributes {
            vis,
            struct_token,
            ident,
            attrs,
        })
    }
}

impl ToTokens for Attributes {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        for attr in &self.attrs {
            attr.struct_def(tokens);
            attr.constructors(tokens);
            attr.default_impl(tokens);
            attr.kind_impl(tokens);
            attr.empty_instance(tokens);
        }
        let fields = self.attrs.iter().map(|x| x.struct_name());
        let docs = self.attrs.iter().map(|x| &x.desc);
        let enum_name = &self.ident;
        quote!(
            pub enum #enum_name{
                #(
                    #[doc=#docs]
                    #fields(#fields)
                ),*
            }
        )
        .to_tokens(tokens);
        let fields = self.attrs.iter().map(|x| x.struct_name());
        quote!(
            #(
                impl From<#fields> for #enum_name{
                    fn from(s:#fields)->Self{
                        Self::#fields(s)
                    }
                }
            )*
        )
        .to_tokens(tokens);
    }
}

struct Attr {
    id: Ident,
    desc: LitStr,
    _vis: TokenStream2,
    ty: Type,
    range: Range,
}

impl Attr {
    fn struct_name(&self) -> Ident {
        subst_ident(self.id.clone())
    }
    fn struct_def(&self, tokens: &mut TokenStream2) {
        let Self { desc, ty, .. } = self;
        let id = self.struct_name();
        match ty.core {
            TypeCore::UnArch(ref ty) => {
                quote!(
                    #[doc= #desc]
                    pub struct #id{
                        value:vs::#ty
                    }
                    impl #id{
                        pub(crate) fn inner_mut(&mut self) -> &mut vs::#ty{
                            &mut self.value
                        }
                        pub(crate) fn inner_c_void(&mut self) -> *mut ::std::ffi::c_void{
                            self.inner_mut() as *mut _ as _
                        }
                    }
                )
                .to_tokens(tokens);
            }
            TypeCore::Arch(ref a) => {
                let arch = a.iter().map(|x| &x.arch).map(|x| {
                    if let Ok(64) = x.base10_parse() {
                        LitStr::new("x86_64", x.span())
                    } else if let Ok(32) = x.base10_parse() {
                        LitStr::new("x86", x.span())
                    } else {
                        LitStr::new(&x.to_string(), x.span())
                    }
                });
                let ty = a.iter().map(|x| &x.core);
                quote!(
                    #(
                        #[cfg(target_arch = #arch)]
                        #[doc= #desc]
                        pub struct #id{
                            value:vs::#ty
                        }
                        #[cfg(target_arch = #arch)]
                        impl #id{
                        pub(crate) fn inner_mut(&mut self) -> &mut vs::#ty{
                            &mut self.value
                        }
                        pub(crate) fn inner_c_void(&mut self) -> *mut ::std::ffi::c_void{
                            self.inner_mut() as *mut _ as _
                        }
                    }
                    )*
                )
                .to_tokens(tokens);
            }
        }
    }
    fn constructors(&self, tokens: &mut TokenStream2) {}
    fn kind_impl(&self, tokens: &mut TokenStream2) {
        let struct_id = self.struct_name();
        let kind_id = &self.id;
        let kind_id_str = kind_id.to_string();
        if kind_id_str.starts_with("VI_ATTR_PXI_MEM_BASE")
            || kind_id_str.starts_with("VI_ATTR_PXI_MEM_SIZE")
        {
            let kind_id_x64 = subst_ident(Ident::new(
                &format!("{}_32", kind_id_str),
                kind_id_str.span(),
            ));
            let kind_id_x32 = subst_ident(Ident::new(
                &format!("{}_64", kind_id_str),
                kind_id_str.span(),
            ));
            quote!(
                #[cfg(target_arch = "x86")]
                impl super::AttrInner for #struct_id{
                    fn kind(&self)->AttrKind{
                        AttrKind::#kind_id_x32
                    }
                }
                #[cfg(target_arch = "x86_64")]
                impl super::AttrInner for #struct_id{
                    fn kind(&self)->AttrKind{
                        AttrKind::#kind_id_x64
                    }
                }
            )
            .to_tokens(tokens);
        } else {
            let kind_id = if let Some(new_id) = kind_id_str.strip_suffix("_32") {
                Ident::new(new_id, kind_id.span())
            } else if let Some(new_id) = kind_id_str.strip_suffix("_64") {
                Ident::new(new_id, kind_id.span())
            } else {
                kind_id.clone()
            };
            let kind_id = subst_ident(kind_id);

            quote!(
                impl super::AttrInner for #struct_id{
                    fn kind(&self)->AttrKind{
                        AttrKind::#kind_id
                    }
                }
            )
            .to_tokens(tokens);
        }
    }
    fn default_impl(&self, tokens: &mut TokenStream2) {}
    fn empty_instance(&self, tokens: &mut TokenStream2) {}
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
            _vis: vis,
            ty,
            range,
        })
    }
}
