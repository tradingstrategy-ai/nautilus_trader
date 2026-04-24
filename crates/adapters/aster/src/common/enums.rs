// -------------------------------------------------------------------------------------------------
//  Copyright (C) 2015-2026 Nautech Systems Pty Ltd. All rights reserved.
//  https://nautechsystems.io
//
//  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
//  You may not use this file except in compliance with the License.
//  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
// -------------------------------------------------------------------------------------------------

//! Aster enumeration types for product types and environments.

use std::fmt::Display;

use nautilus_model::enums::{MarketStatusAction, OrderSide, OrderType, TimeInForce};
use serde::{Deserialize, Serialize};

/// Aster product type identifier.
///
/// Each product type corresponds to a different Aster API domain and
/// has distinct trading rules and instrument specifications.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        module = "nautilus_trader.core.nautilus_pyo3.aster",
        eq,
        from_py_object,
        rename_all = "SCREAMING_SNAKE_CASE"
    )
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass_enum(module = "nautilus_trader.aster")
)]
pub enum AsterProductType {
    /// Spot trading (api.asterdex.com).
    #[default]
    Spot,
    /// Spot Margin trading (uses Spot API with margin endpoints).
    Margin,
    /// USD-M Futures - linear perpetuals and delivery futures (fapi.asterdex.com).
    UsdM,
    /// COIN-M Futures - inverse perpetuals and delivery futures (dapi.asterdex.com).
    CoinM,
    /// European Options (eapi.asterdex.com).
    Options,
}

impl AsterProductType {
    /// Returns the string representation used in API requests.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Spot => "SPOT",
            Self::Margin => "MARGIN",
            Self::UsdM => "USD_M",
            Self::CoinM => "COIN_M",
            Self::Options => "OPTIONS",
        }
    }

    /// Returns the instrument ID suffix for this product type.
    #[must_use]
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Spot => "-SPOT",
            Self::Margin => "-MARGIN",
            Self::UsdM => "-LINEAR",
            Self::CoinM => "-INVERSE",
            Self::Options => "-OPTION",
        }
    }

    /// Returns true if this is a spot product (Spot or Margin).
    #[must_use]
    pub const fn is_spot(self) -> bool {
        matches!(self, Self::Spot | Self::Margin)
    }

    /// Returns true if this is a futures product (USD-M or COIN-M).
    #[must_use]
    pub const fn is_futures(self) -> bool {
        matches!(self, Self::UsdM | Self::CoinM)
    }

    /// Returns true if this is a linear product (Spot, Margin, or USD-M).
    #[must_use]
    pub const fn is_linear(self) -> bool {
        matches!(self, Self::Spot | Self::Margin | Self::UsdM)
    }

    /// Returns true if this is an inverse product (COIN-M).
    #[must_use]
    pub const fn is_inverse(self) -> bool {
        matches!(self, Self::CoinM)
    }

    /// Returns true if this is an options product.
    #[must_use]
    pub const fn is_options(self) -> bool {
        matches!(self, Self::Options)
    }
}

impl Display for AsterProductType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Aster environment type.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        module = "nautilus_trader.core.nautilus_pyo3.aster",
        eq,
        from_py_object,
        rename_all = "SCREAMING_SNAKE_CASE"
    )
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass_enum(module = "nautilus_trader.aster")
)]
pub enum AsterEnvironment {
    /// Production/mainnet environment.
    #[default]
    Mainnet,
    /// Testnet environment.
    Testnet,
    /// Demo trading environment.
    Demo,
}

impl AsterEnvironment {
    /// Returns true if this is the testnet environment.
    #[must_use]
    pub const fn is_testnet(self) -> bool {
        matches!(self, Self::Testnet)
    }

    /// Returns true for any non-production environment.
    #[must_use]
    pub const fn is_sandbox(self) -> bool {
        matches!(self, Self::Testnet | Self::Demo)
    }
}

/// Order side for Aster orders and trades.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AsterSide {
    /// Buy side.
    Buy,
    /// Sell side.
    Sell,
}

