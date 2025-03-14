use dotenv::dotenv;
use base64::prelude::*;
use std::error::Error;
use std::time::Duration;

#[cfg(target_os = "linux")]
use rppal::i2c::I2c;

use bitflags::bitflags;

use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use tower_http::services::ServeDir;
use axum::{
    extract::State, response::sse::{Event, Sse}, routing::{get, get_service}, Router,
};

mod web;
use web::*;


// ADS1115 I2C address when ADDR pin pulled to ground
const ADDR_ADS115:     u16 = 0x48; // Address of first ADS1115 chip  - i2cdetect -y 1 should print 48

// ADS1115 register addresses.
const REG_CONFIGURATION: u8 = 0x01;
const REG_CONVERSION:    u8 = 0x00;

async fn send_notification(message: String) -> Result<bool, reqwest::Error> {
    let ntfy_url = std::env::var("NTFY_URL").expect("NTFY_URL must be set.");
    let ntfy_user = std::env::var("NTFY_USER").expect("NTFY_USER must be set.");
    let ntfy_password = std::env::var("NTFY_PASSWORD").expect("NTFY_PASSWORD must be set.");

    // println!("NTFY_URL: {}", ntfy_url);
    
    let client = reqwest::Client::new();
    let response = client
        .post(ntfy_url)
        .header("Authorization", format!("Basic {}", BASE64_STANDARD.encode(format!("{}:{}", ntfy_user, ntfy_password).as_bytes())))
        .header("Content-Type", "text/plain")
        .body(message)
        .send()
        .await?;
    
    // println!("Response status: {}", response.status());

    Ok(true)
}


// https://cdn-shop.adafruit.com/datasheets/ads1115.pdf
// Konfiguration für Single-Shot Messung auf A0
// 0x83, 0xE3: 
// - Bit 15: Start single-shot conversion
// - Bits 14-12: Input multiplexer to A0
// - Bits 11-9: Gain = 1 (+/- 4.096V)
// - Bit 8: Device operating mode  mode (1) or continuous mode (0)
// - Bits 7-5: Data Rate 128 SPS default
// - Bit 4: COMP_MODE: Comparator mode
// - Bit 3: COMP_POL: Comparator polarity
// - Bit 2: Non-latching comparator
// - Bits 1-0: COMP_QUE: Comparator queue and disable
// let first8bit = 0b10000011; // 0x83
// let second8bit = 0b11100011; // 0xE3
bitflags! {
    #[derive(Debug, Clone, Copy)]
    struct AdsConfig: u16 {
        // Bit 15 - Operational Status / Single-shot conversion start
        const START_CONV     = 0b1000_0000_0000_0000;

        // Bits 14:12 - Input multiplexer
        const MUX_AIN0_AIN1 = 0b0000_0000_0000_0000; // 000: Default
        const MUX_AIN0_AIN3 = 0b0001_0000_0000_0000; // 001
        const MUX_AIN1_AIN3 = 0b0010_0000_0000_0000; // 010
        const MUX_AIN2_AIN3 = 0b0011_0000_0000_0000; // 011
        const MUX_AIN0_GND  = 0b0100_0000_0000_0000; // 100
        const MUX_AIN1_GND  = 0b0101_0000_0000_0000; // 101
        const MUX_AIN2_GND  = 0b0110_0000_0000_0000; // 110
        const MUX_AIN3_GND  = 0b0111_0000_0000_0000; // 111

        // Bits 11:9 - Programmable gain amplifier
        const GAIN_6_144V   = 0b0000_0000_0000_0000; // 000: ±6.144V
        const GAIN_4_096V   = 0b0000_0010_0000_0000; // 001: ±4.096V
        const GAIN_2_048V   = 0b0000_0100_0000_0000; // 010: ±2.048V
        const GAIN_1_024V   = 0b0000_0110_0000_0000; // 011: ±1.024V
        const GAIN_0_512V   = 0b0000_1000_0000_0000; // 100: ±0.512V
        const GAIN_0_256V_1 = 0b0000_1010_0000_0000; // 101: ±0.256V
        const GAIN_0_256V_2 = 0b0000_1100_0000_0000; // 110: ±0.256V
        const GAIN_0_256V_3 = 0b0000_1110_0000_0000; // 111: ±0.256V

        // Device operating mode (Bit 8)
        const MODE_CONTINUOUS = 0b0000_0000_0000_0000; // 0: Continuous conversion mode
        const MODE_SINGLE    = 0b0000_0001_0000_0000; // 1: Power-down single-shot mode (default)

        // Data rate (Bits 7:5)
        const DR_8SPS       = 0b0000_0000_0000_0000; // 000: 8 SPS
        const DR_16SPS      = 0b0000_0000_0010_0000; // 001: 16 SPS
        const DR_32SPS      = 0b0000_0000_0100_0000; // 010: 32 SPS
        const DR_64SPS      = 0b0000_0000_0110_0000; // 011: 64 SPS
        const DR_128SPS     = 0b0000_0000_1000_0000; // 100: 128 SPS (default)
        const DR_250SPS     = 0b0000_0000_1010_0000; // 101: 250 SPS
        const DR_475SPS     = 0b0000_0000_1100_0000; // 110: 475 SPS
        const DR_860SPS     = 0b0000_0000_1110_0000; // 111: 860 SPS

        // Comparator mode (Bit 4)
        const COMP_MODE_TRADITIONAL = 0b0000_0000_0000_0000; // 0: Traditional comparator (default)
        const COMP_MODE_WINDOW     = 0b0000_0000_0001_0000; // 1: Window comparator

        // Comparator polarity (Bit 3)
        const COMP_POL_ACTIVE_LOW  = 0b0000_0000_0000_0000; // 0: Active low (default)
        const COMP_POL_ACTIVE_HIGH = 0b0000_0000_0000_1000; // 1: Active high

        // Latching comparator (Bit 2)
        const COMP_LAT_NONLATCH = 0b0000_0000_0000_0000; // 0: Non-latching (default)
        const COMP_LAT_LATCH    = 0b0000_0000_0000_0100; // 1: Latching

        // Comparator queue and disable (Bits 1:0)
        const COMP_QUE_ASSERT_1 = 0b0000_0000_0000_0000; // 00: Assert after one conversion
        const COMP_QUE_ASSERT_2 = 0b0000_0000_0000_0001; // 01: Assert after two conversions
        const COMP_QUE_ASSERT_4 = 0b0000_0000_0000_0010; // 10: Assert after four conversions
        const COMP_QUE_DISABLE  = 0b0000_0000_0000_0011; // 11: Disable comparator (default)
    }
}

#[cfg(target_os = "linux")]
async fn get_adc0_value() -> Result<(i16, f32), Box<dyn Error + Send + Sync>> {
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