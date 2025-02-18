use std::{collections::HashMap, str::FromStr};

use crate::config;
use ckb_jsonrpc_types::Uint32;
use ckb_sdk::{
    rpc::{
        ckb_indexer::{Cell, Order, ScriptType, SearchKey},
        ckb_light_client::Pagination,
        CkbRpcClient,
    },
    Address, NetworkType,
};
use ckb_types::{packed::Script, prelude::Unpack};
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

pub async fn get_cells(address: String) -> Result<Pagination<Cell>, ckb_sdk::rpc::RpcError> {
    let rpc_url: String = get_rpc();
    tokio::task::spawn_blocking(move || {
        let client = CkbRpcClient::new(&rpc_url);
        let wallet = Address::from_str(&address).unwrap();
        let script: Script = Script::from(&wallet);
        let hash_type = match script.hash_type().as_slice()[0] {
            0 => ckb_jsonrpc_types::ScriptHashType::Data,
            1 => ckb_jsonrpc_types::ScriptHashType::Type,
            2 => ckb_jsonrpc_types::ScriptHashType::Data1,
            4 => ckb_jsonrpc_types::ScriptHashType::Data2,
            _ => panic!(
                "Invalid hash_type value: {}",
                script.hash_type().as_slice()[0]
            ),
        };
        let search_key = SearchKey {
            script: ckb_jsonrpc_types::Script {
                code_hash: script.code_hash().unpack(),
                hash_type,
                args: script.args().into(),
            },
            script_type: ScriptType::Lock,
            script_search_mode: None,
            filter: None,
            with_data: None,
            group_by_transaction: None,
        };

        client.get_cells(search_key, Order::Desc, Uint32::from(1000u32), None)
    })
    .await
    .unwrap()
}

pub async fn get_balances(address: String) -> serde_json::Value {
    let cells = match get_cells(address).await {
        Ok(pagination) => pagination.objects,
        Err(_) => return json!({}),
    };

    let mut balance_map: HashMap<String, u64> = HashMap::new();

    for cell in cells {
        let mut token_address = String::from("CKB");
        let capacity: u64 = cell.output.capacity.value();
        if let Some(output_type) = cell.output.type_ {
            token_address = format!("0x{}", hex::encode(output_type.args.as_bytes()));
        }

        let balance: u128 = if let Some(output_data) = cell.output_data {
            if !output_data.is_empty() {
                let mut amount_bytes = [0u8; 16];
                let len = output_data.as_bytes().len().min(16);
                amount_bytes[..len].copy_from_slice(&output_data.as_bytes()[..len]);
                u128::from_le_bytes(amount_bytes)
            } else {
                capacity as u128
            }
        } else {
            capacity as u128
        };

        let balance_u64 = (balance / 1_0000_0000) as u64;
        *balance_map.entry(token_address).or_insert(0) += balance_u64;
    }

    json!(balance_map)
}
