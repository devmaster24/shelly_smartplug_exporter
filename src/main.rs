use actix_web::{App, get, HttpResponse, HttpServer, Responder, web};
use actix_web::middleware::Logger;
use clap::Parser;
use colog;
use log::{error, warn};

use crate::shelly_service::ShellySmartPlug;

mod shelly_service;

#[derive(Parser, Debug)]
#[command(about = "Prometheus exporter for shelly smart plugs")]
#[command(name = "Shelly Smart Plug Exporter", version, long_about = None)]
struct Args {
    /// IP address of your smart plug(s) on your local network
    #[arg(short, long = "ip-addr", required = true, value_delimiter = ' ')]
    ip_addrs: Vec<String>,

    /// Port to run the webserver at
    #[arg(short = 'p', long, default_value_t = 9001)]
    server_port: u16,

    /// IP -> Hostname mapping in `ip_address:hostname` format
    #[arg(short = 'm', long, required = false)]
    hostname_ip_mapping: Vec<String>,
}


#[derive(Clone)]
struct AppState {
    plugs: Vec<ShellySmartPlug>,
}


#[get("/metrics")]
async fn metrics(state: web::Data<AppState>) -> impl Responder {
    match shelly_service::get_metrics(&state.plugs).await {
        Ok(output) => HttpResponse::Ok().body(output),
        Err(e) => {
            error!("An error occurred during processing - {e}");
            HttpResponse::InternalServerError()
                .body("Failed to process, please check application logs")
        }
    }
}


fn load_plugs(cli_args: &Args) -> Vec<ShellySmartPlug> {
    let mut plugs: Vec<ShellySmartPlug> = vec![];
    for ip in &cli_args.ip_addrs {
        // Will overwrite if user provided a hostname mapping, else just use the IP
        let mut alias = ip.clone();

        for mapping in &cli_args.hostname_ip_mapping {
            if mapping.contains(&ip.clone()) {
                // Since clap has an awkward time having field parsers for Vec<String> adding a
                // little check here to ensure the format is correct. Deciding to warn the user and
                // continue since this isn't a catastrophic error
                // Ref: https://github.com/clap-rs/clap/issues/4808
                if !mapping.contains(":") {
                    warn!("Invalid mapping `{}`! Please use format `ip:hostname`",mapping);
                    break;
                }

                alias = mapping.split(':').collect::<Vec<&str>>()[1].to_string();
                break;
            }
        }

        plugs.push(ShellySmartPlug {
            url: format!("http://{}/rpc/Switch.GetStatus?id=0", ip.clone()),
            alias,
        });
    }

    plugs
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    colog::init();
    let cli = Args::parse();
    let state = AppState { plugs: load_plugs(&cli) };

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .service(metrics)
            .wrap(Logger::default())
    })
        .bind(("0.0.0.0", cli.server_port))?
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_cli() {
        use clap::CommandFactory;
        Args::command().debug_assert();
    }

    #[test]
    fn test_load_plugs_from_cli_args() {
        let test_args = Args {
            ip_addrs: vec![
                "10.0.0.1".to_string(),
                "10.0.0.2".to_string(),
                "10.0.0.3".to_string()
            ],
            server_port: 9002,
            hostname_ip_mapping: vec![
                "10.0.0.1~something_invalid".to_string(),
                "10.0.0.2:valid".to_string()
            ]
        };

        let actual = load_plugs(&test_args);

        assert_eq!(actual.len(), 3);
        assert_eq!(actual[0].alias, "10.0.0.1");
        assert_eq!(actual[0].url, "http://10.0.0.1/rpc/Switch.GetStatus?id=0");
        assert_eq!(actual[1].alias, "valid");
        assert_eq!(actual[2].alias, "10.0.0.3");
    }
}