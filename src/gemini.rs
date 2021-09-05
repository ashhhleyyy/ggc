use std::io;
use std::path::Path;

use chrono::Utc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;

use crate::utils;

pub struct GeminiResponseBody {
    // Gemini status codes are two digits and so should always fit in a u8
    pub status: u8,
    pub meta: String,
    pub body: Vec<u8>,
}

impl GeminiResponseBody {
    pub fn new_ok(body: Vec<u8>) -> Self {
        Self {
            status: 20,
            meta: "text/gemini".to_string(),
            body,
        }
    }

    pub fn not_found() -> Self {
        Self {
            status: 51,
            meta: "Not found".to_string(),
            body: Vec::new(),
        }
    }

    pub fn no_site(host: String) -> Self {
        Self {
            status: 51,
            meta: format!("No site configured for {}", host),
            body: Vec::new(),
        }
    }

    pub fn server_error() -> Self {
        Self {
            status: 41,
            meta: "Internal server error".to_string(),
            body: Vec::new(),
        }
    }

    pub async fn write_to(&self, stream: &mut TlsStream<TcpStream>) -> io::Result<()> {
        let header = format!("{} {}\r\n", self.status, self.meta);
        stream.write(header.as_bytes()).await?;
        stream.write(&self.body).await?;

        Ok(())
    }
}

pub fn normalise_gemini_path(path: &str) -> &str {
    if path.is_empty() {
        "/"
    } else {
        path
    }
}

pub async fn generate_folder_index(path: &Path, disable_footer: bool, hide_version: bool) -> io::Result<String> {
    use tokio::fs;
    let mut index_page = String::new();
    index_page.push_str(&format!("# Index of {}\n\n", path.file_name().unwrap().to_string_lossy()));
    index_page.push_str("=> ../ ../\n");
    let mut entries = fs::read_dir(path).await?;
    while let Some(entry) = entries.next_entry().await? {
        let file_name = entry.file_name().to_string_lossy().to_string();
        if entry.path().is_dir() {
            index_page.push_str(&format!("=> {}/ {}/\n", file_name, file_name))
        } else {
            index_page.push_str(&format!("=> {} {}\n", file_name, file_name))
        }
    }

    index_page.push('\n');
    let current_time = Utc::now();
    if !disable_footer {
        index_page.push_str(&if hide_version {
            format!("> Generated at {}", current_time.format("%Y-%m-%d %H:%M:%S"))
        } else {
            format!("> Generated by {} at {}", utils::SERVER_BUILD_INFO, current_time.format("%Y-%m-%d %H:%M:%S"))
        });
    }

    Ok(index_page)
}

/// Parses and validates a URL based on the restrictions in the gemini specification
///
/// This function performs the following checks:
/// - Authority component is required
/// - User info component is not allowed
/// - Host subcomponent is required
/// - Scheme must be gemini
pub fn parse_gemini_url(u: &str) -> Result<url::Url, GeminiUrlError> {
    let url = url::Url::parse(&*u)?;
    if !url.has_authority() {
        return Err(GeminiUrlError::MissingAuthority);
    }

    if url.username() != "" || url.password().is_some() {
        return Err(GeminiUrlError::UserinfoNotAllowed);
    }

    if url.host().is_none() {
        return Err(GeminiUrlError::MissingHost);
    }

    if url.scheme() != "gemini" {
        return Err(GeminiUrlError::UnknownScheme(url.scheme().to_string()));
    }

    Ok(url)
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum GeminiUrlError {
    #[error("parser error: {0}")]
    ParseError(#[from] url::ParseError),
    #[error("missing authority")]
    MissingAuthority,
    #[error("missing host")]
    MissingHost,
    #[error("userinfo component not allowed")]
    UserinfoNotAllowed,
    #[error("unknown scheme: {0}")]
    UnknownScheme(String),
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse() {
        let result = parse_gemini_url("gemini://gemini.circumlunar.space/docs/specification.gmi");
        assert!(result.is_ok());
        let url = result.ok().unwrap();
        assert_eq!(url.scheme(), "gemini");
        assert_eq!(url.host(), Some(url::Host::Domain("gemini.circumlunar.space")));
        assert_eq!(url.path(), "/docs/specification.gmi");
    }

    #[test]
    fn test_missing_host() {
        let result = parse_gemini_url("gemini://");
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), GeminiUrlError::MissingHost);
    }

    #[test]
    fn test_userinfo_present() {
        let result = parse_gemini_url("gemini://test:test@example.org");
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), GeminiUrlError::UserinfoNotAllowed);
    }

    #[test]
    fn test_unknown_scheme() {
        let result = parse_gemini_url("http://example.org");
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), GeminiUrlError::UnknownScheme("http".to_string()));
    }
}
