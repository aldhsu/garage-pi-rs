use anyhow::{anyhow, Context, Result};
use axum::{
    body::Body,
    extract::{Extension, Path},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use std::net::SocketAddr;
use tracing::info;
use tracing_subscriber;

#[cfg(feature = "rpi")]
use rppal::gpio::Gpio;
use uuid::Uuid;

use tokio::time::Duration;

use local_ipaddress;

use sqlx::sqlite::SqlitePool;

struct GrgError {
    error: anyhow::Error,
}

impl IntoResponse for GrgError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("{:?}", self.error),
        )
            .into_response()
    }
}

impl From<anyhow::Error> for GrgError {
    fn from(error: anyhow::Error) -> Self {
        Self { error }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // initialize tracing
    tracing_subscriber::fmt::init();
    let pool = SqlitePool::connect(&std::env::var("DATABASE_URL")?).await?;

    let app = Router::new()
        .route("/toggle/:key", post(toggle))
        .route("/user", post(create_user))
        .layer(Extension(pool));

    // let address: SocketAddr = format!(
    //     "{}:{}",
    //     local_ipaddress::get().ok_or_else(|| anyhow!("couldn't get local ip"))?,
    //     8080
    // )
    // .parse()?;
    let address: SocketAddr = format!(
        "{}:{}",
        // local_ipaddress::get().ok_or_else(|| anyhow!("couldn't get local ip"))?,
        "127.0.0.1",
        8080
    )
    .parse()?;

    tracing::debug!("listening on {}", address);
    axum::Server::bind(&address)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn toggle(
    Extension(db): Extension<SqlitePool>,
    Path(key): Path<String>,
) -> Result<StatusCode, GrgError> {
    let mut conn = db.acquire().await.context("couldn't get connection")?;
    let result = sqlx::query!(
        r#"
        SELECT * FROM users
        "#
    )
    .fetch_all(&mut conn)
    .await
    .context("couldn't run query")?;

    switch_garage()
        .await
        .map_err(|e| e.into())
        .map(|_| StatusCode::OK)
}

#[cfg(feature = "rpi")]
async fn switch_garage() -> Result<()> {
    let gpio = Gpio::new().context("couldn't get RPi GPIO")?;
    let mut pin = gpio
        .get(2)
        .context("couldn't get access to pin")?
        .into_output();
    pin.set_low();
    tokio::time::sleep(Duration::from_millis(200)).await;
    Ok(())
}

#[cfg(not(feature = "rpi"))]
async fn switch_garage() -> Result<()> {
    Ok(())
}

#[derive(Deserialize)]
struct User {
    name: String,
}

async fn create_user(
    Extension(db): Extension<SqlitePool>,
    Json(request): Json<String>
    ) -> Result<Html<String>, GrgError> {
    let uuid = Uuid::new_v4().to_string();
    let mut conn = db.acquire().await.context("couldn't get connection")?;
    let result = sqlx::query!(
        r#"
        INSERT INTO users (name, key) VALUES (?1, ?2)
        "#,
        request,
        uuid
    )
    .execute(&mut conn)
    .await
    .context("couldn't run query")?;
    Ok(Html(format!("<h1>User code</h1><p>{}</p>", uuid)))
}
