use proc_macro2::{Delimiter, Ident, Span, TokenStream as TokenStream2, TokenTree};
use quote::{quote_spanned, ToTokens};
use syn::{parse::Parse, Path, Result, Token};

use crate::rusty_ident::NestedMacros;

pub struct Input {
    macs: Option<Vec<Path>>,
    exc: Option<Token![!]>,
    inner: AttrProcessed,
}

impl Parse for Input {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let fork = input.fork();
        if let Ok(_) = fork.parse::<NestedMacros>() {
            let nest_macros: NestedMacros = input.parse()?;
            Ok(Self {
                macs: nest_macros.macs.into(),
                exc: nest_macros.exc.into(),
                inner: syn::parse2(nest_macros.body.content)?,
            })
        } else {
            Ok(Self {
                macs: None,
                exc: None,
                inner: input.parse()?,
            })
        }
    }
}

impl ToTokens for Input {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self { macs, exc, inner } = self;
        match (macs, exc) {
            (Some(macs), Some(exc)) => {
                let mut ret = inner.to_token_stream();
                for mac in macs.iter().rev() {
                    let mut m = mac.to_token_stream();
                    exc.to_tokens(&mut m);
                    proc_macro2::Group::new(Delimiter::Brace, ret).to_tokens(&mut m);
                    ret = m;
                }
                ret.to_tokens(tokens)
            }
            _ => inner.to_tokens(tokens),
        }
    }
}

pub struct AttrProcessed(TokenStream2);

impl ToTokens for AttrProcessed {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        self.0.to_tokens(tokens)
    }
}

impl Parse for AttrProcessed {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let mut inner = TokenStream2::new();
        while !input.is_empty() {
            if let Some(ty) = extract_repr_attribute(input, &mut inner)? {
                map_to_repr(ty).to_tokens(&mut inner);
            } else {
                input.parse::<TokenTree>()?.to_tokens(&mut inner);
            }
        }
        Ok(Self(inner))
    }
}

