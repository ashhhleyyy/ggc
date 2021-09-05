#[macro_use]
extern crate log;

use std::io;
use std::io::ErrorKind;
use std::sync::Arc;

use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio_rustls::{rustls, TlsAcceptor};

use crate::config::{load_config, VirtualSiteConfig, Config};
use crate::gemini::{parse_gemini_url, GeminiResponseBody};
use crate::utils::{load_certs, load_private_key};
use std::collections::HashMap;
use tokio_rustls::rustls::ResolvesServerCertUsingSNI;
use tokio_rustls::rustls::sign::CertifiedKey;

mod config;
mod utils;
mod gemini;

pub struct Site {
    listen_port: u16,
    sites: HashMap<String, VirtualSiteConfig>,
    tls_config: Arc<rustls::ServerConfig>,
}

impl Site {
    pub fn new(config: Config) -> Self {
        Self {
            listen_port: config.listen_port,
            sites: config.sites.clone(),
            tls_config: Self::load_server_config(&config),
        }
    }

    fn load_server_config(config: &Config) -> Arc<rustls::ServerConfig> {
        let client_auth = rustls::NoClientAuth::new();

        let mut server_config = rustls::ServerConfig::new(client_auth);
        let mut sni_resolver = ResolvesServerCertUsingSNI::new();

        for (name, site_config) in &config.sites {
            let certs = load_certs(&site_config.server_certificate_file);
            let private_key = load_private_key(&site_config.key_file);
            let key = Arc::new(rustls::sign::any_supported_type(&private_key)
                .expect("invalid private key"));
            sni_resolver.add(&name, CertifiedKey::new(certs, key))
                .expect("invalid certificate or private key");
        }

        server_config.cert_resolver = Arc::new(sni_resolver);

        server_config.key_log = Arc::new(rustls::KeyLogFile::new());

        Arc::new(server_config)
    }

    pub async fn run(&mut self) -> io::Result<()> {
        let acceptor = TlsAcceptor::from(self.tls_config.clone());
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.listen_port)).await?;

        loop {
            let (stream, peer_addr) = listener.accept().await?;
            debug!("new connection from {}", peer_addr);

            let acceptor = acceptor.clone();
            let sites = self.sites.clone();

            let fut = async move {
                let mut stream = acceptor.accept(stream).await?;

                // read the request header
                let mut buf: [u8; 1024] = [0; 1024];
                let mut read_buffer = String::new();

                // read until the CRLF
                while !read_buffer.contains("\r\n") {
                    let read = stream.read(&mut buf).await?;
                    debug!("read {} bytes", read);
                    if read == 0 {
                        break;
                    }
                    let s = match std::str::from_utf8(&buf[..read]) {
                        Ok(s) => s,
                        Err(e) => return Err(io::Error::new(ErrorKind::InvalidData, e)),
                    };
                    read_buffer.push_str(s);
                }
                // Locate the CRLF and extract the request line
                let eol_index = read_buffer.find("\r\n");
                let eol_index = match eol_index {
                    None => return Err(io::Error::new(ErrorKind::InvalidData, "missing \\r\\n sequence")),
                    Some(eol_index) => eol_index,
                };
                let request_url = read_buffer[..eol_index].to_string();

                // parse the url, handling cases where it is invalid according to the spec
                let url = match parse_gemini_url(&request_url) {
                    Ok(url) => url,
                    Err(e) => {
                        return Err(io::Error::new(ErrorKind::InvalidData, e));
                    }
                };

                let host = url.host().expect("host should always be present for gemini urls").to_string();
                let config = sites.get(&host);

                match config {
                    Some(config) => {
                        // pass the path down to the site source, and allow it to send a response.
                        let path = url.path();
                        config.source.serve(&host, &mut stream, gemini::normalise_gemini_path(path)).await?;
                    },
                    None => GeminiResponseBody::no_site(host).write_to(&mut stream).await?,
                }

                Ok(()) as io::Result<()>
            };

            tokio::spawn(async move {
                if let Err(e) = fut.await {
                    error!("connection error: {}", e);
                }
            });
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    info!("Hello, world!");
    info!("Build {} initialising...", utils::SERVER_BUILD_INFO);
    let config = load_config();
    if config.sites.is_empty() {
        warn!("No sites specified in config, all routes will 51!");
    }
    Site::new(config).run().await.expect("server crashed");
}
