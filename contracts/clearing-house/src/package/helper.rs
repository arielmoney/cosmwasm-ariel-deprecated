use cosmwasm_std::{
    Addr, Api, BalanceResponse, BankQuery, MessageInfo, QuerierWrapper, QueryRequest, StdError,
    StdResult, Uint128,
};
use schemars::JsonSchema;
use serde::{Serialize, Deserialize};

pub fn addr_validate_to_lower(api: &dyn Api, addr: &str) -> StdResult<Addr> {
    if addr.to_lowercase() != addr {
        return Err(StdError::generic_err(format!(
            "Address {} should be lowercase",
            addr
        )));
    }
    api.addr_validate(addr)
}

pub fn assert_sent_uusd_balance(message_info: &MessageInfo, input_amount: u128) -> StdResult<()> {
    let amount = Uint128::from(input_amount);
    match message_info.funds.iter().find(|x| x.denom == "uusd") {
        Some(coin) => {
            if amount == coin.amount {
                Ok(())
            } else {
                Err(StdError::generic_err(
                    "Native token balance mismatch between the argument and the transferred",
                ))
            }
        }
        None => {
            if amount.is_zero() {
                Ok(())
            } else {
                Err(StdError::generic_err(
                    "Native token balance mismatch between the argument and the transferred",
                ))
            }
        }
    }
}

pub fn query_balance(querier: &QuerierWrapper, account_addr: Addr) -> StdResult<u128> {
    let balance: BalanceResponse = querier.query(&QueryRequest::Bank(BankQuery::Balance {
        address: String::from(account_addr),
        denom: "uusd".to_string(),
    }))?;
    Ok(balance.amount.amount.u128())
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum VaultInterface {
    Withdraw{
        to_address: Addr,
        amount: Uint128
    },
    Deposit {}

}