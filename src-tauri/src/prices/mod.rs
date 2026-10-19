//! USD prices for assets, keyed by DefiLlama coin ids.

pub mod defillama;

use std::collections::HashMap;

use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::error::Result;

#[derive(Clone, Debug, PartialEq)]
pub struct Quote {
    pub price_usd: Decimal,
    /// 24h change as a ratio (0.05 = +5%), when the provider reports it.
    pub change_24h: Option<Decimal>,
    pub symbol: Option<String>,
}

#[async_trait]
pub trait PriceProvider: Send + Sync {
    /// Current quotes for `keys`. Keys the provider can't price (or prices with
    /// low confidence, typical of spam tokens) are simply absent.
    async fn quotes(&self, keys: &[String]) -> Result<HashMap<String, Quote>>;
}
