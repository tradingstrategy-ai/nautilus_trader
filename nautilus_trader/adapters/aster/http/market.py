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

import sys
import time

import msgspec

from nautilus_trader.adapters.aster.common.enums import AsterAccountType
from nautilus_trader.adapters.aster.common.enums import AsterKlineInterval
from nautilus_trader.adapters.aster.common.enums import AsterSecurityType
from nautilus_trader.adapters.aster.common.schemas.market import AsterAggTrade
from nautilus_trader.adapters.aster.common.schemas.market import AsterDepth
from nautilus_trader.adapters.aster.common.schemas.market import AsterKline
from nautilus_trader.adapters.aster.common.schemas.market import AsterTicker24hr
from nautilus_trader.adapters.aster.common.schemas.market import AsterTickerBook
from nautilus_trader.adapters.aster.common.schemas.market import AsterTickerPrice
from nautilus_trader.adapters.aster.common.schemas.market import AsterTime
from nautilus_trader.adapters.aster.common.schemas.market import AsterTrade
from nautilus_trader.adapters.aster.common.symbol import AsterSymbol
from nautilus_trader.adapters.aster.common.symbol import AsterSymbols
from nautilus_trader.adapters.aster.common.types import AsterBar
from nautilus_trader.adapters.aster.http.client import AsterHttpClient
from nautilus_trader.adapters.aster.http.endpoint import AsterHttpEndpoint
from nautilus_trader.core.correctness import PyCondition
from nautilus_trader.core.datetime import nanos_to_millis
from nautilus_trader.core.nautilus_pyo3 import HttpMethod
from nautilus_trader.model.data import BarType
from nautilus_trader.model.data import OrderBookDeltas
from nautilus_trader.model.data import TradeTick
from nautilus_trader.model.identifiers import InstrumentId


class AsterPingHttp(AsterHttpEndpoint):
    """
    Endpoint for testing connectivity to the REST API.

    `GET /api/v3/ping`
    `GET /fapi/v1/ping`
    `GET /dapi/v1/ping`

    References
    ----------
    https://docs.asterdex.com/spot/en/#test-connectivity
    https://docs.asterdex.com/futures/en/#test-connectivity
    https://docs.asterdex.com/delivery/en/#test-connectivity

    """

    def __init__(
        self,
        client: AsterHttpClient,
        base_endpoint: str,
    ):
        methods = {
            HttpMethod.GET: AsterSecurityType.NONE,
        }
        url_path = base_endpoint + "ping"
        super().__init__(
            client,
            methods,
            url_path,
        )
        self._get_resp_decoder = msgspec.json.Decoder()

    async def get(self) -> dict:
        method_type = HttpMethod.GET
        raw = await self._method(method_type, None)
        return self._get_resp_decoder.decode(raw)


class AsterTimeHttp(AsterHttpEndpoint):
    """
    Endpoint for testing connectivity to the REST API and receiving current server time.

    `GET /api/v3/time`
    `GET /fapi/v1/time`
    `GET /dapi/v1/time`

    References
    ----------
    https://docs.asterdex.com/spot/en/#check-server-time
    https://docs.asterdex.com/futures/en/#check-server-time
    https://docs.asterdex.com/delivery/en/#check-server-time

    """

    def __init__(
        self,
        client: AsterHttpClient,
        base_endpoint: str,
    ):
        methods = {
            HttpMethod.GET: AsterSecurityType.NONE,
        }
        url_path = base_endpoint + "time"
        super().__init__(client, methods, url_path)
        self._get_resp_decoder = msgspec.json.Decoder(AsterTime)

    async def get(self) -> AsterTime:
        method_type = HttpMethod.GET
        raw = await self._method(method_type, None)
        return self._get_resp_decoder.decode(raw)


