use webbrowser;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
const SWITCH_TOKEN: &str = "OThmN2U0MmMyZTNhNGY4NmE3NGViNDNmYmI0MWVkMzk6MGEyNDQ5YTItMDAxYS00NTFlLWFmZWMtM2U4MTI5MDFjNGQ3";
const IOS_TOKEN: &str = "MzQ0NmNkNzI2OTRjNGE0NDg1ZDgxYjc3YWRiYjIxNDE6OTIwOWQ0YTVlMjVhNDU3ZmI5YjA3NDg5ZDMxM2I0MWE=";

fn main() {
    let generator = DeviceGenerator::new();
    let (code, url) = generator.get_device_code().unwrap();

    if webbrowser::open(&url).is_ok() {
        println!("Please go to {} and login", url);
        let token = generator.wait_for_device_completion(&code).unwrap();
        let exchange = generator.get_exchange(&token).unwrap();
        let device = generator.generate_device_auth(exchange).unwrap();
        generator.save_device(device).unwrap();

        let mut input = String::new();
        println!("Press enter to exit...");
        std::io::stdin().read_line(&mut input).unwrap();
    } else {
        println!("Failed to open browser");
    }
}

#[derive(Deserialize, Debug)]
struct ExchangeResponse {
    access_token: String,
    account_id: String,
    client_id: String,
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceAuth {
    account_id: String,
    device_id: String,
    secret: String,
}

struct DeviceGenerator {
    client: reqwest::blocking::Client
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

impl DeviceGenerator {
    pub fn new() -> DeviceGenerator {
        DeviceGenerator {
            client: reqwest::blocking::Client::new()
        }
    }

    fn get_token(&self) -> Result<String> {
        let response = self.client.post("https://account-public-service-prod.ol.epicgames.com/account/api/oauth/token")
            .header("Authorization", format!("basic {}", SWITCH_TOKEN))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body("grant_type=client_credentials")
            .send()?;

        let response_json: serde_json::Value = response.json()?;
        Ok(response_json["access_token"].as_str().unwrap().to_string())
    }

    fn get_device_code(&self) -> Result<(String, String)> {
        let token = self.get_token()?;
        let response = self.client.post("https://account-public-service-prod03.ol.epicgames.com/account/api/oauth/deviceAuthorization")
            .header("Authorization", format!("bearer {}", token))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send()?;

        let response: serde_json::Value = response.json()?;
        Ok((response["device_code"].as_str().unwrap().to_string(), response["verification_uri_complete"].as_str().unwrap().to_string()))
    }


    fn wait_for_device_completion(&self, code: &str) -> Result<String> {
        loop {
            let response = self.client.post("https://account-public-service-prod03.ol.epicgames.com/account/api/oauth/token")
                .header("Authorization", format!("basic {}", SWITCH_TOKEN))
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(format!("grant_type=device_code&device_code={}", code))
                .send()?;

            // TODO: Handle epic errors
            match response.status() {
                StatusCode::OK => {
                    let response_json: serde_json::Value = response.json()?;
                    return Ok(response_json["access_token"].as_str().unwrap().to_string());
                },
                StatusCode::BAD_REQUEST => {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                },
                _ => {
                    return Err("Unknown error".into());
                }
            }

        }
    }

    fn get_exchange(&self, token: &str) -> Result<ExchangeResponse> {
        let response = self.client.get("https://account-public-service-prod.ol.epicgames.com/account/api/oauth/exchange")
            .header("Authorization", format!("bearer {}", token))
            .send()?;

        let token: serde_json::Value = response.json()?;
        let token = token["code"].as_str().unwrap().to_string();


        let response = self.client.post("https://account-public-service-prod03.ol.epicgames.com/account/api/oauth/token")
            .header("Authorization", format!("basic {}", IOS_TOKEN))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(format!("grant_type=exchange_code&exchange_code={}", token))
            .send()?;

        let response_json: ExchangeResponse = response.json()?;
        Ok(response_json)
    }

    fn generate_device_auth(&self, resp: ExchangeResponse) -> Result<DeviceAuth> {
        println!("Generating device auth for account: {}", resp.display_name);
        let url = format!("https://account-public-service-prod.ol.epicgames.com/account/api/public/account/{}/deviceAuth", resp.account_id);

        let response = self.client.post(&url)
            .header("Authorization", format!("bearer {}", resp.access_token))
            .header("Content-Type", "application/json")
            .send()?;

        let data: DeviceAuth = response.json()?;
        Ok(data)
    }

    fn save_device(&self, device: DeviceAuth) -> Result<()> {
        let data = serde_json::to_string_pretty(&device)?;
        std::fs::write("device_auth.json", data)?;
        println!("Device auth saved to device_auth.json");
        Ok(())
    }
}
