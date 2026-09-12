use std::{env, net::SocketAddr, path::PathBuf};

const DEFAULT_MAX_BODY_BYTES: usize = 10 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub data_dir: PathBuf,
    pub public_base_url: String,
    pub max_body_bytes: usize,
}

impl Config {
    /// Reads configuration from the environment, falling back to defaults that
    /// work for a local `cargo run`.
    pub fn from_env() -> Result<Self, String> {
        let bind_addr = match env::var("BIND_ADDR") {
            Ok(addr) => addr
                .parse()
                .map_err(|e| format!("invalid BIND_ADDR `{addr}`: {e}"))?,
            Err(_) => {
                let port = match env::var("PORT") {
                    Ok(p) => p
                        .parse::<u16>()
                        .map_err(|e| format!("invalid PORT `{p}`: {e}"))?,
                    Err(_) => 8080,
                };
                SocketAddr::from(([0, 0, 0, 0], port))
            }
        };

        let data_dir = env::var("DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./data"));

        let public_base_url = env::var("PUBLIC_BASE_URL")
            .unwrap_or_else(|_| format!("http://localhost:{}", bind_addr.port()));
        let public_base_url = public_base_url.trim_end_matches('/').to_string();

        let max_body_bytes = match env::var("MAX_BODY_BYTES") {
            Ok(v) => v
                .parse()
                .map_err(|e| format!("invalid MAX_BODY_BYTES `{v}`: {e}"))?,
            Err(_) => DEFAULT_MAX_BODY_BYTES,
        };

        Ok(Self {
            bind_addr,
            data_dir,
            public_base_url,
            max_body_bytes,
        })
    }

    /// Builds the public view URL. When a title exists, a human-readable slug
    /// is appended for sharing; the slug is cosmetic and ignored for routing.
    pub fn view_uri(&self, id: uuid::Uuid, title: Option<&str>) -> String {
        if let Some(title) = title {
            let slug = slugify(title);
            if !slug.is_empty() {
                return format!("{}/a/{}-{}", self.public_base_url, id, slug);
            }
        }
        format!("{}/a/{}", self.public_base_url, id)
    }
}

fn slugify(title: &str) -> String {
    let mut slug = String::with_capacity(title.len());
    let mut prev_sep = true;
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
            prev_sep = false;
        } else if !prev_sep {
            slug.push('-');
            prev_sep = true;
        }
    }
    if slug.ends_with('-') {
        slug.pop();
    }
    const MAX_SLUG_LEN: usize = 60;
    if slug.len() > MAX_SLUG_LEN {
        slug.truncate(MAX_SLUG_LEN);
        while slug.ends_with('-') {
            slug.pop();
        }
    }
    slug
}
