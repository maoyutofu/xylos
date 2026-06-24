use std::error::Error;
use std::io::{self, Write};
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use xylos::config::{AppConfig, DigestAlgorithm, QuickStartConfig};
use xylos::password::{digest_ha1, hash_password};
use xylos::server;

#[derive(Debug, Parser)]
#[command(name = "xylos")]
#[command(about = "A configurable Rust WebDAV service")]
struct Cli {
    #[arg(short, long, default_value = "config.toml")]
    config: String,
    #[arg(long, value_name = "HOST")]
    host: Option<String>,
    #[arg(long, value_name = "PORT")]
    port: Option<u16>,
    #[arg(long, value_name = "PATH")]
    base_path: Option<String>,
    #[arg(long, value_name = "DIR")]
    root_dir: Option<PathBuf>,
    #[arg(long, value_name = "USERNAME")]
    username: Option<String>,
    #[arg(
        long,
        value_name = "PASSWORD",
        help = "Plaintext password; Xylos hashes it automatically for Basic auth"
    )]
    password: Option<String>,
    #[arg(long, value_name = "REALM")]
    realm: Option<String>,
    #[arg(long, value_name = "LEVEL")]
    log_level: Option<String>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "Generate password_hash and digest_ha1 lines from a plaintext password")]
    HashPassword {
        #[arg(short, long, value_name = "USERNAME")]
        username: Option<String>,
        #[arg(long, default_value = "xylos", value_name = "REALM")]
        realm: String,
        #[arg(long, default_value = "md5", value_name = "ALGORITHM")]
        digest_algorithm: CliDigestAlgorithm,
        #[arg(
            short,
            long,
            value_name = "PASSWORD",
            help = "Plaintext password to hash"
        )]
        password: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum CliDigestAlgorithm {
    Md5,
    Sha256,
}

impl From<CliDigestAlgorithm> for DigestAlgorithm {
    fn from(algorithm: CliDigestAlgorithm) -> Self {
        match algorithm {
            CliDigestAlgorithm::Md5 => DigestAlgorithm::Md5,
            CliDigestAlgorithm::Sha256 => DigestAlgorithm::Sha256,
        }
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(err) = run(cli).await {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<(), Box<dyn Error>> {
    match cli.command {
        Some(Command::HashPassword {
            username,
            realm,
            digest_algorithm,
            password,
        }) => generate_password_config(username, realm, digest_algorithm.into(), password),
        None => serve(cli).await,
    }
}

async fn serve(cli: Cli) -> Result<(), Box<dyn Error>> {
    let config = if cli.uses_quick_start() {
        AppConfig::from_quick_start(cli.into_quick_start_config()?)?
    } else {
        AppConfig::load_from_path(&cli.config)?
    };

    tracing_subscriber::fmt()
        .with_env_filter(config.logging.level.as_str())
        .init();

    tracing::info!("startup config: {}", config.startup_summary());
    server::serve(config).await?;
    Ok(())
}

impl Cli {
    fn uses_quick_start(&self) -> bool {
        self.host.is_some()
            || self.port.is_some()
            || self.base_path.is_some()
            || self.root_dir.is_some()
            || self.username.is_some()
            || self.password.is_some()
            || self.realm.is_some()
            || self.log_level.is_some()
    }

    fn into_quick_start_config(self) -> Result<QuickStartConfig, io::Error> {
        let root_dir = self.root_dir.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "quick start requires `--root-dir`",
            )
        })?;
        let username = self.username.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "quick start requires `--username`",
            )
        })?;
        let password = self.password.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "quick start requires `--password`",
            )
        })?;

        Ok(QuickStartConfig {
            host: self.host.unwrap_or_else(|| "0.0.0.0".into()),
            port: self.port.unwrap_or(8080),
            base_path: self.base_path.unwrap_or_else(|| "/dav".into()),
            root_dir,
            username,
            password,
            realm: self.realm.unwrap_or_else(|| "xylos".into()),
            log_level: self.log_level.unwrap_or_else(|| "info".into()),
        })
    }
}

fn generate_password_config(
    username: Option<String>,
    realm: String,
    digest_algorithm: DigestAlgorithm,
    password: Option<String>,
) -> Result<(), Box<dyn Error>> {
    let username = match username {
        Some(username) => username,
        None => prompt_line("Username: ")?,
    };

    if username.trim().is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "username cannot be empty").into());
    }

    if realm.trim().is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "realm cannot be empty").into());
    }

    let password = match password {
        Some(password) => password,
        None => rpassword::prompt_password("Password: ")?,
    };

    if password.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "password cannot be empty").into());
    }

    let password_hash = hash_password(&password)
        .map_err(|err| io::Error::other(format!("failed to hash password: {err}")))?;
    let digest_ha1 = digest_ha1(digest_algorithm, &username, &realm, &password);
    println!("password_hash = \"{password_hash}\"");
    println!("digest_ha1 = \"{digest_ha1}\"");
    Ok(())
}

fn prompt_line(prompt: &str) -> io::Result<String> {
    print!("{prompt}");
    io::stdout().flush()?;

    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    println!();
    Ok(value.trim_end_matches(['\r', '\n']).to_owned())
}
