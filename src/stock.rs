use serde::Deserialize;
use crate::influx::InfluxConfig;

/// List of Brazilian stocks to collect
const STOCKS: &[&str] = &["PETR4", "BBAS3", "CMIG4", "SAPR11", "BBSE3", "WIZC3"];

/// Base URL for the BRAPI stock API
const BRAPI_BASE_URL: &str = "https://brapi.dev/api/quote";

// ─── API Response Structs ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Clone)]
pub struct StockApiResponse {
    pub results: Vec<StockQuote>,
    #[serde(rename = "requestedAt")]
    pub requested_at: String,
    pub took: Option<i64>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(non_snake_case)]
pub struct StockQuote {
    pub symbol: String,
    pub shortName: String,
    pub longName: String,
    pub currency: String,
    pub regularMarketPrice: f64,
    pub regularMarketDayHigh: f64,
    pub regularMarketDayLow: f64,
    pub regularMarketDayRange: String,
    pub regularMarketChange: f64,
    pub regularMarketChangePercent: f64,
    pub regularMarketTime: String,
    // marketCap can be null in BRAPI -> Option<i64>
    pub marketCap: Option<i64>,
    pub regularMarketVolume: i64,
    pub regularMarketPreviousClose: f64,
    pub regularMarketOpen: f64,
    pub fiftyTwoWeekRange: String,
    pub fiftyTwoWeekLow: f64,
    pub fiftyTwoWeekHigh: f64,
    // priceEarnings and earningsPerShare can be null -> Option<f64>
    pub priceEarnings: Option<f64>,
    pub earningsPerShare: Option<f64>,
    pub logourl: Option<String>,
}

// ─── Fetch Stock Data ────────────────────────────────────────────────────────

/// Fetches a single Brazilian stock quote from BRAPI.
async fn fetch_single_stock(
    symbol: &str,
    token: Option<&String>,
) -> Result<StockQuote, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}/{}", BRAPI_BASE_URL, symbol);
    
    let client = reqwest::Client::new();
    let mut request = client.get(&url);
    
    // Add Authorization header if token is provided
    if let Some(token) = token {
        request = request.header("Authorization", format!("Bearer {}", token));
    }
    
    let response = request
        .send()
        .await?
        .json::<StockApiResponse>()
        .await?;
    // Extract the first result (should only be one for single stock request)
    response.results
        .into_iter()
        .next()
        .ok_or_else(|| format!("No data returned for stock {}", symbol).into())
}

/// Fetches all Brazilian stock quotes from BRAPI (one request per stock).
pub async fn fetch_stocks(
    token: Option<&String>,
) -> Result<Vec<StockQuote>, Box<dyn std::error::Error + Send + Sync>> {
    let mut all_stocks = Vec::new();
    let mut errors = Vec::new();

    for stock_symbol in STOCKS {
        match fetch_single_stock(stock_symbol, token).await {
            Ok(stock) => {
                println!("  ✓ {} : R$ {:.2}", stock.symbol, stock.regularMarketPrice);
                all_stocks.push(stock);
            }
            Err(e) => {
                eprintln!("  ✗ {} : {}", stock_symbol, e);
                errors.push(format!("{}: {}", stock_symbol, e));
            }
        }
        
        // Small delay between requests to avoid rate limiting
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    if all_stocks.is_empty() {
        return Err(format!("Failed to fetch any stocks. Errors: {}", errors.join(", ")).into());
    }

    // Return success even if some stocks failed (partial success)
    if !errors.is_empty() {
        eprintln!("⚠️  Some stocks failed: {}", errors.join(", "));
    }

    Ok(all_stocks)
}

// ─── InfluxDB Insert ─────────────────────────────────────────────────────────

/// Inserts all stock data into InfluxDB 3.x via Line Protocol.
pub async fn insert_stocks(
    config: &InfluxConfig,
    stocks: &[StockQuote],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if stocks.is_empty() {
        return Err("No stock data to insert".into());
    }

    let mut lines: Vec<String> = Vec::new();

    for stock in stocks {
        // Parse optional fields safely
        let pe_ratio = stock.priceEarnings.unwrap_or(0.0);
        let eps = stock.earningsPerShare.unwrap_or(0.0);

        // Escape string fields for InfluxDB line protocol
        let short_name = stock.shortName.replace('"', "\\\"");
        let long_name = stock.longName.replace('"', "\\\"");
        let market_cap = stock.marketCap.unwrap_or(0);
        let pe_ratio = stock.priceEarnings.unwrap_or(0.0);
        let eps = stock.earningsPerShare.unwrap_or(0.0);

        // then use those variables in the format! call:
        let line = format!(
            "brazilian_stocks,symbol={symbol},currency={currency} \
            price={price},\
            day_high={day_high},\
            day_low={day_low},\
            day_change={day_change},\
            day_change_percent={day_change_percent},\
            market_cap={market_cap}i,\
            volume={volume}i,\
            previous_close={prev_close},\
            open={open},\
            week_52_low={week_52_low},\
            week_52_high={week_52_high},\
            pe_ratio={pe_ratio},\
            eps={eps},\
            short_name=\"{short_name}\",\
            long_name=\"{long_name}\",\
            market_time=\"{market_time}\"",
            symbol = stock.symbol,
            currency = stock.currency,
            price = stock.regularMarketPrice,
            day_high = stock.regularMarketDayHigh,
            day_low = stock.regularMarketDayLow,
            day_change = stock.regularMarketChange,
            day_change_percent = stock.regularMarketChangePercent,
            market_cap = market_cap,
            volume = stock.regularMarketVolume,
            prev_close = stock.regularMarketPreviousClose,
            open = stock.regularMarketOpen,
            week_52_low = stock.fiftyTwoWeekLow,
            week_52_high = stock.fiftyTwoWeekHigh,
            pe_ratio = pe_ratio,
            eps = eps,
            short_name = short_name,
            long_name = long_name,
            market_time = stock.regularMarketTime,
        );

        lines.push(line);
    }

    // Write all lines in one request
    let url = format!(
        "{}/api/v3/write_lp?db={}&precision=second",
        config.host.trim_end_matches('/'),
        config.database,
    );

    let body = lines.join("\n");
    let client = reqwest::Client::new();

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.token))
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(body)
        .send()
        .await?;

    if response.status().is_success() {
        println!(
            "✅ Stock data written to InfluxDB 3 ({} stocks)",
            stocks.len(),
        );
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("❌ InfluxDB stock write failed [{status}]: {body}").into());
    }

    Ok(())
}