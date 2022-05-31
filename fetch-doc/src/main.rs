use std::{collections::HashMap, fmt::Display, io::Write};

use scraper::{Html, Selector};
use serde::Deserialize;

type Result<O> = std::result::Result<O, anyhow::Error>;

#[derive(Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct DocIndex {
    items: Vec<DocIndexItem>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct DocIndexItem {
    url: Option<String>,
    nav_path: Option<String>,
    bundle_id: String,
    #[serde(rename(deserialize = "outputclasses"))]
    output_classed: Vec<String>,
    id: Option<String>,
    title: String,
    #[serde(rename(deserialize = "childEntries"))]
    child_entries: Vec<DocIndexItem>,
}

impl DocIndex {
    async fn new(url: &str) -> Result<Self> {
        let response = reqwest::get(url).await?;
        let content: Vec<DocIndexItem> = response.json().await?;
        Ok(Self { items: content })
    }
    fn fetch_list<'a>(&'a self, target: &str) -> Option<impl Iterator<Item = &'a str>> {
        fn fetch_list_in<'a>(
            index: &'a DocIndexItem,
            target: &str,
        ) -> Option<impl Iterator<Item = &'a str>> {
            if index.title == target {
                return Some(
                    index
                        .child_entries
                        .iter()
                        .filter_map(|x| x.nav_path.as_ref().map(|x| x.as_str())),
                );
            } else {
                for child in &index.child_entries {
                    if let Some(s) = fetch_list_in(child, target) {
                        return Some(s);
                    }
                }
            }
            None
        }
        for item in &self.items {
            match fetch_list_in(item, target) {
                Some(e) => return Some(e),
                None => (),
            }
        }
        None
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DocFetcher {
    url: String,
    content: Html,
}

impl DocFetcher {
    async fn new(url: &str) -> Result<Self> {
        let data = reqwest::get(url).await?.text().await?;
        let response: serde_json::Value = serde_json::from_str(&data)?;
        let html = Html::parse_fragment(&response["topic_html"].as_str().unwrap());
        return Ok(Self {
            url: url.to_owned(),
            content: html,
        });
    }
    fn fetch_current<Item: FromHtml>(&self) -> Result<Vec<Item>> {
        Item::from_html(&self.content, &self.url)
    }
}

trait FromHtml: Sized {
    fn from_html(src: &Html, nav: &str) -> Result<Vec<Self>>;
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct DocItem<Other> {
    name: String,
    desc: String,
    other: Other,
}

impl<O: FromHtml> FromHtml for DocItem<O> {
    fn from_html(src: &Html, nav: &str) -> Result<Vec<Self>> {
        let name = extract_text(src, r"article >  article > h1:first-child", nav)?
            .join(" ")
            .to_owned();
        let desc = extract_text(src, r"article >  article > h2 ,article >  article > p", nav)?
            .into_iter()
            .skip_while(|x| !x.contains("Description"))
            .skip(1)
            .take_while(|x| !x.contains("Related Topics"))
            .collect::<Vec<_>>()
            .join(" ")
            .to_owned();
        let other = O::from_html(src, nav)?;
        let ret = name
            .split("/")
            .zip(other.into_iter())
            .map(|(name, other)| Self {
                name: name.to_owned(),
                desc: desc.clone(),
                other,
            })
            .collect();
        Ok(ret)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AttrOther {
    access: String,
    ty: String,
    ranges: ProtocolRange,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Range {
    range: String,
    default: String,
}

impl Display for Range {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "static as {} in {}", self.default, self.range)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProtocolRange {
    General(Range),
    Specific(HashMap<String, Range>),
}

impl Display for ProtocolRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::General(g) => write!(f, "[{}]", g),
            Self::Specific(s) => write!(
                f,
                "\n[{}\n]",
                s.into_iter()
                    .map(|(p, r)| format!("\n\twhile {} {{{}}}", p, r))
                    .collect::<String>(),
            ),
        }
    }
}

fn split_to_protocol(ranges: Vec<String>, defaults: Vec<String>) -> ProtocolRange {
    const PROTOCOL: [&str; 6] = ["GPIB", "VXI", "Serial", "TCPIP", "USB RAW", "USB INSTR"];
    const SEPARATORS: [char; 3] = [' ', ':', ','];
    if ranges.iter().any(|x| {
        PROTOCOL.iter().any(|y| {
            x.contains(&format!("{} ", y).as_str()) || x.contains(&format!("{}:", y).as_str())
        })
    }) {
        let mut ret = HashMap::new();
        let mut default = if defaults.len() == 1 {
            vec![defaults[0].clone(); ranges.len()].into_iter()
        } else {
            defaults.into_iter()
        };
        for r in ranges {
            let def = default.next().unwrap();
            let mut word = r.trim();
            let mut pro = Vec::new();
            'strip: loop {
                word = word.trim_matches(SEPARATORS.as_slice());
                for p in PROTOCOL {
                    if let Some(d) = word.strip_prefix(p) {
                        pro.push((p, def.clone()));
                        word = d;
                        continue 'strip;
                    }
                }
                break 'strip;
            }
            word = word.trim_matches(SEPARATORS.as_slice());
            for (p, d) in pro {
                assert!(ret
                    .insert(
                        p.to_owned(),
                        Range {
                            range: word.to_owned(),
                            default: d
                        }
                    )
                    .is_none())
            }
        }
        ProtocolRange::Specific(ret)
    } else {
        ProtocolRange::General(Range {
            range: ranges.join(" ").to_owned(),
            default: defaults.join(" "),
        })
    }
}

