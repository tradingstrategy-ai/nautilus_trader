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

from typing import Any

import msgspec

from nautilus_trader.adapters.aster.common.enums import AsterAccountType
from nautilus_trader.adapters.aster.common.enums import AsterFuturesPositionSide
from nautilus_trader.adapters.aster.common.enums import AsterOrderSide
from nautilus_trader.adapters.aster.common.enums import AsterOrderType
from nautilus_trader.adapters.aster.common.enums import AsterSecurityType
from nautilus_trader.adapters.aster.common.enums import AsterTimeInForce
from nautilus_trader.adapters.aster.common.schemas.account import AsterOrder
from nautilus_trader.adapters.aster.common.schemas.account import AsterStatusCode
from nautilus_trader.adapters.aster.common.symbol import AsterSymbol
from nautilus_trader.adapters.aster.futures.enums import AsterFuturesMarginType
from nautilus_trader.adapters.aster.futures.schemas.account import AsterFuturesAccountInfo
from nautilus_trader.adapters.aster.futures.schemas.account import AsterFuturesAlgoOrder
from nautilus_trader.adapters.aster.futures.schemas.account import (
    AsterFuturesAlgoOrderCancelResponse,
)
from nautilus_trader.adapters.aster.futures.schemas.account import AsterFuturesDualSidePosition
from nautilus_trader.adapters.aster.futures.schemas.account import AsterFuturesLeverage
from nautilus_trader.adapters.aster.futures.schemas.account import (
    AsterFuturesMarginTypeResponse,
)
from nautilus_trader.adapters.aster.futures.schemas.account import AsterFuturesPositionRisk
from nautilus_trader.adapters.aster.futures.schemas.account import AsterFuturesSymbolConfig
from nautilus_trader.adapters.aster.http.account import AsterAccountHttpAPI
from nautilus_trader.adapters.aster.http.client import AsterHttpClient
from nautilus_trader.adapters.aster.http.endpoint import AsterHttpEndpoint
from nautilus_trader.adapters.aster.http.error import AsterClientError
from nautilus_trader.common.component import LiveClock
from nautilus_trader.common.config import PositiveInt
from nautilus_trader.core.nautilus_pyo3 import HttpMethod


class AsterFuturesPositionModeHttp(AsterHttpEndpoint):
    """
    Endpoint of user's position mode for every FUTURES symbol.

    `GET /fapi/v1/positionSide/dual`
    `GET /dapi/v1/positionSide/dual`

    `POST /fapi/v1/positionSide/dual`
    `POST /dapi/v1/positionSide/dual`

    References
    ----------
    https://docs.asterdex.com/futures/en/#change-position-mode-trade
    https://docs.asterdex.com/delivery/en/#change-position-mode-trade

    """

    def __init__(
        self,
        client: AsterHttpClient,
        base_endpoint: str,
    ):
        methods = {
            HttpMethod.GET: AsterSecurityType.USER_DATA,
            HttpMethod.POST: AsterSecurityType.TRADE,
        }
        url_path = base_endpoint + "positionSide/dual"
        super().__init__(
            client,
            methods,
            url_path,
        )
        self._get_resp_decoder = msgspec.json.Decoder(AsterFuturesDualSidePosition)
        self._post_resp_decoder = msgspec.json.Decoder(AsterStatusCode)

    class GetParameters(msgspec.Struct, omit_defaults=True, frozen=True):
        """
        Parameters of positionSide/dual GET request.

        Parameters
        ----------
        timestamp : str
            The millisecond timestamp of the request.
        recvWindow : str, optional
            The response receive window for the request (cannot be greater than 60000).

        """

        timestamp: str
        recvWindow: str | None = None

    class PostParameters(msgspec.Struct, omit_defaults=True, frozen=True):
        """
        Parameters of positionSide/dual POST request.

        Parameters
        ----------
        timestamp : str
            The millisecond timestamp of the request.
        dualSidePosition : str ('true', 'false')
            The dual side position mode to set...
            `true`: Hedge Mode, `false`: One-way mode.
        recvWindow : str, optional
            The response receive window for the request (cannot be greater than 60000).

        """

        timestamp: str
        dualSidePosition: str
        recvWindow: str | None = None

    async def get(self, params: GetParameters) -> AsterFuturesDualSidePosition:
        method_type = HttpMethod.GET
        raw = await self._method(method_type, params)
        return self._get_resp_decoder.decode(raw)

    async def post(self, params: PostParameters) -> AsterStatusCode:
        method_type = HttpMethod.POST
        raw = await self._method(method_type, params)
        return self._post_resp_decoder.decode(raw)


