use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct UdtTypeScript {
    pub args: String,
    pub code_hash: String,
    pub hash_type: String,
}

#[derive(Deserialize, Debug)]
pub struct CollectionScript {
    pub type_hash: String,
}

#[derive(Deserialize, Debug)]
pub struct UdtAccount {
    pub symbol: Option<String>,
    pub decimal: Option<String>,
    pub amount: String,
    pub type_hash: String,
    pub udt_type: String,
    pub collection: Option<CollectionScript>,
    pub udt_type_script: UdtTypeScript,
}

#[derive(Deserialize, Debug)]
pub struct AddressAttributes {
    pub address_hash: String,
    pub balance: String,
    pub transactions_count: String,
    pub live_cells_count: String,
    pub udt_accounts: Vec<UdtAccount>,
}

#[derive(Deserialize, Debug)]
pub struct AddressData {
    pub id: String,
    #[serde(rename = "type")]
    pub data_type: String,
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