impl TryFrom<OrderSide> for AsterSide {
    type Error = anyhow::Error;

    fn try_from(value: OrderSide) -> Result<Self, Self::Error> {
        match value {
            OrderSide::Buy => Ok(Self::Buy),
            OrderSide::Sell => Ok(Self::Sell),
            _ => anyhow::bail!("Unsupported `OrderSide` for Aster: {value:?}"),
        }
    }
}

impl From<AsterSide> for OrderSide {
    fn from(value: AsterSide) -> Self {
        match value {
            AsterSide::Buy => Self::Buy,
            AsterSide::Sell => Self::Sell,
        }
    }
}

/// Position side for dual-side position mode.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        module = "nautilus_trader.core.nautilus_pyo3.aster",
        eq,
        from_py_object
    )
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass_enum(module = "nautilus_trader.aster")
)]
pub enum AsterPositionSide {
    /// Single position mode (both).
    Both,
    /// Long position.
    Long,
    /// Short position.
    Short,
    /// Unknown or undocumented value.
    #[serde(other)]
    Unknown,
}

/// Margin type applied to a position.
///
/// Serializes to the POST format (`CROSSED`/`ISOLATED`) expected by
/// `/fapi/v1/marginType`. Deserializes from both POST and GET/WS
/// formats (`cross`/`isolated`) via serde aliases.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        module = "nautilus_trader.core.nautilus_pyo3.aster",
        eq,
        from_py_object,
        rename_all = "SCREAMING_SNAKE_CASE"
    )
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass_enum(module = "nautilus_trader.aster")
)]
pub enum AsterMarginType {
    /// Cross margin.
    #[serde(rename = "CROSSED", alias = "cross")]
    Cross,
    /// Isolated margin.
    #[serde(rename = "ISOLATED", alias = "isolated")]
    Isolated,
    /// Unknown or undocumented value.
    #[default]
    #[serde(other)]
    Unknown,
}

/// Working type for trigger price evaluation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AsterWorkingType {
    /// Use the contract price.
    ContractPrice,
    /// Use the mark price.
    MarkPrice,
    /// Unknown or undocumented value.
    #[serde(other)]
    Unknown,
}

/// Order status lifecycle values.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AsterOrderStatus {
    /// Order accepted and working.
    New,
    /// Pending new (order list accepted but not yet on book).
    PendingNew,
    /// Partially filled.
    PartiallyFilled,
    /// Fully filled.
    Filled,
    /// Canceled by user or system.
    Canceled,
    /// Pending cancel (not commonly used).
    PendingCancel,
    /// Rejected by exchange.
    Rejected,
    /// Expired.
    Expired,
    /// Expired in match (IOC/FOK not executed).
    ExpiredInMatch,
    /// Liquidation with insurance fund.
    NewInsurance,
    /// Counterparty liquidation (Auto-Deleveraging).
    NewAdl,
    /// Unknown or undocumented value.
    #[serde(other)]
    Unknown,
}

/// Algo order status lifecycle values (Aster Futures Algo Service).
///
/// These statuses are specific to conditional orders submitted via the
/// `/fapi/v1/algoOrder` endpoint (STOP_MARKET, STOP_LIMIT, TAKE_PROFIT,
/// TAKE_PROFIT_MARKET, TRAILING_STOP_MARKET).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AsterAlgoStatus {
    /// Algo order accepted and waiting for trigger condition.
    New,
    /// Algo order trigger condition met, forwarding to matching engine.
    Triggering,
    /// Algo order successfully placed in matching engine.
    Triggered,
    /// Algo order lifecycle completed (check executed qty for fill status).
    Finished,
    /// Algo order canceled by user.
    Canceled,
    /// Algo order expired (GTD expiration).
    Expired,
    /// Algo order rejected by exchange.
    Rejected,
    /// Unknown or undocumented value.
    #[serde(other)]
    Unknown,
}

