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
Aster cryptocurreny exchange integration adapter.

This subpackage provides an instrument provider, data and execution clients,
configurations, data types and constants for connecting to and interacting with
Aster's API.

For convenience, the most commonly used symbols are re-exported at the
subpackage's top level, so downstream code can simply import from
``nautilus_trader.adapters.aster``.

"""

from typing import Final

import pyarrow as pa

from nautilus_trader.adapters.aster.common.constants import ASTER
from nautilus_trader.adapters.aster.common.constants import ASTER_CLIENT_ID
from nautilus_trader.adapters.aster.common.constants import ASTER_VENUE
from nautilus_trader.adapters.aster.common.enums import AsterAccountType
from nautilus_trader.adapters.aster.common.enums import AsterKeyType
from nautilus_trader.adapters.aster.common.types import AsterBar
from nautilus_trader.adapters.aster.common.types import AsterTicker
from nautilus_trader.adapters.aster.config import AsterDataClientConfig
from nautilus_trader.adapters.aster.config import AsterExecClientConfig
from nautilus_trader.adapters.aster.config import AsterInstrumentProviderConfig
from nautilus_trader.adapters.aster.factories import AsterLiveDataClientFactory
from nautilus_trader.adapters.aster.factories import AsterLiveExecClientFactory
from nautilus_trader.adapters.aster.factories import get_cached_aster_http_client
from nautilus_trader.adapters.aster.futures.providers import AsterFuturesInstrumentProvider
from nautilus_trader.adapters.aster.futures.types import AsterFuturesMarkPriceUpdate
from nautilus_trader.adapters.aster.loaders import AsterOrderBookDeltaDataLoader
from nautilus_trader.adapters.aster.spot.providers import AsterSpotInstrumentProvider
from nautilus_trader.core import nautilus_pyo3
from nautilus_trader.serialization import register_serializable_type
from nautilus_trader.serialization.arrow.schema import NAUTILUS_ARROW_SCHEMA
from nautilus_trader.serialization.arrow.serializer import make_dict_deserializer
from nautilus_trader.serialization.arrow.serializer import make_dict_serializer
from nautilus_trader.serialization.arrow.serializer import register_arrow
from nautilus_trader.serialization.arrow.serializer import register_rust_custom_serializer


register_serializable_type(
    AsterBar,
    AsterBar.to_dict,
    AsterBar.from_dict,
)

register_serializable_type(
    AsterTicker,
    AsterTicker.to_dict,
    AsterTicker.from_dict,
)


_aster_mod = nautilus_pyo3.aster  # type: ignore[attr-defined]


def _convert_aster_bar_to_pyo3(bar: AsterBar) -> object:
    return _aster_mod.AsterBar.from_dict(AsterBar.to_dict(bar))


register_rust_custom_serializer(
    "AsterBar",
    _aster_mod.aster_bar_to_arrow_record_batch_bytes,
    _convert_aster_bar_to_pyo3,
    data_cls=AsterBar,
)


ASTER_FUTURES_MARK_PRICE_UPDATE_ARROW_SCHEMA: Final[pa.schema] = pa.schema(
    {
        "instrument_id": pa.dictionary(pa.int64(), pa.string()),
        "mark": pa.string(),
        "index": pa.string(),
        "estimated_settle": pa.string(),
        "funding_rate": pa.string(),
        "next_funding_ns": pa.uint64(),
        "ts_event": pa.uint64(),
        "ts_init": pa.uint64(),
    },
)

NAUTILUS_ARROW_SCHEMA[AsterFuturesMarkPriceUpdate] = (
    ASTER_FUTURES_MARK_PRICE_UPDATE_ARROW_SCHEMA
)

register_arrow(
    AsterFuturesMarkPriceUpdate,
    ASTER_FUTURES_MARK_PRICE_UPDATE_ARROW_SCHEMA,
    encoder=make_dict_serializer(ASTER_FUTURES_MARK_PRICE_UPDATE_ARROW_SCHEMA),
    decoder=make_dict_deserializer(AsterFuturesMarkPriceUpdate),
)

decode_aster_spot_client_order_id = nautilus_pyo3.aster.decode_aster_spot_client_order_id  # type: ignore[attr-defined]
decode_aster_futures_client_order_id = (
    nautilus_pyo3.aster.decode_aster_futures_client_order_id  # type: ignore[attr-defined]
)

__all__ = [
    "ASTER",
    "ASTER_CLIENT_ID",
    "ASTER_VENUE",
    "AsterAccountType",
    "AsterDataClientConfig",
    "AsterExecClientConfig",
    "AsterFuturesInstrumentProvider",
    "AsterFuturesMarkPriceUpdate",
    "AsterInstrumentProviderConfig",
    "AsterKeyType",
    "AsterLiveDataClientFactory",
    "AsterLiveExecClientFactory",
    "AsterOrderBookDeltaDataLoader",
    "AsterSpotInstrumentProvider",
    "decode_aster_futures_client_order_id",
    "decode_aster_spot_client_order_id",
    "get_cached_aster_http_client",
]