class AsterFuturesAllOpenOrdersHttp(AsterHttpEndpoint):
    """
    Endpoint of all open FUTURES orders.

    `DELETE /fapi/v1/allOpenOrders`
    `DELETE /dapi/v1/allOpenOrders`

    References
    ----------
    https://docs.asterdex.com/futures/en/#cancel-all-open-orders-trade
    https://docs.asterdex.com/delivery/en/#cancel-all-open-orders-trade

    """

    def __init__(
        self,
        client: AsterHttpClient,
        base_endpoint: str,
    ):
        methods = {
            HttpMethod.DELETE: AsterSecurityType.TRADE,
        }
        url_path = base_endpoint + "allOpenOrders"
        super().__init__(
            client,
            methods,
            url_path,
        )
        self._delete_resp_decoder = msgspec.json.Decoder(AsterStatusCode)

    class DeleteParameters(msgspec.Struct, omit_defaults=True, frozen=True):
        """
        Parameters of allOpenOrders DELETE request.

        Parameters
        ----------
        timestamp : str
            The millisecond timestamp of the request.
        symbol : AsterSymbol
            The symbol of the request
        recvWindow : str, optional
            The response receive window for the request (cannot be greater than 60000).

        """

        timestamp: str
        symbol: AsterSymbol
        recvWindow: str | None = None

    async def delete(self, params: DeleteParameters) -> AsterStatusCode:
        method_type = HttpMethod.DELETE
        raw = await self._method(method_type, params)
        return self._delete_resp_decoder.decode(raw)


class AsterFuturesCancelMultipleOrdersHttp(AsterHttpEndpoint):
    """
    Endpoint of cancel multiple FUTURES orders.

    `DELETE /fapi/v1/batchOrders`
    `DELETE /dapi/v1/batchOrders`

    References
    ----------
    https://docs.asterdex.com/futures/en/#cancel-multiple-orders-trade
    https://docs.asterdex.com/delivery/en/#cancel-multiple-orders-trade

    """

    def __init__(
        self,
        client: AsterHttpClient,
        base_endpoint: str,
    ):
        methods = {
            HttpMethod.DELETE: AsterSecurityType.TRADE,
        }
        url_path = base_endpoint + "batchOrders"
        super().__init__(
            client,
            methods,
            url_path,
        )
        self._delete_resp_decoder = msgspec.json.Decoder(
            list[AsterOrder] | dict[str, Any],
            strict=False,
        )

    class DeleteParameters(msgspec.Struct, omit_defaults=True, frozen=True):
        """
        Parameters of batchOrders DELETE request.

        Parameters
        ----------
        timestamp : str
            The millisecond timestamp of the request.
        symbol : AsterSymbol
            The symbol of the request
        recvWindow : str, optional
            The response receive window for the request (cannot be greater than 60000).

        """

        timestamp: str
        symbol: AsterSymbol
        orderIdList: str | None = None
        origClientOrderIdList: str | None = None
        recvWindow: str | None = None

    async def delete(self, params: DeleteParameters) -> list[AsterOrder]:
        method_type = HttpMethod.DELETE
        raw = await self._method(method_type, params)
        return self._delete_resp_decoder.decode(raw)


class AsterFuturesAccountHttp(AsterHttpEndpoint):
    """
    Endpoint of current FUTURES account information.

    `GET /fapi/v2/account`
    `GET /dapi/v1/account`

    References
    ----------
    https://docs.asterdex.com/futures/en/#account-information-v2-user_data
    https://docs.asterdex.com/delivery/en/#account-information-user_data

    """

    def __init__(
        self,
        client: AsterHttpClient,
        base_endpoint: str,
    ):
        methods = {
            HttpMethod.GET: AsterSecurityType.USER_DATA,
        }
        url_path = base_endpoint + "account"
        super().__init__(
            client,
            methods,
            url_path,
        )
        self._resp_decoder = msgspec.json.Decoder(AsterFuturesAccountInfo)

    class GetParameters(msgspec.Struct, omit_defaults=True, frozen=True):
        """
        Parameters of account GET request.

        Parameters
        ----------
        timestamp : str
            The millisecond timestamp of the request.
        recvWindow : str, optional
            The response receive window for the request (cannot be greater than 60000).

        """

        timestamp: str
        recvWindow: str | None = None

    async def get(self, params: GetParameters) -> AsterFuturesAccountInfo:
        method_type = HttpMethod.GET
        raw = await self._method(method_type, params)
        return self._resp_decoder.decode(raw)


class AsterFuturesPositionRiskHttp(AsterHttpEndpoint):
    """
    Endpoint of information of all FUTURES positions.

    `GET /fapi/v3/positionRisk`
    `GET /dapi/v1/positionRisk`

    References
    ----------
    https://developers.aster.com/docs/derivatives/usds-margined-futures/trade/rest-api/Position-Information-V3
    https://docs.asterdex.com/delivery/en/#position-information-user_data

    """

    def __init__(
        self,
        client: AsterHttpClient,
        base_endpoint: str,
    ):
        methods = {
            HttpMethod.GET: AsterSecurityType.USER_DATA,
        }
        url_path = base_endpoint + "positionRisk"
        super().__init__(
            client,
            methods,
            url_path,
        )
        self._get_resp_decoder = msgspec.json.Decoder(list[AsterFuturesPositionRisk])

    class GetParameters(msgspec.Struct, omit_defaults=True, frozen=True):
        """
        Parameters of positionRisk GET request.

        Parameters
        ----------
        timestamp : str
            The millisecond timestamp of the request.
        symbol : AsterSymbol, optional
            The symbol of the request.
        recvWindow : str, optional
            The response receive window for the request (cannot be greater than 60000).

        """

        timestamp: str
        symbol: AsterSymbol | None = None
        recvWindow: str | None = None

    async def get(self, params: GetParameters) -> list[AsterFuturesPositionRisk]:
        method_type = HttpMethod.GET
        raw = await self._method(method_type, params)
        return self._get_resp_decoder.decode(raw)


