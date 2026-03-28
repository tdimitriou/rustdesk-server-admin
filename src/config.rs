use std::env;

#[derive(Clone)]
pub struct Config {
    pub listen_addr: String,
    pub hbbs_db_path: Option<String>,
    pub admin_password: String,
    pub session_secret: Vec<u8>,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let host = env::var("ADMIN_HOST").unwrap_or_else(|_| "127.0.0.1".into());
        let port: u16 = env::var("ADMIN_PORT")
            .unwrap_or_else(|_| "3030".into())
            .parse()
            .map_err(|_| "ADMIN_PORT must be a valid TCP port number (0–65535)")?;
        let listen_addr = format!("{host}:{port}");

        let hbbs_db_path = env::var("HBBS_DB_PATH").ok().filter(|s| !s.trim().is_empty());

        let admin_password = env::var("ADMIN_PASSWORD").unwrap_or_default();
        if admin_password.is_empty() {
            return Err(
                "ADMIN_PASSWORD must be set (non-empty) for the admin UI to accept logins."
                    .into(),
            );
        }

        let session_secret = match env::var("ADMIN_SESSION_SECRET") {
            Ok(s) if !s.trim().is_empty() => s.into_bytes(),
            _ => {
                tracing::warn!(
                    "ADMIN_SESSION_SECRET not set; deriving weak key from ADMIN_PASSWORD (set ADMIN_SESSION_SECRET in production)"
                );
                let mut k = vec![0u8; 32];
                let p = admin_password.as_bytes();
                for (i, b) in k.iter_mut().enumerate() {
                    *b = p
                        .get(i % p.len())
                        .copied()
                        .unwrap_or(0)
                        .wrapping_add((i as u8).wrapping_mul(31));
                }
                k
            }
        };

        Ok(Config {
            listen_addr,
            hbbs_db_path,
            admin_password,
            session_secret,
        })
    }
}
