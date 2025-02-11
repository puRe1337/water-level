use dotenv::dotenv;
use base64::prelude::*;

async fn send_notification(message: String) -> Result<bool, reqwest::Error> {
    let ntfy_url = std::env::var("NTFY_URL").expect("NTFY_URL must be set.");
    let ntfy_user = std::env::var("NTFY_USER").expect("NTFY_USER must be set.");
    let ntfy_password = std::env::var("NTFY_PASSWORD").expect("NTFY_PASSWORD must be set.");

    println!("NTFY_URL: {}", ntfy_url);
    
    let client = reqwest::Client::new();
    let response = client
        .post(ntfy_url)
        .header("Authorization", format!("Basic {}", BASE64_STANDARD.encode(format!("{}:{}", ntfy_user, ntfy_password).as_bytes())))
        .header("Content-Type", "text/plain")
        .body(message)
        .send()
        .await?;
    
    println!("Response status: {}", response.status());

    Ok(true)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let msg = "Test Msg".to_string();
    send_notification(msg).await?;

    Ok(())
}
