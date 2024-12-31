use std::time::Duration;
use chrono::Utc;
use log::error;
use reqwest::Client;
use serde_json::Value;
use once_cell::sync::Lazy;


const API_TIMEOUT: Duration = Duration::from_secs(10);
static HTTP_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .timeout(API_TIMEOUT)
        .build()
        .unwrap()
});


#[derive(Clone)]
pub struct ShellySmartPlug {
    pub url: String,
    pub alias: String
}


pub async fn get_metrics(plugs: &Vec<ShellySmartPlug>) -> Result<String, &str> {
    let mut output = "".to_string();
    let mut first = true;

    for plug in plugs {
        let raw_data = call_shelly_plug(&plug.url).await?;
        let fmt_data = convert_to_prometheus(raw_data, &plug.alias);

        // Add leading space on all but the first run
        if !first {
            output += "\n";
        }
        first = false;

        output += fmt_data.as_str();
    }

    Ok(output)
}

fn convert_to_prometheus(http_data: Value, alias: &String) -> String {
    format!(
r"current_datetime{{hostname={alias}}} {datetime}
power_watts{{hostname={alias}}} {power_watts}
voltage{{hostname={alias}}} {voltage}
current_amps{{hostname={alias}}} {current}
temperature_celsius{{hostname={alias}}} {temp_c}
temperature_fahrenheit{{hostname={alias}}} {temp_f}
running_total_power_consumed_watts{{hostname={alias}}} {total_watts}",
        datetime = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        power_watts = http_data["apower"],
        voltage = http_data["voltage"],
        current = http_data["current"],
        temp_c = http_data["temperature"]["tC"],
        temp_f = http_data["temperature"]["tF"],
        total_watts = http_data["aenergy"]["total"]
    )
}

async fn call_shelly_plug(url: &String) -> Result<Value, &str> {
    let output = match HTTP_CLIENT.get(url).send().await {
        Ok(data) => data,
        Err(err) => {
            error!("Failed to build the request at URI {url} - {err}");
            return Err("Failed to connect to API!");
        }
    };

    let http_status_code = output.status().as_u16();
    if http_status_code < 200 || http_status_code > 299 {
        let http_byte_resp = output.bytes().await.unwrap_or_default().to_vec();
        let http_raw_data = String::from_utf8(http_byte_resp)
            .expect("Found invalid UTF-8 data!");

        error!("Expected 200 http status code, got {} with body `{}`", http_status_code, http_raw_data);
        return Err("API request failed with non 200 status code");
    }

    let payload = match output.json::<Value>().await {
        Ok(data) => data,
        Err(err) => {
            error!("Non-JSON response returned - {err}");
            return Err("Invalid response!");
        }
    };

    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Server, ServerGuard};
    use test_context::{test_context, AsyncTestContext};
    use serde_json::json;

    struct TestSetup {
        fake_server: ServerGuard,
        good_shelly_data: String
    }

    impl AsyncTestContext for TestSetup {
        async fn setup() -> TestSetup {
            TestSetup {
                fake_server: Server::new_async().await,
                good_shelly_data: json!({
                    "apower": 1.0,
                    "voltage": 2.0,
                    "current": 3.0,
                    "temperature": {
                        "tC": 20.1,
                        "tF": 68.2
                    },
                    "aenergy": {
                        "total": 45645634.12
                    }
                }).to_string()
            }
        }
    }

    #[test_context(TestSetup)]
    #[tokio::test]
    async fn test_invalid_url(ctx: &mut TestSetup) {
        let test_path = format!("{}/not-home", ctx.fake_server.url());
        let test_path_bad = "https://i-do-not-exist.com:9001/aaaa".to_string();

        ctx.fake_server.mock("GET", "/not-home")
            .with_status(404)
            .create_async()
            .await;

        // Check that we can get a non-200 error to an endpoint which exists (our mock server)
        let actual = call_shelly_plug(&test_path).await;
        assert_eq!(actual, Err("API request failed with non 200 status code"));

        // Check that we can't even dial into a URL which doesn't exist
        let actual_bad = call_shelly_plug(&test_path_bad).await;
        assert_eq!(actual_bad, Err("Failed to connect to API!"));
    }

    #[test_context(TestSetup)]
    #[tokio::test]
    async fn test_bad_response(ctx: &mut TestSetup) {
        let test_path = format!("{}/", ctx.fake_server.url());

        ctx.fake_server.mock("GET", "/")
            .with_status(200)
            .with_body("lol I'm not json")
            .create_async()
            .await;

        let actual = call_shelly_plug(&test_path).await;
        assert_eq!(actual, Err("Invalid response!"));
    }

    #[test_context(TestSetup)]
    #[tokio::test]
    async fn test_get_metrics(ctx: &mut TestSetup) {
        let test_path = format!("{}/", ctx.fake_server.url());
        let plugs: Vec<ShellySmartPlug> = vec![
            ShellySmartPlug{ url: test_path.clone(), alias: "alias1".to_string() },
            ShellySmartPlug{ url: test_path.clone(), alias: "alias2".to_string() }
        ];

        ctx.fake_server.mock("GET", "/")
            .with_status(200)
            .with_body(ctx.good_shelly_data.clone())
            .create_async()
            .await;

        let actual = get_metrics(&plugs).await.unwrap();
        let act_arr = actual.split("\n").collect::<Vec<&str>>();

        // Check that the \n logic works to combine multiple entries properly
        assert_eq!(act_arr[1], "power_watts{hostname=alias1} 1.0");
        assert_eq!(act_arr[6], "running_total_power_consumed_watts{hostname=alias1} 45645634.12");
        assert_eq!(act_arr[13], "running_total_power_consumed_watts{hostname=alias2} 45645634.12");

        assert!(actual.contains("power_watts{hostname=alias2} 1.0"));
        assert!(actual.contains("current_amps{hostname=alias1} 3.0"));
        assert!(actual.contains("temperature_celsius{hostname=alias1} 20.1"));
        assert!(actual.contains("temperature_fahrenheit{hostname=alias1} 68.2"));
        assert!(actual.contains("voltage{hostname=alias1} 2.0"));
        assert!(actual.contains("current_datetime{hostname=alias1}"));
    }
}