/// Algo order type for Aster Futures Algo Service.
///
/// Currently only `Conditional` is supported by Aster.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AsterAlgoType {
    /// Conditional algo order (stop, take-profit, trailing stop).
    #[default]
    Conditional,
    /// Unknown or undocumented value.
    #[serde(other)]
    Unknown,
}

/// Futures order type enumeration.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AsterFuturesOrderType {
    /// Limit order.
    Limit,
    /// Market order.
    Market,
    /// Stop (stop-limit) order.
    Stop,
    /// Stop market order.
    StopMarket,
    /// Take profit (limit) order.
    TakeProfit,
    /// Take profit market order.
    TakeProfitMarket,
    /// Trailing stop market order.
    TrailingStopMarket,
    /// Liquidation order created by exchange.
    Liquidation,
    /// Auto-deleveraging order created by exchange.
    Adl,
    /// Unknown or undocumented value.
    #[serde(other)]
    Unknown,
}

impl From<AsterFuturesOrderType> for OrderType {
    fn from(value: AsterFuturesOrderType) -> Self {
        match value {
            AsterFuturesOrderType::Limit => Self::Limit,
            AsterFuturesOrderType::Market => Self::Market,
            AsterFuturesOrderType::Stop => Self::StopLimit,
            AsterFuturesOrderType::StopMarket => Self::StopMarket,
            AsterFuturesOrderType::TakeProfit => Self::LimitIfTouched,
            AsterFuturesOrderType::TakeProfitMarket => Self::MarketIfTouched,
            AsterFuturesOrderType::TrailingStopMarket => Self::TrailingStopMarket,
            AsterFuturesOrderType::Liquidation
            | AsterFuturesOrderType::Adl
            | AsterFuturesOrderType::Unknown => Self::Market, // Exchange-generated orders
        }
    }
}

/// Time in force options.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AsterTimeInForce {
    /// Good till canceled.
    Gtc,
    /// Immediate or cancel.
    Ioc,
    /// Fill or kill.
    Fok,
    /// Good till crossing (post-only).
    Gtx,
    /// Good till date.
    /// NOTE: Aster does not support GTD. Kept for deserialization compatibility;
    /// the Python config sets `use_gtd=False` to prevent submission.
    Gtd,
    /// Request-for-quote interactive (USD-M Futures).
    Rpi,
    /// Aster-specific: order not visible in order book.
    #[serde(rename = "HIDDEN")]
    Hidden,
    /// Unknown or undocumented value.
    #[serde(other)]
    Unknown,
}

impl TryFrom<TimeInForce> for AsterTimeInForce {
    type Error = anyhow::Error;

    fn try_from(value: TimeInForce) -> Result<Self, Self::Error> {
        match value {
            TimeInForce::Gtc => Ok(Self::Gtc),
            TimeInForce::Ioc => Ok(Self::Ioc),
            TimeInForce::Fok => Ok(Self::Fok),
            TimeInForce::Gtd => Ok(Self::Gtd),
            _ => anyhow::bail!("Unsupported `TimeInForce` for Aster: {value:?}"),
        }
    }
}

/// Income type for account income history.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AsterIncomeType {
    /// Internal transfers.
    Transfer,
    /// Welcome bonus.
    WelcomeBonus,
    /// Realized profit and loss.
    RealizedPnl,
    /// Funding fee payments/receipts.
    FundingFee,
    /// Trading commission.
    Commission,
    /// Commission rebate.
    CommissionRebate,
    /// API rebate.
    ApiRebate,
    /// Insurance clear.
    InsuranceClear,
    /// Referral kickback.
    ReferralKickback,
    /// Contest reward.
    ContestReward,
    /// Cross collateral transfer.
    CrossCollateralTransfer,
    /// Options premium fee.
    OptionsPremiumFee,
    /// Options settle profit.
    OptionsSettleProfit,
    /// Internal transfer.
    InternalTransfer,
    /// Auto exchange.
    AutoExchange,
    /// Delivered settlement.
    #[serde(rename = "DELIVERED_SETTELMENT")]
    DeliveredSettlement,
    /// Coin swap deposit.
    CoinSwapDeposit,
    /// Coin swap withdraw.
    CoinSwapWithdraw,
    /// Position limit increase fee.
    PositionLimitIncreaseFee,
    /// Strategy UM futures transfer.
    StrategyUmfuturesTransfer,
    /// Fee return.
    FeeReturn,
    /// BFUSD reward.
    BfusdReward,
    /// Unknown or undocumented value.
    #[serde(other)]
    Unknown,
}

