const { task } = require("@iboss/terrain");

task(async ({ wallets, refs, config, client }) => {
  try {
    await client.execute(wallets.localterra2, "clearing-house", {
      deposit_collateral: {
        "amount": 1000000000
      }
    }, { "uusd": 1000000000 })
    // await client.execute(wallets.localterra2, "clearing-house", {
    //   withdraw_collateral: {
    //     "amount": 50000
    //   }
    // })
  } catch (error) {
    console.log(error.response);
  }
  const user = await client.query("clearing-house", {
    get_user: {
      user_address: wallets.localterra2.key.accAddress
    }
  })
  console.log(user);
});
