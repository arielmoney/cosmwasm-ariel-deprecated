const { task } = require("@iboss/terrain");

task(async ({ wallets, refs, config, client }) => {
  try {
    await client.execute(wallets.localterra2, "clearing-house", {
      settle_funding_payment: {
      }
    })
  } catch (error) {
    console.log(error.response.data);
  }
});
