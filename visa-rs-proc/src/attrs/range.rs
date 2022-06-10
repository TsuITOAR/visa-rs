use std::{hash::Hash, str::FromStr};

use proc_macro2::{Span, TokenStream as TokenStream2, TokenTree};
use quote::{quote, quote_spanned, ToTokens};
use syn::{
    braced, parenthesized,
    parse::{Parse, ParseStream, Result},
    spanned::Spanned,
    token::Paren,
    Ident, LitInt, Token,
};

use crate::match_tokens;

use super::Type;

fn is_na(input: ParseStream) -> Option<Span> {
    match_tokens(input, "N/A")
}
fn is_not_specified(input: ParseStream) -> Option<Span> {
    match_tokens(input, "Not specified")
}

mod kw {
    use syn::custom_keyword;
    custom_keyword!(to);
}

#[derive(Clone)]
pub enum DefaultValue {
    Num(LitInt),
    Ident(Ident),
    Key {
        key_name: Vec<TokenTree>,
        char: LitInt,
    },
    NumDesc {
        num: LitInt,
        desc: TokenTree,
    },
    NA(Span),
}

impl DefaultValue {
    pub fn default_expr(&self) -> Option<TokenStream2> {
        match self {
            DefaultValue::Num(n) => n.to_token_stream(),
            DefaultValue::Ident(i) => quote!(Self::#i.value),
            DefaultValue::Key { char, .. } => char.to_token_stream(),
            DefaultValue::NumDesc { num, .. } => num.to_token_stream(),
            DefaultValue::NA(_) => return None,
        }
        .into()
    }
}

impl PartialEq for DefaultValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (DefaultValue::Num(a), DefaultValue::Num(b)) => a.base10_digits() == b.base10_digits(),

            (DefaultValue::Ident(a), DefaultValue::Ident(b)) => a == b,

            (DefaultValue::Key { char: ca, .. }, DefaultValue::Key { char: cb, .. }) => {
                ca.base10_digits() == cb.base10_digits()
            }

            (
                DefaultValue::NumDesc { num: na, desc: da },
                DefaultValue::NumDesc { num: nb, desc: db },
            ) => na.base10_digits() == nb.base10_digits() && da.to_string() == db.to_string(),

            (DefaultValue::NA(_), DefaultValue::NA(_)) => true,
            _ => false,
        }
    }
}

impl Eq for DefaultValue {}

impl DefaultValue {
    fn source_span(&self) -> Span {
        match self {
            DefaultValue::Num(n) => n.span(),
            DefaultValue::Ident(n) => n.span(),
            DefaultValue::Key { char, .. } => char.span(),
            DefaultValue::NumDesc { num, .. } => num.span(),
            DefaultValue::NA(s) => s.clone(),
        }
    }
}

