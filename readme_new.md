# 💵 Dollar BRL — USD to BRL Exchange Rate Monitor

A Rust application that fetches the **USD → BRL exchange rate** from the
[AwesomeAPI](https://docs.awesomeapi.com.br/api-de-moedas) and stores it in
**InfluxDB 3.x** for time-series analysis.

---

## 📁 Project Structure

```
dollar-brl/
├── docker-compose.yml
├── Dockerfile
├── .env
├── .gitignore
├── scripts/
│   ├── build.sh
│   └── run.sh
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
| `reqwest` | 0.12 | HTTP client — calls AwesomeAPI and InfluxDB 3 write API |
| `tokio` | 1 | Async runtime |
| `serde` + `serde_json` | 1 | Deserialize JSON API responses |

---

## 🚀 Getting Started

### 1. Clone the repository

```bash
git clone https://github.com/your-username/dollar-brl.git
cd dollar-brl
```

---

### 2. Configure Environment Variables

Create a `.env` file at the project root:

```bash
INFLUX_HOST=http://influxdb3:8181
INFLUX_TOKEN=apiv3_your_long_secret_token_here
INFLUX_DATABASE=exchange-rates
```

> 🔒 **Never commit your `.env` file!** It is already listed in `.gitignore`.

---

### 3. Build the Docker Image

```bash
chmod +x scripts/build.sh
./scripts/build.sh
```

---

### 4. Start InfluxDB 3.x

```bash
docker compose up influxdb3 -d
```

---

### 5. Create the Admin Token

```bash
sudo docker exec influxdb3 influxdb3 create token --admin
```

📋 **Copy the token output immediately — it will only be shown once!**

Update the `INFLUX_TOKEN` value in your `.env` file with the generated token.

---

### 6. Create the Database

```bash
sudo docker exec influxdb3 influxdb3 create database exchange-rates \
  --token apiv3_your_long_secret_token_here
```

---

### 7. Run the Full Stack

```bash
docker compose up -d
```

---

### Run Locally (without Docker Compose)

To run only the app container against a running InfluxDB instance:

```bash
chmod +x scripts/run.sh
./scripts/run.sh
```

---

## 🔍 Querying Your Data

```bash
sudo docker exec -it influxdb3 influxdb3 query \
  --database exchange-rates \
  --token apiv3_your_long_secret_token_here \
  "SELECT time, bid, ask, high, low FROM exchange_rate ORDER BY time DESC LIMIT 10"
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

## 🛠️ Docker Commands

| Command | Purpose |
|---|---|
| `docker compose up -d` | Start full stack in background |
| `docker compose up influxdb3 -d` | Start only InfluxDB |
| `docker compose down` | Stop and remove containers |
| `docker compose down -v` | Stop and delete all data |
| `docker compose logs -f` | Follow all logs |
| `docker compose logs -f dollar-brl` | Follow app logs only |
| `docker compose ps` | Check container health |

---

## 📐 Architecture

```
┌─────────────────────────────────────────┐
│           Docker Compose Stack          │
│                                         │
│  ┌─────────────┐     ┌───────────────┐  │
│  │  dollar-brl │────▶│  influxdb3    │  │
│  │  (Rust app) │8181 │               │  │
│  └─────────────┘     │  Apache Arrow │  │
│        │             │  + Parquet    │  │
│        │ HTTP GET    │  Storage      │  │
│        ▼             └───────────────┘  │
│  ┌─────────────┐                        │
│  │ AwesomeAPI  │                        │
│  │  USD/BRL    │                        │
│  └─────────────┘                        │
└─────────────────────────────────────────┘
```

---

## 📄 License

MIT