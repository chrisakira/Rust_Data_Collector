use reqwest::Client;
use crate::api::CurrencyRate;

/// Configuration for connecting to InfluxDB 3.x.
pub struct InfluxConfig {
    pub host: String,     // e.g. "http://localhost:8181"
    pub token: String,    // Admin token generated on first run
    pub database: String, // e.g. "exchange-rates"
}

/// Inserts a CurrencyRate into InfluxDB 3.x via the
/// native /api/v3/write_lp HTTP endpoint using Line Protocol.
///
/// No external InfluxDB crate needed — just plain HTTP with reqwest.
///
/// # Arguments
/// * `config` - InfluxDB 3 connection config
/// * `rate`   - The exchange rate data to insert
pub async fn insert_rate(
    config: &InfluxConfig,
    rate: &CurrencyRate,
) -> Result<(), Box<dyn std::error::Error>> {
    // Parse numeric fields (Line Protocol requires unquoted floats)
    let bid: f64        = rate.bid.parse().unwrap_or(0.0);
    let ask: f64        = rate.ask.parse().unwrap_or(0.0);
    let high: f64       = rate.high.parse().unwrap_or(0.0);
    let low: f64        = rate.low.parse().unwrap_or(0.0);
    let var_bid: f64    = rate.varBid.parse().unwrap_or(0.0);
    let pct_change: f64 = rate.pctChange.parse().unwrap_or(0.0);

    // Build the Line Protocol string:
    // <measurement>,<tag_key>=<tag_val> <field_key>=<field_val>
    // Tags   → indexed strings, no quotes
    // Fields → floats unquoted, strings in double quotes
    let line_protocol = format!(
        "exchange_rate,from={from},to={to},pair={pair} \
         bid={bid},ask={ask},high={high},low={low},\
         var_bid={var_bid},pct_change={pct_change},\
         name=\"{name}\",timestamp_api=\"{ts}\"",
        from       = rate.code,
        to         = rate.codein,
        pair       = format!("{}-{}", rate.code, rate.codein),
        bid        = bid,
        ask        = ask,
        high       = high,
        low        = low,
        var_bid    = var_bid,
        pct_change = pct_change,
        name       = rate.name.replace('"', "\\\""),
        ts         = rate.create_date,
    );

    // Build the write URL: /api/v3/write_lp?db=<database>&precision=second
    let url = format!(
        "{}/api/v3/write_lp?db={}&precision=second",
        config.host.trim_end_matches('/'),
        config.database,
    );

    let client = Client::new();

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.token))
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(line_protocol)
        .send()
        .await?;

    if response.status().is_success() {
        println!(
            "✅ Data written to InfluxDB 3 database '{}'",
            config.database
        );
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("❌ InfluxDB write failed [{status}]: {body}").into());
    }

    Ok(())
}