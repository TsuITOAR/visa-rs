use std::io::Write;

use scraper::{Html, Selector};

type Result<O> = std::result::Result<O, Box<dyn std::error::Error>>;

#[derive(Clone)]
struct DocFetcher {
    content: Html,
    next_filter: fn(&str) -> bool,
}

impl DocFetcher {
    async fn new(url: &str, filter: fn(&str) -> bool) -> Result<Self> {
        let content = reqwest::get(url).await?;
        let html = Html::parse_document(&content.text().await?);
        return Ok(Self {
            content: html,
            next_filter: filter,
        });
    }
    fn fetch_current<O: FromHtml>(&self) -> Result<DocItem<O>> {
        DocItem::<O>::from_html(&self.content)
    }
    async fn try_next(&self, selector: &str) -> Result<Option<Self>> {
        use selectors::Element;
        let selector = Selector::parse(selector).unwrap();
        let next_node = self.content.select(&selector).next().unwrap();
        if (self.next_filter)(next_node.text().next().unwrap()) {
            let next_url = next_node
                .parent_element()
                .unwrap()
                .value()
                .attr("href")
                .unwrap();
            return Ok(Some(Self::new(next_url, self.next_filter).await?));
        }
        Ok(None)
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
            r"#bundle\:ni-visa\/enus\/ni-visa\/vi_attr_4882_compliant\.html > article > table > tbody > tr:nth-child(2) > td:nth-child(2) > p > span",
        )?;
        let range = extract_text(
            src,
            r"#bundle\:ni-visa\/enus\/ni-visa\/vi_attr_4882_compliant\.html > article > table > tbody > tr:nth-child(2) > td:nth-child(2) > p > span",
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
    const URL_ATTR: &str =
        "https://www.ni.com/docs/zh-CN/bundle/ni-visa/page/ni-visa/attributes.html";
    let fetcher = DocFetcher::new(URL_ATTR, |x: &str| x.starts_with("VI_ATTR_")).await?;
    let mut ret = Vec::new();
    let next_selector = r"#root > div.wrapper > div > div > div > div.topicPage_topicPageContainer.topicPageContainer.zDocsTopicPage.topicPage_zDocsTopicPage__16mFX > div.topicContainer > div.topic_articleContainer.articleContainer > article > div.article__actions-wrapper > ul > li:nth-child(2) > div > a:nth-child(3) > span.article__option-content > span";
    let mut next = fetcher;
    while let Some(n) = next.try_next(next_selector).await? {
        ret.push(n.fetch_current()?);
        next = n;
    }
    let mut file = std::fs::File::create(file)?;
    ret.into_iter().for_each(|x: DocItem<AttrOther>| {
        file.write_all(x.to_string().as_ref()).unwrap();
    });
    Ok(())
}