/// Price match mode for futures maker orders.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AsterPriceMatch {
    /// No price match (default).
    None,
    /// Match opposing side.
    Opponent,
    /// Match opposing side with 5 tick offset.
    #[serde(rename = "OPPONENT_5")]
    Opponent5,
    /// Match opposing side with 10 tick offset.
    #[serde(rename = "OPPONENT_10")]
    Opponent10,
    /// Match opposing side with 20 tick offset.
    #[serde(rename = "OPPONENT_20")]
    Opponent20,
    /// Join current queue on same side.
    Queue,
    /// Join queue with 5 tick offset.
    #[serde(rename = "QUEUE_5")]
    Queue5,
    /// Join queue with 10 tick offset.
    #[serde(rename = "QUEUE_10")]
    Queue10,
    /// Join queue with 20 tick offset.
    #[serde(rename = "QUEUE_20")]
    Queue20,
    /// Unknown or undocumented value.
    #[serde(other)]
    Unknown,
}

impl AsterPriceMatch {
    /// Parses a price match mode from a string param value.
    ///
    /// Accepts uppercase Aster API values like `"OPPONENT"`, `"OPPONENT_5"`, `"QUEUE_10"`.
    ///
    /// # Errors
    ///
    /// Returns an error if the value is not a recognized price match mode.
    pub fn from_param(s: &str) -> anyhow::Result<Self> {
        let value = s.to_uppercase();
        serde_json::from_value(serde_json::Value::String(value))
            .map_err(|_| anyhow::anyhow!("Invalid price_match value: {s:?}"))
            .and_then(|pm: Self| {
                if pm == Self::None || pm == Self::Unknown {
                    anyhow::bail!("Invalid price_match value: {s:?}")
                }
                Ok(pm)
            })
    }
}

/// Self-trade prevention mode.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AsterSelfTradePreventionMode {
    /// No self-trade prevention.
    None,
    /// Expire maker orders on self-trade.
    ExpireMaker,
    /// Expire taker orders on self-trade.
    ExpireTaker,
    /// Expire both sides on self-trade.
    ExpireBoth,
    /// Decrement and cancel (spot).
    Decrement,
    /// Transfer to sub-account (spot).
    Transfer,
    /// Unknown or undocumented value.
    #[serde(other)]
    Unknown,
}

/// Trading status for symbols.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AsterTradingStatus {
    /// Trading is active.
    Trading,
    /// Pending activation.
    PendingTrading,
    /// Pre-trading session.
    PreTrading,
    /// Post-trading session.
    PostTrading,
    /// End of day.
    EndOfDay,
    /// Trading halted.
    Halt,
    /// Auction match.
    AuctionMatch,
    /// Break period.
    Break,
    /// Unknown or undocumented value.
    #[serde(other)]
    Unknown,
}

impl From<AsterTradingStatus> for MarketStatusAction {
    fn from(status: AsterTradingStatus) -> Self {
        match status {
            AsterTradingStatus::Trading => Self::Trading,
            AsterTradingStatus::PendingTrading | AsterTradingStatus::PreTrading => {
                Self::PreOpen
            }
            AsterTradingStatus::PostTrading => Self::PostClose,
            AsterTradingStatus::EndOfDay => Self::Close,
            AsterTradingStatus::Halt => Self::Halt,
            AsterTradingStatus::AuctionMatch => Self::Cross,
            AsterTradingStatus::Break => Self::Pause,
            AsterTradingStatus::Unknown => Self::NotAvailableForTrading,
        }
    }
}

