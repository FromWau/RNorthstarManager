use std::env;
use std::fs::{self, File};
use std::io::{self, Seek, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::AtomicBool;

use futures_util::stream::StreamExt;
use reqwest::header::{HeaderValue, USER_AGENT};
use serde::{Deserialize, Serialize};
use zip::ZipArchive;

#[tokio::main]
async fn main() {
    let contents = fs::read_to_string("example.yaml").expect("Should be able to read file");
    let config: Config = serde_yaml::from_str::<Config>(&contents).unwrap();

    println!("Checking Launcher");
    let file = Path::new(&config.launcher.file);

    if !file.exists() {
        let url = String::from_str("https://api.github.com/repos/R2Northstar/Northstar/releases/latest").unwrap();
        // let archive = download_latest_release_assets(url).await;

        // let archive = match archive {
        //     Ok(archive) => File::open(archive).unwrap(),
        //     Err(_) => {
        //         println!("Failed to download latest release");
        //         return;
        //     }
        // };

        let archive = File::open(PathBuf::from("Northstar.release.v1.24.4.zip")).unwrap();

        let install_dir = "test";
        extract_zip(archive, install_dir);
    }
}

async fn download_latest_release_assets(url: String) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header(USER_AGENT, HeaderValue::from_static("reqwest"))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err("Failed to get latest release".into());
    }

    let release: Release = response.json().await?;

    if let Some(asset) = release.assets.first() {
        let file_name = asset.name.to_string();
        let mut dest_file = File::create(&file_name)?;

        let asset_response = client.get(&asset.browser_download_url).send().await?;

        if !asset_response.status().is_success() {
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

        dest_file.seek(std::io::SeekFrom::Start(0))?; // Rewind the file pointer
        Ok(PathBuf::from(&file_name))
    } else {
        Err("No assets found in the latest release".into())
    }
}

fn extract_zip(archive: File, dir: &str) {
    let cwd = env::current_dir().unwrap();
    let extract_dir = if dir == "." { cwd.clone() } else { cwd.join(dir) };

    println!("Extracting files to: {:?}", extract_dir);

    if !extract_dir.exists() {
        fs::create_dir(extract_dir.clone()).expect("Failed to create directory");
    }

    let mut archive = ZipArchive::new(archive).unwrap_or_else(|err| {
        panic!("Failed to open zip archive: {}", err);
    });

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let file_name = file.name();
        let target_path = extract_dir.join(file_name);

        if file.is_dir() {
            fs::create_dir_all(&target_path).expect("Failed to create directory while extracting zip");
        } else {
            if let Some(parent_dir) = target_path.parent() {
                fs::create_dir_all(parent_dir).expect("Failed to create needed parent dirs while extracting zip");
            }

            let mut output_file = File::create(&target_path).expect("Failed to create file while extracting zip");
            io::copy(&mut file, &mut output_file).expect("Failed to copy file while extracting zip");
        }

        println!("Extracted: {:?}", target_path);
    }

    println!("Files extracted");
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
