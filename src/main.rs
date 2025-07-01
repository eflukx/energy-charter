use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use chrono_tz::Tz;
use futures::stream::StreamExt;
use moka::future::Cache;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use tokio::fs;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
enum EnergyType {
    Electricity,
    Gas,
}

impl fmt::Display for EnergyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnergyType::Electricity => write!(f, "electricity"),
            EnergyType::Gas => write!(f, "gas"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "UPPERCASE")]
enum Interval {
    Hour,
    Day,
    Month,
}

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Interval::Hour => write!(f, "HOUR"),
            Interval::Day => write!(f, "DAY"),
            Interval::Month => write!(f, "MONTH"),
        }
    }
}

#[derive(Deserialize, Debug)]
struct EnergyParams {
    #[serde(rename = "type")]
    energy_type: EnergyType,
    #[serde(rename = "startDate")]
    start_date: DateTime<Utc>,
    #[serde(rename = "endDate")]
    end_date: DateTime<Utc>,
    interval: Option<Interval>,
}

impl EnergyParams {
    /// Creëert een unieke string voor gebruik als cache key met seconden-precisie.
    fn get_cache_key(&self) -> String {
        let interval = self.interval.unwrap_or(Interval::Hour);
        format!(
            "{}:{}:{}:{}",
            self.energy_type,
            self.start_date.to_rfc3339_opts(SecondsFormat::Secs, true),
            self.end_date.to_rfc3339_opts(SecondsFormat::Secs, true),
            interval
        )
    }

    /// Bouwt de volledige URL voor de externe ANWB API met seconden-precisie.
    fn build_api_url(&self) -> String {
        let interval = self.interval.unwrap_or(Interval::Hour);
        format!(
            "https://api.anwb.nl/energy/energy-services/v1/tarieven/{}?startDate={}&endDate={}&interval={}",
            self.energy_type,
            self.start_date.to_rfc3339_opts(SecondsFormat::Secs, true),
            self.end_date.to_rfc3339_opts(SecondsFormat::Secs, true),
            interval
        )
    }
}

impl fmt::Display for EnergyParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "type={}, start={}, end={}, interval={}",
            self.energy_type,
            self.start_date,
            self.end_date,
            self.interval.unwrap_or(Interval::Hour)
        )
    }
}

type EnergyData = serde_json::Value;

#[derive(Clone)]
struct AppState {
    http_client: Client,
    cache: Cache<String, EnergyData>,
    static_file_path: String,
}

#[derive(Debug)]
struct AppConfig {
    listen_addr: String,
    static_file_path: String,
    cache_warmup_concurrency: usize,
    cache_capacity: u64,
    cache_warmup_days: i64,
    timezone: Tz,
}