/// Contract status for coin-margined futures.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AsterContractStatus {
    /// Trading is active.
    Trading,
    /// Pending trading.
    PendingTrading,
    /// Pre-delivering.
    PreDelivering,
    /// Delivering.
    Delivering,
    /// Delivered.
    Delivered,
    /// Pre-settle.
    PreSettle,
    /// Settling.
    Settling,
    /// Closed.
    Close,
    /// Pre-delist.
    PreDelisting,
    /// Delisting in progress.
    Delisting,
    /// Contract down.
    Down,
    /// Unknown or undocumented value.
    #[serde(other)]
    Unknown,
}

impl From<AsterContractStatus> for MarketStatusAction {
    fn from(status: AsterContractStatus) -> Self {
        match status {
            AsterContractStatus::Trading => Self::Trading,
            AsterContractStatus::PendingTrading => Self::PreOpen,
            AsterContractStatus::PreDelivering
            | AsterContractStatus::PreDelisting
            | AsterContractStatus::PreSettle => Self::PreClose,
            AsterContractStatus::Delivering
            | AsterContractStatus::Delivered
            | AsterContractStatus::Settling
            | AsterContractStatus::Close => Self::Close,
            AsterContractStatus::Delisting => Self::Suspend,
            AsterContractStatus::Down | AsterContractStatus::Unknown => {
                Self::NotAvailableForTrading
            }
        }
    }
}

/// WebSocket stream event types.
///
/// These are the "e" field values in WebSocket JSON messages.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AsterWsEventType {
    /// Aggregate trade event.
    AggTrade,
    /// Individual trade event.
    Trade,
    /// Book ticker (best bid/ask) event.
    BookTicker,
    /// Depth update (order book delta) event.
    DepthUpdate,
    /// Mark price update event.
    MarkPriceUpdate,
    /// Kline/candlestick event.
    Kline,
    /// Forced liquidation order event.
    ForceOrder,
    /// 24-hour rolling ticker event.
    #[serde(rename = "24hrTicker")]
    Ticker24Hr,
    /// 24-hour rolling mini ticker event.
    #[serde(rename = "24hrMiniTicker")]
    MiniTicker24Hr,

    // User data stream events
    /// Account update (balance and position changes).
    #[serde(rename = "ACCOUNT_UPDATE")]
    AccountUpdate,
    /// Order/trade update event.
    #[serde(rename = "ORDER_TRADE_UPDATE")]
    OrderTradeUpdate,
    /// Algo order update event (Aster Futures Algo Service).
    #[serde(rename = "ALGO_UPDATE")]
    AlgoUpdate,
    /// Margin call warning event.
    #[serde(rename = "MARGIN_CALL")]
    MarginCall,
    /// Account configuration update (leverage change).
    #[serde(rename = "ACCOUNT_CONFIG_UPDATE")]
    AccountConfigUpdate,
    /// Listen key expired event.
    #[serde(rename = "listenKeyExpired")]
    ListenKeyExpired,

    /// Unknown or undocumented event type.
    #[serde(other)]
    Unknown,
}

impl AsterWsEventType {
    /// Returns the wire format string for this event type.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AggTrade => "aggTrade",
            Self::Trade => "trade",
            Self::BookTicker => "bookTicker",
            Self::DepthUpdate => "depthUpdate",
            Self::MarkPriceUpdate => "markPriceUpdate",
            Self::Kline => "kline",
            Self::ForceOrder => "forceOrder",
            Self::Ticker24Hr => "24hrTicker",
            Self::MiniTicker24Hr => "24hrMiniTicker",
            Self::AccountUpdate => "ACCOUNT_UPDATE",
            Self::OrderTradeUpdate => "ORDER_TRADE_UPDATE",
            Self::AlgoUpdate => "ALGO_UPDATE",
            Self::MarginCall => "MARGIN_CALL",
            Self::AccountConfigUpdate => "ACCOUNT_CONFIG_UPDATE",
            Self::ListenKeyExpired => "listenKeyExpired",
            Self::Unknown => "unknown",
        }
    }
}

