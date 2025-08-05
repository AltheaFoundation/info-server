#[macro_use]
extern crate lazy_static;

pub mod tls;
pub mod total_suppy;

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
    tls::{load_certs, load_private_key},
    total_suppy::get_supply_info,
};
use actix_cors::Cors;
use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use env_logger::Env;
use log::{error, info};
use rustls::ServerConfig;
use total_suppy::chain_total_supply_thread;

pub const ALTHEA_NODE_GRPC: &str = "https://rpc.althea.zone:9090";
pub const ALTHEA_PREFIX: &str = "althea";
pub const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

#[get("/total_supply")]
async fn get_total_supply() -> impl Responder {
    // if we have already computed supply info return it, if not return an error
    match get_supply_info() {
        Some(v) => HttpResponse::Ok().json(v.total_supply),
        None => HttpResponse::InternalServerError()
            .json("Info not yet generated, please query in 5 minutes"),
    }
}

/// If the liquid supply is lower than this value it's stale or otherwise invalid and we should
/// return an error.
pub const SUPPLY_CHECKPOINT: u128 = 500000000000000;
#[get("/total_liquid_supply")]
async fn get_total_liquid_supply() -> impl Responder {
    // if we have already computed supply info return it, if not return an error
    match get_supply_info() {
        Some(v) => {
            if v.total_liquid_supply > SUPPLY_CHECKPOINT.into() {
                HttpResponse::Ok().json(v.total_liquid_supply)
            } else {
                error!("Invalid supply data, got total liquid supply of {:#?}", v);
                HttpResponse::InternalServerError()
                    .json("Invalid supply data, Althea fullnode is stale")
            }
        }
        None => HttpResponse::InternalServerError()
            .json("Info not yet generated, please query in 5 minutes"),
    }
}

#[get("/supply_info")]
async fn get_all_supply_info() -> impl Responder {
    // if we have already computed supply info return it, if not return an error
    match get_supply_info() {
        Some(v) => {
            if v.total_liquid_supply > SUPPLY_CHECKPOINT.into() {
                HttpResponse::Ok().json(v)
            } else {
                error!("Invalid supply data, got total liquid supply of {:#?}", v);
                HttpResponse::InternalServerError()
                    .json("Invalid supply data, Althea fullnode is stale")
            }
        }
        None => HttpResponse::InternalServerError()
            .json("Info not yet generated, please query in 5 minutes"),
    }
}

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
            .service(get_total_supply)
            .service(get_total_liquid_supply)
            .service(get_all_supply_info)
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
