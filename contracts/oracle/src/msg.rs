use cosmwasm_std::{Addr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    RegisterAsset {
        asset: String,
        price_feeder: Addr,
    },
    RevokeAsset {
        asset: String,
    },
    FeedPrice {
        asset: String,
        price: Uint128
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    Price {
        asset: String,
    },
    AssetInfo {
        asset: String,
    },
    PriceLuna {},
    PriceBTC {}
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: Addr,
    pub base_denom: String,
    // pub mirror_oracle: Addr,
    // pub anchor_oracle: Addr, 
    // pub band_oracle: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PriceResponse {
    pub asset: String,
    pub price: Uint128,
    pub last_updated: u64,
    // pub multiplier: Decimal,
    // pub is_revoked: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PriceResponseLuna {
    pub asset: String,
    pub price: Uint128,
    pub last_updated: u64,
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InfoResponse {
    pub asset: String,
    pub feeder: Addr,
    pub price: Uint128,
    pub last_updated: u64,
    // pub multiplier: Decimal,
    // pub source_type: String,
    // pub is_revoked: bool,
}

// pub struct CollateralInfosResponse {
//     pub collaterals: Vec<CollateralInfoResponse>,
// }