fn split_to_items(s: String) -> Vec<String> {
    let mut ret = Vec::new();
    let mut cur = String::new();
    for word in s.split_ascii_whitespace() {
        if word.starts_with("VI_") && word.ends_with(':') {
            if !cur.is_empty() {
                ret.push(cur);
            }
            cur = word.to_owned();
        } else {
            cur = cur + " " + word;
        }
    }
    if !cur.is_empty() {
        ret.push(cur);
    }
    ret
}

impl FromHtml for AttrOther {
    fn from_html(src: &Html, nav: &str) -> Result<Vec<Self>> {
        let access = extract_text(
            src,
            r"article >  article > h2 + table > tbody > tr > td:nth-child(1) > p",
            nav,
        )?;
        let ty = extract_text(
            src,
            r"article >  article > h2 + table > tbody > tr > td:nth-child(2) > p",
            nav,
        )?;
        let range = extract_text(
            src,
            r"article >  article > h2 + table > tbody > tr > td:nth-child(3)",
            nav,
        )?;
        let default = extract_text(
            src,
            r"article >  article > h2 + table > tbody > tr > td:nth-child(4) > p",
            nav,
        )?;
        let ty = split_to_items(ty.join(" "));
        if ty.len() == 1 {
            Ok(vec![Self {
                access: access.into_iter().next().unwrap(),
                ty: ty.into_iter().next().unwrap(),
                ranges: split_to_protocol(range, default),
            }])
        } else {
            // if mulitple items, none is protocol specific
            Ok(access
                .into_iter()
                .cycle()
                .zip(ty.into_iter())
                .zip(
                    split_to_items(range.join(" "))
                        .into_iter()
                        .cycle()
                        .zip(split_to_items(default.join(" ")).into_iter().cycle()),
                )
                .map(|((a, t), (r, d))| Self {
                    access: a,
                    ty: t,
                    ranges: ProtocolRange::General(Range {
                        range: r,
                        default: d,
                    }),
                })
                .collect())
        }
    }
}

fn extract_text(src: &Html, selector: &str, nav: &str) -> Result<Vec<String>> {
    let s = Selector::parse(selector).unwrap();
    let node = src.select(&s);
    //.unwrap_or_else(|| panic!("failed select '{}' in '{}'", selector, nav));
    let ret: Vec<_> = node
        .map(|x| {
            x.text()
                .map(|x| x.trim().replace(['\u{2013}', '\u{2011}'], "-"))
                .filter(|x| !x.is_empty() && x.is_ascii())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|x| !x.is_empty())
        .collect();
    if ret.is_empty() {
        return Ok(vec![format!("failed select '{}' in '{}'", selector, nav)]);
    }
    Ok(ret)
}
impl<O: Display> Display for DocItem<O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "const {}: r#\"{}\"#\n{}\n",
            self.name, self.desc, self.other
        )
    }
}

impl Display for AttrOther {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}) ({}) {}", self.access, self.ty, self.ranges,)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    extract_attr_info_to("attr.txt").await
}

async fn extract_attr_info_to(file: &str) -> Result<()> {
    //https://docs-be.ni.com/api/bundle/ni-visa/toc?language=enus
    const INDEX_URL: &str = "https://docs-be.ni.com/api/bundle/ni-visa/toc?language=enus";
    let fetcher = DocIndex::new(INDEX_URL).await?;
    let fetch_list = fetcher.fetch_list("Attributes").unwrap();
    let ret: Vec<_> = fetch_list
        .map(|path| {
            let path = path.to_owned();
            tokio::spawn(async move {
                eprintln!("fetching {}", path);
                let url = format!("https://docs-be.ni.com/api/bundle/ni-visa/page/{}", path);
                let ret = DocFetcher::new(&url).await?;
                eprintln!("finished fetching {}", path);
                ret.fetch_current::<DocItem<AttrOther>>()
            })
        })
        .collect();
    let mut file = std::fs::File::create(file)?;
    for doc in ret {
        let content = doc
            .await??
            .into_iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        write!(file, "{}\n", content)?;
    }
    Ok(())
}
