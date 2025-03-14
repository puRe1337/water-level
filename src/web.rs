use std::sync::{Arc, RwLock};
use axum::{
    extract::State, response::sse::{Event, Sse}, routing::{get, get_service}, Router,
    Json, extract::Query
};
use tokio::sync::broadcast;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize)]
pub struct AdcValue {
    pub raw_value: i16,
    pub voltage: f32,
    pub timestamp: u64,
    pub threshold: i32,
}

pub struct AppState {
    pub tx: broadcast::Sender<AdcValue>,
    pub threshold: RwLock<i32>,
}

#[derive(Deserialize)]
pub struct ThresholdRequest {
    value: i32,
}

#[derive(Serialize)]
pub struct ThresholdResponse {
    threshold: i32,
}

pub async fn get_threshold(
    State(state): State<Arc<AppState>>
) -> Json<ThresholdResponse> {
    let threshold = *state.threshold.read().unwrap();
    Json(ThresholdResponse { threshold })
}

pub async fn set_threshold(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ThresholdRequest>,
) -> Json<ThresholdResponse> {
    let mut threshold = state.threshold.write().unwrap();
    *threshold = request.value;
    Json(ThresholdResponse { threshold: *threshold })
}