impl Display for AsterWsEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// WebSocket request method (operation type).
///
/// Used for subscription requests on both Spot and Futures WebSocket APIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AsterWsMethod {
    /// Subscribe to streams.
    Subscribe,
    /// Unsubscribe from streams.
    Unsubscribe,
}

/// Filter type identifiers returned in exchange info.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AsterFilterType {
    /// Price filter.
    PriceFilter,
    /// Percent price filter.
    PercentPrice,
    /// Percent price by side filter (spot).
    PercentPriceBySide,
    /// Lot size filter.
    LotSize,
    /// Market lot size filter.
    MarketLotSize,
    /// Notional filter (spot).
    Notional,
    /// Min notional filter (futures).
    MinNotional,
    /// Iceberg parts filter (spot).
    IcebergParts,
    /// Maximum number of orders filter.
    MaxNumOrders,
    /// Maximum number of algo orders filter.
    MaxNumAlgoOrders,
    /// Maximum number of iceberg orders filter (spot).
    MaxNumIcebergOrders,
    /// Maximum position filter (spot).
    MaxPosition,
    /// Trailing delta filter (spot).
    TrailingDelta,
    /// Maximum number of order amends filter (spot).
    MaxNumOrderAmends,
    /// Maximum number of order lists filter (spot).
    MaxNumOrderLists,
    /// Maximum asset filter (spot).
    MaxAsset,
    /// Exchange-level maximum number of orders.
    ExchangeMaxNumOrders,
    /// Exchange-level maximum number of algo orders.
    ExchangeMaxNumAlgoOrders,
    /// Exchange-level maximum number of iceberg orders.
    ExchangeMaxNumIcebergOrders,
    /// Exchange-level maximum number of order lists.
    ExchangeMaxNumOrderLists,
    /// T+1 sell restriction filter (spot).
    TPlusSell,
    /// Unknown or undocumented value.
    #[serde(other)]
    Unknown,
}

impl Display for AsterEnvironment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mainnet => write!(f, "Mainnet"),
            Self::Testnet => write!(f, "Testnet"),
            Self::Demo => write!(f, "Demo"),
        }
    }
}

/// Rate limit type for API request quotas.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AsterRateLimitType {
    /// Weighted request limit.
    RequestWeight,
    /// Order placement limit.
    Orders,
    /// Raw request count limit (spot).
    RawRequests,
    /// Unknown or undocumented value.
    #[serde(other)]
    Unknown,
}

/// Rate limit time interval.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AsterRateLimitInterval {
    /// One second interval.
    Second,
    /// One minute interval.
    Minute,
    /// One day interval.
    Day,
    /// Unknown or undocumented value.
    #[serde(other)]
    Unknown,
}

/// Kline (candlestick) interval.
///
/// # References
/// - <https://developers.asterdex.com/docs/aster-spot-api-docs/rest-api/market-data-endpoints>
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AsterKlineInterval {
    /// 1 second (only for spot).
    #[serde(rename = "1s")]
    Second1,
    /// 1 minute.
    #[default]
    #[serde(rename = "1m")]
    Minute1,
    /// 3 minutes.
    #[serde(rename = "3m")]
    Minute3,
    /// 5 minutes.
    #[serde(rename = "5m")]
    Minute5,
    /// 15 minutes.
    #[serde(rename = "15m")]
    Minute15,
    /// 30 minutes.
    #[serde(rename = "30m")]
    Minute30,
    /// 1 hour.
    #[serde(rename = "1h")]
    Hour1,
    /// 2 hours.
    #[serde(rename = "2h")]
    Hour2,
    /// 4 hours.
    #[serde(rename = "4h")]
    Hour4,
    /// 6 hours.
    #[serde(rename = "6h")]
    Hour6,
    /// 8 hours.
    #[serde(rename = "8h")]
    Hour8,
    /// 12 hours.
    #[serde(rename = "12h")]
    Hour12,
    /// 1 day.
    #[serde(rename = "1d")]
    Day1,
    /// 3 days.
    #[serde(rename = "3d")]
    Day3,
    /// 1 week.
    #[serde(rename = "1w")]
    Week1,
    /// 1 month.
    #[serde(rename = "1M")]
    Month1,
}