class AsterFuturesSymbolConfigHttp(AsterHttpEndpoint):
    """
    Endpoint for symbol configuration.

    `GET /fapi/v1/symbolConfig`

    References
    ----------
    https://developers.aster.com/docs/derivatives/usds-margined-futures/account/rest-api/Symbol-Config

    """

    def __init__(
        self,
        client: AsterHttpClient,
        base_endpoint: str,
    ):
        methods = {
            HttpMethod.GET: AsterSecurityType.USER_DATA,
        }
        url_path = base_endpoint + "symbolConfig"
        super().__init__(
            client,
            methods,
            url_path,
        )
        self._get_resp_decoder = msgspec.json.Decoder(list[AsterFuturesSymbolConfig])

    class GetParameters(msgspec.Struct, omit_defaults=True, frozen=True):
        """
        Parameters of symbolConfig GET request.

        Parameters
        ----------
        timestamp : str
            The millisecond timestamp of the request.
        symbol : AsterSymbol, optional
            The symbol of the request.
        recvWindow : str, optional
            The response receive window for the request (cannot be greater than 60000).

        """

        timestamp: str
        symbol: AsterSymbol | None = None
        recvWindow: str | None = None

    async def get(self, params: GetParameters) -> list[AsterFuturesSymbolConfig]:
        method_type = HttpMethod.GET
        raw = await self._method(method_type, params)
        return self._get_resp_decoder.decode(raw)


class AsterFuturesLeverageHttp(AsterHttpEndpoint):
    """
    Initial leverage.

    `POST /fapi/v1/leverage`

    References
    ----------
    https://developers.aster.com/docs/derivatives/usds-margined-futures/trade/rest-api/Change-Initial-Leverage

    """

    def __init__(
        self,
        client: AsterHttpClient,
        base_endpoint: str,
    ):
        methods = {
            HttpMethod.POST: AsterSecurityType.TRADE,
        }
        url_path = base_endpoint + "leverage"

        super().__init__(client, methods, url_path)
        self._resp_decoder = msgspec.json.Decoder(AsterFuturesLeverage)

    class PostParameters(msgspec.Struct, omit_defaults=True, frozen=True):
        """
        Initial leverage POST endpoint parameters.

        Parameters
        ----------
        symbol : AsterSymbol
        leverage : PositiveInt
            Target initial leverage: int from 1 to 125
        timestamp : str
            The millisecond timestamp of the request.
        recvWindow : str, optional
            The response receive window in milliseconds for the request.

        """

        symbol: AsterSymbol
        leverage: PositiveInt
        timestamp: str
        recvWindow: str | None = None

    async def post(
        self,
        params: PostParameters,
    ) -> AsterFuturesLeverage:
        method_type = HttpMethod.POST
        raw = await self._method(method_type, params)
        return self._resp_decoder.decode(raw)


class AsterFuturesMarginTypeHttp(AsterHttpEndpoint):
    """
    Margin type.

    `POST /fapi/v1/marginType`

    References
    ----------
    https://developers.aster.com/docs/derivatives/usds-margined-futures/trade/rest-api/Change-Margin-Type

    """

    def __init__(
        self,
        client: AsterHttpClient,
        base_endpoint: str,
    ):
        methods = {
            HttpMethod.POST: AsterSecurityType.TRADE,
        }
        url_path = base_endpoint + "marginType"
        super().__init__(client, methods, url_path)
        self._resp_decoder = msgspec.json.Decoder(AsterFuturesMarginTypeResponse)

    class PostParameters(msgspec.Struct, omit_defaults=True, frozen=True):
        """
        Margin type POST endpoint parameters.

        Parameters
        ----------
        symbol : AsterSymbol
        marginType : str
            ISOLATED or CROSSED
        timestamp : str
            The millisecond timestamp of the request.
        recvWindow : str, optional
            The response receive window in milliseconds for the request.

        """

        symbol: AsterSymbol
        marginType: str
        timestamp: str
        recvWindow: str | None = None

    async def post(
        self,
        params: PostParameters,
    ) -> AsterFuturesMarginTypeResponse:
        try:
            raw = await self._method(HttpMethod.POST, params)
        except AsterClientError as e:
            if e.message["msg"] == "No need to change margin type.":
                return AsterFuturesMarginTypeResponse(code=200, msg="success")
            raise
        return self._resp_decoder.decode(raw)


