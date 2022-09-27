use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned, ToTokens};
use range::Range;
use syn::{
    braced, bracketed, parenthesized,
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Ident, LitInt, LitStr, Result, Token,
};

use crate::{attrs::range::RangeCore, match_tokens, subst_ident};
mod range;
pub struct Attributes {
    _vis: Token![pub],
    _struct_token: Token![struct],
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
            _vis: vis,
            _struct_token: struct_token,
            ident,
            attrs,
        })
    }
}

impl ToTokens for Attributes {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        for attr in &self.attrs {
            if let TypeCore::UnArch(ref t)=attr.ty.core{
                if t=="ViString"{
                    continue;
                }
            }
            attr.struct_def(tokens);
            attr.constructors(tokens);
            attr.default_impl(tokens);
            attr.kind_impl(tokens);
        }
        let fields = self.attrs.iter().map(|x| x.struct_name());
        let docs = self.attrs.iter().map(|x| &x.desc);
        let enum_name = &self.ident;
        quote!(
            #[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
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

        let field_name = self.attrs.iter().map(|x| x.struct_name());

        let type_name = self.attrs.iter().map(|x| struct_name_to_kind_name(&x.id));

        let match_field = field_name
            .zip(type_name)
            .map(|(f, t)| {
                let f = f.clone();
                t.into_iter().map(move |(ty, cfg)| {
                    quote!(
                            #cfg
                            AttrKind::#ty => Self::from(<#f as super::AttrInner>::zero())
                    )
                })
            })
            .flatten();
        let fields1 = self.attrs.iter().map(|x| x.struct_name());
        let fields2 = self.attrs.iter().map(|x| x.struct_name());
        let as_attr_state=self.attrs.iter().map(|x| {
            let field=x.struct_name();
            if let TypeCore::UnArch(ref t)=x.ty.core{
                if t=="ViString"{
                    return quote_spanned!(t.span()=> 
                        Self::#field(s)=>
                            {
                                log::warn!("setting read only attribute {}", ::std::stringify!(#field));
                                s.value.as_ptr() as _
                            }
                    )
                }
            }
            return quote_spanned!(x.id.span()=> Self::#field(s)=>s.value as _)
        }
        );
        quote!(
            impl #enum_name{
                pub(crate) unsafe fn from_kind(kind:AttrKind) -> Self{
                    match kind{
                        #(
                            #match_field
                        ,)*
                        s=>unimplemented!("attribute '{:?}' not listed in NI-VISA document, so not supported yet",s)
                    }
                }

                pub(crate) fn mut_c_void(&mut self)->*mut ::std::ffi::c_void{
                    use super::AttrInner;
                    match self{
                        #(Self::#fields1(s)=>s.mut_c_void()),*
                    }
                }
                
                pub fn kind(&self)-> AttrKind{
                    use super::AttrInner;
                    match self{
                        #(Self::#fields2(s)=>s.kind()),*
                    }
                }

                pub(crate) fn as_attr_state(&self)-> vs::ViAttrState{
                    match self{
                        #(#as_attr_state),*
                    }
                }
            }
        )
        .to_tokens(tokens);
    }
}

fn match_ident(tar: &Ident, check: &Ident) {
    if check != tar {
        /* tar.span().unwrap().help("attribute name must match").emit();
        check
            .span()
            .unwrap()
            .error("attribute name must match")
            .emit(); */
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
        self.ty
            .attr_name
            .as_ref()
            .map(|n| match_ident(&self.id, &n));
        let Self { desc, ty, .. } = self;
        let id = self.struct_name();
        match ty.core {
            TypeCore::UnArch(ref ty) => {
                quote!(
                    #[doc= #desc]
                    #[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
                    pub struct #id{
                        value:vs::#ty
                    }
                    impl #id{
                        pub fn into_inner(self)->vs::#ty{
                            self.value
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
                        #[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
                        pub struct #id{
                            value:vs::#ty
                        }
                        #[cfg(target_arch = #arch)]
                        impl #id{
                            pub fn into_inner(self)->vs::#ty{
                                self.value
                            }
                        }
                    )*
                )
                .to_tokens(tokens);
            }
        }
    }
    fn constructors(&self, tokens: &mut TokenStream2) {
        let mut c = |n: &RangeCore| {
            n.check_attr_name(&self.id);
            let mut constructors = TokenStream2::new();
            n.to_constructor(&self.ty, &mut constructors);
            let struct_name = self.struct_name();
            quote!(
                impl #struct_name{
                    #constructors
                }
            )
            .to_tokens(tokens);
        };
        match self.range {
            Range::NoPort(ref n) => c(n),
            Range::Port(ref p) => {
                let n = RangeCore::merge_ranges(p.iter().map(|x| &x.core));
                c(&n);
            }
        }
    }
    fn kind_impl(&self, tokens: &mut TokenStream2) {
        let struct_id = self.struct_name();
        struct_name_to_kind_name(&self.id).for_each(|(kind_id, cfg)| {
            let kind_id = subst_ident(kind_id);
            quote_spanned!(self.id.span()=>
                    #cfg
                    impl super::AttrInner for #struct_id{
                        const KIND:AttrKind=AttrKind::#kind_id;
                        unsafe fn zero() -> Self {
                            Self{value:0 as _}
                        }
                        fn mut_c_void(&mut self)->*mut ::std::ffi::c_void{
                            &mut self.value as *mut _ as _
                        }
                    }
            )
            .to_tokens(tokens)
        });
    }

    fn default_impl(&self, tokens: &mut TokenStream2) {
        let struct_name = self.struct_name();
        let mut c = |n: &RangeCore| {
            if let Some(def) = n.default.default_expr() {
                quote!(
                    impl ::std::default::Default for #struct_name{
                        fn default() -> Self {
                             Self{value:#def}
                        }
                    }
                )
                .to_tokens(tokens);
            }
        };
        match self.range {
            Range::NoPort(ref n) => c(n),
            Range::Port(ref p) => {
                let n = RangeCore::merge_ranges(p.iter().map(|x| &x.core));
                c(&n);
            }
        }
    }
}

fn struct_name_to_kind_name(id: &Ident) -> impl Iterator<Item = (Ident, TokenStream2)> {
    let kind_id = id;
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

        return vec![
            (kind_id_x64, quote!(#[cfg(target_arch = "x86_64")])),
            (kind_id_x32, quote!(#[cfg(target_arch = "x86")])),
        ]
        .into_iter();
    } else {
        let kind_id = if let Some(new_id) = kind_id_str.strip_suffix("_32") {
            Ident::new(new_id, kind_id.span())
        } else if let Some(new_id) = kind_id_str.strip_suffix("_64") {
            Ident::new(new_id, kind_id.span())
        } else {
            kind_id.clone()
        };
        let kind_id = subst_ident(kind_id);
        return vec![(kind_id, TokenStream2::new())].into_iter();
    }
}

pub struct Type {
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
            if match_tokens(input, "-bit applications").is_none() {
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
                if match_tokens(input, "-bit applications").is_none() {
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
