use std::{fs, io};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::AtomicBool;

use futures_util::stream::StreamExt;
use reqwest::header::{HeaderValue, USER_AGENT};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() {
    let contents = fs::read_to_string("example.yaml").expect("Should be able to read file");
    let config: Config = serde_yaml::from_str::<Config>(&contents).unwrap();

    let file = Path::new(&config.launcher.file);

    if !file.exists() {
        download_latest_release_assets().await.unwrap();
    }
}

async fn download_latest_release_assets() -> Result<(), Box<dyn std::error::Error>> {
    let url = "https://api.github.com/repos/R2Northstar/Northstar/releases/latest";

    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header(USER_AGENT, HeaderValue::from_static("reqwest"))
        .send()
        .await?;

    if response.status().is_success() {
        let release: Release = response.json().await?;

        if let Some(asset) = release.assets.first() {
            let file_name = format!("{}", asset.name);
            let mut dest_file = File::create(&file_name)?;

            let asset_response = client
                .get(&asset.browser_download_url)
                .send()
                .await?;

            // Check if the request was successful
            if asset_response.status().is_success() {
                // Write the response bytes to the file, displaying progress
                let mut content_length = 0;
                if let Some(len) = asset_response.content_length() {
                    content_length = len;
                }

                let mut downloaded: u64 = 0;
                let mut stream = asset_response.bytes_stream(); // Change to bytes_stream() to get a stream of bytes

                while let Some(chunk) = stream.next().await {
                    let chunk = chunk?;
                    downloaded += chunk.len() as u64;
                    dest_file.write_all(&chunk)?;
                    let progress = (downloaded as f64 / content_length as f64) * 100.0;
                    print!("\rDownloading {:.2}% ({}/{})", progress, downloaded, content_length);
                    io::stdout().flush()?; // Flush stdout to ensure the progress is immediately displayed
                }
                print!("\n");

                println!("Download completed");
            } else {
                println!("Failed to download asset: {}", asset_response.status());
            }
        } else {
            println!("No assets found in the latest release");
        }
    } else {
        println!("Failed to fetch releases: {}", response.status());
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct Release {
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    github_token: Option<String>,
    log_level: Option<String>,
    launcher: Launcher,
    mods: Vec<Mod>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Launcher {
    file: String,
    args: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Mod {
    name: String,
    repo: String,
    last_update: Option<String>,
    file: Option<String>,
    install_dir: Option<String>,
    ignore_updates: Option<AtomicBool>,
    ignore_pre_releases: Option<AtomicBool>,
    exclude_files: Option<Vec<String>>,
}
