const { task } = require("@iboss/terrain");

task(async ({ wallets, refs, config, client }) => {
  
  const trade_history = await client.query("historical-store", {
    get_trade_history: {
    }
  })

  console.log("trade history all>>>>>>");
  console.log(trade_history);

  const trade_his = await client.query("historical-store", {
    get_trade_history_by_address: {
      user_address: "terra17lmam6zguazs5q5u6z5mmx76uj63gldnse2pdp"
    }
  })

  console.log("trade history all>>>>>>");
  console.log(trade_his);
});
