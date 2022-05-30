use syn::{
    parse::{Parse, ParseStream, Result},
    Ident, Token, LitInt, parenthesized, Error,
};

fn is_not_specified(input: ParseStream) -> bool {
    let fork = input.fork();
    if !fork.parse::<Ident>().is_ok_and(|n| n == "Not") {
        return false;
    }
    if !fork.parse::<Ident>().is_ok_and(|a| a == "specified") {
        return false;
    }
    input.parse::<Ident>().unwrap();
    input.parse::<Ident>().unwrap();
    true
}

pub enum Range {
    TarSpec((ArchRaStm, ArchRaStm)),
    NoTarSpec(UnArchRaStm),
}

enum RangeStream {
    UnArch(UnArchRaStm),
    Arch(ArchRaStm),
}

//unarch specific range stream
pub enum UnArchRaStm {
    NA,
    Unreachable,
    Stream(Vec<RangeTree>),
}

//arch specific range stream
pub struct ArchRaStm {
    stream: UnArchRaStm,
    arch: LitInt,
}

enum RangeTree {
    Single(Bound),
    Bounded((Bound, Bound)),
}

enum Bound {
    Ident { id: Ident, value: LitInt },
    Num(LitInt),
}

mod kw {
    syn::custom_keyword!(to);
}

impl Parse for Bound {
    fn parse(input: ParseStream) -> Result<Self> {
        let look = input.lookahead1();
        if look.peek(Ident) {
            let id = input.parse()?;
            let value;
            parenthesized!(value in input);
            let value = value.parse()?;
            Ok(Self::Ident { id, value })
        } else if look.peek(LitInt) {
            Ok(Self::Num(input.parse()?))
        } else {
            Err(look.error())
        }
    }
}

impl Parse for RangeTree {
    fn parse(input: ParseStream) -> Result<Self> {
        let look = input.lookahead1();
        if input.peek2(kw::to) {
            let lower = input.parse()?;
            input.parse::<kw::to>()?;
            let upper = input.parse()?;
            Ok(Self::Bounded((lower, upper)))
        } else if look.peek(Ident) || look.peek(LitInt) {
            Ok(Self::Single(input.parse()?))
        } else {
            Err(look.error())
        }
    }
}

impl Parse for RangeStream {
    fn parse(input: ParseStream) -> Result<Self> {
        fn arch_spec(input: ParseStream) -> Result<Option<LitInt>> {
            if input.peek(Token![for]) {
                let arch: LitInt = input.parse()?;
                input.parse::<Token![-]>()?;
                let bit = input.parse::<Ident>()?;
                if bit != "bit" {
                    return Err(Error::new_spanned(bit, "expected 'bit'"));
                }
                let app = input.parse::<Ident>()?;
                if app != "applications" {
                    return Err(Error::new_spanned(bit, "expected 'applications'"));
                }
                return Ok(arch.into());
            } else {
                Ok(None)
            }
        }
        if crate::is_na(input) {
            let range = UnArchRaStm::NA;
            Ok(match arch_spec(input)? {
                None => Self::UnArch(range),
                Some(a) => Self::Arch(ArchRaStm {
                    arch: a,
                    stream: range,
                }),
            })
        } else if is_not_specified(input) {
            let range = UnArchRaStm::Unreachable;
            Ok(match arch_spec(input)? {
                None => Self::UnArch(range),
                Some(a) => Self::Arch(ArchRaStm {
                    arch: a,
                    stream: range,
                }),
            })
        } else {
            let mut ret = Vec::new();
            while !input.is_empty() && !input.peek(Token![>]) {
                if let Some(arch) = arch_spec(input)? {
                    return Ok(Self::Arch(ArchRaStm {
                        stream: UnArchRaStm::Stream(ret),
                        arch,
                    }));
                } else {
                    ret.push(input.parse()?);
                }
                input.parse::<Option<Token![;]>>()?;
                input.parse::<Option<Token![,]>>()?;
            }
            Ok(Self::UnArch(UnArchRaStm::Stream(ret)))
        }
    }
}

impl Parse for Range {
    fn parse(input: ParseStream) -> Result<Self> {
        match input.parse::<RangeStream>()? {
            RangeStream::UnArch(r) => Ok(Self::NoTarSpec(r)),
            RangeStream::Arch(a1) => {
                let a2 = input.parse::<RangeStream>()?;
                if let RangeStream::Arch(a2) = a2 {
                    Ok(Self::TarSpec((a1, a2)))
                } else {
                    Err(input.error("expected another arch specification"))
                }
            }
        }
    }
}
