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
from nautilus_trader.adapters.aster.futures.schemas.market import AsterFuturesExchangeInfo
from nautilus_trader.adapters.aster.http.client import AsterHttpClient
from nautilus_trader.adapters.aster.http.endpoint import AsterHttpEndpoint
from nautilus_trader.adapters.aster.http.market import AsterMarketHttpAPI
from nautilus_trader.core.nautilus_pyo3 import HttpMethod


class AsterFuturesExchangeInfoHttp(AsterHttpEndpoint):
    """
    Endpoint of FUTURES exchange trading rules and symbol information.

    `GET /fapi/v1/exchangeInfo`
    `GET /dapi/v1/exchangeInfo`

    References
    ----------
    https://docs.asterdex.com/futures/en/#exchange-information
    https://docs.asterdex.com/delivery/en/#exchange-information

    """

    def __init__(
        self,
        client: AsterHttpClient,
        base_endpoint: str,
    ):
        methods = {
            HttpMethod.GET: AsterSecurityType.NONE,
        }
        url_path = base_endpoint + "exchangeInfo"
        super().__init__(
            client,
            methods,
            url_path,
        )
        self._get_resp_decoder = msgspec.json.Decoder(AsterFuturesExchangeInfo)

    async def get(self) -> AsterFuturesExchangeInfo:
        method_type = HttpMethod.GET
        raw = await self._method(method_type, None)
        return self._get_resp_decoder.decode(raw)


class AsterFuturesMarketHttpAPI(AsterMarketHttpAPI):
    """
    Provides access to the Aster Futures HTTP REST API.

    Parameters
    ----------
    client : AsterHttpClient
        The Aster REST API client.
    account_type : AsterAccountType
        The Aster account type, used to select the endpoint.

    """

    def __init__(
        self,
        client: AsterHttpClient,
        account_type: AsterAccountType = AsterAccountType.USDT_FUTURES,
    ):
        super().__init__(
            client=client,
            account_type=account_type,
        )

        if not account_type.is_futures:
            raise RuntimeError(  # pragma: no cover (design-time error)
                f"`AsterAccountType` not USDT_FUTURES or COIN_FUTURES, was {account_type}",  # pragma: no cover
            )

        self._endpoint_futures_exchange_info = AsterFuturesExchangeInfoHttp(
            client,
            self.base_endpoint,
        )

    async def query_futures_exchange_info(self) -> AsterFuturesExchangeInfo:
        """
        Retrieve Aster Futures exchange information.
        """
        return await self._endpoint_futures_exchange_info.get()
