use std::{collections::HashMap, io::Write};

use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

type Result<O> = std::result::Result<O, Box<dyn std::error::Error>>;

#[derive(Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct DocIndex {
    items: Vec<DocIndexItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
    fn fetch_list<'a>(&'a self, parent: &str) -> Option<impl Iterator<Item = &'a str>> {
        fn fetch_list_in<'a>(
            index: &'a DocIndexItem,
            parent: &str,
        ) -> Option<impl Iterator<Item = &'a str>> {
            for child in &index.child_entries {
                if child.title == parent {
                    return Some(
                        child
                            .child_entries
                            .iter()
                            .filter_map(|x| x.url.as_ref().map(|x| x.as_str())),
                    );
                }
            }
            None
        }
        for item in &self.items {
            match fetch_list_in(item, parent) {
                Some(e) => return Some(e),
                None => (),
            }
        }
        None
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DocFetcher {
    content: Html,
}

impl DocFetcher {
    async fn new(url: &str) -> Result<Self> {
        let content: HashMap<String, String> = reqwest::get(url).await?.json().await?;
        let content = &content["topic_html"];
        let html = Html::parse_document(content);
        return Ok(Self { content: html });
    }
    fn fetch_current<O: FromHtml>(&self) -> Result<DocItem<O>> {
        DocItem::<O>::from_html(&self.content)
    }
}

trait FromHtml: Sized {
    fn from_html(src: &Html) -> Result<Self>;
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct DocItem<Other> {
    name: String,
    desc: String,
    other: Other,
}

impl<O: FromHtml> FromHtml for DocItem<O> {
    fn from_html(src: &Html) -> Result<Self> {
        let name = extract_text(
            src,
            r"#root > div.wrapper > div > div > div > div.topicPage_topicPageContainer.topicPageContainer.zDocsTopicPage.topicPage_zDocsTopicPage__16mFX > div.topicContainer > div.topic_articleContainer.articleContainer > article > div.article__content.topic_articleContainer__DjiK9 > div > div > h1",
        )?;
        let desc = extract_text(
            src,
            r"#bundle\:ni-visa\/enus\/ni-visa\/vi_attr_4882_compliant\.html > article > p:nth-child(7)",
        )?;
        let other = O::from_html(src)?;
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
    fn from_html(src: &Html) -> Result<Self> {
        let access = extract_text(
            src,
            r"#bundle\:ni-visa\/enus\/ni-visa\/vi_attr_4882_compliant\.html > article > table > tbody > tr:nth-child(2) > td:nth-child(1) > p",
        )?;
        let ty = extract_text(
            src,
            r"#bundle\:ni-visa\/enus\/ni-visa\/vi_attr_4882_compliant\.html > article > table > tbody > tr:nth-child(2) > td:nth-child(2) > p",
        )?;
        let range = extract_text(
            src,
            r"#bundle\:ni-visa\/enus\/ni-visa\/vi_attr_4882_compliant\.html > article > table > tbody > tr:nth-child(2) > td:nth-child(3) > p",
        )?;
        let default = extract_text(
            src,
            r"#bundle\:ni-visa\/enus\/ni-visa\/vi_attr_4882_compliant\.html > article > table > tbody > tr:nth-child(2) > td:nth-child(4) > p",
        )?;
        Ok(Self {
            access,
            ty,
            range,
            default,
        })
    }
}

fn extract_text(src: &Html, selector: &str) -> Result<String> {
    let selector = Selector::parse(selector).unwrap();
    Ok(src.select(&selector).next().unwrap().text().collect())
}
impl<O: ToString> ToString for DocItem<O> {
    fn to_string(&self) -> String {
        [self.name.clone(), self.desc.clone(), self.other.to_string()].join("\t")
    }
}
impl ToString for AttrOther {
    fn to_string(&self) -> String {
        [
            self.clone().access,
            self.clone().ty,
            self.clone().range,
            self.clone().default,
        ]
        .join("\t")
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
    println!("{}", serde_json::to_string(&fetcher.items)?);
    let fetch_list = fetcher.fetch_list("Attributes").unwrap();
    let ret: Vec<_> = fetch_list.map(|url| DocFetcher::new(url)).collect();
    let mut file = std::fs::File::create(file)?;
    for doc in ret {
        write!(
            file,
            "{}\n",
            doc.await?
                .fetch_current::<DocItem<AttrOther>>()?
                .to_string()
        )?;
    }
    Ok(())
}