class AsterDepthHttp(AsterHttpEndpoint):
    """
    Endpoint of orderbook depth.

    `GET /api/v3/depth`
    `GET /fapi/v1/depth`
    `GET /dapi/v1/depth`

    References
    ----------
    https://docs.asterdex.com/spot/en/#order-book
    https://docs.asterdex.com/futures/en/#order-book
    https://docs.asterdex.com/delivery/en/#order-book

    """

    def __init__(
        self,
        client: AsterHttpClient,
        base_endpoint: str,
    ):
        methods = {
            HttpMethod.GET: AsterSecurityType.NONE,
        }
        url_path = base_endpoint + "depth"
        super().__init__(
            client,
            methods,
            url_path,
        )
        self._get_resp_decoder = msgspec.json.Decoder(AsterDepth)

    class GetParameters(msgspec.Struct, omit_defaults=True, frozen=True):
        """
        Orderbook depth GET endpoint parameters.

        Parameters
        ----------
        symbol : AsterSymbol
            The trading pair.
        limit : int, optional, default 100
            The limit for the response.
            SPOT/MARGIN (GET /api/v3/depth)
                Default 100; max 5000.
            FUTURES (GET /*api/v1/depth)
                Default 500; max 1000.
                Valid limits:[5, 10, 20, 50, 100, 500, 1000].

        """

        symbol: AsterSymbol
        limit: int | None = None

    async def get(self, params: GetParameters) -> AsterDepth:
        method_type = HttpMethod.GET
        raw = await self._method(method_type, params)
        return self._get_resp_decoder.decode(raw)


class AsterTradesHttp(AsterHttpEndpoint):
    """
    Endpoint of recent market trades.

    `GET /api/v3/trades`
    `GET /fapi/v1/trades`
    `GET /dapi/v1/trades`

    References
    ----------
    https://docs.asterdex.com/spot/en/#recent-trades-list
    https://docs.asterdex.com/futures/en/#recent-trades-list
    https://docs.asterdex.com/delivery/en/#recent-trades-list

    """

    def __init__(
        self,
        client: AsterHttpClient,
        base_endpoint: str,
    ):
        methods = {
            HttpMethod.GET: AsterSecurityType.NONE,
        }
        url_path = base_endpoint + "trades"
        super().__init__(
            client,
            methods,
            url_path,
        )
        self._get_resp_decoder = msgspec.json.Decoder(list[AsterTrade])

    class GetParameters(msgspec.Struct, omit_defaults=True, frozen=True):
        """
        GET parameters for recent trades.

        Parameters
        ----------
        symbol : AsterSymbol
            The trading pair.
        limit : int, optional
            The limit for the response. Default 500; max 1000.

        """

        symbol: AsterSymbol
        limit: int | None = None

    async def get(self, params: GetParameters) -> list[AsterTrade]:
        method_type = HttpMethod.GET
        raw = await self._method(method_type, params)
        return self._get_resp_decoder.decode(raw)


class AsterHistoricalTradesHttp(AsterHttpEndpoint):
    """
    Endpoint of older market historical trades.

    `GET /api/v3/historicalTrades`
    `GET /fapi/v1/historicalTrades`
    `GET /dapi/v1/historicalTrades`

    References
    ----------
    https://docs.asterdex.com/spot/en/#old-trade-lookup-market_data
    https://docs.asterdex.com/futures/en/#old-trades-lookup-market_data
    https://docs.asterdex.com/delivery/en/#old-trades-lookup-market_data

    """

    def __init__(
        self,
        client: AsterHttpClient,
        base_endpoint: str,
    ):
        methods = {
            HttpMethod.GET: AsterSecurityType.MARKET_DATA,
        }
        url_path = base_endpoint + "historicalTrades"
        super().__init__(
            client,
            methods,
            url_path,
        )
        self._get_resp_decoder = msgspec.json.Decoder(list[AsterTrade])

    class GetParameters(msgspec.Struct, omit_defaults=True, frozen=True):
        """
        GET parameters for historical trades.

        Parameters
        ----------
        symbol : AsterSymbol
            The trading pair.
        limit : int, optional
            The limit for the response. Default 500; max 1000.
        fromId : int, optional
            Trade ID to fetch from. Default gets most recent trades

        """

        symbol: AsterSymbol
        limit: int | None = None
        fromId: int | None = None

    async def get(self, params: GetParameters) -> list[AsterTrade]:
        method_type = HttpMethod.GET
        raw = await self._method(method_type, params)
        return self._get_resp_decoder.decode(raw)


