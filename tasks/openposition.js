const { task } = require("@iboss/terrain");

const { calculateTradeAmount } = require("./helper");
task(async ({ wallets, refs, config, client }) => {
  try {
    let quote = calculateTradeAmount(100000000000);
    console.log(quote);
    await client.execute(wallets.localterra2, "clearing-house", {
      open_position: {
        "market_index": 1,
        "is_direction_long": true,
        "quote_asset_amount": quote.toString()
      }
    })
    

    // await client.execute(wallets.localterra2, "clearing-house", {
    //   close_position: {
    //     "market_index": 1
    //   }
    // })
  } catch (error) {
    console.log(error.response.data);
  }
});
