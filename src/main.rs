use std::{
    path::{Path, PathBuf},
    pin::pin,
};

use clap::Parser;
use eyre::{Report, Result, WrapErr};
use futures::{stream, StreamExt};
use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use reqwest::{Client, ClientBuilder};
use scraper::{element_ref::Text, Html, Selector};
use tokio::{
    fs::File,
    io::{stdin, AsyncReadExt},
};

#[derive(Parser)]
struct Args {
    /// Path of the file that contains the URLs, one per line. Unless this
    /// option is set, reads from the standard input.
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// Template. Use `%title` and `%url` as placeholders.
    ///
    /// Default is `%title <%url>`.
    #[arg(short, long)]
    template: Option<String>,

    /// Doesn't emit links if the page doesn't have a title. By default, this
    /// is set to `false` and if a page doesn't have a title, `@@@ NO TITLE @@@`
    /// will be used.
    #[arg(long, default_value = "false")]
    skip_when_no_title: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let template = args.template.as_deref().unwrap_or("%title <%url>");

    let contents = read_file_string(args.file.as_deref()).await?;

    // Creates an iterator of futures.
    let titles_iter = non_empty_lines(&contents).map(|url| async move {
        let maybe_title = load_url_and_get_title(url).await?;
        Ok::<_, Report>((maybe_title, url))
    });

    // Processes 10 futures concurrently.
    let mut urls_stream = stream::iter(titles_iter).buffered(10);

    while let Some(tup) = urls_stream.next().await {
        let (maybe_title, url) = tup?;
        let maybe_title = maybe_title.as_deref().or_else(|| {
            eprintln!("(no title for `{url}`)");
            (!args.skip_when_no_title).then_some("@@@ NO TITLE @@@")
        });
        if let Some(title) = maybe_title {
            let text = process_template(template, title, url);
            println!("{text}");
        }
    }

    Ok(())
}

/// Given a template, processes it by interpolating the given `title` and `url`
/// strings. Expects to substitute `%title` and `%url` in the given template.
fn process_template(template: &str, title: &str, url: &str) -> String {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new("%(title|url)").unwrap());

    let text = RE.replace_all(template, |cap: &Captures| match &cap[0] {
        "%title" => title,
        "%url" => url,
        _ => unreachable!(),
    });

    text.into_owned()
}

/// Reads the contents of the given path, if it exists. Otherwise, reads from
/// the standard input.
async fn read_file_string(path: Option<&Path>) -> Result<String> {
    async fn read(reader: impl AsyncReadExt) -> Result<String> {
        let mut buf = String::new();
        pin!(reader).read_to_string(&mut buf).await?;
        Ok(buf)
    }
    match path {
        Some(path) => read(File::open(path).await?).await,
        None => read(stdin()).await,
    }
}

/// Returns an iterator over the non-empty lines of the provided string slice.
fn non_empty_lines(contents: &str) -> impl Iterator<Item = &str> {
    contents
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
}

/// Fetches the content of the given URL and retrieves its page title, if it
/// is present. If there is no title, `None` is returned.
async fn load_url_and_get_title(url: &str) -> Result<Option<String>> {
    let html = load_html(url).await?;
    parse_html_and_get_title(&html).await
}

/// Fetches the given URL, returning the full page HTML as a string.
async fn load_html(url: &str) -> Result<String> {
    // One doesn't really need this since one's only using the client once.
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

/// Parses the given HTML string and retrieves the text of the `title` tag,
/// if it is present.
async fn parse_html_and_get_title(html: &str) -> Result<Option<String>> {
    static SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("title").unwrap());

    /// Produces a string by iterating over all text nodes. A space character is
    /// inserted between two text nodes.
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
    let fst = elements
        .next() // Only get the first title tag.
        .map(|el| join_text(el.text())) // Get full text from html text node.
        .filter(|title| !title.is_empty()); // Map empty strings to none.

    Ok(fst)
}
