const { task } = require("@iboss/terrain");

task(async ({ wallets, refs, config, client }) => {
  // let wallet = wallets.validator;    // wallet for localterra
  let wallet = wallets.admin;   // wallet for testnet
  const clearing_house = client.refs["clearing-house"].contractAddresses.default;
  await client.execute(wallet, "insurance-vault", {
    update_clearing_house: {
      "new_clearing_house": clearing_house
    },
  });
  const insurance_config = await client.query("insurance-vault", { get_config: {} });  
  console.log("insurance config ", insurance_config);

  await client.execute(wallet, "collateral-vault", {
    update_clearing_house: {
      "new_clearing_house": clearing_house
    },
  });
  const collateral_config = await client.query("collateral-vault", { get_config: {} });  
  console.log("collateral config ", collateral_config);

  await client.execute(wallet, "historical-store", {
    update_clearing_house: {
      "new_house": clearing_house
    },
  });
  const history_config = await client.query("historical-store", { get_config: {} });  
  console.log("history config ", history_config);

  // await client.execute(wallet, "clearing-house", {
  //   update_history_store: {
  //     "history_contract": client.refs["historical-store"].contractAddresses.default
  //   },
  // });
});