impl AsterKlineInterval {
    /// Returns the string representation used by Aster API.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Second1 => "1s",
            Self::Minute1 => "1m",
            Self::Minute3 => "3m",
            Self::Minute5 => "5m",
            Self::Minute15 => "15m",
            Self::Minute30 => "30m",
            Self::Hour1 => "1h",
            Self::Hour2 => "2h",
            Self::Hour4 => "4h",
            Self::Hour6 => "6h",
            Self::Hour8 => "8h",
            Self::Hour12 => "12h",
            Self::Day1 => "1d",
            Self::Day3 => "3d",
            Self::Week1 => "1w",
            Self::Month1 => "1M",
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use serde_json::json;

    use super::*;

    #[rstest]
    fn test_product_type_as_str() {
        assert_eq!(AsterProductType::Spot.as_str(), "SPOT");
        assert_eq!(AsterProductType::Margin.as_str(), "MARGIN");
        assert_eq!(AsterProductType::UsdM.as_str(), "USD_M");
        assert_eq!(AsterProductType::CoinM.as_str(), "COIN_M");
        assert_eq!(AsterProductType::Options.as_str(), "OPTIONS");
    }

    #[rstest]
    fn test_product_type_suffix() {
        assert_eq!(AsterProductType::Spot.suffix(), "-SPOT");
        assert_eq!(AsterProductType::Margin.suffix(), "-MARGIN");
        assert_eq!(AsterProductType::UsdM.suffix(), "-LINEAR");
        assert_eq!(AsterProductType::CoinM.suffix(), "-INVERSE");
        assert_eq!(AsterProductType::Options.suffix(), "-OPTION");
    }

    #[rstest]
    fn test_product_type_predicates() {
        assert!(AsterProductType::Spot.is_spot());
        assert!(AsterProductType::Margin.is_spot());
        assert!(!AsterProductType::UsdM.is_spot());

        assert!(AsterProductType::UsdM.is_futures());
        assert!(AsterProductType::CoinM.is_futures());
        assert!(!AsterProductType::Spot.is_futures());

        assert!(AsterProductType::CoinM.is_inverse());
        assert!(!AsterProductType::UsdM.is_inverse());

        assert!(AsterProductType::Options.is_options());
        assert!(!AsterProductType::Spot.is_options());
    }

    #[rstest]
    #[case("\"REQUEST_WEIGHT\"", AsterRateLimitType::RequestWeight)]
    #[case("\"ORDERS\"", AsterRateLimitType::Orders)]
    #[case("\"RAW_REQUESTS\"", AsterRateLimitType::RawRequests)]
    #[case("\"UNDOCUMENTED\"", AsterRateLimitType::Unknown)]
    fn test_rate_limit_type_deserializes(
        #[case] raw: &str,
        #[case] expected: AsterRateLimitType,
    ) {
        let value: AsterRateLimitType = serde_json::from_str(raw).unwrap();
        assert_eq!(value, expected);
    }

    #[rstest]
    #[case("\"SECOND\"", AsterRateLimitInterval::Second)]
    #[case("\"MINUTE\"", AsterRateLimitInterval::Minute)]
    #[case("\"DAY\"", AsterRateLimitInterval::Day)]
    #[case("\"WEEK\"", AsterRateLimitInterval::Unknown)]
    fn test_rate_limit_interval_deserializes(
        #[case] raw: &str,
        #[case] expected: AsterRateLimitInterval,
    ) {
        let value: AsterRateLimitInterval = serde_json::from_str(raw).unwrap();
        assert_eq!(value, expected);
    }

