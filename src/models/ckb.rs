use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct UdtTypeScript {
    pub args: Option<String>,
    pub code_hash: Option<String>,
    pub hash_type: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct CollectionScript {
    pub type_hash: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct UdtAccount {
    pub symbol: Option<String>,
    pub decimal: Option<String>,
    pub amount: Option<String>,
    pub type_hash: Option<String>,
    pub udt_type: Option<String>,
    pub collection: Option<CollectionScript>,
    pub udt_type_script: UdtTypeScript,
}

#[derive(Deserialize, Debug)]
pub struct AddressAttributes {
    pub address_hash: Option<String>,
    pub balance: Option<String>,
    pub transactions_count: Option<String>,
    pub live_cells_count: Option<String>,
    pub udt_accounts: Vec<UdtAccount>,
}

#[derive(Deserialize, Debug)]
pub struct AddressData {
    pub id: Option<String>,

    #[serde(rename = "type")]
    pub data_type: Option<String>,
    pub attributes: AddressAttributes,
}

#[derive(Deserialize, Debug)]
pub struct AddressResponse {
    pub data: Vec<AddressData>,
}

#[derive(Deserialize, Debug)]
pub struct CKBBalance {
    pub symbol: String,
    pub balance: f64,
    pub type_hash: String,
}

#[derive(Deserialize, Debug)]
pub struct TokenInfo {
    pub symbol: Option<String>,
    pub decimal: Option<String>,
    pub description: Option<String>,
    pub full_name: Option<String>,
    pub udt_type: Option<String>,
    pub type_script: Option<UdtTypeScript>,
}

#[derive(Deserialize, Debug)]
pub struct TokenData {
    pub id: String,
    #[serde(rename = "type")]
    pub data_type: String,
    pub attributes: TokenInfo,
}

#[derive(Deserialize, Debug)]
pub struct TokenResponse {
    pub data: TokenData,
}

#[derive(Deserialize, Debug)]
pub struct NFTTypeScript {
    pub args: String,
    pub code_hash: String,
    pub hash_type: String,
    pub script_hash: String,
}

#[derive(Deserialize, Debug)]
pub struct NFTInfo {
    pub name: String,
    pub standard: String,
    pub type_script: NFTTypeScript,
}
