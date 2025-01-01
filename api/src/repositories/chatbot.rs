use crate::config;
use reqwest::blocking::{Client, Response};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};

pub fn ask_bot(user_msg: &str, bot_name: &str, user_address: &str) -> String {
    let agent_url: String = config::get("agent_url");
    let app_key: String = config::get("app_key");

    // Create the full URL
    let url = format!(
        "{}/v2/chat?user_address={}&token_address={}",
        agent_url, user_address, bot_name
    );

    // Prepare headers
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert("x-app-key", HeaderValue::from_str(&app_key).unwrap());

    // Prepare the request body
    let body = serde_json::json!({
        "msg": user_msg
    });

    // Make the HTTP request
    let client = Client::new();
    let response: Result<Response, reqwest::Error> =
        client.post(&url).headers(headers).json(&body).send();

    match response {
        Ok(res) => {
            if res.status().is_success() {
                match res.json::<serde_json::Value>() {
                    Ok(data) => data["response"]
                        .as_str()
                        .unwrap_or("No response from bot")
                        .to_string(),
                    Err(_) => "Failed to parse bot response.".to_string(),
                }
            } else {
                log::error!("Request failed with status: {}", res.status());
                "Request failed.".to_string()
            }
        }
        Err(err) => {
            log::error!("Error sending request: {}", err);
            "Sorry, something went wrong.".to_string()
        }
    }
}