class AsterFuturesAlgoOrderHttp(AsterHttpEndpoint):
    """
    Endpoint for managing Aster Futures algo (conditional) orders.

    `POST /fapi/v1/algoOrder` - Place an algo order
    `DELETE /fapi/v1/algoOrder` - Cancel an algo order
    `GET /fapi/v1/algoOrder` - Query an algo order

    References
    ----------
    https://developers.aster.com/docs/derivatives/usds-margined-futures/trade/rest-api/New-Algo-Order

    Notes
    -----
    Effective 2025-12-09, Aster migrated conditional orders (STOP_MARKET,
    TAKE_PROFIT_MARKET, STOP, TAKE_PROFIT, TRAILING_STOP_MARKET) to the Algo
    Service. The traditional `/fapi/v1/order` endpoint now returns error -4120
    for these order types.

    """

    def __init__(
        self,
        client: AsterHttpClient,
        base_endpoint: str,
    ):
        methods = {
            HttpMethod.GET: AsterSecurityType.USER_DATA,
            HttpMethod.POST: AsterSecurityType.TRADE,
            HttpMethod.DELETE: AsterSecurityType.TRADE,
        }
        url_path = base_endpoint + "algoOrder"
        super().__init__(
            client,
            methods,
            url_path,
        )

        self._post_resp_decoder = msgspec.json.Decoder(AsterFuturesAlgoOrder)
        self._delete_resp_decoder = msgspec.json.Decoder(AsterFuturesAlgoOrderCancelResponse)
        self._get_resp_decoder = msgspec.json.Decoder(AsterFuturesAlgoOrder)

    class GetDeleteParameters(msgspec.Struct, omit_defaults=True, frozen=True):
        """
        Parameters for algo order GET & DELETE requests.

        Parameters
        ----------
        timestamp : str
            The millisecond timestamp of the request.
        algoId : int, optional
            The algo order identifier.
        clientAlgoId : str, optional
            The client-specified algo order identifier.
        recvWindow : str, optional
            The response receive window for the request (cannot be greater than 60000).

        Notes
        -----
        Either `algoId` or `clientAlgoId` must be sent.

        """

        timestamp: str
        algoId: int | None = None
        clientAlgoId: str | None = None
        recvWindow: str | None = None

    class PostParameters(msgspec.Struct, omit_defaults=True, frozen=True):
        """
        Parameters for algo order POST (create) request.

        Parameters
        ----------
        symbol : AsterSymbol
            The trading pair symbol.
        side : AsterOrderSide
            The order side (BUY or SELL).
        type : AsterOrderType
            The order type (STOP_MARKET, TAKE_PROFIT_MARKET, STOP, TAKE_PROFIT,
            TRAILING_STOP_MARKET).
        algoType : str
            The algo type. Only "CONDITIONAL" is supported.
        timestamp : str
            The millisecond timestamp of the request.
        positionSide : AsterFuturesPositionSide, optional
            Position side for hedge mode.
        quantity : str, optional
            Order quantity. Cannot be used with closePosition=true.
        price : str, optional
            Order price for STOP/TAKE_PROFIT orders.
        triggerPrice : str, optional
            The trigger price for conditional orders.
        timeInForce : AsterTimeInForce, optional
            Time in force (GTC, IOC, FOK). Default is GTC.
        workingType : str, optional
            Trigger type: MARK_PRICE or CONTRACT_PRICE (default).
        priceMatch : str, optional
            Price match mode for BBO matching.
        closePosition : str, optional
            Close all position with STOP_MARKET or TAKE_PROFIT_MARKET.
        priceProtect : str, optional
            Price protection. Default is false.
        reduceOnly : str, optional
            Reduce only flag. Cannot be used in Hedge Mode.
        activatePrice : str, optional
            Activation price for TRAILING_STOP_MARKET orders.
        callbackRate : str, optional
            Callback rate for TRAILING_STOP_MARKET (0.1-10, where 1 = 1%).
        clientAlgoId : str, optional
            Client-specified algo order ID.
        goodTillDate : int, optional
            GTD expiration timestamp in milliseconds (only second-level precision retained).
        recvWindow : str, optional
            The response receive window for the request.

        """

        symbol: AsterSymbol
        side: AsterOrderSide
        type: AsterOrderType
        algoType: str
        timestamp: str
        positionSide: AsterFuturesPositionSide | None = None
        quantity: str | None = None
        price: str | None = None
        triggerPrice: str | None = None
        timeInForce: AsterTimeInForce | None = None
        workingType: str | None = None
        priceMatch: str | None = None
        closePosition: str | None = None
        priceProtect: str | None = None
        reduceOnly: str | None = None
        activatePrice: str | None = None
        callbackRate: str | None = None
        clientAlgoId: str | None = None
        goodTillDate: int | None = None
        recvWindow: str | None = None

    async def get(self, params: GetDeleteParameters) -> AsterFuturesAlgoOrder:
        method_type = HttpMethod.GET
        raw = await self._method(method_type, params)
        return self._get_resp_decoder.decode(raw)

    async def delete(self, params: GetDeleteParameters) -> AsterFuturesAlgoOrderCancelResponse:
        method_type = HttpMethod.DELETE
        raw = await self._method(method_type, params)
        return self._delete_resp_decoder.decode(raw)

    async def post(self, params: PostParameters) -> AsterFuturesAlgoOrder:
        method_type = HttpMethod.POST
        raw = await self._method(method_type, params)
        return self._post_resp_decoder.decode(raw)


class AsterFuturesOpenAlgoOrdersHttp(AsterHttpEndpoint):
    """
    Endpoint for fetching all open algo (conditional) orders.

    `GET /fapi/v1/openAlgoOrders`

    References
    ----------
    https://developers.aster.com/docs/derivatives/usds-margined-futures/trade/rest-api/Current-All-Algo-Open-Orders

    """

    def __init__(
        self,
        client: AsterHttpClient,
        base_endpoint: str,
    ):
        methods = {
            HttpMethod.GET: AsterSecurityType.USER_DATA,
        }
        url_path = base_endpoint + "openAlgoOrders"
        super().__init__(
            client,
            methods,
            url_path,
        )

        self._get_resp_decoder = msgspec.json.Decoder(list[AsterFuturesAlgoOrder])

    class GetParameters(msgspec.Struct, omit_defaults=True, frozen=True):
        """
        Parameters for open algo orders GET request.

        Parameters
        ----------
        timestamp : str
            The millisecond timestamp of the request.
        algoType : str, optional
            Filter by algo type.
        symbol : AsterSymbol, optional
            Filter by symbol. If omitted, orders for all symbols returned.
        algoId : int, optional
            Filter by specific algo order ID.
        recvWindow : str, optional
            The response receive window for the request (cannot be greater than 60000).

        """

        timestamp: str
        algoType: str | None = None
        symbol: AsterSymbol | None = None
        algoId: int | None = None
        recvWindow: str | None = None

    async def get(self, params: GetParameters) -> list[AsterFuturesAlgoOrder]:
        method_type = HttpMethod.GET
        raw = await self._method(method_type, params)
        return self._get_resp_decoder.decode(raw)


