const { task } = require("@iboss/terrain");

task(async ({ wallets, refs, config, client }) => {
  try {
    const user = await client.query("clearing-house", {
      get_user_market_position: {
        user_address: "terra1elvps6xapsxh56lzxca2zncu9x8av2xd2q9rvv",
        index: 1
      }
    })
    console.log(user);

    const userp = await client.query("clearing-house", {
      get_user_positions: {
        user_address: "terra1elvps6xapsxh56lzxca2zncu9x8av2xd2q9rvv"
      }
    })
    console.log(userp);

    // const trade_history = await client.query("historical-store", {
    //   get_trade_history_by_address: {
    //     user_address: "terra17lmam6zguazs5q5u6z5mmx76uj63gldnse2pdp"
    //   }
    // })

    // console.log("trade history>>>>>>");
    // console.log(trade_history);

  } catch (error) {
    console.log(error);
  }

});