class AsterAggTradesHttp(AsterHttpEndpoint):
    """
    Endpoint of compressed and aggregated market trades. Market trades that fill in
    100ms with the same price and same taking side will have the quantity aggregated.

    `GET /api/v3/aggTrades`
    `GET /fapi/v1/aggTrades`
    `GET /dapi/v1/aggTrades`

    References
    ----------
    https://docs.asterdex.com/spot/en/#compressed-aggregate-trades-list
    https://docs.asterdex.com/futures/en/#compressed-aggregate-trades-list
    https://docs.asterdex.com/delivery/en/#compressed-aggregate-trades-list

    """

    def __init__(
        self,
        client: AsterHttpClient,
        base_endpoint: str,
    ):
        methods = {
            HttpMethod.GET: AsterSecurityType.NONE,
        }
        url_path = base_endpoint + "aggTrades"
        super().__init__(
            client,
            methods,
            url_path,
        )
        self._get_resp_decoder = msgspec.json.Decoder(list[AsterAggTrade])

    class GetParameters(msgspec.Struct, omit_defaults=True, frozen=True):
        """
        GET parameters for aggregate trades.

        Parameters
        ----------
        symbol : AsterSymbol
            The trading pair.
        limit : int, optional
            The limit for the response. Default 500; max 1000.
        fromId : int, optional
            Trade ID to fetch from INCLUSIVE.
        startTime : int, optional
            Timestamp in ms to get aggregate trades from INCLUSIVE.
        endTime : int, optional
            Timestamp in ms to get aggregate trades until INCLUSIVE.

        """

        symbol: AsterSymbol
        limit: int | None = None
        fromId: int | None = None
        startTime: int | None = None
        endTime: int | None = None

    async def get(self, params: GetParameters) -> list[AsterAggTrade]:
        method_type = HttpMethod.GET
        raw = await self._method(method_type, params)
        return self._get_resp_decoder.decode(raw)


class AsterKlinesHttp(AsterHttpEndpoint):
    """
    Endpoint of Kline/candlestick bars for a symbol. Klines are uniquely identified by
    their open time.

    `GET /api/v3/klines`
    `GET /fapi/v1/klines`
    `GET /dapi/v1/klines`

    References
    ----------
    https://docs.asterdex.com/spot/en/#kline-candlestick-data
    https://docs.asterdex.com/futures/en/#kline-candlestick-data
    https://docs.asterdex.com/delivery/en/#kline-candlestick-data

    """

    def __init__(
        self,
        client: AsterHttpClient,
        base_endpoint: str,
    ):
        methods = {
            HttpMethod.GET: AsterSecurityType.NONE,
        }
        url_path = base_endpoint + "klines"
        super().__init__(
            client,
            methods,
            url_path,
        )
        self._get_resp_decoder = msgspec.json.Decoder(list[AsterKline])

    class GetParameters(msgspec.Struct, omit_defaults=True, frozen=True):
        """
        GET parameters for klines.

        Parameters
        ----------
        symbol : AsterSymbol
            The trading pair.
        interval : str
            The interval of kline, e.g 1m, 5m, 1h, 1d, etc.
        limit : int, optional
            The limit for the response. Default 500; max 1000.
        startTime : int, optional
            Timestamp in ms to get klines from INCLUSIVE.
        endTime : int, optional
            Timestamp in ms to get klines until INCLUSIVE.

        """

        symbol: AsterSymbol
        interval: AsterKlineInterval
        limit: int | None = None
        startTime: int | None = None
        endTime: int | None = None

    async def get(self, params: GetParameters) -> list[AsterKline]:
        method_type = HttpMethod.GET
        raw = await self._method(method_type, params)
        return self._get_resp_decoder.decode(raw)


