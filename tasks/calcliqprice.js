const { task } = require("@iboss/terrain");

task(async ({ wallets, refs, config, client }) => {
  let marketPosition = await client.query("clearing-house", {
    get_user_market_position: {
      user_address: wallets.localterra2.key.accAddress,
      market_index: 1
    }
  })
  let positionBaseSizeChange = 0;
  let partial = false;

  let totalCollateral = (await client.query("clearing-house", {
    get_user: {
      user_address: wallets.localterra2.key.accAddress
    }
  })).collateral;
  



});


public liquidationPrice(
  marketPosition: Pick < UserPosition, 'marketIndex' >,
  positionBaseSizeChange: BN = ZERO,
  partial = false
): BN {
  // solves formula for example canBeLiquidated below

  /* example: assume BTC price is $40k (examine 10% up/down)

      if 10k deposit and levered 10x short BTC => BTC up $400 means:
      1. higher base_asset_value (+$4k)
      2. lower collateral (-$4k)
      3. (10k - 4k)/(100k + 4k) => 6k/104k => .0576

      for 10x long, BTC down $400:
      3. (10k - 4k) / (100k - 4k) = 6k/96k => .0625 */

  const totalCollateral = this.getTotalCollateral();

  // calculate the total position value ignoring any value from the target market of the trade
  const totalPositionValueExcludingTargetMarket =
    this.getTotalPositionValueExcludingMarket(marketPosition.marketIndex);

  const currentMarketPosition =
    this.getUserPosition(marketPosition.marketIndex) ||
    this.getEmptyPosition(marketPosition.marketIndex);

  const currentMarketPositionBaseSize = currentMarketPosition.baseAssetAmount;

  const proposedBaseAssetAmount = currentMarketPositionBaseSize.add(
    positionBaseSizeChange
  );

  // calculate position for current market after trade
  const proposedMarketPosition: UserPosition = {
    marketIndex: marketPosition.marketIndex,
    baseAssetAmount: proposedBaseAssetAmount,
    lastCumulativeFundingRate:
      currentMarketPosition.lastCumulativeFundingRate,
    quoteAssetAmount: new BN(0),
    openOrders: new BN(0),
  };

  if (proposedBaseAssetAmount.eq(ZERO)) return new BN(-1);

  const market = this.clearingHouse.getMarket(
    proposedMarketPosition.marketIndex
  );

  const proposedMarketPositionValue = calculateBaseAssetValue(
    market,
    proposedMarketPosition
  );

  // total position value after trade
  const totalPositionValueAfterTrade =
    totalPositionValueExcludingTargetMarket.add(proposedMarketPositionValue);

  const marginRequirementExcludingTargetMarket =
    this.getUserPositionsAccount().positions.reduce(
      (totalMarginRequirement, position) => {
        if (!position.marketIndex.eq(marketPosition.marketIndex)) {
          const market = this.clearingHouse.getMarket(position.marketIndex);
          const positionValue = calculateBaseAssetValue(market, position);
          const marketMarginRequirement = positionValue
            .mul(
              partial
                ? new BN(market.marginRatioPartial)
                : new BN(market.marginRatioMaintenance)
            )
            .div(MARGIN_PRECISION);
          totalMarginRequirement = totalMarginRequirement.add(
            marketMarginRequirement
          );
        }
        return totalMarginRequirement;
      },
      ZERO
    );

  const freeCollateralExcludingTargetMarket = totalCollateral.sub(
    marginRequirementExcludingTargetMarket
  );

  // if the position value after the trade is less than free collateral, there is no liq price
  if (
    totalPositionValueAfterTrade.lte(freeCollateralExcludingTargetMarket) &&
    proposedMarketPosition.baseAssetAmount.abs().gt(ZERO)
  ) {
    return new BN(-1);
  }

  const marginRequirementAfterTrade =
    marginRequirementExcludingTargetMarket.add(
      proposedMarketPositionValue
        .mul(
          partial
            ? new BN(market.marginRatioPartial)
            : new BN(market.marginRatioMaintenance)
        )
        .div(MARGIN_PRECISION)
    );
  const freeCollateralAfterTrade = totalCollateral.sub(
    marginRequirementAfterTrade
  );

  const marketMaxLeverage = partial
    ? this.getMaxLeverage(proposedMarketPosition.marketIndex, 'Partial')
    : this.getMaxLeverage(proposedMarketPosition.marketIndex, 'Maintenance');

  let priceDelta;
  if (proposedBaseAssetAmount.lt(ZERO)) {
    priceDelta = freeCollateralAfterTrade
      .mul(marketMaxLeverage) // precision is TEN_THOUSAND
      .div(marketMaxLeverage.add(TEN_THOUSAND))
      .mul(PRICE_TO_QUOTE_PRECISION)
      .mul(AMM_RESERVE_PRECISION)
      .div(proposedBaseAssetAmount);
  } else {
    priceDelta = freeCollateralAfterTrade
      .mul(marketMaxLeverage) // precision is TEN_THOUSAND
      .div(marketMaxLeverage.sub(TEN_THOUSAND))
      .mul(PRICE_TO_QUOTE_PRECISION)
      .mul(AMM_RESERVE_PRECISION)
      .div(proposedBaseAssetAmount);
  }

  let markPriceAfterTrade;
  if (positionBaseSizeChange.eq(ZERO)) {
    markPriceAfterTrade = calculateMarkPrice(
      this.clearingHouse.getMarket(marketPosition.marketIndex)
    );
  } else {
    const direction = positionBaseSizeChange.gt(ZERO)
      ? PositionDirection.LONG
      : PositionDirection.SHORT;
    markPriceAfterTrade = calculateTradeSlippage(
      direction,
      positionBaseSizeChange.abs(),
      this.clearingHouse.getMarket(marketPosition.marketIndex),
      'base'
    )[3]; // newPrice after swap
  }

  if (priceDelta.gt(markPriceAfterTrade)) {
    return new BN(-1);
  }

  return markPriceAfterTrade.sub(priceDelta);
}
