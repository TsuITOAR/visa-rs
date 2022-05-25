use std::io::Write;

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
        let html = Html::parse_fragment(&response["topic_html"].to_string());
        return Ok(Self {
            url: url.to_owned(),
            content: html,
        });
    }
    fn fetch_current<Item: FromHtml>(&self) -> Result<Item> {
        Item::from_html(&self.content, &self.url)
    }
}

trait FromHtml: Sized {
    fn from_html(src: &Html, nav: &str) -> Result<Self>;
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct DocItem<Other> {
    name: String,
    desc: String,
    other: Other,
}

impl<O: FromHtml> FromHtml for DocItem<O> {
    fn from_html(src: &Html, nav: &str) -> Result<Self> {
        let name = extract_text(src, r"article >  article > h1:first-child", nav)?
            .join(" ")
            .to_owned();
        let desc = extract_text(src, r"article >  article > *", nav)?
            .into_iter()
            .skip_while(|x| !x.contains("Description"))
            .skip(1)
            .take_while(|x| !x.contains("Related Topics"))
            .collect::<Vec<_>>()
            .join(" ")
            .to_owned();
        let other = O::from_html(src, nav)?;
        Ok(Self { name, desc, other })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct AttrOther {
    access: String,
    ty: String,
    range: String,
    default: String,
}

impl FromHtml for AttrOther {
    fn from_html(src: &Html, nav: &str) -> Result<Self> {
        let access = extract_text(
            src,
            r"article >  article > table > tbody > tr:nth-child(2) > td:nth-child(1) > *",
            nav,
        )?
        .join(" ")
        .to_owned();
        let ty = extract_text(
            src,
            r"article >  article > table > tbody > tr:nth-child(2) > td:nth-child(2) > *",
            nav,
        )?
        .join(" ")
        .to_owned();
        let range = extract_text(
            src,
            r"article >  article > table > tbody > tr:nth-child(2) > td:nth-child(3) > *",
            nav,
        )?
        .join(" ")
        .to_owned();
        let default = extract_text(
            src,
            r"article >  article > table > tbody > tr:nth-child(2) > td:nth-child(4) > *",
            nav,
        )?
        .join(" ")
        .to_owned();
        Ok(Self {
            access,
            ty,
            range,
            default,
        })
    }
}

fn extract_text(src: &Html, selector: &str, nav: &str) -> Result<Vec<String>> {
    let s = Selector::parse(selector).unwrap();
    let node = src.select(&s);
    let ret: Vec<_> = node
        .map(|x| x.text().collect::<Vec<_>>().join(" "))
        .collect();
    if ret.is_empty() {
        let message = format!("failed select '{}' in '{}'", selector, nav);
        eprintln!("{}", message);
        return Ok(vec![message]);
    }
    Ok(ret)
}
impl<O: ToString> ToString for DocItem<O> {
    fn to_string(&self) -> String {
        format!(
            "const {}: r#\"{}\"#\n{}\n",
            self.name,
            self.desc,
            self.other.to_string(),
        )
    }
}

impl ToString for AttrOther {
    fn to_string(&self) -> String {
        format!(
            "({}) ({})::<{}> = {}",
            self.access, self.ty, self.range, self.default,
        )
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
        let content = doc.await??.to_string();
        write!(file, "{}\n", content)?;
    }
    Ok(())
}
