const { task } = require("@iboss/terrain");

task(async ({ wallets, refs, config, client }) => {
  // your logic here
  const market_info = await client.query("historical-store", { get_trade_history_by_address: { user_address: "terra18vd8fpwxzck93qlwghaj6arh4p7c5n896xzem5" } });
  console.log("market Info ", market_info);
});
