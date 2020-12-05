#![deny(warnings)]
#![warn(rust_2018_idioms)]
// use std::env;

use hyper::{body::HttpBody as _, Client};
use tokio::io::{self, AsyncWriteExt as _};
use tokio::task::spawn;

//TODO:记个笔记。
//TODO:提示^^^^^^ no `Client` in the root， 是因为我没有打开 hyper 的features。。我真笨。。

// A simple type alias so as to DRY.
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
//cargo run --example=hyper --features=tokio-support http://baidu.com
#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    // Some simple CLI args requirements...
    // let url = match env::args().nth(1) {
    //     Some(url) => url,
    //     None => {
    //         println!("Usage: client <url>");
    //         return Ok(());
    //     }
    // };

    // HTTPS requires picking a TLS implementation, so give a better
    // warning if the user tries to request an 'https' URL.
    let url = "http://baidu.com".parse::<hyper::Uri>().unwrap();
    if url.scheme_str() != Some("http") {
        println!("This example only works with 'http' URLs.");
        return Ok(());
    }

    spawn(fetch_baidu());

    // spawn(fetch_url(url.clone()));
    // spawn(fetch_url(url.clone()));

    spawn(fetch_url(url)).await.unwrap()
}

async fn fetch_url(url: hyper::Uri) -> Result<()> {
    let client = Client::new();

    let mut res = client.get(url).await?;

    println!("Response: {}", res.status());
    println!("Headers: {:#?}\n", res.headers());

    // Stream the body, writing each chunk to stdout as we get it
    // (instead of buffering and printing at the end).
    while let Some(next) = res.data().await {
        let chunk = next?;
        io::stdout().write_all(&chunk).await?;
    }

    println!("\n\nDone!");

    Ok(())
}

async fn fetch_baidu() -> Result<()> {
    let client = Client::new();
    let url = "http://baidu.com".parse::<hyper::Uri>().unwrap();

    let mut res = client.get(url).await?;

    println!("Response: {}", res.status());
    println!("Headers: {:#?}\n", res.headers());
    while let Some(next) = res.data().await {
        let chunk = next?;
        io::stdout().write_all(&chunk).await?;
    }

    println!("\n\nDone!");

    Ok(())
}