impl Parse for DefaultValue {
    fn parse(input: ParseStream) -> Result<Self> {
        if let Some(span) = is_na(input) {
            Ok(Self::NA(span))
        } else {
            let look = input.lookahead1();
            if look.peek(LitInt) {
                let n = input.parse()?;
                if input.peek(Paren) {
                    let c;
                    parenthesized!(c in input);
                    Ok(Self::NumDesc {
                        num: n,
                        desc: c.parse()?,
                    })
                } else {
                    Ok(Self::Num(n))
                }
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

pub enum Port {
    PXI,
    Serial,
    GPIB,
    VXI,
    TCPIP,
    USBRaw,
    USBInstr,
}

impl FromStr for Port {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        use Port::*;
        Ok(match s {
            "PXI" => PXI,
            "Serial" => Serial,
            "GPIB" => GPIB,
            "VXI" => VXI,
            "TCPIP" => TCPIP,
            "USB RAW" => USBRaw,
            "USB INSTR" => USBInstr,
            _ => return Err(Self::Err::from("unknown port")),
        })
    }
}

impl Parse for Port {
    fn parse(input: ParseStream) -> Result<Self> {
        const PORT: [&str; 7] = [
            "PXI",
            "Serial",
            "GPIB",
            "VXI",
            "TCPIP",
            "USB RAW",
            "USB INSTR",
        ];
        for p in PORT.iter() {
            if match_tokens(input, *p).is_some() {
                return Ok(Self::from_str(*p).unwrap());
            }
        }
        Err(input.error("Unknown port"))
    }
}

pub enum Range {
    NoPort(RangeCore),
    Port(Vec<PortRange>),
}

impl Parse for Range {
    fn parse(input: ParseStream) -> Result<Self> {
        let look = input.lookahead1();
        if look.peek(Token![while]) {
            let mut r = Vec::new();
            while !input.is_empty() {
                r.push(input.parse()?);
            }
            Ok(Range::Port(r))
        } else if look.peek(Token![static]) {
            Ok(Range::NoPort(input.parse()?))
        } else {
            Err(look.error())
        }
    }
}

pub struct PortRange {
    pub(crate) core: RangeCore,
    pub(crate) _port: Port,
}

impl Parse for PortRange {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![while]>()?;
        let port = input.parse()?;
        let c;
        braced!(c in input);
        Ok(Self {
            _port: port,
            core: c.parse()?,
        })
    }
}

pub struct RangeCore {
    pub(crate) default: DefaultValue,
    pub(crate) attr_name: Option<Ident>,
    pub(crate) bound: Bound,
}

impl RangeCore {
    pub fn merge_ranges<'a>(ranges: impl Iterator<Item = &'a Self>) -> Self {
        let (ranges, default) = ranges.fold(
            (std::collections::HashSet::new(), None),
            |mut init, item| {
                if let Bound::NoArch(ref n) = item.bound {
                    if let BoundCore::Stream(bounds) = n {
                        init.0.extend(bounds.into_iter());
                        if init.1.as_ref().map(|x| x != &item.default).unwrap_or(false) {
                            item.default
                                .source_span()
                                .unwrap()
                                .error("defaults of different protocols should be same")
                                .emit();
                        }
                        init.1.replace(item.default.clone());
                    } else {
                        unreachable!()
                    }
                    init
                } else {
                    unreachable!()
                }
            },
        );
        let ranges: Vec<_> = ranges.into_iter().cloned().collect();
        let default = default.unwrap();
        Self {
            default,
            attr_name: None,
            bound: Bound::NoArch(BoundCore::Stream(ranges)),
        }
    }
    pub fn check_attr_name(&self, tar: &Ident) {
        self.attr_name.as_ref().map(|n| super::match_ident(tar, n));
    }
}

impl RangeCore {
    pub fn to_constructor(&self, ty: &Type, tokens: &mut proc_macro2::TokenStream) {
        match self.bound {
            Bound::Arch(ref arch_bound) => match ty.core {
                super::TypeCore::Arch(ref arch_ty) => {
                    arch_bound
                        .iter()
                        .zip(arch_ty.iter())
                        .for_each(|(bound, tya)| {
                            assert!(
                                tya.arch.base10_parse::<u8>().unwrap()
                                    == bound.arch.base10_parse::<u8>().unwrap()
                            );
                            if let Ok(64) = bound.arch.base10_parse() {
                                let cfg = quote!(#[cfg(target_arch = "x86_64")]);
                                bound.core.to_constructor(&tya.core, &cfg.into(), tokens);
                            } else if let Ok(32) = bound.arch.base10_parse() {
                                let cfg = quote!(#[cfg(target_arch = "x86")]);
                                bound.core.to_constructor(&tya.core, &cfg.into(), tokens);
                            }
                        });
                }
                super::TypeCore::UnArch(ref tyu) => arch_bound.iter().for_each(|bound| {
                    if let Ok(64) = bound.arch.base10_parse() {
                        let cfg = quote!(#[cfg(target_arch = "x86_64")]);
                        bound.core.to_constructor(&tyu, &cfg.into(), tokens);
                    } else if let Ok(32) = bound.arch.base10_parse() {
                        let cfg = quote!(#[cfg(target_arch = "x86")]);
                        bound.core.to_constructor(&tyu, &cfg.into(), tokens);
                    }
                }),
            },
            Bound::NoArch(ref n) => match ty.core {
                super::TypeCore::Arch(ref arch_ty) => arch_ty.iter().for_each(|tya| {
                    if let Ok(64) = tya.arch.base10_parse() {
                        let cfg = quote!(#[cfg(target_arch = "x86_64")]);
                        n.to_constructor(&tya.core, &cfg.into(), tokens);
                    } else if let Ok(32) = tya.arch.base10_parse() {
                        let cfg = quote!(#[cfg(target_arch = "x86")]);
                        n.to_constructor(&tya.core, &cfg.into(), tokens);
                    }
                }),
                super::TypeCore::UnArch(ref u) => n.to_constructor(u, &None, tokens),
            },
        }
    }
}

impl Parse for RangeCore {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![static]>()?;
        input.parse::<Token![as]>()?;
        let default = input.parse()?;
        input.parse::<Token![in]>()?;
        let ident;
        if input.peek2(Token![:]) {
            ident = Some(input.parse()?);
            input.parse::<Token![:]>()?;
        } else {
            ident = None;
        };
        Ok(Self {
            default,
            attr_name: ident,
            bound: input.parse()?,
        })
    }
}

pub enum Bound {
    Arch(Vec<ArchBound>),
    NoArch(BoundCore),
}

impl Parse for Bound {
    fn parse(input: ParseStream) -> Result<Self> {
        let core: BoundCore = input.parse()?;
        if input.peek(Token![for]) {
            input.parse::<Token![for]>()?;
            let arch: LitInt = input.parse()?;
            if match_tokens(input, "-bit applications").is_none() {
                return Err(input.error("expected '-bit applications' after architecture"));
            }
            let mut ret = vec![ArchBound { arch, core }];
            while !input.is_empty() {
                let core = input.parse()?;
                input.parse::<Token![for]>()?;
                ret.push(ArchBound {
                    core,
                    arch: input.parse()?,
                });
                if match_tokens(input, "-bit applications").is_none() {
                    return Err(input.error("expected '-bit applications' after architecture"));
                }
            }
            return Ok(Bound::Arch(ret));
        } else {
            Ok(Bound::NoArch(core))
        }
    }
}

pub struct ArchBound {
    core: BoundCore,
    arch: LitInt,
}

pub enum BoundCore {
    NA(Span),
    Unreachable(Span),
    Stream(Vec<BoundItem>),
}

impl BoundCore {
    fn to_constructor(&self, ty: &Ident, cfg: &Option<TokenStream2>, tokens: &mut TokenStream2) {
        let new_uncheck = quote_spanned!( ty.span()=>
            #cfg
            pub unsafe fn new_unchecked(value:vs::#ty)->Self{
                Self{value}
            }
        );
        let mut new = None;
        let mut new_check = None;
        match self {
            BoundCore::NA(_) => {
                new = quote_spanned!(ty.span()=>
                    #cfg
                    pub fn new(value:vs::#ty)->Self{
                        Self{value}
                    }
                )
                .into();
                new_check = quote_spanned!(ty.span()=>
                    #cfg
                    pub fn new_checked(value:vs::#ty)->Option<Self>{
                        Some(Self{value})
                    }
                )
                .into();
            }
            BoundCore::Unreachable(_) => (),
            BoundCore::Stream(s) => {
                s.iter()
                    .for_each(|x| x.sub_constructor(ty, cfg).to_tokens(tokens));
                let checks = s.iter().map(|x| x.check_range(ty));
                new_check = quote_spanned!(ty.span()=>
                    #cfg
                    #[allow(unused_parens)]
                    pub fn new_checked(value:vs::#ty)->Option<Self>{
                        if #(#checks)||*{
                            Some(Self{value})
                        }else{
                            None
                        }
                    }
                )
                .into()
            }
        }
        quote!(
            #new
            #new_uncheck
            #new_check
        )
        .to_tokens(tokens);
    }
}

impl Parse for BoundCore {
    fn parse(input: ParseStream) -> Result<Self> {
        if let Some(span) = is_na(input) {
            Ok(Self::NA(span))
        } else if let Some(span) = is_not_specified(input) {
            Ok(Self::Unreachable(span))
        } else {
            let mut ret = Vec::new();
            while !input.is_empty() && !input.peek(Token![for]) {
                ret.push(input.parse()?);
                input.parse::<Option<Token![;]>>()?;
                input.parse::<Option<Token![,]>>()?;
            }
            input.parse::<Option<Token![;]>>()?;
            input.parse::<Option<Token![,]>>()?;
            Ok(Self::Stream(ret))
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub enum BoundItem {
    Single(BoundToken),
    Range((BoundToken, BoundToken)),
    NamedRange {
        name: Ident,
        range: (BoundToken, BoundToken),
    },
}

impl BoundItem {
    fn check_range(&self, ty: &Ident) -> TokenStream2 {
        match self {
            BoundItem::Single(s) => quote_spanned!(s.span()=>#s as vs::#ty == value),
            BoundItem::Range((l, h)) => {
                quote_spanned!(
                    l.span().join(h.span()).unwrap()=>
                    (#l as vs::#ty <= value && value <= #h as vs::#ty)
                )
            }
            BoundItem::NamedRange {
                range: (l, h),
                name,
            } => {
                quote_spanned!(
                        name.span().join(l.span().join(h.span()).unwrap()).unwrap()=>
                    (#l as vs::#ty<=value && value<=#h as vs::#ty)
                )
            }
        }
    }
    fn sub_constructor(&self, ty: &Ident, cfg: &Option<TokenStream2>) -> Option<TokenStream2> {
        match self {
            BoundItem::Single(s @ BoundToken::Ident { id, .. }) => quote!(
                #cfg
                pub const #id: Self = Self { value: #s as _};
            )
            .into(),
            BoundItem::NamedRange { name, .. } => {
                let method_name = Ident::new(
                    &name.to_string().chars().fold(String::new(), |mut i, x| {
                        if x.is_ascii_uppercase() && !i.is_empty() {
                            i.push('_');
                            i.push(x.to_ascii_lowercase());
                        } else {
                            i.push(x.to_ascii_lowercase());
                        }
                        i
                    }),
                    name.span(),
                );
                let check = self.check_range(ty);
                quote!(
                    #cfg
                    #[allow(unused_parens)]
                    pub fn #method_name(value:vs::#ty)->Option<Self>{
                        if #check{
                            Some(Self{value})
                        }else{None}
                    }
                )
                .into()
            }
            BoundItem::Range(r) => match r {
                (BoundToken::Ident { id: id_l, .. }, BoundToken::Ident { id: id_h, .. })
                    if id_l.to_string().ends_with(char::is_numeric)
                        && id_h.to_string().ends_with(char::is_numeric)
                        && id_l.to_string().trim_end_matches(char::is_numeric)
                            == id_h.to_string().trim_end_matches(char::is_numeric) =>
                {
                    let span = id_h.span();
                    let id_h = id_h.to_string();
                    let id_l = id_l.to_string();
                    let const_prefix = id_h.trim_end_matches(char::is_numeric).to_ascii_lowercase();
                    let range = (id_l
                        .rmatches(char::is_numeric)
                        .next()
                        .unwrap()
                        .parse::<u8>()
                        .unwrap())
                        ..(id_h
                            .rmatches(char::is_numeric)
                            .next()
                            .unwrap()
                            .parse()
                            .unwrap());
                    let range = range
                        .into_iter()
                        .map(|x| Ident::new(&format!("{}{:1}", const_prefix, x), span));
                    let method_name = Ident::new(&const_prefix.to_ascii_lowercase(), span);
                    let check = self.check_range(ty);
                    quote!(

                        #(
                            #cfg
                            pub const #range:Self = Self { value: vs::#range as _};
                        )*
                        #cfg
                        fn #method_name(value:vs::#ty)->Option<Self>{
                            if #check{
                                Some(Self{value})
                            }else{
                                None
                            }
                        }
                    )
                    .into()
                }
                (BoundToken::Num(_), BoundToken::Num(_)) => None,
                other => {
                    other.0.span().unwrap().note("unexpected pattern").emit();
                    other.1.span().unwrap().note("unexpected pattern").emit();
                    None
                }
            },
            _ => None,
        }
    }
}

impl Parse for BoundItem {
    fn parse(input: ParseStream) -> Result<Self> {
        let look = input.lookahead1();
        if look.peek(LitInt) {
            let s = input.parse()?;
            if input.peek(kw::to) {
                input.parse::<kw::to>()?;
                Ok(BoundItem::Range((s, input.parse()?)))
            } else {
                Ok(BoundItem::Single(s))
            }
        } else if look.peek(Ident) {
            let id: Ident = input.parse()?;
            let look = input.lookahead1();
            if look.peek(Paren) {
                let c;
                parenthesized!(c in input);
                if c.peek2(kw::to) {
                    let b = c.parse()?;
                    c.parse::<kw::to>()?;
                    return Ok(BoundItem::NamedRange {
                        name: id,
                        range: (b, c.parse()?),
                    });
                } else {
                    let b: LitInt = c.parse()?;
                    if input.peek(kw::to) {
                        input.parse::<kw::to>()?;
                        return Ok(BoundItem::Range((BoundToken::Num(b), input.parse()?)));
                    } else {
                        return Ok(BoundItem::Single(BoundToken::Ident { id, value: Some(b) }));
                    }
                }
            } else if id == "VI_TRUE" || id == "VI_FALSE" {
                return Ok(BoundItem::Single(BoundToken::Ident { id, value: None }));
            } else {
                Err(look.error())
            }
        } else {
            Err(look.error())
        }
    }
}
#[derive(Clone)]
pub enum BoundToken {
    Ident { id: Ident, value: Option<LitInt> },
    Num(LitInt),
}

impl PartialEq for BoundToken {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                BoundToken::Ident {
                    id: id1,
                    value: value1,
                },
                BoundToken::Ident {
                    id: id2,
                    value: value2,
                },
            ) => {
                id1 == id2
                    && value1.as_ref().map(|x| x.base10_digits())
                        == value2.as_ref().map(|x| x.base10_digits())
            }

            (BoundToken::Num(a), BoundToken::Num(b)) => a.base10_digits() == b.base10_digits(),
            _ => false,
        }
    }
}

impl Eq for BoundToken {}

impl Hash for BoundToken {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            BoundToken::Ident { id, value } => {
                id.hash(state);
                value.as_ref().map(|x| x.base10_digits()).hash(state);
            }
            BoundToken::Num(n) => n.base10_digits().hash(state),
        }
    }
}

impl ToTokens for BoundToken {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            BoundToken::Ident { id, value } => {
                let span = id.span().join(value.span()).unwrap();
                value
                    .as_ref()
                    .map(|x| quote_spanned!(span=> #x))
                    .unwrap_or(quote_spanned!(span=> vs::#id))
            }
            .to_tokens(tokens),
            BoundToken::Num(v) => quote_spanned!(v.span()=>#v).to_tokens(tokens),
        }
    }
}

impl Parse for BoundToken {
    fn parse(input: ParseStream) -> Result<Self> {
        let look = input.lookahead1();
        if look.peek(Ident) {
            let id = input.parse()?;
            let look = input.lookahead1();
            let value = if look.peek(Paren) {
                let value;
                parenthesized!(value in input);
                Some(value.parse()?)
            } else if id == "VI_TRUE" || id == "VI_FALSE" {
                None
            } else {
                return Err(look.error());
            };
            Ok(Self::Ident { id, value })
        } else if look.peek(LitInt) {
            Ok(Self::Num(input.parse()?))
        } else {
            Err(look.error())
        }
    }
}
