#[macro_use]
extern crate lazy_static;

pub mod config;
pub mod endpoints;
pub mod tls;
pub mod total_suppy;
pub mod tvl;

const DEVELOPMENT: bool = cfg!(feature = "development");
const SSL: bool = !DEVELOPMENT;
const DOMAIN: &str = if cfg!(test) || DEVELOPMENT {
    "localhost"
} else {
    "info.althea.zone"
};
/// The backend RPC port for the info server fucntions implemented in this repo
const INFO_SERVER_PORT: u16 = 9000;

use crate::{
    endpoints::{
        endpoint_get_all_supply_info, endpoint_get_total_liquid_supply, endpoint_get_total_supply,
        endpoint_get_unpriced_tvl,
    },
    tls::{load_certs, load_private_key},
};
use actix_cors::Cors;
use actix_web::{App, HttpServer};
use env_logger::Env;
use log::info;
use rustls::ServerConfig;
use total_suppy::chain_total_supply_thread;

pub const ALTHEA_NODE_GRPC: &str = "https://rpc.althea.zone:9090";
pub const ALTHEA_PREFIX: &str = "althea";
pub const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    openssl_probe::init_ssl_cert_env_vars();
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    // starts a background thread for downloading transactions
    chain_total_supply_thread();

    let info_server = HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_header()
                    .allow_any_method(),
            )
            .service(endpoint_get_total_supply)
            .service(endpoint_get_total_liquid_supply)
            .service(endpoint_get_all_supply_info)
            .service(endpoint_get_unpriced_tvl)
    });

    let info_server = if SSL {
        let cert_chain = load_certs(&format!("/etc/letsencrypt/live/{DOMAIN}/fullchain.pem"));
        let keys = load_private_key(&format!("/etc/letsencrypt/live/{DOMAIN}/privkey.pem"));
        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert_chain, keys)
            .unwrap();

        info!("Binding to SSL");

        info_server.bind_rustls(format!("{DOMAIN}:{INFO_SERVER_PORT}"), config.clone())?
    } else {
        info_server.bind(format!("{DOMAIN}:{INFO_SERVER_PORT}"))?
    };

    info_server.run().await?;

    Ok(())
}
