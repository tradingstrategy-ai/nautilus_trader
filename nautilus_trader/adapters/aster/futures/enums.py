# -------------------------------------------------------------------------------------------------
#  Copyright (C) 2015-2026 Nautech Systems Pty Ltd. All rights reserved.
#  https://nautechsystems.io
#
#  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
#  You may not use this file except in compliance with the License.
#  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.
# -------------------------------------------------------------------------------------------------
"""
Defines Aster Futures specific enums.

References
----------
https://docs.asterdex.com/futures/en/#public-endpoints-info

"""

from decimal import Decimal
from enum import Enum
from enum import unique

from nautilus_trader.adapters.aster.common.enums import AsterEnumParser
from nautilus_trader.adapters.aster.common.enums import AsterOrderType
from nautilus_trader.model.enums import OrderType
from nautilus_trader.model.enums import PositionSide
from nautilus_trader.model.enums import TimeInForce
from nautilus_trader.model.enums import TriggerType
from nautilus_trader.model.orders import Order


@unique
class AsterFuturesContractType(Enum):
    """
    Represents a Aster Futures derivatives contract type.
    """

    PERPETUAL = "PERPETUAL"
    CURRENT_MONTH = "CURRENT_MONTH"
    NEXT_MONTH = "NEXT_MONTH"
    CURRENT_QUARTER = "CURRENT_QUARTER"
    NEXT_QUARTER = "NEXT_QUARTER"
    PERPETUAL_DELIVERING = "PERPETUAL_DELIVERING"
    CURRENT_QUARTER_DELIVERING = "CURRENT_QUARTER DELIVERING"  # Underscore omission intentional
    TRADIFI_PERPETUAL = "TRADIFI_PERPETUAL"  # TradFi-Perps (stock/commodity perpetuals)


@unique
class AsterFuturesContractStatus(Enum):
    """
    Represents a Aster Futures contract status.
    """

    PENDING_TRADING = "PENDING_TRADING"
    TRADING = "TRADING"
    PRE_DELIVERING = "PRE_DELIVERING"
    DELIVERING = "DELIVERING"
    DELIVERED = "DELIVERED"
    PRE_SETTLE = "PRE_SETTLE"
    SETTLING = "SETTLING"
    CLOSE = "CLOSE"


@unique
class AsterFuturesWorkingType(Enum):
    """
    Represents a Aster Futures working type.
    """

    MARK_PRICE = "MARK_PRICE"
    CONTRACT_PRICE = "CONTRACT_PRICE"


@unique
class AsterFuturesMarginType(Enum):
    """
    Represents a Aster Futures margin type.
    """

    ISOLATED = "ISOLATED"
    CROSS = "CROSSED"


@unique
class AsterFuturesPositionUpdateReason(Enum):
    """
    Represents a Aster Futures position and balance update reason.
    """

    DEPOSIT = "DEPOSIT"
    WITHDRAW = "WITHDRAW"
    ORDER = "ORDER"
    FUNDING_FEE = "FUNDING_FEE"
    WITHDRAW_REJECT = "WITHDRAW_REJECT"
    ADJUSTMENT = "ADJUSTMENT"
    INSURANCE_CLEAR = "INSURANCE_CLEAR"
    ADMIN_DEPOSIT = "ADMIN_DEPOSIT"
    ADMIN_WITHDRAW = "ADMIN_WITHDRAW"
    MARGIN_TRANSFER = "MARGIN_TRANSFER"
    MARGIN_TYPE_CHANGE = "MARGIN_TYPE_CHANGE"
    ASSET_TRANSFER = "ASSET_TRANSFER"
    OPTIONS_PREMIUM_FEE = "OPTIONS_PREMIUM_FEE"
    OPTIONS_SETTLE_PROFIT = "OPTIONS_SETTLE_PROFIT"
    AUTO_EXCHANGE = "AUTO_EXCHANGE"
    COIN_SWAP_DEPOSIT = "COIN_SWAP_DEPOSIT"
    COIN_SWAP_WITHDRAW = "COIN_SWAP_WITHDRAW"
    ADL = "ADL"