class AsterTicker24hrHttp(AsterHttpEndpoint):
    """
    Endpoint of 24-hour rolling window price change statistics.

    `GET /api/v3/ticker/24hr`
    `GET /fapi/v1/ticker/24hr`
    `GET /dapi/v1/ticker/24hr`

    Warnings
    --------
    Care should be taken when accessing this endpoint with no symbol specified.
    The weight usage can be very large, which may cause rate limits to be hit.

    References
    ----------
    https://docs.asterdex.com/spot/en/#24hr-ticker-price-change-statistics
    https://docs.asterdex.com/futures/en/#24hr-ticker-price-change-statistics
    https://docs.asterdex.com/delivery/en/#24hr-ticker-price-change-statistics

    """

    def __init__(
        self,
        client: AsterHttpClient,
        base_endpoint: str,
    ):
        methods = {
            HttpMethod.GET: AsterSecurityType.NONE,
        }
        url_path = base_endpoint + "ticker/24hr"
        super().__init__(
            client,
            methods,
            url_path,
        )
        self._get_obj_resp_decoder = msgspec.json.Decoder(AsterTicker24hr)
        self._get_arr_resp_decoder = msgspec.json.Decoder(list[AsterTicker24hr])

    class GetParameters(msgspec.Struct, omit_defaults=True, frozen=True):
        """
        GET parameters for 24hr ticker.

        Parameters
        ----------
        symbol : AsterSymbol
            The trading pair. When given, endpoint will return a single AsterTicker24hr
            When omitted, endpoint will return a list of AsterTicker24hr for all trading pairs.
        symbols : AsterSymbols
            SPOT/MARGIN only!
            List of trading pairs. When given, endpoint will return a list of AsterTicker24hr.
        type : str
            SPOT/MARGIN only!
            Select between FULL and MINI 24hr ticker responses to save bandwidth.

        """

        symbol: AsterSymbol | None = None
        symbols: AsterSymbols | None = None  # SPOT/MARGIN only
        type: str | None = None  # SPOT/MARIN only

    async def _get(self, params: GetParameters) -> list[AsterTicker24hr]:
        method_type = HttpMethod.GET
        raw = await self._method(method_type, params)
        if params.symbol is not None:
            return [self._get_obj_resp_decoder.decode(raw)]
        else:
            return self._get_arr_resp_decoder.decode(raw)


class AsterTickerPriceHttp(AsterHttpEndpoint):
    """
    Endpoint of latest price for a symbol or symbols.

    `GET /api/v3/ticker/price`
    `GET /fapi/v2/ticker/price`
    `GET /dapi/v1/ticker/price`

    References
    ----------
    https://docs.asterdex.com/spot/en/#symbol-price-ticker
    https://developers.aster.com/docs/derivatives/usds-margined-futures/market-data/rest-api/Symbol-Price-Ticker-v2
    https://docs.asterdex.com/delivery/en/#symbol-price-ticker

    Notes
    -----
    USDT-margined futures uses v2 (v1 deprecated). Coin-margined remains on v1.

    """

    def __init__(
        self,
        client: AsterHttpClient,
        base_endpoint: str,
    ):
        methods = {
            HttpMethod.GET: AsterSecurityType.NONE,
        }
        # Only USDT-margined futures has v2, coin-margined still uses v1
        endpoint = base_endpoint.replace("/fapi/v1/", "/fapi/v2/")
        url_path = endpoint + "ticker/price"
        super().__init__(
            client,
            methods,
            url_path,
        )
        self._get_obj_resp_decoder = msgspec.json.Decoder(AsterTickerPrice)
        self._get_arr_resp_decoder = msgspec.json.Decoder(list[AsterTickerPrice])

    class GetParameters(msgspec.Struct, omit_defaults=True, frozen=True):
        """
        GET parameters for price ticker.

        Parameters
        ----------
        symbol : AsterSymbol
            The trading pair. When given, endpoint will return a single AsterTickerPrice.
            When omitted, endpoint will return a list of AsterTickerPrice for all trading pairs.
        symbols : str
            SPOT/MARGIN only!
            List of trading pairs. When given, endpoint will return a list of AsterTickerPrice.

        """

        symbol: AsterSymbol | None = None
        symbols: AsterSymbols | None = None  # SPOT/MARGIN only

    async def _get(self, params: GetParameters) -> list[AsterTickerPrice]:
        method_type = HttpMethod.GET
        raw = await self._method(method_type, params)
        if params.symbol is not None:
            return [self._get_obj_resp_decoder.decode(raw)]
        else:
            return self._get_arr_resp_decoder.decode(raw)


