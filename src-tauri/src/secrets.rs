//! Bring-your-own API keys, kept in the OS keychain (Windows Credential
//! Manager, macOS Keychain, Secret Service on Linux) and never in the
//! database or backups.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;

use crate::{
    error::{Error, Result},
    state::AppState,
};

/// Keychain service name; each provider is an account under it.
const SERVICE: &str = "io.github.raggzinn.chainfolio";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Provider {
    Etherscan,
    Helius,
    Coingecko,
    Trongrid,
}

impl Provider {
    pub const ALL: [Provider; 4] = [
        Provider::Etherscan,
        Provider::Helius,
        Provider::Coingecko,
        Provider::Trongrid,
    ];

    fn account(self) -> &'static str {
        match self {
            Self::Etherscan => "etherscan",
            Self::Helius => "helius",
            Self::Coingecko => "coingecko",
            Self::Trongrid => "trongrid",
        }
    }
}

pub trait SecretStore: Send + Sync {
    fn get(&self, provider: Provider) -> Result<Option<String>>;
    fn set(&self, provider: Provider, key: &str) -> Result<()>;
    fn delete(&self, provider: Provider) -> Result<()>;
}

/// The operating system's credential store.
pub struct Keychain;

fn keychain_error(err: keyring::Error) -> Error {
    Error::Keychain(err.to_string())
}

impl Keychain {
    fn entry(provider: Provider) -> Result<keyring::Entry> {
        keyring::Entry::new(SERVICE, provider.account()).map_err(keychain_error)
    }
}