fn load_config() -> AppConfig {
    let listen_addr = std::env::var("LISTEN_ADDR").unwrap_or("127.0.0.1:3000".to_string());

    let static_file_path = std::env::var("STATIC_FILE_PATH").unwrap_or("index.html".to_string());

    let cache_warmup_concurrency: usize = std::env::var("CACHE_WARMUP_CONCURRENCY")
        .unwrap_or("10".to_string())
        .parse()
        .expect("CACHE_WARMUP_CONCURRENCY moet een geldig getal zijn.");

    let cache_capacity: u64 = std::env::var("CACHE_CAPACITY")
        .unwrap_or("10000".to_string())
        .parse()
        .expect("CACHE_CAPACITY moet een geldig getal zijn.");

    let cache_warmup_days: i64 = std::env::var("CACHE_WARMUP_DAYS")
        .unwrap_or("7".to_string())
        .parse()
        .expect("CACHE_WARMUP_DAYS moet een geldig getal zijn.");

    let timezone_str = std::env::var("TIMEZONE").unwrap_or("Europe/Amsterdam".to_string());
    let timezone = Tz::from_str(&timezone_str).expect("Ongeldige TIMEZONE environment variabele.");

    AppConfig {
        listen_addr,
        static_file_path,
        cache_warmup_concurrency,
        cache_capacity,
        cache_warmup_days,
        timezone,
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "energy_proxy=info,tower_http=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = load_config();
    tracing::info!("Configuratie geladen: {:?}", config);

    let cache = Cache::builder().max_capacity(config.cache_capacity).build();

    let app_state = AppState {
        http_client: Client::new(),
        cache,
        static_file_path: config.static_file_path,
    };

    tokio::spawn(warm_up_cache(
        app_state.clone(),
        config.cache_warmup_concurrency,
        config.cache_warmup_days,
        config.timezone,
    ));

    let app = Router::new()
        .route("/", get(serve_frontend))
        .route("/api/energy", get(get_energy_data))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(&config.listen_addr)
        .await
        .unwrap();
    tracing::info!("Server luistert op {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn serve_frontend(State(state): State<AppState>) -> impl IntoResponse {
    tracing::info!(
        "Frontend request ontvangen, lezen van '{}'",
        &state.static_file_path
    );
    match fs::read_to_string(&state.static_file_path).await {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("Kon HTML-bestand niet lezen: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Kon de interface niet laden.",
            )
                .into_response()
        }
    }
}

async fn get_energy_data(
    State(state): State<AppState>,
    Query(params): Query<EnergyParams>,
) -> impl IntoResponse {
    tracing::info!("Energie-API request: {}", params);

    let cache_key = params.get_cache_key();

    if let Some(cached_data) = state.cache.get(&cache_key).await {
        tracing::info!("Cache HIT voor key: {}", cache_key);
        return Ok(Json(cached_data));
    }

    tracing::info!("Cache MISS voor key: {}", cache_key);

    match fetch_and_cache(&state, &params).await {
        Ok(data) => Ok(Json(data)),
        Err(e) => Err(e),
    }
}

async fn fetch_and_cache(
    state: &AppState,
    params: &EnergyParams,
) -> Result<EnergyData, (StatusCode, String)> {
    let anwb_api_url = params.build_api_url();
    let cache_key = params.get_cache_key();

    let response = state
        .http_client
        .get(&anwb_api_url)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Fout bij aanroepen van externe API: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Kon de externe API niet bereiken.".to_string(),
            )
        })?;

    if response.status().is_success() {
        let data = response.json::<EnergyData>().await.map_err(|e| {
            tracing::error!("Fout bij parsen van JSON: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Fout bij het verwerken van de data.".to_string(),
            )
        })?;

        state.cache.insert(cache_key.clone(), data.clone()).await;
        tracing::info!("Data opgeslagen in cache voor key: {}", cache_key);
        Ok(data)
    } else {
        tracing::error!("Externe API gaf een foutstatus: {}", response.status());
        Err((
            StatusCode::BAD_GATEWAY,
            "De externe API gaf een fout terug.".to_string(),
        ))
    }
}

async fn warm_up_cache(state: AppState, concurrency: usize, warmup_days: i64, timezone: Tz) {
    tracing::info!(
        "Starten met het opwarmen van de cache voor {} dagen (concurrency: {}, timezone: {})...",
        warmup_days,
        concurrency,
        timezone
    );

    // start tomorrow, to include day ahead prices when available
    let day_ahead = Utc::now().with_timezone(&timezone).date_naive() + Duration::days(1);

    let mut tasks = Vec::new();
    for i in 0..warmup_days {
        let target_day = day_ahead - Duration::days(i);
        tasks.push((target_day, EnergyType::Electricity));
        tasks.push((target_day, EnergyType::Gas));
    }

    futures::stream::iter(tasks)
        .for_each_concurrent(concurrency, |(target_day, energy_type)| {
            let state = state.clone();
            async move {
                // Bepaal de start en eindtijd van de dag in de lokale timezone, en converteer dan naar UTC.
                let start_date = target_day
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_local_timezone(timezone)
                    .unwrap()
                    .to_utc();

                let params = EnergyParams {
                    energy_type,
                    start_date,
                    end_date: start_date + Duration::days(1),
                    interval: Some(Interval::Hour),
                };

                tracing::debug!("Cache warmer: ophalen van {}", params);
                let _ = fetch_and_cache(&state, &params).await;
            }
        })
        .await;

    tracing::info!("Cache opwarmen voltooid.");
}
