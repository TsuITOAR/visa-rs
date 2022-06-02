use std::str::FromStr;

use proc_macro2::{TokenStream, TokenTree};
use syn::{
    braced, parenthesized,
    parse::{Parse, ParseStream, Result},
    token::Paren,
    Ident, LitInt, Token,
};

use crate::match_tokens;


fn is_na(input: ParseStream) -> bool {
    match_tokens(input, "N/A")
}
fn is_not_specified(input: ParseStream) -> bool {
    match_tokens(input, "Not specified")
}

mod kw {
    use syn::custom_keyword;
    custom_keyword!(to);
}

enum DefaultValue {
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
    NA,
}

impl Parse for DefaultValue {
    fn parse(input: ParseStream) -> Result<Self> {
        if is_na(input) {
            Ok(Self::NA)
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
            if match_tokens(input, *p) {
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
    core: RangeCore,
    port: Port,
}

impl Parse for PortRange {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![while]>()?;
        let port = input.parse()?;
        let c;
        braced!(c in input);
        Ok(Self {
            port,
            core: c.parse()?,
        })
    }
}

pub struct RangeCore {
    default: DefaultValue,
    attr_name: Option<Ident>,
    bound: Bound,
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
            if !match_tokens(input, "-bit applications") {
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
                if !match_tokens(input, "-bit applications") {
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
    NA,
    Unreachable,
    Stream(Vec<BoundItem>),
}

impl Parse for BoundCore {
    fn parse(input: ParseStream) -> Result<Self> {
        if is_na(input) {
            Ok(Self::NA)
        } else if is_not_specified(input) {
            Ok(Self::Unreachable)
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

pub enum BoundItem {
    Single(BoundToken),
    Range((BoundToken, BoundToken)),
    NamedRange {
        name: Ident,
        range: (BoundToken, BoundToken),
    },
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
            let n: Ident = input.parse()?;
            if input.peek(Paren) {
                let c;
                parenthesized!(c in input);
                if c.peek2(kw::to) {
                    let b = c.parse()?;
                    c.parse::<kw::to>()?;
                    return Ok(BoundItem::NamedRange {
                        name: n,
                        range: (b, c.parse()?),
                    });
                } else {
                    let b: LitInt = c.parse()?;
                    if input.peek(kw::to) {
                        input.parse::<kw::to>()?;
                        return Ok(BoundItem::Range((BoundToken::Num(b), input.parse()?)));
                    } else {
                        return Ok(BoundItem::Single(BoundToken::Ident {
                            id: n,
                            value: Some(b),
                        }));
                    }
                }
            } else {
                return Ok(BoundItem::Single(BoundToken::Ident { id: n, value: None }));
            }
        } else {
            Err(look.error())
        }
    }
}

pub enum BoundToken {
    Ident { id: Ident, value: Option<LitInt> },
    Num(LitInt),
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
            } else if id == "ViTrue" || id == "ViFalse" {
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