impl SecretStore for Keychain {
    fn get(&self, provider: Provider) -> Result<Option<String>> {
        match Self::entry(provider)?.get_password() {
            Ok(key) => Ok(Some(key)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(keychain_error(err)),
        }
    }

    fn set(&self, provider: Provider, key: &str) -> Result<()> {
        Self::entry(provider)?
            .set_password(key)
            .map_err(keychain_error)
    }

    fn delete(&self, provider: Provider) -> Result<()> {
        match Self::entry(provider)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(keychain_error(err)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyStatus {
    pub provider: Provider,
    pub configured: bool,
    /// Last four characters, so the user can tell which key is stored.
    pub hint: Option<String>,
}

pub fn statuses(store: &dyn SecretStore) -> Result<Vec<ApiKeyStatus>> {
    Provider::ALL
        .iter()
        .map(|&provider| {
            let key = store.get(provider)?;
            Ok(ApiKeyStatus {
                provider,
                configured: key.is_some(),
                hint: key.map(|k| {
                    let tail: String = k.chars().rev().take(4).collect();
                    tail.chars().rev().collect()
                }),
            })
        })
        .collect()
}

/// Sends one cheap authenticated request with `key` and reports whether the
/// provider accepted it.
pub async fn test_key(http: &reqwest::Client, provider: Provider, key: &str) -> Result<()> {
    let with_params = |base: &str, params: &[(&str, &str)]| {
        reqwest::Url::parse_with_params(base, params).map_err(|e| Error::provider(e.to_string()))
    };
    let request = match provider {
        Provider::Etherscan => http.get(with_params(
            "https://api.etherscan.io/v2/api",
            &[
                ("chainid", "1"),
                ("module", "account"),
                ("action", "balance"),
                ("address", "0x0000000000000000000000000000000000000000"),
                ("tag", "latest"),
                ("apikey", key),
            ],
        )?),
        Provider::Helius => http
            .post(with_params(
                "https://mainnet.helius-rpc.com/",
                &[("api-key", key)],
            )?)
            .json(&serde_json::json!({ "jsonrpc": "2.0", "id": 1, "method": "getHealth" })),
        Provider::Coingecko => {
            // Demo keys only work on the public host, paid keys on the pro one.
            let demo = http
                .get("https://api.coingecko.com/api/v3/ping")
                .header("x-cg-demo-api-key", key);
            let response = demo.send().await?;
            let status = response.status().as_u16();
            let body = response.json().await.unwrap_or(Value::Null);
            if interpret(provider, status, &body).is_ok() {
                return Ok(());
            }
            http.get("https://pro-api.coingecko.com/api/v3/ping")
                .header("x-cg-pro-api-key", key)
        }
        Provider::Trongrid => http
            .get("https://api.trongrid.io/wallet/getnowblock")
            .header("TRON-PRO-API-KEY", key),
    };
    let response = request.send().await?;
    let status = response.status().as_u16();
    let body = response.json().await.unwrap_or(Value::Null);
    interpret(provider, status, &body)
}

/// Turns a provider's answer into accepted / rejected key / other failure.
fn interpret(provider: Provider, status: u16, body: &Value) -> Result<()> {
    if matches!(status, 401 | 403) {
        return Err(Error::InvalidApiKey);
    }
    if !(200..300).contains(&status) {
        return Err(Error::provider(format!("HTTP {status}")));
    }
    match provider {
        // Etherscan answers 200 with {"status":"0","result":"Invalid API Key"}.
        Provider::Etherscan if body["status"] != "1" => {
            let result = body["result"].as_str().unwrap_or_default();
            if result.to_ascii_lowercase().contains("api key") {
                Err(Error::InvalidApiKey)
            } else {
                Err(Error::provider(result))
            }
        }
        Provider::Helius if body.get("error").is_some() => Err(Error::provider(
            body["error"]["message"].as_str().unwrap_or("RPC error"),
        )),
        Provider::Coingecko if body["status"]["error_code"].is_number() => {
            Err(Error::InvalidApiKey)
        }
        _ => Ok(()),
    }
}

#[tauri::command]
pub fn list_api_keys() -> Result<Vec<ApiKeyStatus>> {
    statuses(&Keychain)
}

#[tauri::command]
pub fn set_api_key(provider: Provider, key: String) -> Result<()> {
    let key = key.trim();
    if key.is_empty() {
        return Err(Error::InvalidApiKey);
    }
    Keychain.set(provider, key)
}

#[tauri::command]
pub fn remove_api_key(provider: Provider) -> Result<()> {
    Keychain.delete(provider)
}

/// Tests `key`, or the stored key when `key` is omitted.
#[tauri::command]
pub async fn test_api_key(
    state: State<'_, AppState>,
    provider: Provider,
    key: Option<String>,
) -> Result<()> {
    let key = match key.filter(|k| !k.trim().is_empty()) {
        Some(key) => key.trim().to_string(),
        None => Keychain.get(provider)?.ok_or(Error::InvalidApiKey)?,
    };
    test_key(&state.http, provider, &key).await
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Mutex};

    use serde_json::json;

    use super::*;

    #[derive(Default)]
    struct MemoryStore(Mutex<HashMap<Provider, String>>);

    impl SecretStore for MemoryStore {
        fn get(&self, provider: Provider) -> Result<Option<String>> {
            Ok(self.0.lock().unwrap().get(&provider).cloned())
        }
        fn set(&self, provider: Provider, key: &str) -> Result<()> {
            self.0.lock().unwrap().insert(provider, key.into());
            Ok(())
        }
        fn delete(&self, provider: Provider) -> Result<()> {
            self.0.lock().unwrap().remove(&provider);
            Ok(())
        }
    }

    #[test]
    fn statuses_show_only_a_hint_of_each_key() {
        let store = MemoryStore::default();
        store.set(Provider::Helius, "abcdef-1234-wxyz").unwrap();
        let list = statuses(&store).unwrap();
        assert_eq!(list.len(), 4);
        let helius = list
            .iter()
            .find(|s| s.provider == Provider::Helius)
            .unwrap();
        assert!(helius.configured);
        assert_eq!(helius.hint.as_deref(), Some("wxyz"));
        assert!(!list[0].configured);
        store.delete(Provider::Helius).unwrap();
        assert!(statuses(&store).unwrap().iter().all(|s| !s.configured));
    }

    #[test]
    fn interprets_provider_answers() {
        let invalid = |r: Result<()>| matches!(r, Err(Error::InvalidApiKey));
        assert!(invalid(interpret(
            Provider::Etherscan,
            200,
            &json!({"status": "0", "message": "NOTOK", "result": "Invalid API Key (#err2)"})
        )));
        assert!(
            interpret(
                Provider::Etherscan,
                200,
                &json!({"status": "1", "message": "OK", "result": "0"})
            )
            .is_ok()
        );
        assert!(invalid(interpret(Provider::Helius, 401, &Value::Null)));
        assert!(interpret(Provider::Helius, 200, &json!({"result": "ok"})).is_ok());
        assert!(invalid(interpret(
            Provider::Coingecko,
            200,
            &json!({"status": {"error_code": 10002}})
        )));
        assert!(
            interpret(
                Provider::Coingecko,
                200,
                &json!({"gecko_says": "(V3) To the Moon!"})
            )
            .is_ok()
        );
        assert!(invalid(interpret(
            Provider::Trongrid,
            401,
            &json!({"Error": "ApiKey not exists"})
        )));
        assert!(matches!(
            interpret(Provider::Trongrid, 500, &Value::Null),
            Err(Error::Provider(_))
        ));
    }

    #[tokio::test]
    #[ignore = "network"]
    async fn every_provider_rejects_a_bogus_key() {
        let http = reqwest::Client::new();
        for provider in Provider::ALL {
            let result = test_key(&http, provider, "chainfolio-bogus-key").await;
            assert!(
                matches!(result, Err(Error::InvalidApiKey)),
                "{provider:?}: {result:?}"
            );
        }
    }

    #[test]
    fn providers_serialize_in_camel_case() {
        assert_eq!(
            serde_json::to_string(&Provider::Coingecko).unwrap(),
            "\"coingecko\""
        );
    }
}
