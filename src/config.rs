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

    pub fn view_uri(&self, id: uuid::Uuid) -> String {
        format!("{}/a/{}", self.public_base_url, id)
    }
}