fn extract_repr_attribute(
    input: syn::parse::ParseStream,
    tokens: &mut TokenStream2,
) -> Result<Option<Ident>> {
    let fork = input.fork();
    use syn::parse::discouraged::Speculative;
    let mut ret = None;
    if fork.peek(Token![#]) {
        if let Ok(attr) = fork.call(syn::Attribute::parse_outer) {
            let mut iter = attr.into_iter();
            for a in &mut iter {
                if a.path.is_ident("repr") {
                    input.advance_to(&fork);
                    let group: proc_macro2::Group = syn::parse2(a.tokens)?;
                    ret = Some(syn::parse2(group.stream())?);
                    break;
                } else {
                    a.to_tokens(tokens);
                }
            }
            iter.for_each(|a| a.to_tokens(tokens));
            input.advance_to(&fork);
        }
    }
    Ok(ret)
}

fn map_to_repr(ty: Ident) -> TokenStream2 {
    use visa_sys as vs;
    let align = if ty == "ViEventType" {
        unsigned_ty_token::<vs::ViEventType>(ty.span())
    } else if ty == "ViUInt16" {
        unsigned_ty_token::<vs::ViUInt16>(ty.span())
    } else if ty == "ViEvent" {
        unsigned_ty_token::<vs::ViEvent>(ty.span())
    } else if ty == "ViEventType" {
        unsigned_ty_token::<vs::ViEventType>(ty.span())
    } else if ty == "ViEventFilter" {
        unsigned_ty_token::<vs::ViEventFilter>(ty.span())
    } else if ty == "ViAttr" {
        unsigned_ty_token::<vs::ViAttr>(ty.span())
    } else if ty == "ViStatus" {
        signed_ty_token::<vs::ViStatus>(ty.span())
    } else if ty == "ViUInt32" {
        signed_ty_token::<vs::ViUInt32>(ty.span())
    } else {
        unimplemented!("{}", ty.to_string())
    };
    quote_spanned!(ty.span()=>#[repr(#align)])
}

fn unsigned_ty_token<T: Sized>(span: Span) -> Ident {
    use std::mem::size_of;
    let t = size_of::<T>();
    if t == size_of::<u8>() {
        Ident::new("u8", span)
    } else if t == size_of::<u16>() {
        Ident::new("u16", span)
    } else if t == size_of::<u32>() {
        Ident::new("u32", span)
    } else if t == size_of::<u64>() {
        Ident::new("u64", span)
    } else if t == size_of::<u128>() {
        Ident::new("u128", span)
    } else {
        unimplemented!()
    }
}

fn signed_ty_token<T: Sized>(span: Span) -> Ident {
    use std::mem::size_of;
    let t = size_of::<T>();
    if t == size_of::<i8>() {
        Ident::new("i8", span)
    } else if t == size_of::<i16>() {
        Ident::new("i16", span)
    } else if t == size_of::<i32>() {
        Ident::new("i32", span)
    } else if t == size_of::<i64>() {
        Ident::new("i64", span)
    } else if t == size_of::<i128>() {
        Ident::new("i128", span)
    } else {
        unimplemented!()
    }
}

/// copied from visa-sys. If add visa-sys as a dependency,
/// would failed linking when running macros in visa-rs
mod visa_sys {
    #![allow(non_camel_case_types)]
    #![allow(unused)]
    pub type __builtin_va_list = *mut ::std::os::raw::c_char;

    pub type va_list = __builtin_va_list;
    pub type __gnuc_va_list = __builtin_va_list;
    pub type ViUInt64 = ::std::os::raw::c_ulonglong;
    pub type ViInt64 = ::std::os::raw::c_longlong;
    pub type ViPUInt64 = *mut ViUInt64;
    pub type ViAUInt64 = *mut ViUInt64;
    pub type ViPInt64 = *mut ViInt64;
    pub type ViAInt64 = *mut ViInt64;
    pub type ViUInt32 = ::std::os::raw::c_ulong;
    pub type ViPUInt32 = *mut ViUInt32;
    pub type ViAUInt32 = *mut ViUInt32;
    pub type ViInt32 = ::std::os::raw::c_long;
    pub type ViPInt32 = *mut ViInt32;
    pub type ViAInt32 = *mut ViInt32;
    pub type ViUInt16 = ::std::os::raw::c_ushort;
    pub type ViPUInt16 = *mut ViUInt16;
    pub type ViAUInt16 = *mut ViUInt16;
    pub type ViInt16 = ::std::os::raw::c_short;
    pub type ViPInt16 = *mut ViInt16;
    pub type ViAInt16 = *mut ViInt16;
    pub type ViUInt8 = ::std::os::raw::c_uchar;
    pub type ViPUInt8 = *mut ViUInt8;
    pub type ViAUInt8 = *mut ViUInt8;
    pub type ViInt8 = ::std::os::raw::c_schar;
    pub type ViPInt8 = *mut ViInt8;
    pub type ViAInt8 = *mut ViInt8;
    pub type ViChar = ::std::os::raw::c_char;
    pub type ViPChar = *mut ViChar;
    pub type ViAChar = *mut ViChar;
    pub type ViByte = ::std::os::raw::c_uchar;
    pub type ViPByte = *mut ViByte;
    pub type ViAByte = *mut ViByte;
    pub type ViAddr = *mut ::std::os::raw::c_void;
    pub type ViPAddr = *mut ViAddr;
    pub type ViAAddr = *mut ViAddr;
    pub type ViReal32 = f32;
    pub type ViPReal32 = *mut ViReal32;
    pub type ViAReal32 = *mut ViReal32;
    pub type ViReal64 = f64;
    pub type ViPReal64 = *mut ViReal64;
    pub type ViAReal64 = *mut ViReal64;
    pub type ViBuf = ViPByte;
    pub type ViConstBuf = *const ViByte;
    pub type ViPBuf = ViPByte;
    pub type ViABuf = *mut ViPByte;
    pub type ViString = ViPChar;
    pub type ViConstString = *const ViChar;
    pub type ViPString = ViPChar;
    pub type ViAString = *mut ViPChar;
    pub type ViRsrc = ViString;
    pub type ViConstRsrc = ViConstString;
    pub type ViPRsrc = ViString;
    pub type ViARsrc = *mut ViString;
    pub type ViBoolean = ViUInt16;
    pub type ViPBoolean = *mut ViBoolean;
    pub type ViABoolean = *mut ViBoolean;
    pub type ViStatus = ViInt32;
    pub type ViPStatus = *mut ViStatus;
    pub type ViAStatus = *mut ViStatus;
    pub type ViVersion = ViUInt32;
    pub type ViPVersion = *mut ViVersion;
    pub type ViAVersion = *mut ViVersion;
    pub type ViObject = ViUInt32;
    pub type ViPObject = *mut ViObject;
    pub type ViAObject = *mut ViObject;
    pub type ViSession = ViObject;
    pub type ViPSession = *mut ViSession;
    pub type ViASession = *mut ViSession;
    pub type ViAttr = ViUInt32;
    pub type ViEvent = ViObject;
    pub type ViPEvent = *mut ViEvent;
    pub type ViFindList = ViObject;
    pub type ViPFindList = *mut ViFindList;
    pub type ViBusAddress = ViUInt64;
    pub type ViBusSize = ViUInt64;
    pub type ViAttrState = ViUInt64;
    pub type ViBusAddress64 = ViUInt64;
    pub type ViPBusAddress64 = *mut ViBusAddress64;
    pub type ViEventType = ViUInt32;
    pub type ViPEventType = *mut ViEventType;
    pub type ViAEventType = *mut ViEventType;
    pub type ViPAttrState = *mut ::std::os::raw::c_void;
    pub type ViPAttr = *mut ViAttr;
    pub type ViAAttr = *mut ViAttr;
    pub type ViKeyId = ViString;
    pub type ViConstKeyId = ViConstString;
    pub type ViPKeyId = ViPString;
    pub type ViJobId = ViUInt32;
    pub type ViPJobId = *mut ViJobId;
    pub type ViAccessMode = ViUInt32;
    pub type ViPAccessMode = *mut ViAccessMode;
    pub type ViPBusAddress = *mut ViBusAddress;
    pub type ViEventFilter = ViUInt32;
    pub type ViVAList = va_list;
    pub type ViHndlr = ::std::option::Option<
        unsafe extern "system" fn(
            vi: ViSession,
            eventType: ViEventType,
            event: ViEvent,
            userHandle: ViAddr,
        ) -> ViStatus,
    >;
}
