use serde::{Serialize, Serializer, ser::SerializeStruct};

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not a valid EVM, Solana, Bitcoin, TRON or XRP Ledger address")]
    InvalidAddress,
    #[error("address checksum does not match")]
    BadChecksum,
    #[error("could not resolve {0}")]
    NameNotFound(String),
    #[error("this wallet is already being tracked")]
    Duplicate,
    #[error("wallet not found")]
    NotFound,
    #[error("label cannot be empty")]
    EmptyLabel,
    #[error("the database is locked")]
    Locked,
    #[error("wrong password")]
    WrongPassword,
    #[error("password must be at least 8 characters")]
    WeakPassword,
    #[error("this alert can never fire; check its threshold")]
    InvalidAlert,
    #[error("the API key was rejected")]
    InvalidApiKey,
    #[error("keychain: {0}")]
    Keychain(String),
    #[error("{0}")]
    Provider(String),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    Db(#[from] rusqlite::Error),
    #[error(transparent)]
    Migration(#[from] refinery::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Error {
    /// Stable identifier the frontend maps to a translated message.
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidAddress => "invalidAddress",
            Self::BadChecksum => "badChecksum",
            Self::NameNotFound(_) => "nameNotFound",
            Self::Duplicate => "duplicate",
            Self::NotFound => "notFound",
            Self::EmptyLabel => "emptyLabel",
            Self::Locked => "locked",
            Self::WrongPassword => "wrongPassword",
            Self::WeakPassword => "weakPassword",
            Self::InvalidAlert => "invalidAlert",
            Self::InvalidApiKey => "invalidApiKey",
            Self::Keychain(_) => "keychain",
            Self::Provider(_) | Self::Http(_) => "network",
            Self::Db(_) | Self::Migration(_) | Self::Io(_) => "storage",
        }
    }

    pub fn provider(message: impl Into<String>) -> Self {
        Self::Provider(message.into())
    }
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("Error", 2)?;
        s.serialize_field("code", self.code())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}
