use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};

use crate::error::{Error, Result};

#[derive(Deserialize)]
struct Response<T> {
    result: Option<T>,
    error: Option<RpcError>,
}

#[derive(Deserialize)]
struct RpcError {
    message: String,
}

/// Calls a JSON-RPC method, trying each public endpoint in order until one
/// answers. Free endpoints rate-limit aggressively, so a fallback matters.
pub async fn call<T: DeserializeOwned>(
    http: &reqwest::Client,
    endpoints: &[&str],
    method: &str,
    params: Value,
) -> Result<T> {
    let body = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
    let mut last = Error::provider(format!("no endpoint configured for {method}"));
    for endpoint in endpoints {
        match call_one(http, endpoint, &body).await {
            Ok(value) => return Ok(value),
            Err(err) => {
                log_failure(endpoint, method, &err);
                last = err;
            }
        }
    }
    Err(last)
}

async fn call_one<T: DeserializeOwned>(
    http: &reqwest::Client,
    endpoint: &str,
    body: &Value,
) -> Result<T> {
    let response: Response<T> = http
        .post(endpoint)
        .json(body)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    match (response.result, response.error) {
        (Some(result), _) => Ok(result),
        (None, Some(err)) => Err(Error::provider(err.message)),
        (None, None) => Err(Error::provider("empty JSON-RPC response")),
    }
}

#[derive(Deserialize)]
struct BatchItem {
    id: usize,
    result: Option<Value>,
}

/// Sends many calls of one method as a single JSON-RPC batch. Each slot of the
/// result is `None` when that individual call failed.
pub async fn batch(
    http: &reqwest::Client,
    endpoints: &[&str],
    method: &str,
    params: Vec<Value>,
) -> Result<Vec<Option<Value>>> {
    if params.is_empty() {
        return Ok(Vec::new());
    }
    let body: Vec<Value> = params
        .iter()
        .enumerate()
        .map(|(id, p)| json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": p }))
        .collect();
    let mut last = Error::provider(format!("no endpoint configured for {method}"));
    for endpoint in endpoints {
        let response = async {
            let items: Vec<BatchItem> = http
                .post(*endpoint)
                .json(&body)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            Ok::<_, Error>(items)
        };
        match response.await {
            Ok(items) => {
                let mut out = vec![None; params.len()];
                for item in items {
                    if let Some(slot) = out.get_mut(item.id) {
                        *slot = item.result;
                    }
                }
                return Ok(out);
            }
            Err(err) => {
                log_failure(endpoint, method, &err);
                last = err;
            }
        }
    }
    Err(last)
}

fn log_failure(endpoint: &str, method: &str, err: &Error) {
    if cfg!(debug_assertions) {
        eprintln!("[rpc] {method} via {endpoint} failed: {err}");
    }
}
