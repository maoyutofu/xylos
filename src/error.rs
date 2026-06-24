use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("failed to read config file `{path}`: {source}")]
    ConfigRead {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config file `{path}`: {source}")]
    ConfigParse {
        path: String,
        #[source]
        source: toml::de::Error,
    },
    #[error("invalid quick start arguments: {0}")]
    QuickStartInvalid(String),
    #[error("invalid config: {0}")]
    ConfigInvalid(String),
    #[error("invalid socket address `{addr}`: {source}")]
    AddrParse {
        addr: String,
        #[source]
        source: std::net::AddrParseError,
    },
    #[error("failed to create root directory `{path}`: {source}")]
    RootDirCreate {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("root directory `{path}` does not exist")]
    RootDirMissing { path: String },
    #[error("root path `{path}` is not a directory")]
    RootDirNotDirectory { path: String },
    #[error("failed to canonicalize root directory `{path}`: {source}")]
    RootDirCanonicalize {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("server error: {0}")]
    Server(#[from] std::io::Error),
}
