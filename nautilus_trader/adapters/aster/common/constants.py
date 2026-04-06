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

from decimal import Decimal
from typing import Final

from nautilus_trader.adapters.aster.common.enums import AsterErrorCode
from nautilus_trader.model.enums import OrderType
from nautilus_trader.model.identifiers import ClientId
from nautilus_trader.model.identifiers import Venue


ASTER: Final[str] = "ASTER"

ASTER_VENUE: Final[Venue] = Venue(ASTER)
ASTER_CLIENT_ID: Final[ClientId] = ClientId(ASTER)

ASTER_MIN_CALLBACK_RATE: Final[Decimal] = Decimal("0.1")
ASTER_MAX_CALLBACK_RATE: Final[Decimal] = Decimal("10.0")

# Aster Spot LIMIT_MAKER rejection message (error code -2010).
# This message is specific to post-only (LIMIT_MAKER) orders that would match immediately.
ASTER_POST_ONLY_REJECT_MSG: Final[str] = "Order would immediately match and take."

# Set of Aster error codes for which Nautilus will attempt retries,
# potentially temporary conditions where a retry might make sense.
ASTER_RETRY_ERRORS: set[AsterErrorCode] = {
    AsterErrorCode.DISCONNECTED,
    AsterErrorCode.TOO_MANY_REQUESTS,  # Short retry delays may result in bans
    AsterErrorCode.TIMEOUT,
    AsterErrorCode.SERVER_BUSY,
    AsterErrorCode.INVALID_TIMESTAMP,
    AsterErrorCode.CANCEL_REJECTED,
    AsterErrorCode.ME_RECVWINDOW_REJECT,
}

# Set of Aster error codes for which Nautilus will log a warning on failure, rather than an error
ASTER_RETRY_WARNINGS: set[AsterErrorCode] = {
    AsterErrorCode.FOK_ORDER_REJECT,
    AsterErrorCode.GTX_ORDER_REJECT,
    AsterErrorCode.ORDER_WOULD_IMMEDIATELY_TRIGGER,
}

# Valid `priceMatch` argument values for Aster Futures order placement.
ASTER_PRICE_MATCH_VALUES: Final[frozenset[str]] = frozenset(
    {
        "OPPONENT",
        "OPPONENT_5",
        "OPPONENT_10",
        "OPPONENT_20",
        "QUEUE",
        "QUEUE_5",
        "QUEUE_10",
        "QUEUE_20",
    },
)

ASTER_PRICE_MATCH_ORDER_TYPES: Final[frozenset[OrderType]] = frozenset(
    {
        OrderType.LIMIT,
        OrderType.STOP_LIMIT,
        OrderType.LIMIT_IF_TOUCHED,
    },
)

# Conditional order types that require the Algo Order API for Aster Futures (as of 2025-12-09)
ASTER_FUTURES_ALGO_ORDER_TYPES: Final[frozenset[OrderType]] = frozenset(
    {
        OrderType.STOP_MARKET,
        OrderType.STOP_LIMIT,
        OrderType.MARKET_IF_TOUCHED,
        OrderType.LIMIT_IF_TOUCHED,
        OrderType.TRAILING_STOP_MARKET,
    },
)
