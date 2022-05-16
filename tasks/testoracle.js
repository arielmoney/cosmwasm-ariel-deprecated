const { task } = require("@iboss/terrain");

task(async ({ wallets, refs, config, client }) => {
  
  const insurance_config = await client.query("oracle", { price_luna: {} });  
  console.log("insurance config ", insurance_config);

  // await client.execute(wallet, "collateral-vault", {
  //   update_clearing_house: {
  //     "new_clearing_house": clearing_house
  //   },
  // });
});
