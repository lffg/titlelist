use std::path::{Path, PathBuf};

use clap::Parser;
use eyre::{Report, Result, WrapErr};
use futures::{stream, StreamExt};
use once_cell::sync::Lazy;

#[derive(Parser)]
struct Args {
    /// Path of the file that contains the URLs, one per line.
    path: PathBuf,

    /// Template. Use `%title` and `%url` as placeholders.
    ///
    /// Default is `%title <%url>`.
    #[arg(short, long)]
    template: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let template = args.template.as_deref().unwrap_or("%title <%url>");

    let contents = load_file(&args.path).await?;
    let titles_iter = get_urls(&contents).map(|url| async move {
        let maybe_title = process_url(url).await?;
        Ok::<_, Report>((maybe_title, url))
    });

    let mut urls_stream = stream::iter(titles_iter).buffered(10);
    while let Some(tup) = urls_stream.next().await {
        let (maybe_title, url) = tup?;
        let title = maybe_title.as_deref().unwrap_or_else(|| {
            eprintln!("(no title for `{url}`)");
            "@@@ NO TITLE @@@"
        });
        let text = process_template(template, title, url);
        println!("{text}");
    }

    Ok(())
}

fn process_template(template: &str, title: &str, url: &str) -> String {
    use regex::{Captures, Regex};

    static RE: Lazy<Regex> = Lazy::new(|| Regex::new("%(title|url)").unwrap());

    let text = RE.replace_all(template, |cap: &Captures| match &cap[0] {
        "%title" => title,
        "%url" => url,
        _ => unreachable!(),
    });

    text.into_owned()
}

async fn load_file(path: &Path) -> Result<String> {
    tokio::fs::read_to_string(path)
        .await
        .wrap_err("failed to load file")
}

fn get_urls(contents: &str) -> impl Iterator<Item = &str> {
    contents
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
}

async fn process_url(url: &str) -> Result<Option<String>> {
    let html = load_html(url).await?;
    parse_html_and_get_title(&html).await
}

async fn load_html(url: &str) -> Result<String> {
    use reqwest::{Client, ClientBuilder};

    static CLIENT: Lazy<Client> = Lazy::new(|| {
        ClientBuilder::new()
            .user_agent("load title tags")
            .build()
            .unwrap()
    });

    CLIENT
        .get(url)
        .send()
        .await
        .wrap_err_with(|| format!("failed to get: `{url}`"))?
        .text()
        .await
        .map_err(Into::into)
}

async fn parse_html_and_get_title(html: &str) -> Result<Option<String>> {
    use scraper::{element_ref::Text, Html, Selector};

    static SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("title").unwrap());

    fn join_text(text: Text<'_>) -> String {
        let mut s = String::new();
        for text_node in text {
            s.push_str(text_node.trim());
            s.push(' ');
        }
        s.pop();
        s
    }

    let fragment = Html::parse_fragment(html);

    let mut elements = fragment.select(&SELECTOR);
    let fst = elements.next().map(|el| join_text(el.text()));

    Ok(fst)
}