class AsterFuturesAllAlgoOrdersHttp(AsterHttpEndpoint):
    """
    Endpoint for querying all algo (conditional) orders including historical.

    `GET /fapi/v1/allAlgoOrders`

    References
    ----------
    https://developers.aster.com/docs/derivatives/usds-margined-futures/trade/rest-api/Query-All-Algo-Orders

    """

    def __init__(
        self,
        client: AsterHttpClient,
        base_endpoint: str,
    ):
        methods = {
            HttpMethod.GET: AsterSecurityType.USER_DATA,
        }
        url_path = base_endpoint + "allAlgoOrders"
        super().__init__(
            client,
            methods,
            url_path,
        )

        self._get_resp_decoder = msgspec.json.Decoder(list[AsterFuturesAlgoOrder])

    class GetParameters(msgspec.Struct, omit_defaults=True, frozen=True):
        """
        Parameters for all algo orders GET request.

        Parameters
        ----------
        timestamp : str
            The millisecond timestamp of the request.
        symbol : AsterSymbol
            The symbol to query (required).
        algoId : int, optional
            If set, retrieves orders >= that algoId; otherwise returns most recent.
        startTime : str, optional
            Query start timestamp in milliseconds.
        endTime : str, optional
            Query end timestamp in milliseconds.
        page : int, optional
            Pagination index.
        limit : int, optional
            Result limit (default 500, max 1000).
        recvWindow : str, optional
            The response receive window for the request (cannot be greater than 60000).

        """

        timestamp: str
        symbol: AsterSymbol
        algoId: int | None = None
        startTime: str | None = None
        endTime: str | None = None
        page: int | None = None
        limit: int | None = None
        recvWindow: str | None = None

    async def get(self, params: GetParameters) -> list[AsterFuturesAlgoOrder]:
        method_type = HttpMethod.GET
        raw = await self._method(method_type, params)
        return self._get_resp_decoder.decode(raw)


class AsterFuturesCancelAllAlgoOrdersHttp(AsterHttpEndpoint):
    """
    Endpoint for canceling all open algo (conditional) orders for a symbol.

    `DELETE /fapi/v1/algoOpenOrders`

    References
    ----------
    https://developers.aster.com/docs/derivatives/usds-margined-futures/trade/rest-api/Cancel-All-Algo-Open-Orders

    """

    def __init__(
        self,
        client: AsterHttpClient,
        base_endpoint: str,
    ):
        methods = {
            HttpMethod.DELETE: AsterSecurityType.TRADE,
        }
        url_path = base_endpoint + "algoOpenOrders"
        super().__init__(
            client,
            methods,
            url_path,
        )

        self._delete_resp_decoder = msgspec.json.Decoder(AsterStatusCode)

    class DeleteParameters(msgspec.Struct, omit_defaults=True, frozen=True):
        """
        Parameters for cancel all algo open orders DELETE request.

        Parameters
        ----------
        timestamp : str
            The millisecond timestamp of the request.
        symbol : AsterSymbol
            The symbol to cancel all algo orders for.
        recvWindow : str, optional
            The response receive window for the request (cannot be greater than 60000).

        """

        timestamp: str
        symbol: AsterSymbol
        recvWindow: str | None = None

    async def delete(self, params: DeleteParameters) -> AsterStatusCode:
        method_type = HttpMethod.DELETE
        raw = await self._method(method_type, params)
        return self._delete_resp_decoder.decode(raw)


class AsterFuturesIncomeRecord(msgspec.Struct, kw_only=True, frozen=True):
    """
    Single record from ``GET /fapi/v3/income`` (Aster income history).

    ``tranId`` is unique per ``incomeType`` per user — used as the dedup key
    for application-side flow tracking (deposits, withdrawals, funding, etc.).

    ``income`` is a signed decimal string: positive for credits
    (deposits, rebates) and negative for debits (withdrawals, commissions).

    References
    ----------
    https://github.com/asterdex/api-docs/blob/master/V3(Recommended)/EN/aster-finance-futures-api-v3.md#get-income-historyuser_data
    """

    symbol: str
    incomeType: str
    income: str
    asset: str
    info: str
    time: int
    tranId: str
    tradeId: str | None = None