class AsterTickerBookHttp(AsterHttpEndpoint):
    """
    Endpoint of best price/qty on the order book for a symbol or symbols.

    `GET /api/v3/ticker/bookTicker`
    `GET /fapi/v1/ticker/bookTicker`
    `GET /dapi/v1/ticker/bookTicker`

    References
    ----------
    https://docs.asterdex.com/spot/en/#symbol-order-book-ticker
    https://docs.asterdex.com/futures/en/#symbol-order-book-ticker
    https://docs.asterdex.com/delivery/en/#symbol-order-book-ticker

    """

    def __init__(
        self,
        client: AsterHttpClient,
        base_endpoint: str,
    ):
        methods = {
            HttpMethod.GET: AsterSecurityType.NONE,
        }
        url_path = base_endpoint + "ticker/bookTicker"
        super().__init__(
            client,
            methods,
            url_path,
        )
        self._get_arr_resp_decoder = msgspec.json.Decoder(list[AsterTickerBook])
        self._get_obj_resp_decoder = msgspec.json.Decoder(AsterTickerBook)

    class GetParameters(msgspec.Struct, omit_defaults=True, frozen=True):
        """
        GET parameters for order book ticker.

        Parameters
        ----------
        symbol : str
            The trading pair. When given, endpoint will return a single AsterTickerBook
            When omitted, endpoint will return a list of AsterTickerBook for all trading pairs.
        symbols : str
            SPOT/MARGIN only!
            List of trading pairs. When given, endpoint will return a list of AsterTickerBook.

        """

        symbol: AsterSymbol | None = None
        symbols: AsterSymbols | None = None  # SPOT/MARGIN only

    async def _get(self, params: GetParameters) -> list[AsterTickerBook]:
        method_type = HttpMethod.GET
        raw = await self._method(method_type, params)
        if params.symbol is not None:
            return [self._get_obj_resp_decoder.decode(raw)]
        else:
            return self._get_arr_resp_decoder.decode(raw)


