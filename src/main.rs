#![feature(async_closure)]

extern crate egg_mode;
#[macro_use]
extern crate dotenv_codegen;

use futures::TryStreamExt;

use egg_mode::entities::MediaType::Photo;
use egg_mode::stream::StreamMessage;

use url::Url;

use std::io::ErrorKind;

use std::path::Path;

use tokio::prelude::io::AsyncWriteExt;

mod config;
use config::Config;

use std::io::Write;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load().await;
    let token = config.token;
    let username = config.screen_name;

    println!("Welcome, {}, let's get started!", username);

    let mut try_count = 0;
    // let mut total_count = 0u64;

    loop {
        let stream = egg_mode::stream::sample(&token).try_for_each_concurrent(10, async move |m| {
            try_count = 0;
            if let StreamMessage::Tweet(tweet) = m {
                if let Some(media) = tweet.extended_entities {
                    for info in media.media {
                        if let Photo = info.media_type {
                            if let Err(e) = download_from_url(info.media_url_https).await {
                                println!("Failed download: {}", e)
                            }
                        }
                    }
                }
            }

            futures::future::ok(()).await
        });

        println!("Garbage collecting...");
        if let Err(e) = stream.await {
            println!("Stream error: {}", e);
            println!("Disconnected");

            if try_count > 10 {
                break;
            }
            try_count += 1;
            println!("Trying to reconnect... {} time(s)", try_count)
        }
    }

    println!("Try enough times! bye");
    Ok(())
}

async fn download_from_url(image_url: String) -> Result<(), Box<dyn std::error::Error>> {
    let res = reqwest::get(&image_url).await?;
    let url = Url::parse(&image_url)?;

    if let Some(segments) = url.path_segments().map(|c| c.collect::<Vec<_>>()) {
        let filename = segments[segments.len() - 1];
        let bytes = res.bytes().await?;
        // println!("{} bytes", bytes.len());
        print!("\r{}: {} bytes", filename, bytes.len());
        std::io::stdout().flush()?;
        save_as_file(&filename, bytes.to_vec().as_slice()).await?;
    }

    Ok(())
}
async fn save_as_file<P: AsRef<Path>>(
    filename: &P,
    bytes: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let save_dir = Path::new("dest");

    if let Err(e) = tokio::fs::create_dir(save_dir).await {
        match e.kind() {
            ErrorKind::AlreadyExists => {}
            _ => panic!(e),
        }
    }
    let mut file = tokio::fs::File::create(save_dir.join(filename)).await?;

    file.write_all(bytes).await?;
    Ok(())
}
