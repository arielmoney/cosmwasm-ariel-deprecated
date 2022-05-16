const terra = require("@terra-money/terra.js");
const Int = terra.Int, Dec = terra.Dec;
const MAX_LEVERAGE = new Int(5);
const TEN_THOUSAND = new Int(10000);
const ONE_MANTISSA = new Int(100000);
const MARK_PRICE_PRECISION = new Int(10 ** 10);
const QUOTE_PRECISION = new Int(10 ** 6);
const FUNDING_PAYMENT_PRECISION = new Int(10000);
const PEG_PRECISION = new Int(1000);

const AMM_RESERVE_PRECISION = new Int(10 ** 13);
const BASE_PRECISION = AMM_RESERVE_PRECISION;
const AMM_TO_QUOTE_PRECISION_RATIO =
    AMM_RESERVE_PRECISION.div(QUOTE_PRECISION); // 10^7
const PRICE_TO_QUOTE_PRECISION =
    MARK_PRICE_PRECISION.div(QUOTE_PRECISION);
const AMM_TIMES_PEG_TO_QUOTE_PRECISION_RATIO =
    AMM_RESERVE_PRECISION.mul(PEG_PRECISION).div(QUOTE_PRECISION); // 10^10
const MARGIN_PRECISION = TEN_THOUSAND;

//10000000
const calculateTradeAmount = (amountOfCollateral) => {
    amountOfCollateral = new Int(amountOfCollateral);

    const fee = ONE_MANTISSA.div(new Int(1000));
    const tradeAmount = amountOfCollateral
        .mul(MAX_LEVERAGE)
        .mul(ONE_MANTISSA.sub(MAX_LEVERAGE.mul(fee)))
        .div(ONE_MANTISSA);
    return tradeAmount;
};

const mantissaSqrtScale = new Int(Math.sqrt(MARK_PRICE_PRECISION.toNumber()));
const ammInitialQuoteAssetAmount = new Int(5 * 10 ** 13).mul(mantissaSqrtScale);
const ammInitialBaseAssetAmount = new Int(5 * 10 ** 13).mul(mantissaSqrtScale);

const calculateTradeSlippage = (direction, amount, market) => {

}
class OracleSource{
    
}

module.exports = {
    calculateTradeAmount,
    mantissaSqrtScale,
    ammInitialQuoteAssetAmount,
    ammInitialBaseAssetAmount
};
