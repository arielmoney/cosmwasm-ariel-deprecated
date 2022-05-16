#[cfg(test)]
mod tests {
    use crate::contract::{instantiate, query};
    use crate::msg::{InstantiateMsg, QueryMsg, ConfigResponse, PriceResponseLuna};

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        let msg = InstantiateMsg { };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!("creator", value.owner);

        let pres = query(deps.as_ref(), mock_env(), QueryMsg::PriceLuna {}).unwrap();
        let pvalue: PriceResponseLuna = from_binary(&pres).unwrap();
        println!("price of Luna is ${}", pvalue.price);
    }

}
