const { task, terrajs } = require("@iboss/terrain");
const MsgSend = terrajs.MsgSend;

const terra = new terrajs.LCDClient({
  URL: 'http://localhost:1317',
  chainID: 'localterra'
});

// const terra = new terrajs.LCDClient({
//   "chainID": "bombay-12",
//   "URL": "https://bombay-lcd.terra.dev"
// });


task(async ({ wallets, refs, config, client }) => {
  try {
    // const wallet = wallets.validator;
    const wallet = wallets.admin;
    const send = new MsgSend(
      wallet.key.accAddress,
      client.refs["clearing-house"].contractAddresses.default,
      { uusd: 10000000 }
    );
    const tx = await wallet.createAndSignTx({
      msgs: [send],
      memo: "Hello"
    });
    const txResult = await terra.tx.broadcast(tx);
    console.log(txResult);

    var send1 = new MsgSend(
      wallet.key.accAddress,
      client.refs["collateral-vault"].contractAddresses.default,
      { uusd: 10000000 }
    );
    var tx1 = await wallet.createAndSignTx({
      msgs: [send1],
      memo: "Hello"
    });
    var txResult1 = await terra.tx.broadcast(tx1);
    console.log(txResult1);

    var send1 = new MsgSend(
      wallet.key.accAddress,
      client.refs["insurance-vault"].contractAddresses.default,
      { uusd: 10000000 }
    );
    var tx1 = await wallet.createAndSignTx({
      msgs: [send1],
      memo: "Hello"
    });
    var txResult1 = await terra.tx.broadcast(tx1);
    console.log(txResult1);


  } catch (error) {
    // console.log(error.response.data);
    console.log(error);
  }
  console.log(JSON.stringify(await client.bank.balance(client.refs["clearing-house"].contractAddresses.default)))
  console.log(JSON.stringify(await client.bank.balance(client.refs["collateral-vault"].contractAddresses.default)))
  console.log(JSON.stringify(await client.bank.balance(client.refs["insurance-vault"].contractAddresses.default)))




  // console.log(JSON.stringify(await client.bank.balance(wallets.validator.key.accAddress)))
});
