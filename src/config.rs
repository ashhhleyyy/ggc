use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, ErrorKind};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio_rustls::server::TlsStream;
use tokio::net::TcpStream;
use std::io;
use crate::gemini::{GeminiResponseBody, generate_folder_index};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub listen_port: u16,
    pub sites: HashMap<String, VirtualSiteConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen_port: 1965,
            sites: HashMap::new(),
        }
    }
}

pub(crate) fn load_config() -> Config {
    let path = Path::new("config.toml");
    if path.exists() {
        let mut buf = String::new();
        File::open(path).expect("failed to open config")
            .read_to_string(&mut buf).expect("failed to read config");
        toml::from_str(&buf).expect("failed to parse config")
    } else {
        let config = Config::default();

        let content = toml::to_string(&config).expect("failed to write default config");
        fs::write(path, content).expect("failed to write default config");

        config
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualSiteConfig {
    // keys
    pub server_certificate_file: PathBuf,
    pub key_file: PathBuf,

    #[serde(flatten)]
    pub source: SiteSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SiteSource {
    FlatDir {
        directory: PathBuf,
        #[serde(default)]
        auto_index: bool,
        #[serde(default)]
        disable_footer: bool,
        #[serde(default)]
        hide_version: bool,
    },
}

impl SiteSource {
    pub async fn serve(&self, host: &str, stream: &mut TlsStream<TcpStream>, path: &str) -> io::Result<()> {
        info!("[{}] GET {}", host, path);
        match self {
            SiteSource::FlatDir {
                directory, auto_index,
                disable_footer, hide_version
            } => serve_static(&directory, *auto_index, *disable_footer, *hide_version, stream, path).await,
        }
    }
}

async fn serve_static(directory: &Path, auto_index: bool, disable_footer: bool, hide_version: bool,
                      stream: &mut TlsStream<TcpStream>, url_path: &str) -> io::Result<()> {
    let mut path = directory.to_path_buf();

    if url_path.len() > 1 {
        path.push(&url_path[1..]);
    }

    if path.is_dir() {
        let mut index_file = path.clone();
        index_file.push("index.gmi");
        if index_file.exists() {
            serve_file(index_file, stream).await
        } else {
            if auto_index {
                serve_str(&generate_folder_index(&path, disable_footer, hide_version).await?, stream).await
            } else {
                GeminiResponseBody::not_found().write_to(stream).await
            }
        }
    } else {
        serve_file(path, stream).await
    }
}

async fn serve_str(s: &str, stream: &mut TlsStream<TcpStream>) -> io::Result<()> {
    GeminiResponseBody::new_ok(s.as_bytes().to_vec()).write_to(stream).await?;
    Ok(())
}

async fn serve_file(path: PathBuf, stream: &mut TlsStream<TcpStream>) -> io::Result<()> {
    use tokio::fs;

    let response = match fs::read(path).await {
        Ok(file) => GeminiResponseBody::new_ok(file),
        Err(e) => {
            match e.kind() {
                ErrorKind::NotFound => GeminiResponseBody::not_found(),
                _ => {
                    GeminiResponseBody::server_error().write_to(stream).await?;
                    return Err(e);
                },
            }
        },
    };

    response.write_to(stream).await?;

    Ok(())
}
