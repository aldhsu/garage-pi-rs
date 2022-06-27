use anyhow::{anyhow, Context, Result};
use axum::{
    body::Body,
    response::Response,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::net::SocketAddr;
use tracing::info;
use tracing_subscriber;

use rppal::gpio::Gpio;

use tokio::time::Duration;

use local_ipaddress;

struct GrgError {
    error: anyhow::Error,
}

impl IntoResponse for GrgError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("{:?}", self.error)).into_response()
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

    let app = Router::new().route("/", get(root));

    let address: SocketAddr = format!(
        "{}:{}",
        local_ipaddress::get().ok_or_else(|| anyhow!("couldn't get local ip"))?,
        8080
    )
    .parse()?;

    tracing::debug!("listening on {}", address);
    axum::Server::bind(&address)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn root() -> Result<String, GrgError> {
    switch_garage().await.map_err(|e| e.into())
}

async fn switch_garage() -> Result<String> {
    let gpio = Gpio::new().context("couldn't get RPi GPIO")?;
    let mut pin = gpio.get(2).context("couldn't get access to pin")?.into_output();
    pin.set_low();
    tokio::time::sleep(Duration::from_millis(200)).await;
    Ok("we did it".into())
}
