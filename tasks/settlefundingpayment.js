const { task } = require("@iboss/terrain");

task(async ({ wallets, refs, config, client }) => {
  // try {
  //   await client.execute(wallets.localterra2, "clearing-house", {
  //     settle_funding_payment: {
  //     }
  //   })
  // } catch (error) {
  //   console.error(error);
  // }

  let fphis = await client.query("historical-store", {
    get_funding_payment_history: {
      "user_address": wallets.localterra2.key.accAddress
    }
  })
  console.log(fphis);
});
