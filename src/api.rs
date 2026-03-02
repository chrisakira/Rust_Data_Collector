use serde::Deserialize;
use std::collections::HashMap;

/// Represents the exchange rate data returned by the AwesomeAPI.
#[derive(Debug, Deserialize, Clone)]
#[allow(non_snake_case)]
pub struct CurrencyRate {
    pub code: String,
    pub codein: String,
    pub name: String,
    pub high: String,
    pub low: String,
    pub varBid: String,
    pub pctChange: String,
    pub bid: String,
    pub ask: String,
    pub timestamp: String,
    pub create_date: String,
}

/// Fetches the latest USD → BRL rate from AwesomeAPI.
pub async fn fetch_usd_brl() -> Result<CurrencyRate, Box<dyn std::error::Error>> {
    let url = "https://economia.awesomeapi.com.br/json/last/USD-BRL";

    let response = reqwest::get(url)
        .await?
        .json::<HashMap<String, CurrencyRate>>()
        .await?;

    response
        .get("USDBRL")
        .cloned()
        .ok_or_else(|| "USDBRL key not found in API response".into())
}