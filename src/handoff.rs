use std::time::Duration;

const STOP_TIMEOUT: Duration = Duration::from_secs(10);

// Asks the revision serving traffic to hand over the database.
pub async fn stop_serving_revision() {
    let (Ok(endpoint), Ok(revision)) = (std::env::var("ENDPOINT"), std::env::var("K_REVISION"))
    else {
        return; // Not running on Cloud Run (e.g. `make run`).
    };
    let result = reqwest::Client::new()
        .post(format!("https://{endpoint}/stop?revision={revision}"))
        .header(reqwest::header::CONTENT_LENGTH, 0)
        .timeout(STOP_TIMEOUT)
        .send()
        .await
        .and_then(|response| response.error_for_status());
    match result {
        Ok(_) => println!("stopped the serving revision"),
        Err(e) => println!("stopped nothing: {e}"),
    }
}