class AsterFuturesIncomeHttp(AsterHttpEndpoint):
    """
    Endpoint for income / transfer history.

    ``GET /fapi/v3/income``

    References
    ----------
    https://github.com/asterdex/api-docs/blob/master/V3(Recommended)/EN/aster-finance-futures-api-v3.md#get-income-historyuser_data

    """

    def __init__(self, client: AsterHttpClient, base_endpoint: str):
        methods = {HttpMethod.GET: AsterSecurityType.USER_DATA}
        super().__init__(client, methods, base_endpoint + "income")
        self._resp_decoder = msgspec.json.Decoder(list[AsterFuturesIncomeRecord])

    class GetParameters(msgspec.Struct, omit_defaults=True, frozen=True):
        """
        Parameters of income GET request.

        Parameters
        ----------
        timestamp : str
            Millisecond timestamp of the request.
        incomeType : str, optional
            Filter by type: TRANSFER, REALIZED_PNL, FUNDING_FEE, COMMISSION,
            INSURANCE_CLEAR, WELCOME_BONUS, MARKET_MERCHANT_RETURN_REWARD.
        startTime : int, optional
            Inclusive start time in ms.
        endTime : int, optional
            Inclusive end time in ms.
        limit : int, optional
            Default 100; max 1000.
        recvWindow : str, optional
            Response receive window (max 60000 ms).

        """

        timestamp: str
        incomeType: str | None = None
        startTime: int | None = None
        endTime: int | None = None
        limit: int | None = None
        recvWindow: str | None = None

    async def get(self, params: GetParameters) -> list[AsterFuturesIncomeRecord]:
        method_type = HttpMethod.GET
        raw = await self._method(method_type, params)
        return self._resp_decoder.decode(raw)


