const { task, terrajs } = require("@iboss/terrain");
const Int = terrajs.Int;
const MsgSend = terrajs.MsgSend;
const { calculateTradeAmount,
  ammInitialBaseAssetAmount,
  ammInitialQuoteAssetAmount,
  mantissaSqrtScale } = require("./helper");

task(async ({ wallets, refs, config, client }) => {
  try {
    // let wallet = wallets.validator;    // wallet for localterra
    // let wallet = wallets.admin;   // wallet for testnet
    // const FIFTEEN_DAYS = 15 * 24 * 3600;
    // await client.execute(wallet, "clearing-house", {
    //   initialize_market: {
    //     "market_index": 1,
    //     "market_name": "LUNA-PERP",
    //     "amm_base_asset_reserve": ammInitialBaseAssetAmount.toString(),
    //     "amm_quote_asset_reserve": ammInitialQuoteAssetAmount.toString(),
    //     "amm_periodicity": FIFTEEN_DAYS,
    //     "amm_peg_multiplier": "48987",   //48.987 
    //     "oracle_source_code": 0,
    //     "margin_ratio_partial": 625,
    //     "margin_ratio_initial": 2000,
    //     "margin_ratio_maintenance": 500
    //   },
    // });
  } catch (error) {
    console.log(error.response.data);
    // console.log(error);
  }
  const market_info = await client.query("clearing-house", { get_market_info: { market_index: 1 } });
  console.log("market Info ", market_info);
});
