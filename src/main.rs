use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;

use futures_util::stream::StreamExt;
use reqwest::header::{HeaderValue, USER_AGENT};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() {
    let contents = fs::read_to_string("example.yaml").expect("Should be able to read file");
    let config: Config = serde_yaml::from_str::<Config>(&contents).unwrap();

    println!("Checking Launcher");
    let file = Path::new(&config.launcher.file);

    if !file.exists() {
        let url = String::from_str("https://api.github.com/repos/R2Northstar/Northstar/releases/latest").unwrap();
        let is_downloaded = download_latest_release_assets(url).await;

        if is_downloaded.is_err() {
            println!("Failed to download latest release");
            return;
        }

        println!("Launcher downloaded");

        let target_path = Path::new(".");
        println!("Extracting Launcher to {:?}", target_path);
    }
}

async fn download_latest_release_assets(url: String) -> Result<File, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header(USER_AGENT, HeaderValue::from_static("reqwest"))
        .send()
        .await?;

    if !response.status().is_success() {
        println!("Failed to get latest release: {}", response.status());
        return Err("Failed to get latest release".into());
    }

    let release: Release = response.json().await?;

    if let Some(asset) = release.assets.first() {
        let file_name = asset.name.to_string();
        let mut dest_file = File::create(&file_name)?;

        let asset_response = client.get(&asset.browser_download_url).send().await?;

        if !asset_response.status().is_success() {
            println!("Failed to download asset: {}", asset_response.status());
            return Err("Failed to download asset".into());
        }

        // Write the response bytes to the file, displaying progress
        let mut content_length = 0;
        if let Some(len) = asset_response.content_length() {
            content_length = len;
        }

        let mut downloaded: u64 = 0;
        let mut stream = asset_response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            downloaded += chunk.len() as u64;
            dest_file.write_all(&chunk)?;
            let progress = (downloaded as f64 / content_length as f64) * 100.0;
            print!("\rDownloading {:.2}% ({}/{})", progress, downloaded, content_length);
            io::stdout().flush()?; // Flush stdout to ensure the progress is immediately displayed
        }
        println!();

        println!("Download completed");

        Ok(dest_file)
    } else {
        println!("No assets found in the latest release");

        Err("No assets found in the latest release".into())
    }
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