class AsterMarketHttpAPI:
    """
    Provides access to the Aster Market HTTP REST API.

    Parameters
    ----------
    client : AsterHttpClient
        The Aster REST API client.
    account_type : AsterAccountType
        The Aster account type, used to select the endpoint prefix.

    Warnings
    --------
    This class should not be used directly, but through a concrete subclass.

    """

    def __init__(
        self,
        client: AsterHttpClient,
        account_type: AsterAccountType,
    ):
        PyCondition.not_none(client, "client")
        self.client = client

        if account_type.is_spot_or_margin:
            self.base_endpoint = "/api/v3/"
        elif account_type == AsterAccountType.USDT_FUTURES:
            self.base_endpoint = "/fapi/v1/"
        elif account_type == AsterAccountType.COIN_FUTURES:
            self.base_endpoint = "/dapi/v1/"
        else:
            raise RuntimeError(  # pragma: no cover (design-time error)
                f"invalid `AsterAccountType`, was {account_type}",  # pragma: no cover
            )

        # Create Endpoints
        self._endpoint_ping = AsterPingHttp(client, self.base_endpoint)
        self._endpoint_time = AsterTimeHttp(client, self.base_endpoint)
        self._endpoint_depth = AsterDepthHttp(client, self.base_endpoint)
        self._endpoint_trades = AsterTradesHttp(client, self.base_endpoint)
        self._endpoint_historical_trades = AsterHistoricalTradesHttp(client, self.base_endpoint)
        self._endpoint_agg_trades = AsterAggTradesHttp(client, self.base_endpoint)
        self._endpoint_klines = AsterKlinesHttp(client, self.base_endpoint)
        self._endpoint_ticker_24hr = AsterTicker24hrHttp(client, self.base_endpoint)
        self._endpoint_ticker_price = AsterTickerPriceHttp(client, self.base_endpoint)
        self._endpoint_ticker_book = AsterTickerBookHttp(client, self.base_endpoint)

    async def ping(self) -> dict:
        """
        Ping Aster REST API.
        """
        return await self._endpoint_ping.get()

    async def request_server_time(self) -> int:
        """
        Request server time from Aster.
        """
        response = await self._endpoint_time.get()
        return response.serverTime

    async def query_depth(
        self,
        symbol: str,
        limit: int | None = None,
    ) -> AsterDepth:
        """
        Query order book depth for a symbol.
        """
        return await self._endpoint_depth.get(
            params=self._endpoint_depth.GetParameters(
                symbol=AsterSymbol(symbol),
                limit=limit,
            ),
        )

    async def request_order_book_snapshot(
        self,
        instrument_id: InstrumentId,
        ts_init: int,
        limit: int | None = None,
    ) -> OrderBookDeltas:
        """
        Request snapshot of order book depth.
        """
        depth = await self.query_depth(instrument_id.symbol.value, limit)
        return depth.parse_to_order_book_snapshot(
            instrument_id=instrument_id,
            ts_init=ts_init,
        )

    async def query_trades(
        self,
        symbol: str,
        limit: int | None = None,
    ) -> list[AsterTrade]:
        """
        Query trades for symbol.
        """
        return await self._endpoint_trades.get(
            params=self._endpoint_trades.GetParameters(
                symbol=AsterSymbol(symbol),
                limit=limit,
            ),
        )

    async def request_trade_ticks(
        self,
        instrument_id: InstrumentId,
        limit: int | None = None,
    ) -> list[TradeTick]:
        """
        Request TradeTicks from Aster.
        """
        trades = await self.query_trades(instrument_id.symbol.value, limit)
        return [
            trade.parse_to_trade_tick(
                instrument_id=instrument_id,
            )
            for trade in trades
        ]

    async def query_agg_trades(
        self,
        symbol: str,
        limit: int | None = None,
        start_time: int | None = None,
        end_time: int | None = None,
        from_id: int | None = None,
    ) -> list[AsterAggTrade]:
        """
        Query aggregated trades for symbol.
        """
        return await self._endpoint_agg_trades.get(
            params=self._endpoint_agg_trades.GetParameters(
                symbol=AsterSymbol(symbol),
                limit=limit,
                startTime=start_time,
                endTime=end_time,
                fromId=from_id,
            ),
        )

    async def request_agg_trade_ticks(
        self,
        instrument_id: InstrumentId,
        limit: int | None = 1000,
        start_time: int | None = None,
        end_time: int | None = None,
        from_id: int | None = None,
    ) -> list[TradeTick]:
        """
        Request TradeTicks from Aster aggregated trades.

        If start_time and end_time are both specified, will request *all* TradeTicks in
        the interval, making multiple requests if necessary.

        """
        ticks: list[TradeTick] = []
        next_start_time = start_time

        if end_time is None:
            end_time = sys.maxsize

        if from_id is not None and (start_time or end_time) is not None:
            raise RuntimeError(
                "Cannot specify both fromId and startTime or endTime.",
            )

        # Only split into separate requests if both start_time and end_time are specified
        max_interval = (1000 * 60 * 60) - 1  # 1ms under an hour, as specified in Futures docs.
        last_id = 0
        interval_limited = False

        def _calculate_next_end_time(start_time: int, end_time: int) -> tuple[int, bool]:
            next_interval = start_time + max_interval
            interval_limited = next_interval < end_time
            next_end_time = next_interval if interval_limited is True else end_time
            return next_end_time, interval_limited

        if start_time is not None and end_time is not None:
            next_end_time, interval_limited = _calculate_next_end_time(start_time, end_time)
        else:
            next_end_time = end_time

        while True:
            response = await self.query_agg_trades(
                instrument_id.symbol.value,
                limit,
                start_time=next_start_time,
                end_time=next_end_time,
                from_id=from_id,
            )

            for trade in response:
                if not trade.a > last_id:
                    # Skip duplicate trades
                    continue
                ticks.append(
                    trade.parse_to_trade_tick(
                        instrument_id=instrument_id,
                    ),
                )

            if limit and len(response) < limit and interval_limited is False:
                # end loop regardless when limit is not hit
                break
            if (
                start_time is None
                or end_time is None
                or next_end_time >= nanos_to_millis(time.time_ns())
            ):
                break
            else:
                last = response[-1]
                last_id = last.a
                next_start_time = last.T
                next_end_time, interval_limited = _calculate_next_end_time(
                    next_start_time,
                    end_time,
                )
                continue

        return ticks

    async def query_historical_trades(
        self,
        symbol: str,
        limit: int | None = None,
        from_id: int | None = None,
    ) -> list[AsterTrade]:
        """
        Query historical trades for symbol.
        """
        return await self._endpoint_historical_trades.get(
            params=self._endpoint_historical_trades.GetParameters(
                symbol=AsterSymbol(symbol),
                limit=limit,
                fromId=from_id,
            ),
        )

    async def request_historical_trade_ticks(
        self,
        instrument_id: InstrumentId,
        limit: int | None = None,
        from_id: int | None = None,
    ) -> list[TradeTick]:
        """
        Request historical TradeTicks from Aster.
        """
        historical_trades = await self.query_historical_trades(
            symbol=instrument_id.symbol.value,
            limit=limit,
            from_id=from_id,
        )
        return [
            trade.parse_to_trade_tick(
                instrument_id=instrument_id,
            )
            for trade in historical_trades
        ]

    async def query_klines(
        self,
        symbol: str,
        interval: AsterKlineInterval,
        limit: int | None = None,
        start_time: int | None = None,
        end_time: int | None = None,
    ) -> list[AsterKline]:
        """
        Query klines for a symbol over an interval.
        """
        return await self._endpoint_klines.get(
            params=self._endpoint_klines.GetParameters(
                symbol=AsterSymbol(symbol),
                interval=interval,
                limit=limit,
                startTime=start_time,
                endTime=end_time,
            ),
        )

    async def request_aster_bars(
        self,
        bar_type: BarType,
        interval: AsterKlineInterval,
        limit: int | None = None,
        start_time: int | None = None,
        end_time: int | None = None,
    ) -> list[AsterBar]:
        """
        Request Aster Bars from Klines.
        """
        end_time_ms = int(end_time) if end_time is not None else sys.maxsize
        all_bars: list[AsterBar] = []
        while True:
            klines = await self.query_klines(
                symbol=bar_type.instrument_id.symbol.value,
                interval=interval,
                limit=limit,
                start_time=start_time,
                end_time=end_time,
            )
            bars: list[AsterBar] = [kline.parse_to_aster_bar(bar_type) for kline in klines]
            all_bars.extend(bars)

            # Update the start_time to fetch the next set of bars
            if klines:
                next_start_time = klines[-1].open_time + 1
            else:
                # Handle the case when klines is empty
                break

            # No more bars to fetch
            if (limit and len(klines) < limit) or next_start_time >= end_time_ms:
                break

            start_time = next_start_time

        return all_bars

    async def query_ticker_24hr(
        self,
        symbol: str | None = None,
        symbols: list[str] | None = None,
        response_type: str | None = None,
    ) -> list[AsterTicker24hr]:
        """
        Query 24hr ticker for symbol or symbols.
        """
        if symbol is not None and symbols is not None:
            raise RuntimeError(
                "Cannot specify both symbol and symbols parameters.",
            )
        return await self._endpoint_ticker_24hr._get(
            params=self._endpoint_ticker_24hr.GetParameters(
                symbol=AsterSymbol(symbol) if symbol else None,
                symbols=AsterSymbols(symbols) if symbols else None,
                type=response_type,
            ),
        )

    async def query_ticker_price(
        self,
        symbol: str | None = None,
        symbols: list[str] | None = None,
    ) -> list[AsterTickerPrice]:
        """
        Query price ticker for symbol or symbols.
        """
        if symbol is not None and symbols is not None:
            raise RuntimeError(
                "Cannot specify both symbol and symbols parameters.",
            )
        return await self._endpoint_ticker_price._get(
            params=self._endpoint_ticker_price.GetParameters(
                symbol=AsterSymbol(symbol) if symbol else None,
                symbols=AsterSymbols(symbols) if symbols else None,
            ),
        )

    async def query_ticker_book(
        self,
        symbol: str | None = None,
        symbols: list[str] | None = None,
    ) -> list[AsterTickerBook]:
        """
        Query book ticker for symbol or symbols.
        """
        if symbol is not None and symbols is not None:
            raise RuntimeError(
                "Cannot specify both symbol and symbols parameters.",
            )
        return await self._endpoint_ticker_book._get(
            params=self._endpoint_ticker_book.GetParameters(
                symbol=AsterSymbol(symbol) if symbol else None,
                symbols=AsterSymbols(symbols) if symbols else None,
            ),
        )
