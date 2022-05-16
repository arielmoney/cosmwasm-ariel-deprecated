const { task } = require("@iboss/terrain");

task(async ({ wallets, refs, config, client }) => {
  // const user = await client.query("clearing-house", {get_user: {
  //   user_address: "terra17lmam6zguazs5q5u6z5mmx76uj63gldnse2pdp"
  // }})
  // console.log(user);
  //check user balance
  // console.log(wallets.localterra2.key.accAddress);
  // console.log(JSON.stringify(await client.bank.balance(wallets.localterra2.key.accAddress)))
  //balance of collateral vault
  // console.log("collateral vault ==>>");
  // console.log(JSON.stringify(await client.bank.balance("terra1efp5yc92xalspatuz9nwggquggg5ndtfexhlur")))
  //balance of clearing house
  // console.log("clearing house balance>>>>>>");  
  // console.log(JSON.stringify(await client.bank.balance("terra1yvkswpju0fuy48sm6j874gx59gxjg2eum3cjtz")))


  //check deposit history
  // const deposit_history = await client.query("historical-store", {get_deposit_history: {
  //   user_address: wallets.localterra2.key.accAddress
  // }})

  // console.log("deposit history>>>>>>");
  // console.log(deposit_history);



  const trade_history = await client.query("historical-store", {get_trade_history: {
    start_after: "5"
  }})
  
  console.log("trade history>>>>>>");
  console.log(trade_history);
});
