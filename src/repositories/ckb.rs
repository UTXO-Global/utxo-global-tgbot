use std::collections::HashMap;

use crate::{
    config,
    models::ckb::{AddressResponse, NFTInfo, TokenInfo, TokenResponse},
    serialize::error::AppError,
};
use ckb_sdk::{rpc::CkbRpcClient, NetworkType};
use reqwest::{header, Client};
use serde_json::json;

pub const CKB_TESTNET_EXPLORER_API: &str = "https://testnet-api.explorer.nervos.org/api";
pub const CKB_MAINNET_EXPLORER_API: &str = "https://mainnet-api.explorer.nervos.org/api";
pub const CKB_TESTNET_RPC: &str = "https://testnet.ckb.dev/rpc";
pub const CKB_MAINNET_RPC: &str = "https://mainnet.ckb.dev/rpc";

pub fn get_ckb_network() -> NetworkType {
    let network: String = config::get("network");
    match network.as_str() {
        "mainnet" => NetworkType::Mainnet,
        _ => NetworkType::Testnet,
    }
}

pub fn get_explorer_api_url(network: NetworkType) -> String {
    if network == NetworkType::Mainnet {
        return CKB_MAINNET_EXPLORER_API.to_owned();
    }

    CKB_TESTNET_EXPLORER_API.to_owned()
}

pub fn get_rpc() -> String {
    let network = get_ckb_network();
    if network == NetworkType::Mainnet {
        return CKB_MAINNET_RPC.to_owned();
    }

    CKB_TESTNET_RPC.to_owned()
}

pub async fn get_ckb_client() -> CkbRpcClient {
    let rpc_url: String = get_rpc();
    tokio::task::spawn_blocking(move || CkbRpcClient::new(&rpc_url))
        .await
        .expect("Failed to create CkbRpcClient")
}

pub async fn get_xudt_info(type_hash: String) -> Option<TokenInfo> {
    let network = get_ckb_network();
    let path = &format!("/v1/xudts/{}", type_hash);
    if let Ok(info) = proxy_request("GET", network, path, None).await {
        if let Ok(token_res) = serde_json::from_value::<TokenResponse>(info) {
            return Some(token_res.data.attributes);
        }
    }

    None
}

pub async fn get_collection_info(type_hash: String) -> Option<NFTInfo> {
    let network = get_ckb_network();
    let path = &format!("/v2/nft/collections/{}", type_hash);
    if let Ok(info) = proxy_request("GET", network, path, None).await {
        if let Ok(nft_info) = serde_json::from_value::<NFTInfo>(info) {
            return Some(nft_info);
        }
    }

    None
}

pub async fn get_balances(address: String) -> serde_json::Value {
    let network = get_ckb_network();
    let path = &format!("/v1/addresses/{}", address);
    let mut balance_map: HashMap<String, f64> = HashMap::new();
    if let Ok(info) = proxy_request("GET", network, path, None).await {
        if let Ok(address_response) = serde_json::from_value::<AddressResponse>(info) {
            if let Some(address_data) = address_response.data.first() {
                let attributes = &address_data.attributes;
                let balance = attributes.balance.parse::<f64>().unwrap_or(0.0);
                let balance_occupied: f64 =
                    attributes.live_cells_count.parse::<f64>().unwrap_or(0.0);
                balance_map.insert(
                    "CKB".to_string(),
                    (balance - balance_occupied) / 10f64.powi(8),
                );

                for udt in &attributes.udt_accounts {
                    if udt.udt_type == "spore_cell" {
                        if let Some(collection) = &udt.collection {
                            balance_map.insert(collection.type_hash.clone(), 1.0);
                        }
                    } else {
                        let amount = udt.amount.parse::<f64>().unwrap_or(0.0);
                        let decimal = udt
                            .decimal
                            .clone()
                            .unwrap_or("1".to_owned())
                            .parse::<u32>()
                            .unwrap_or(1);

                        let balance = if decimal > 0 {
                            amount / 10f64.powi(decimal as i32)
                        } else {
                            amount
                        };

                        balance_map.insert(udt.type_hash.clone(), balance);
                    }
                }
            }
        }
    }
    json!(balance_map)
}

async fn proxy_request(
    method: &str,
    network: NetworkType,
    path: &str,
    body: Option<String>,
) -> Result<serde_json::Value, AppError> {
    let ckb_api_url: String = get_explorer_api_url(network);
    let endpoint: String = format!("{}/{}", ckb_api_url, path.to_owned());
    let client = Client::new();
    let mut request_builder = match method {
        "GET" => client.get(endpoint),
        "POST" => client.post(endpoint),
        "PUT" => client.put(endpoint),
        _ => return Err(AppError::new(500).message("Method not allowed")),
    };

    request_builder = request_builder
        .header(header::ACCEPT, "application/vnd.api+json")
        .header(header::CONTENT_TYPE, "application/vnd.api+json")
        .header(header::USER_AGENT, "curl/7.68.0");

    if let Some(body) = body {
        request_builder = request_builder.body(body);
    }

    let response = request_builder
        .send()
        .await
        .map_err(|error| AppError::new(500).message(&error.to_string()))?;

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|error| AppError::new(500).message(&error.to_string()))?;

    Ok(result)
}
