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

import msgspec

from nautilus_trader.adapters.aster.common.enums import AsterAccountType
from nautilus_trader.adapters.aster.common.enums import AsterSecurityType
from nautilus_trader.adapters.aster.common.symbol import AsterSymbol
from nautilus_trader.adapters.aster.futures.schemas.wallet import AsterFuturesCommissionRate
from nautilus_trader.adapters.aster.http.client import AsterHttpClient
from nautilus_trader.adapters.aster.http.endpoint import AsterHttpEndpoint
from nautilus_trader.common.component import LiveClock
from nautilus_trader.core.nautilus_pyo3 import HttpMethod


class AsterFuturesCommissionRateHttp(AsterHttpEndpoint):
    """
    Endpoint of maker/taker commission rate information.

    `GET /fapi/v1/commissionRate`
    `GET /dapi/v1/commissionRate`

    References
    ----------
    https://docs.asterdex.com/futures/en/#user-commission-rate-user_data
    https://docs.asterdex.com/delivery/en/#user-commission-rate-user_data

    """

    def __init__(
        self,
        client: AsterHttpClient,
        base_endpoint: str,
    ):
        methods = {
            HttpMethod.GET: AsterSecurityType.USER_DATA,
        }
        super().__init__(
            client,
            methods,
            base_endpoint + "commissionRate",
        )
        self._get_resp_decoder = msgspec.json.Decoder(AsterFuturesCommissionRate)

    class GetParameters(msgspec.Struct, omit_defaults=True, frozen=True):
        """
        GET parameters for fetching commission rate.

        Parameters
        ----------
        symbol : AsterSymbol
            Receive commission rate of the provided symbol.
        timestamp : str
            Millisecond timestamp of the request.
        recvWindow : str, optional
            The number of milliseconds after timestamp the request is valid.

        """

        timestamp: str
        symbol: AsterSymbol
        recvWindow: str | None = None

    async def get(self, params: GetParameters) -> AsterFuturesCommissionRate:
        method_type = HttpMethod.GET
        raw = await self._method(method_type, params)
        return self._get_resp_decoder.decode(raw)


class AsterFuturesWalletHttpAPI:
    """
    Provides access to the Aster Futures Wallet HTTP REST API.

    Parameters
    ----------
    client : AsterHttpClient
        The Aster REST API client.

    """

    def __init__(
        self,
        client: AsterHttpClient,
        clock: LiveClock,
        account_type: AsterAccountType = AsterAccountType.USDT_FUTURES,
    ):
        self.client = client
        self._clock = clock

        if account_type == AsterAccountType.USDT_FUTURES:
            self.base_endpoint = "/fapi/v1/"
        elif account_type == AsterAccountType.COIN_FUTURES:
            self.base_endpoint = "/dapi/v1/"

        if not account_type.is_futures:
            raise RuntimeError(  # pragma: no cover (design-time error)
                f"`AsterAccountType` not USDT_FUTURES or COIN_FUTURES, was {account_type}",  # pragma: no cover
            )

        self._endpoint_futures_commission_rate = AsterFuturesCommissionRateHttp(
            client,
            self.base_endpoint,
        )

    def _timestamp(self) -> str:
        """
        Create Aster timestamp from internal clock.
        """
        return str(self._clock.timestamp_ms())

    async def query_futures_commission_rate(
        self,
        symbol: str,
        recv_window: str | None = None,
    ) -> AsterFuturesCommissionRate:
        """
        Get Futures commission rates for a given symbol.
        """
        rate = await self._endpoint_futures_commission_rate.get(
            params=self._endpoint_futures_commission_rate.GetParameters(
                timestamp=self._timestamp(),
                symbol=AsterSymbol(symbol),
                recvWindow=recv_window,
            ),
        )
        return rate
