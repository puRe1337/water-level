use dotenv::dotenv;
use base64::prelude::*;
use std::error::Error;
use std::time::Duration;

#[cfg(target_os = "linux")]
mod ads_constants;
#[cfg(target_os = "linux")]
mod ads_config;

#[cfg(target_os = "linux")]
use {
    std::thread,
    rppal::i2c::I2c,
    ads_constants::*
};

use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use tower_http::services::ServeDir;
use axum::{
    extract::State, response::sse::{Event, Sse}, routing::{get, get_service}, Router,
};

mod web;
use web::*;

async fn send_notification(message: String) -> Result<bool, reqwest::Error> {
    let ntfy_url = std::env::var("NTFY_URL").expect("NTFY_URL must be set.");
    let ntfy_user = std::env::var("NTFY_USER").expect("NTFY_USER must be set.");
    let ntfy_password = std::env::var("NTFY_PASSWORD").expect("NTFY_PASSWORD must be set.");

    // println!("NTFY_URL: {}", ntfy_url);
    
    let client = reqwest::Client::new();
    let _response = client
        .post(ntfy_url)
        .header("Authorization", format!("Basic {}", BASE64_STANDARD.encode(format!("{}:{}", ntfy_user, ntfy_password).as_bytes())))
        .header("Content-Type", "text/plain")
        .body(message)
        .send()
        .await?;
    
    // println!("Response status: {}", response.status());

    Ok(true)
}


#[cfg(target_os = "linux")]
async fn get_adc0_value() -> Result<(i16, f32), Box<dyn Error + Send + Sync>> {
    use ads_config::AdsConfig;

    let mut i2c = I2c::new()?;
    i2c.set_slave_address(ADDR_ADS115)?;


    let config = AdsConfig::START_CONV           // 1000 .... .... ....
                    | AdsConfig::MUX_AIN0_AIN1              // ...000 ... .... ....
                    | AdsConfig::GAIN_4_096V                // .... .001 .... ....
                    | AdsConfig::MODE_SINGLE                // .... .... 1... ....
                    | AdsConfig::DR_128SPS                  // .... .... .111 ....
                    | AdsConfig::COMP_MODE_TRADITIONAL      // .... .... .... 0...
                    | AdsConfig::COMP_POL_ACTIVE_LOW        // .... .... .... .0..
                    | AdsConfig::COMP_LAT_NONLATCH          // .... .... .... ..0.
                    | AdsConfig::COMP_QUE_DISABLE;          // .... .... .... ...11
    

    let bytes = config.bits().to_be_bytes();
    // println!("Config: {:02X} {:02X}", bytes[0], bytes[1]);
    i2c.block_write(REG_CONFIGURATION, &bytes)?;

    // i2c.block_write(REG_CONFIGURATION, &[first8bit, second8bit])?;
    
    // Warte auf Konversion (max. 8ms bei 128 SPS)
    thread::sleep(Duration::from_millis(8));
    
    // Lese Konvertierungsergebnis
    let mut buffer = [0u8; 2];
    i2c.block_read(REG_CONVERSION, &mut buffer)?;
    
    // Konvertiere zu 16-bit Integer
    let raw_value = i16::from_be_bytes(buffer);
    
    println!("Raw ADC Value: {}", raw_value);

    if raw_value > 10000 {
        let msg = format!("Wert höher als 10000: {}", raw_value);
        send_notification(msg).await?;
    }
    
    // Berechne Spannung (LSB = 0.125mV bei ±4.096V)
    let voltage = (raw_value as f32) * 0.000125;
    println!("Voltage: {:.3}V", voltage);
    
    Ok((raw_value, voltage))
}

#[cfg(target_os = "macos")]
async fn get_adc0_value() -> Result<(i16, f32), Box<dyn Error + Send + Sync>> {
    // Dummy implementation for macOS
    
    // random between 0 and 20000
    let raw_value = rand::random::<i16>() % 20000;
    let voltage = (raw_value as f32) * 0.000125;

    Ok((raw_value, voltage))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    // Default threshold
    let threshold = RwLock::new(10000);

    // Broadcast channel für Messwerte
    let (tx, _) = broadcast::channel::<AdcValue>(100);
    
    // Create app state
    let app_state = Arc::new(AppState {
        tx,
        threshold,
    });

    // Starte ADC Messungen in separatem Task
    let state_clone = app_state.clone();
    tokio::spawn(async move {
        loop {
            if let Ok((raw_value, voltage)) = get_adc0_value().await {
                // Get current threshold
                let threshold = *state_clone.threshold.read().unwrap();
                
                // Send notification if exceeds threshold
                if i32::from(raw_value) > threshold {
                    let msg = format!("Wert höher als {}: {}", threshold, raw_value);
                    let _ = send_notification(msg).await;
                }
                
                let value = AdcValue {
                    raw_value,
                    voltage,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    threshold, // Include current threshold in data
                };
                let _ = state_clone.tx.send(value);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    // Setup web server
    let app = Router::new()
        .route("/events", get(sse_handler))
        .route("/threshold", get(get_threshold).post(set_threshold))
        .nest_service("/", get_service(ServeDir::new("static")))
        .with_state(app_state);

    println!("Server running on http://localhost:3000");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

async fn sse_handler(
    State(state): State<Arc<AppState>>
) -> Sse<impl futures::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let mut rx = state.tx.subscribe();
    let stream = async_stream::stream! {
        while let Ok(value) = rx.recv().await {
            yield Ok(Event::default().json_data(value).unwrap());
        }
    };
    Sse::new(stream)
}