    #[rstest]
    #[case(AsterMarginType::Cross, "CROSSED", "cross")]
    #[case(AsterMarginType::Isolated, "ISOLATED", "isolated")]
    fn test_margin_type_serde_roundtrip(
        #[case] variant: AsterMarginType,
        #[case] post_format: &str,
        #[case] get_format: &str,
    ) {
        let serialized = serde_json::to_value(variant).unwrap();
        assert_eq!(serialized, json!(post_format));

        let from_post: AsterMarginType =
            serde_json::from_str(&format!("\"{post_format}\"")).unwrap();
        assert_eq!(from_post, variant);

        let from_get: AsterMarginType =
            serde_json::from_str(&format!("\"{get_format}\"")).unwrap();
        assert_eq!(from_get, variant);
    }

    #[rstest]
    fn test_margin_type_unknown_fallback() {
        let value: AsterMarginType = serde_json::from_str("\"SOMETHING_NEW\"").unwrap();
        assert_eq!(value, AsterMarginType::Unknown);
    }

    #[rstest]
    fn test_rate_limit_enums_serialize_to_aster_strings() {
        assert_eq!(
            serde_json::to_value(AsterRateLimitType::RequestWeight).unwrap(),
            json!("REQUEST_WEIGHT")
        );
        assert_eq!(
            serde_json::to_value(AsterRateLimitInterval::Minute).unwrap(),
            json!("MINUTE")
        );
    }

    #[rstest]
    #[case("\"NONE\"", AsterPriceMatch::None)]
    #[case("\"OPPONENT\"", AsterPriceMatch::Opponent)]
    #[case("\"OPPONENT_5\"", AsterPriceMatch::Opponent5)]
    #[case("\"OPPONENT_10\"", AsterPriceMatch::Opponent10)]
    #[case("\"OPPONENT_20\"", AsterPriceMatch::Opponent20)]
    #[case("\"QUEUE\"", AsterPriceMatch::Queue)]
    #[case("\"QUEUE_5\"", AsterPriceMatch::Queue5)]
    #[case("\"QUEUE_10\"", AsterPriceMatch::Queue10)]
    #[case("\"QUEUE_20\"", AsterPriceMatch::Queue20)]
    #[case("\"SOMETHING_NEW\"", AsterPriceMatch::Unknown)]
    fn test_price_match_deserializes(#[case] raw: &str, #[case] expected: AsterPriceMatch) {
        let value: AsterPriceMatch = serde_json::from_str(raw).unwrap();
        assert_eq!(value, expected);
    }

    #[rstest]
    #[case(AsterPriceMatch::None, "NONE")]
    #[case(AsterPriceMatch::Opponent, "OPPONENT")]
    #[case(AsterPriceMatch::Opponent5, "OPPONENT_5")]
    #[case(AsterPriceMatch::Opponent10, "OPPONENT_10")]
    #[case(AsterPriceMatch::Opponent20, "OPPONENT_20")]
    #[case(AsterPriceMatch::Queue, "QUEUE")]
    #[case(AsterPriceMatch::Queue5, "QUEUE_5")]
    #[case(AsterPriceMatch::Queue10, "QUEUE_10")]
    #[case(AsterPriceMatch::Queue20, "QUEUE_20")]
    fn test_price_match_serializes(#[case] variant: AsterPriceMatch, #[case] expected: &str) {
        let serialized = serde_json::to_value(variant).unwrap();
        assert_eq!(serialized, json!(expected));
    }

    #[rstest]
    #[case("OPPONENT", AsterPriceMatch::Opponent)]
    #[case("opponent", AsterPriceMatch::Opponent)]
    #[case("OPPONENT_5", AsterPriceMatch::Opponent5)]
    #[case("opponent_5", AsterPriceMatch::Opponent5)]
    #[case("QUEUE_20", AsterPriceMatch::Queue20)]
    #[case("queue_20", AsterPriceMatch::Queue20)]
    fn test_price_match_from_param_valid(#[case] input: &str, #[case] expected: AsterPriceMatch) {
        let result = AsterPriceMatch::from_param(input).unwrap();
        assert_eq!(result, expected);
    }

    #[rstest]
    #[case("NONE")]
    #[case("invalid")]
    #[case("")]
    fn test_price_match_from_param_invalid(#[case] input: &str) {
        assert!(AsterPriceMatch::from_param(input).is_err());
    }
}
