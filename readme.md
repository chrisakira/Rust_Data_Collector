# 💵 Dollar BRL — USD to BRL Exchange Rate Monitor

A Rust application that fetches the **USD → BRL exchange rate** from the
[AwesomeAPI](https://docs.awesomeapi.com.br/api-de-moedas) and stores it in
**InfluxDB 3.x** for time-series analysis.

---

## 📁 Project Structure

```
dollar-brl/
├── docker-compose.yml
├── .env
├── .gitignore
├── Cargo.toml
└── src/
    ├── main.rs
    ├── api.rs
    └── influx.rs
```

---

## 🧰 Prerequisites

- [Rust](https://rustup.rs/) (edition 2021 or later)
- [Docker](https://www.docker.com/) + [Docker Compose](https://docs.docker.com/compose/)
- Internet access (to reach the AwesomeAPI)

---

## ⚙️ Dependencies

| Crate | Version | Purpose |
|---|---|---|
| `reqwest` | 0.12 | HTTP client — calls AwesomeAPI **and** InfluxDB 3 write API |
| `tokio` | 1 | Async runtime |
| `serde` + `serde_json` | 1 | Deserialize JSON API responses |

> ℹ️ No InfluxDB-specific crate is needed. InfluxDB 3.x exposes a plain
> HTTP endpoint (`/api/v3/write_lp`) that accepts **Line Protocol** text,
> so we write to it directly with `reqwest`.

---

## 🚀 Getting Started

### 1. Clone the repository

```bash
git clone https://github.com/your-username/dollar-brl.git
cd dollar-brl
```

---

### 2. Start InfluxDB 3.x with Docker

```bash
docker compose up -d
```

Wait for the container to be healthy:

```bash
docker compose ps
# influxdb3   running (healthy)
```

---

### 3. Create the Admin Token

> ⚠️ InfluxDB 3.x starts with **no tokens** on a fresh install.
> You must create an admin token **before** doing anything else.

```bash
sudo docker exec influxdb3 influxdb3 create token --admin
```

📋 **Copy the token output immediately — it will only be shown once!**

It will look like this:
```
Token: apiv3_your_long_secret_token_here
```

---

### 4. Create the Database

```bash
sudo docker exec influxdb3 influxdb3 create database exchange-rates \
  --token apiv3_your_long_secret_token_here
```

Verify it was created:

```bash
sudo docker exec influxdb3 influxdb3 show databases \
  --token apiv3_your_long_secret_token_here
```

Expected output:

```
+----------------+
| iox::database  |
+----------------+
| exchange-rates |
+----------------+
```

---

### 5. Configure Environment Variables

Create a `.env` file at the project root:

```bash
# InfluxDB 3.x connection settings
INFLUX_HOST=http://localhost:8181
INFLUX_TOKEN=apiv3_your_long_secret_token_here
INFLUX_DATABASE=exchange-rates
```

Then export them in your shell:

```bash
export INFLUX_HOST="http://localhost:8181"
export INFLUX_TOKEN="apiv3_your_long_secret_token_here"
export INFLUX_DATABASE="exchange-rates"
```

> 🔒 **Never commit your `.env` file!** It is already listed in `.gitignore`.

---

### 6. Run the Application

```bash
cargo run
```

Expected output:

```
📡 Fetching USD → BRL exchange rate...
💵 USD → BRL : R$ 5.8123 (ask: R$ 5.8133)
📈 High: R$ 5.8500 | 📉 Low: R$ 5.7900
📊 Change: -0.02 (-0.03%)
🕒 Updated at: 2026-03-02 14:32:00

📦 Inserting data into InfluxDB 3...
✅ Data written to InfluxDB 3 database 'exchange-rates'
```

---

## 🔍 Querying Your Data

### Via Docker CLI

> ⚠️ Use `--database` (not `--dbname`) and the table name is `exchange_rate` (singular).

Single line (safest, avoids shell line-break issues):

```bash
sudo docker exec -it influxdb3 influxdb3 query --database exchange-rates --token apiv3_your_long_secret_token_here "SELECT time, bid, ask, high, low FROM exchange_rate ORDER BY time DESC LIMIT 10"
```

Or multi-line with backslash continuation — make sure every line except the last ends with ` \`:

```bash
sudo docker exec -it influxdb3 influxdb3 query \
  --database exchange-rates \
  --token apiv3_your_long_secret_token_here \
  "SELECT time, bid, ask, high, low FROM exchange_rate ORDER BY time DESC LIMIT 10"
```

### Via SQL (InfluxDB 3.x supports native SQL)

```sql
SELECT time, bid, ask, high, low, pct_change
FROM exchange_rate
WHERE pair = 'USD-BRL'
  AND time >= now() - INTERVAL '1 hour'
ORDER BY time DESC;
```

---

## 📊 Data Schema

| Type | Name | Example Value |
|---|---|---|
| **Measurement** | `exchange_rate` | — |
| **Tag** | `from` | `USD` |
| **Tag** | `to` | `BRL` |
| **Tag** | `pair` | `USD-BRL` |
| **Field** | `bid` | `5.8123` |
| **Field** | `ask` | `5.8133` |
| **Field** | `high` | `5.8500` |
| **Field** | `low` | `5.7900` |
| **Field** | `var_bid` | `-0.02` |
| **Field** | `pct_change` | `-0.03` |
| **Field** | `name` | `Dólar Americano/Real Brasileiro` |
| **Field** | `timestamp_api` | `1709389920` |

---

## 🛠️ Useful Docker Commands

| Command | Purpose |
|---|---|
| `docker compose up -d` | Start InfluxDB 3 in background |
| `docker compose down` | Stop and remove containers |
| `docker compose down -v` | Stop and **delete all data** ⚠️ |
| `docker compose logs -f influxdb3` | Follow live logs |
| `docker compose ps` | Check container health |
| `docker exec -it influxdb3 influxdb3 --help` | Access InfluxDB 3 CLI |

---

## 📐 Architecture

```
┌─────────────────────────────────────────┐
│              Your Machine               │
│                                         │
│  ┌─────────────┐     ┌───────────────┐  │
│  │  Rust App   │────▶│  InfluxDB 3   │  │
│  │  cargo run  │8181 │  (Docker)     │  │
│  └─────────────┘     │               │  │
│        │             │  Apache Arrow │  │
│        │ HTTP GET    │  + Parquet    │  │
│        ▼             │  Storage      │  │
│  ┌─────────────┐     └───────────────┘  │
│  │ AwesomeAPI  │                        │
│  │  USD/BRL    │                        │
│  └─────────────┘                        │
└─────────────────────────────────────────┘
```

---

## ❓ Troubleshooting

### `401 Unauthorized` when creating database

InfluxDB 3.x requires an admin token for all operations.
Create the admin token first, then retry with `--token`:

```bash
sudo docker exec influxdb3 influxdb3 create token --admin
sudo docker exec influxdb3 influxdb3 create database exchange-rates \
  --token apiv3_your_token
```

---

### `--dbname: command not found` or `required arguments not provided`

The correct flag is `--database`, not `--dbname`. Always use:

```bash
sudo docker exec -it influxdb3 influxdb3 query \
  --database exchange-rates \
  --token apiv3_your_token \
  "SELECT ... FROM exchange_rate ..."
```

> ⚠️ The table name is `exchange_rate` (singular), **not** `exchange_rates`.

---

### `no matching package named influxdb3-client`

`influxdb3-client` is not published on crates.io as a standalone package.
This project uses plain `reqwest` HTTP calls to the `/api/v3/write_lp`
endpoint instead — no InfluxDB-specific crate is required.

---

### `INFLUX_TOKEN env var must be set` on `cargo run`

Export the environment variables in your current shell session:

```bash
export INFLUX_TOKEN="apiv3_your_long_secret_token_here"
```

Or run the app inline:

```bash
INFLUX_HOST=http://localhost:8181 \
INFLUX_TOKEN=apiv3_your_token \
INFLUX_DATABASE=exchange-rates \
cargo run
```

---

### Container not starting

```bash
docker compose logs -f influxdb3
```

---

## 📄 License

MIT