@unique
class AsterFuturesEventType(Enum):
    """
    Represents a Aster Futures event type.
    """

    LISTEN_KEY_EXPIRED = "listenKeyExpired"
    MARGIN_CALL = "MARGIN_CALL"
    ACCOUNT_UPDATE = "ACCOUNT_UPDATE"
    ORDER_TRADE_UPDATE = "ORDER_TRADE_UPDATE"
    ACCOUNT_CONFIG_UPDATE = "ACCOUNT_CONFIG_UPDATE"
    TRADE_LITE = "TRADE_LITE"
    STRATEGY_UPDATE = "STRATEGY_UPDATE"
    GRID_UPDATE = "GRID_UPDATE"
    CONDITIONAL_ORDER_TRIGGER_REJECT = "CONDITIONAL_ORDER_TRIGGER_REJECT"
    ALGO_UPDATE = "ALGO_UPDATE"


class AsterFuturesEnumParser(AsterEnumParser):
    """
    Provides parsing methods for enums used by the 'Aster Futures' exchange.
    """

    def __init__(self) -> None:
        super().__init__()

        self.futures_ext_to_int_order_type = {
            AsterOrderType.LIMIT: OrderType.LIMIT,
            AsterOrderType.MARKET: OrderType.MARKET,
            AsterOrderType.STOP: OrderType.STOP_LIMIT,
            AsterOrderType.STOP_MARKET: OrderType.STOP_MARKET,
            AsterOrderType.TAKE_PROFIT: OrderType.LIMIT_IF_TOUCHED,
            AsterOrderType.TAKE_PROFIT_MARKET: OrderType.MARKET_IF_TOUCHED,
            AsterOrderType.TRAILING_STOP_MARKET: OrderType.TRAILING_STOP_MARKET,
            AsterOrderType.LIQUIDATION: OrderType.MARKET,
            AsterOrderType.ADL: OrderType.MARKET,
        }
        # Exclude exchange-generated order types (LIQUIDATION, ADL) from outbound mapping
        self.futures_int_to_ext_order_type = {
            b: a
            for a, b in self.futures_ext_to_int_order_type.items()
            if a not in (AsterOrderType.LIQUIDATION, AsterOrderType.ADL)
        }

        self.futures_valid_time_in_force = {
            TimeInForce.GTC,
            TimeInForce.GTD,
            TimeInForce.FOK,
            TimeInForce.IOC,
        }

        self.futures_valid_order_types = {
            OrderType.MARKET,
            OrderType.LIMIT,
            OrderType.STOP_MARKET,
            OrderType.STOP_LIMIT,
            OrderType.MARKET_IF_TOUCHED,
            OrderType.LIMIT_IF_TOUCHED,
            OrderType.TRAILING_STOP_MARKET,
        }

    def parse_aster_order_type(self, order_type: AsterOrderType) -> OrderType:
        try:
            return self.futures_ext_to_int_order_type[order_type]
        except KeyError:
            raise RuntimeError(  # pragma: no cover (design-time error)
                f"unrecognized Aster Futures order type, was {order_type}",  # pragma: no cover
            )

    def parse_internal_order_type(self, order: Order) -> AsterOrderType:
        try:
            return self.futures_int_to_ext_order_type[order.order_type]
        except KeyError:
            raise RuntimeError(  # pragma: no cover (design-time error)
                f"unrecognized or unsupported internal order type, was {order.order_type}",  # pragma: no cover
            )

    def parse_aster_trigger_type(self, trigger_type: str) -> TriggerType:
        if trigger_type == AsterFuturesWorkingType.CONTRACT_PRICE.value:
            return TriggerType.LAST_PRICE
        elif trigger_type == AsterFuturesWorkingType.MARK_PRICE.value:
            return TriggerType.MARK_PRICE
        else:
            return None

    def parse_futures_position_side(
        self,
        net_size: Decimal,
    ) -> PositionSide:
        if net_size > 0:
            return PositionSide.LONG
        elif net_size < 0:
            return PositionSide.SHORT
        else:
            return PositionSide.FLAT
