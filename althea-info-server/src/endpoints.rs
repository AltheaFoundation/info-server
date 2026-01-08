use actix_web::{get, HttpResponse, Responder};
use log::error;

use crate::{total_suppy::get_supply_info, tvl::get_unpriced_tvl, ALTHEA_NODE_GRPC};

#[get("/total_supply")]
async fn endpoint_get_total_supply() -> impl Responder {
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
async fn endpoint_get_total_liquid_supply() -> impl Responder {
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
async fn endpoint_get_all_supply_info() -> impl Responder {
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

#[get("/unpriced_tvl")]
async fn endpoint_get_unpriced_tvl() -> impl Responder {
    // Try to get the TVL, on failure return an error
    match get_unpriced_tvl(ALTHEA_NODE_GRPC.to_string()).await {
        Ok(v) => HttpResponse::Ok().json(v),
        Err(e) => {
            error!("Error getting unpriced TVL: {:#?}", e);
            HttpResponse::InternalServerError().json("Error getting unpriced TVL")
        }
    }
}