class AsterFuturesAccountHttpAPI(AsterAccountHttpAPI):
    """
    Provides access to the Aster Futures Account/Trade HTTP REST API.

    Parameters
    ----------
    client : AsterHttpClient
        The Aster REST API client.
    account_type : AsterAccountType
        The Aster account type.

    """

    def __init__(
        self,
        client: AsterHttpClient,
        clock: LiveClock,
        account_type: AsterAccountType = AsterAccountType.USDT_FUTURES,
    ):
        super().__init__(
            client=client,
            clock=clock,
            account_type=account_type,
        )

        if not account_type.is_futures:
            raise RuntimeError(  # pragma: no cover (design-time error)
                f"`AsterAccountType` not USDT_FUTURES or COIN_FUTURES, was {account_type}",  # pragma: no cover
            )
        v2_endpoint_base = self.base_endpoint
        v3_endpoint_base = self.base_endpoint

        if account_type == AsterAccountType.USDT_FUTURES:
            v2_endpoint_base = "/fapi/v2/"
            v3_endpoint_base = "/fapi/v3/"

        # Create endpoints
        self._endpoint_futures_position_mode = AsterFuturesPositionModeHttp(
            client,
            self.base_endpoint,
        )
        self._endpoint_futures_all_open_orders = AsterFuturesAllOpenOrdersHttp(
            client,
            self.base_endpoint,
        )
        self._endpoint_futures_cancel_multiple_orders = AsterFuturesCancelMultipleOrdersHttp(
            client,
            self.base_endpoint,
        )
        self._endpoint_futures_account = AsterFuturesAccountHttp(client, v2_endpoint_base)
        self._endpoint_futures_position_risk = AsterFuturesPositionRiskHttp(
            client,
            v3_endpoint_base,
        )
        self._endpoint_futures_leverage = AsterFuturesLeverageHttp(client, self.base_endpoint)
        self._endpoint_futures_margin_type = AsterFuturesMarginTypeHttp(
            client,
            self.base_endpoint,
        )
        self._endpoint_futures_symbol_config = AsterFuturesSymbolConfigHttp(
            client,
            self.base_endpoint,
        )
        self._endpoint_futures_algo_order = AsterFuturesAlgoOrderHttp(
            client,
            self.base_endpoint,
        )
        self._endpoint_futures_open_algo_orders = AsterFuturesOpenAlgoOrdersHttp(
            client,
            self.base_endpoint,
        )
        self._endpoint_futures_all_algo_orders = AsterFuturesAllAlgoOrdersHttp(
            client,
            self.base_endpoint,
        )
        self._endpoint_futures_cancel_all_algo_orders = AsterFuturesCancelAllAlgoOrdersHttp(
            client,
            self.base_endpoint,
        )
        self._endpoint_futures_income = AsterFuturesIncomeHttp(client, v3_endpoint_base)

    async def query_income_history(
        self,
        income_type: str | None = None,
        start_time: int | None = None,
        end_time: int | None = None,
        limit: int | None = None,
        recv_window: str | None = None,
    ) -> list[AsterFuturesIncomeRecord]:
        """
        Fetch Aster Futures income / transfer history.

        Returns records newest-first. ``AsterFuturesIncomeRecord.tranId`` is
        the unique identifier per ``incomeType`` per user — use it as a
        dedup key when tracking flows across polls.

        Parameters
        ----------
        income_type : str, optional
            Filter by type. Use ``"TRANSFER"`` to capture deposits and
            withdrawals only. Pass ``None`` to receive all types.
        start_time : int, optional
            Inclusive start time in Unix ms.
        end_time : int, optional
            Inclusive end time in Unix ms.
        limit : int, optional
            Max records to return; default 100, max 1000.
        recv_window : str, optional
            Optional receive window (ms, max 60000).

        Returns
        -------
        list[AsterFuturesIncomeRecord]
            Records ordered newest-first by the venue.

        References
        ----------
        https://github.com/asterdex/api-docs/blob/master/V3(Recommended)/EN/aster-finance-futures-api-v3.md#get-income-historyuser_data
        """
        return await self._endpoint_futures_income.get(
            params=self._endpoint_futures_income.GetParameters(
                timestamp=self._timestamp(),
                incomeType=income_type,
                startTime=start_time,
                endTime=end_time,
                limit=limit,
                recvWindow=recv_window,
            ),
        )

    async def query_futures_hedge_mode(
        self,
        recv_window: str | None = None,
    ) -> AsterFuturesDualSidePosition:
        """
        Check Aster Futures hedge mode (dualSidePosition).
        """
        return await self._endpoint_futures_position_mode.get(
            params=self._endpoint_futures_position_mode.GetParameters(
                timestamp=self._timestamp(),
                recvWindow=recv_window,
            ),
        )

    async def set_leverage(
        self,
        symbol: AsterSymbol,
        leverage: PositiveInt,
        recv_window: str | None = None,
    ) -> AsterFuturesLeverage:
        """
        Set Aster Futures initial leverage.
        """
        return await self._endpoint_futures_leverage.post(
            self._endpoint_futures_leverage.PostParameters(
                symbol=symbol,
                leverage=leverage,
                timestamp=self._timestamp(),
                recvWindow=recv_window,
            ),
        )

    async def set_margin_type(
        self,
        symbol: AsterSymbol,
        margin_type: AsterFuturesMarginType,
        recv_window: str | None = None,
    ) -> AsterFuturesMarginTypeResponse:
        """
        Change symbol level margin type.

        :param symbol : AsterSymbol
        :param margin_type : AsterFuturesMarginType
        :param recv_window : str, optional
        :return: AsterFuturesMarginTypeResponse

        """
        return await self._endpoint_futures_margin_type.post(
            self._endpoint_futures_margin_type.PostParameters(
                symbol=symbol,
                marginType=margin_type.value,
                timestamp=self._timestamp(),
                recvWindow=recv_window,
            ),
        )

    async def set_futures_hedge_mode(
        self,
        dual_side_position: bool,
        recv_window: str | None = None,
    ) -> AsterStatusCode:
        """
        Set Aster Futures hedge mode (dualSidePosition).
        """
        return await self._endpoint_futures_position_mode.post(
            params=self._endpoint_futures_position_mode.PostParameters(
                timestamp=self._timestamp(),
                dualSidePosition=str(dual_side_position).lower(),
                recvWindow=recv_window,
            ),
        )

    async def cancel_all_open_orders(
        self,
        symbol: str,
        recv_window: str | None = None,
    ) -> bool:
        """
        Delete all Futures open orders.

        Returns whether successful.

        """
        response = await self._endpoint_futures_all_open_orders.delete(
            params=self._endpoint_futures_all_open_orders.DeleteParameters(
                timestamp=self._timestamp(),
                symbol=AsterSymbol(symbol),
                recvWindow=recv_window,
            ),
        )
        return response.code == 200

    async def cancel_multiple_orders(
        self,
        symbol: str,
        client_order_ids: list[str],
        recv_window: str | None = None,
    ) -> bool:
        """
        Delete multiple Futures orders.

        Returns whether successful.

        """
        stringified_client_order_ids = str(client_order_ids).replace(" ", "").replace("'", '"')
        await self._endpoint_futures_cancel_multiple_orders.delete(
            params=self._endpoint_futures_cancel_multiple_orders.DeleteParameters(
                timestamp=self._timestamp(),
                symbol=AsterSymbol(symbol),
                origClientOrderIdList=stringified_client_order_ids,
                recvWindow=recv_window,
            ),
        )
        return True

    async def query_futures_account_info(
        self,
        recv_window: str | None = None,
    ) -> AsterFuturesAccountInfo:
        """
        Check Aster Futures account information.
        """
        return await self._endpoint_futures_account.get(
            params=self._endpoint_futures_account.GetParameters(
                timestamp=self._timestamp(),
                recvWindow=recv_window,
            ),
        )

    async def query_futures_position_risk(
        self,
        symbol: str | None = None,
        recv_window: str | None = None,
    ) -> list[AsterFuturesPositionRisk]:
        """
        Check all Futures position's info for a symbol.
        """
        return await self._endpoint_futures_position_risk.get(
            params=self._endpoint_futures_position_risk.GetParameters(
                timestamp=self._timestamp(),
                symbol=AsterSymbol(symbol) if symbol else None,
                recvWindow=recv_window,
            ),
        )

    async def query_futures_symbol_config(
        self,
        symbol: str | None = None,
        recv_window: str | None = None,
    ) -> list[AsterFuturesSymbolConfig]:
        """
        Check Futures symbol configuration including leverage settings.
        """
        return await self._endpoint_futures_symbol_config.get(
            params=self._endpoint_futures_symbol_config.GetParameters(
                timestamp=self._timestamp(),
                symbol=AsterSymbol(symbol) if symbol else None,
                recvWindow=recv_window,
            ),
        )

    async def new_algo_order(
        self,
        symbol: str,
        side: AsterOrderSide,
        order_type: AsterOrderType,
        position_side: AsterFuturesPositionSide | None = None,
        quantity: str | None = None,
        price: str | None = None,
        trigger_price: str | None = None,
        time_in_force: AsterTimeInForce | None = None,
        working_type: str | None = None,
        price_match: str | None = None,
        close_position: str | None = None,
        price_protect: str | None = None,
        reduce_only: str | None = None,
        activation_price: str | None = None,
        callback_rate: str | None = None,
        client_algo_id: str | None = None,
        good_till_date: int | None = None,
        recv_window: str | None = None,
    ) -> AsterFuturesAlgoOrder:
        """
        Send a new conditional (algo) order to Aster Futures.

        This endpoint is required for STOP_MARKET, TAKE_PROFIT_MARKET, STOP,
        TAKE_PROFIT, and TRAILING_STOP_MARKET orders as of 2025-12-09.

        """
        return await self._endpoint_futures_algo_order.post(
            params=self._endpoint_futures_algo_order.PostParameters(
                symbol=AsterSymbol(symbol),
                side=side,
                type=order_type,
                algoType="CONDITIONAL",
                timestamp=self._timestamp(),
                positionSide=position_side,
                quantity=quantity,
                price=price,
                triggerPrice=trigger_price,
                timeInForce=time_in_force,
                workingType=working_type,
                priceMatch=price_match,
                closePosition=close_position,
                priceProtect=price_protect,
                reduceOnly=reduce_only,
                activatePrice=activation_price,
                callbackRate=callback_rate,
                clientAlgoId=client_algo_id,
                goodTillDate=good_till_date,
                recvWindow=recv_window,
            ),
        )

    async def cancel_algo_order(
        self,
        algo_id: int | None = None,
        client_algo_id: str | None = None,
        recv_window: str | None = None,
    ) -> AsterFuturesAlgoOrderCancelResponse:
        """
        Cancel an active algo order.
        """
        if algo_id is None and client_algo_id is None:
            raise RuntimeError(
                "Either algoId or clientAlgoId must be sent.",
            )
        return await self._endpoint_futures_algo_order.delete(
            params=self._endpoint_futures_algo_order.GetDeleteParameters(
                timestamp=self._timestamp(),
                algoId=algo_id,
                clientAlgoId=client_algo_id,
                recvWindow=recv_window,
            ),
        )

    async def query_algo_order(
        self,
        algo_id: int | None = None,
        client_algo_id: str | None = None,
        recv_window: str | None = None,
    ) -> AsterFuturesAlgoOrder:
        """
        Query an algo order status.
        """
        if algo_id is None and client_algo_id is None:
            raise RuntimeError(
                "Either algoId or clientAlgoId must be sent.",
            )
        return await self._endpoint_futures_algo_order.get(
            params=self._endpoint_futures_algo_order.GetDeleteParameters(
                timestamp=self._timestamp(),
                algoId=algo_id,
                clientAlgoId=client_algo_id,
                recvWindow=recv_window,
            ),
        )

    async def query_open_algo_orders(
        self,
        symbol: str | None = None,
        recv_window: str | None = None,
    ) -> list[AsterFuturesAlgoOrder]:
        """
        Query all currently open algo orders.

        Parameters
        ----------
        symbol : str, optional
            Filter by symbol. If omitted, orders for all symbols returned.
        recv_window : str, optional
            The response receive window for the request.

        Returns
        -------
        list[AsterFuturesAlgoOrder]

        """
        return await self._endpoint_futures_open_algo_orders.get(
            params=self._endpoint_futures_open_algo_orders.GetParameters(
                timestamp=self._timestamp(),
                symbol=AsterSymbol(symbol) if symbol else None,
                recvWindow=recv_window,
            ),
        )

    async def query_all_algo_orders(
        self,
        symbol: str,
        start_time: int | None = None,
        end_time: int | None = None,
        page: int | None = None,
        limit: int | None = None,
        recv_window: str | None = None,
    ) -> list[AsterFuturesAlgoOrder]:
        """
        Query all algo orders including historical (triggered, cancelled, finished).

        Parameters
        ----------
        symbol : str
            The symbol to query (required).
        start_time : int, optional
            Query start timestamp in milliseconds.
        end_time : int, optional
            Query end timestamp in milliseconds.
        page : int, optional
            Pagination index (1-based).
        limit : int, optional
            Result limit (default 500, max 1000).
        recv_window : str, optional
            The response receive window for the request.

        Returns
        -------
        list[AsterFuturesAlgoOrder]

        """
        return await self._endpoint_futures_all_algo_orders.get(
            params=self._endpoint_futures_all_algo_orders.GetParameters(
                timestamp=self._timestamp(),
                symbol=AsterSymbol(symbol),
                startTime=str(start_time) if start_time else None,
                endTime=str(end_time) if end_time else None,
                page=page,
                limit=limit,
                recvWindow=recv_window,
            ),
        )

    async def cancel_all_open_algo_orders(
        self,
        symbol: str,
        recv_window: str | None = None,
    ) -> bool:
        """
        Cancel all open algo orders for a specific symbol.

        Parameters
        ----------
        symbol : str
            The symbol to cancel all algo orders for.
        recv_window : str, optional
            The response receive window for the request.

        Returns
        -------
        bool
            True if successful.

        """
        response = await self._endpoint_futures_cancel_all_algo_orders.delete(
            params=self._endpoint_futures_cancel_all_algo_orders.DeleteParameters(
                timestamp=self._timestamp(),
                symbol=AsterSymbol(symbol),
                recvWindow=recv_window,
            ),
        )
        return response.code == 200
