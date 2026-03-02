use serde::Deserialize;
use crate::influx::InfluxConfig;

const WEATHER_API_URL: &str = "https://api.open-meteo.com/v1/forecast\
    ?latitude=-25.4278\
    &longitude=-49.2731\
    &daily=sunrise,sunset,daylight_duration,uv_index_max,uv_index_clear_sky_max,\
precipitation_sum,precipitation_hours,rain_sum,precipitation_probability_max\
    &hourly=temperature_2m,relative_humidity_2m,apparent_temperature,\
precipitation_probability,precipitation,rain,showers,weather_code,\
visibility,cloud_cover_high,cloud_cover_mid,cloud_cover_low,cloud_cover\
    &current=temperature_2m,relative_humidity_2m,apparent_temperature,\
is_day,rain,precipitation,showers,cloud_cover,weather_code\
    &minutely_15=temperature_2m,relative_humidity_2m,apparent_temperature,\
precipitation,rain,sunshine_duration,weather_code\
    &timezone=America%2FSao_Paulo";

// ─── API Response Structs ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Clone)]
pub struct WeatherResponse {
    pub latitude: f64,
    pub longitude: f64,
    pub timezone: String,
    pub current: CurrentWeather,
    pub current_units: CurrentUnits,
    pub hourly: HourlyWeather,
    pub hourly_units: HourlyUnits,
    pub daily: DailyWeather,
    pub daily_units: DailyUnits,
    pub minutely_15: Minutely15Weather,
    pub minutely_15_units: Minutely15Units,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CurrentWeather {
    pub time: String,
    pub temperature_2m: f64,
    pub relative_humidity_2m: f64,
    pub apparent_temperature: f64,
    pub is_day: u8,
    pub rain: f64,
    pub precipitation: f64,
    pub showers: f64,
    pub cloud_cover: f64,
    pub weather_code: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CurrentUnits {
    pub temperature_2m: String,
    pub relative_humidity_2m: String,
    pub apparent_temperature: String,
    pub rain: String,
    pub precipitation: String,
    pub showers: String,
    pub cloud_cover: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HourlyWeather {
    pub time: Vec<String>,
    pub temperature_2m: Vec<f64>,
    pub relative_humidity_2m: Vec<f64>,
    pub apparent_temperature: Vec<f64>,
    pub precipitation_probability: Vec<Option<f64>>,
    pub precipitation: Vec<f64>,
    pub rain: Vec<f64>,
    pub showers: Vec<f64>,
    pub weather_code: Vec<u32>,
    pub visibility: Vec<f64>,
    pub cloud_cover_high: Vec<Option<f64>>,
    pub cloud_cover_mid: Vec<Option<f64>>,
    pub cloud_cover_low: Vec<Option<f64>>,
    pub cloud_cover: Vec<f64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HourlyUnits {
    pub temperature_2m: String,
    pub relative_humidity_2m: String,
    pub apparent_temperature: String,
    pub precipitation_probability: String,
    pub precipitation: String,
    pub rain: String,
    pub visibility: String,
    pub cloud_cover: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DailyWeather {
    pub time: Vec<String>,
    pub sunrise: Vec<String>,
    pub sunset: Vec<String>,
    pub daylight_duration: Vec<f64>,
    pub uv_index_max: Vec<f64>,
    pub uv_index_clear_sky_max: Vec<f64>,
    pub precipitation_sum: Vec<f64>,
    pub precipitation_hours: Vec<f64>,
    pub rain_sum: Vec<f64>,
    pub precipitation_probability_max: Vec<Option<f64>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DailyUnits {
    pub time: String,
    pub daylight_duration: String,
    pub uv_index_max: String,
    pub precipitation_sum: String,
    pub precipitation_hours: String,
    pub rain_sum: String,
    pub precipitation_probability_max: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Minutely15Weather {
    pub time: Vec<String>,
    pub temperature_2m: Vec<f64>,
    pub relative_humidity_2m: Vec<f64>,
    pub apparent_temperature: Vec<f64>,
    pub precipitation: Vec<f64>,
    pub rain: Vec<f64>,
    pub sunshine_duration: Vec<f64>,
    pub weather_code: Vec<u32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Minutely15Units {
    pub temperature_2m: String,
    pub relative_humidity_2m: String,
    pub apparent_temperature: String,
    pub precipitation: String,
    pub rain: String,
    pub sunshine_duration: String,
}

// ─── Fetch ───────────────────────────────────────────────────────────────────

pub async fn fetch_weather() -> Result<WeatherResponse, Box<dyn std::error::Error + Send + Sync>> {
    let response = reqwest::get(WEATHER_API_URL)
        .await?
        .json::<WeatherResponse>()
        .await?;

    Ok(response)
}

// ─── InfluxDB Insert ─────────────────────────────────────────────────────────

/// Inserts all weather data points into InfluxDB 3.x via Line Protocol.
/// Each data group (current, hourly, daily, minutely_15) is written
/// to its own measurement for easy querying.
pub async fn insert_weather(
    config: &InfluxConfig,
    weather: &WeatherResponse,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut lines: Vec<String> = Vec::new();

    let lat = weather.latitude;
    let lon = weather.longitude;
    let tz  = weather.timezone.replace('/', "_");

    // ── Current Weather ──────────────────────────────────────────────────────
    lines.push(format!(
        "weather_current,latitude={lat},longitude={lon},timezone={tz} \
         temperature={temp},apparent_temperature={apparent},\
         relative_humidity={humidity},is_day={is_day}i,\
         rain={rain},precipitation={precip},showers={showers},\
         cloud_cover={cloud},weather_code={code}i",
        lat      = lat,
        lon      = lon,
        tz       = tz,
        temp     = weather.current.temperature_2m,
        apparent = weather.current.apparent_temperature,
        humidity = weather.current.relative_humidity_2m,
        is_day   = weather.current.is_day,
        rain     = weather.current.rain,
        precip   = weather.current.precipitation,
        showers  = weather.current.showers,
        cloud    = weather.current.cloud_cover,
        code     = weather.current.weather_code,
    ));

    // ── Hourly Weather (7 days × 24h = 168 points) ───────────────────────────
    for (i, time) in weather.hourly.time.iter().enumerate() {
        lines.push(format!(
            "weather_hourly,latitude={lat},longitude={lon},timezone={tz},forecast_time={time} \
             temperature={temp},apparent_temperature={apparent},\
             relative_humidity={humidity},\
             precipitation_probability={precip_prob},\
             precipitation={precip},rain={rain},showers={showers},\
             weather_code={code}i,visibility={vis},\
             cloud_cover_high={cloud_high},cloud_cover_mid={cloud_mid},\
             cloud_cover_low={cloud_low},cloud_cover={cloud}",
            lat         = lat,
            lon         = lon,
            tz          = tz,
            time        = time,
            temp        = weather.hourly.temperature_2m[i],
            apparent    = weather.hourly.apparent_temperature[i],
            humidity    = weather.hourly.relative_humidity_2m[i],
            precip_prob = weather.hourly.precipitation_probability[i].unwrap_or(0.0),
            precip      = weather.hourly.precipitation[i],
            rain        = weather.hourly.rain[i],
            showers     = weather.hourly.showers[i],
            code        = weather.hourly.weather_code[i],
            vis         = weather.hourly.visibility[i],
            cloud_high  = weather.hourly.cloud_cover_high[i].unwrap_or(0.0),
            cloud_mid   = weather.hourly.cloud_cover_mid[i].unwrap_or(0.0),
            cloud_low   = weather.hourly.cloud_cover_low[i].unwrap_or(0.0),
            cloud       = weather.hourly.cloud_cover[i],
        ));
    }

    // ── Daily Weather (7 days) ───────────────────────────────────────────────
    for (i, time) in weather.daily.time.iter().enumerate() {
        lines.push(format!(
            "weather_daily,latitude={lat},longitude={lon},timezone={tz},forecast_date={date} \
             sunrise=\"{sunrise}\",sunset=\"{sunset}\",\
             daylight_duration={daylight},\
             uv_index_max={uv_max},uv_index_clear_sky_max={uv_clear},\
             precipitation_sum={precip_sum},precipitation_hours={precip_hours},\
             rain_sum={rain_sum},\
             precipitation_probability_max={precip_prob_max}",
            lat             = lat,
            lon             = lon,
            tz              = tz,
            date            = time,
            sunrise         = weather.daily.sunrise[i],
            sunset          = weather.daily.sunset[i],
            daylight        = weather.daily.daylight_duration[i],
            uv_max          = weather.daily.uv_index_max[i],
            uv_clear        = weather.daily.uv_index_clear_sky_max[i],
            precip_sum      = weather.daily.precipitation_sum[i],
            precip_hours    = weather.daily.precipitation_hours[i],
            rain_sum        = weather.daily.rain_sum[i],
            precip_prob_max = weather.daily.precipitation_probability_max[i].unwrap_or(0.0),
        ));
    }

    // ── Minutely 15 (7 days × 96 points/day = 672 points) ───────────────────
    for (i, time) in weather.minutely_15.time.iter().enumerate() {
        lines.push(format!(
            "weather_minutely_15,latitude={lat},longitude={lon},timezone={tz},forecast_time={time} \
             temperature={temp},apparent_temperature={apparent},\
             relative_humidity={humidity},precipitation={precip},\
             rain={rain},sunshine_duration={sunshine},weather_code={code}i",
            lat      = lat,
            lon      = lon,
            tz       = tz,
            time     = time,
            temp     = weather.minutely_15.temperature_2m[i],
            apparent = weather.minutely_15.apparent_temperature[i],
            humidity = weather.minutely_15.relative_humidity_2m[i],
            precip   = weather.minutely_15.precipitation[i],
            rain     = weather.minutely_15.rain[i],
            sunshine = weather.minutely_15.sunshine_duration[i],
            code     = weather.minutely_15.weather_code[i],
        ));
    }

    // ── Write all lines in one request ───────────────────────────────────────
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
            "✅ Weather data written to InfluxDB 3 ({} current + {} hourly + {} daily + {} minutely_15 points)",
            1,
            weather.hourly.time.len(),
            weather.daily.time.len(),
            weather.minutely_15.time.len(),
        );
    } else {
        let status = response.status();
        let body   = response.text().await.unwrap_or_default();
        return Err(format!("❌ InfluxDB weather write failed [{status}]: {body}").into());
    }

    Ok(())
}