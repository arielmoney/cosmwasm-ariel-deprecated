const { task } = require("@iboss/terrain");

task(async ({ wallets, refs, config, client }) => {
  // your logic here
  try {
    await client.execute(wallets.admin, "clearing-house", {
      oracle_feeder: {
        "market_index": 1,
        "price": "659450000000"
      }
    })  
  } catch (error) {
      console.log(error.response.data);
  }
  
});
