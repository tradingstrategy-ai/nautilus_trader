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

import asyncio
from collections.abc import Awaitable
from collections.abc import Callable
from decimal import Decimal

from nautilus_trader.adapters.aster.common.constants import ASTER_MAX_CALLBACK_RATE
from nautilus_trader.adapters.aster.common.constants import ASTER_MIN_CALLBACK_RATE
from nautilus_trader.adapters.aster.common.constants import ASTER_PRICE_MATCH_ORDER_TYPES
from nautilus_trader.adapters.aster.common.constants import ASTER_PRICE_MATCH_VALUES
from nautilus_trader.adapters.aster.common.constants import ASTER_RETRY_WARNINGS
from nautilus_trader.adapters.aster.common.constants import ASTER_POST_ONLY_REJECT_MSG
from nautilus_trader.adapters.aster.common.credentials import is_ed25519_private_key
from nautilus_trader.adapters.aster.common.enums import AsterAccountType
from nautilus_trader.adapters.aster.common.enums import AsterEnumParser
from nautilus_trader.adapters.aster.common.enums import AsterEnvironment
from nautilus_trader.adapters.aster.common.enums import AsterErrorCode
from nautilus_trader.adapters.aster.common.enums import AsterFuturesPositionSide
from nautilus_trader.adapters.aster.common.enums import AsterKeyType
from nautilus_trader.adapters.aster.common.enums import AsterTimeInForce
from nautilus_trader.adapters.aster.common.schemas.account import AsterOrder
from nautilus_trader.adapters.aster.common.schemas.account import AsterUserTrade
from nautilus_trader.adapters.aster.common.symbol import AsterSymbol
from nautilus_trader.adapters.aster.common.urls import get_ws_api_base_url
from nautilus_trader.adapters.aster.common.urls import get_ws_base_url
from nautilus_trader.adapters.aster.config import AsterExecClientConfig
from nautilus_trader.adapters.aster.http.account import AsterAccountHttpAPI
from nautilus_trader.adapters.aster.http.client import AsterHttpClient
from nautilus_trader.adapters.aster.http.error import AsterError
from nautilus_trader.adapters.aster.http.error import get_aster_error_code
from nautilus_trader.adapters.aster.http.error import should_retry
from nautilus_trader.adapters.aster.http.market import AsterMarketHttpAPI
from nautilus_trader.adapters.aster.websocket.user import AsterUserDataWebSocketClient
from nautilus_trader.cache.cache import Cache
from nautilus_trader.common.component import LiveClock
from nautilus_trader.common.component import MessageBus
from nautilus_trader.common.enums import LogColor
from nautilus_trader.common.enums import LogLevel
from nautilus_trader.common.providers import InstrumentProvider
from nautilus_trader.core.correctness import PyCondition
from nautilus_trader.core.datetime import nanos_to_millis
from nautilus_trader.core.datetime import secs_to_millis
from nautilus_trader.core.uuid import UUID4
from nautilus_trader.execution.messages import CancelAllOrders
from nautilus_trader.execution.messages import CancelOrder
from nautilus_trader.execution.messages import GenerateFillReports
from nautilus_trader.execution.messages import GenerateOrderStatusReport
from nautilus_trader.execution.messages import GenerateOrderStatusReports
from nautilus_trader.execution.messages import GeneratePositionStatusReports
from nautilus_trader.execution.messages import ModifyOrder
from nautilus_trader.execution.messages import QueryAccount
from nautilus_trader.execution.messages import SubmitOrder
from nautilus_trader.execution.messages import SubmitOrderList
from nautilus_trader.execution.reports import FillReport
from nautilus_trader.execution.reports import OrderStatusReport
from nautilus_trader.execution.reports import PositionStatusReport
from nautilus_trader.live.execution_client import LiveExecutionClient
from nautilus_trader.live.retry import RetryManagerPool
from nautilus_trader.model.enums import AccountType
from nautilus_trader.model.enums import OmsType
from nautilus_trader.model.enums import OrderSide
from nautilus_trader.model.enums import OrderStatus
from nautilus_trader.model.enums import OrderType
from nautilus_trader.model.enums import PositionSide
from nautilus_trader.model.enums import TrailingOffsetType
from nautilus_trader.model.enums import TriggerType
from nautilus_trader.model.enums import order_side_to_str
from nautilus_trader.model.enums import trailing_offset_type_to_str
from nautilus_trader.model.enums import trigger_type_to_str
from nautilus_trader.model.identifiers import AccountId
from nautilus_trader.model.identifiers import ClientId
from nautilus_trader.model.identifiers import ClientOrderId
from nautilus_trader.model.identifiers import InstrumentId
from nautilus_trader.model.identifiers import PositionId
from nautilus_trader.model.identifiers import Symbol
from nautilus_trader.model.identifiers import VenueOrderId
from nautilus_trader.model.objects import Price
from nautilus_trader.model.objects import Quantity
from nautilus_trader.model.orders import LimitOrder
from nautilus_trader.model.orders import MarketOrder
from nautilus_trader.model.orders import Order
from nautilus_trader.model.orders import StopLimitOrder
from nautilus_trader.model.orders import StopMarketOrder
from nautilus_trader.model.orders import TrailingStopMarketOrder
from nautilus_trader.model.position import Position


class AsterCommonExecutionClient(LiveExecutionClient):
    """
    Execution client providing common functionality for the Aster exchanges.

    Parameters
    ----------
    loop : asyncio.AbstractEventLoop
        The event loop for the client.
    client : AsterHttpClient
        The Aster HTTP client.
    account : AsterAccountHttpAPI
        The Aster Account HTTP API.
    market : AsterMarketHttpAPI
        The Aster Market HTTP API.
    enum_parser : AsterEnumParser
        The parser for Aster enums.
    msgbus : MessageBus
        The message bus for the client.
    cache : Cache
        The cache for the client.
    clock : LiveClock
        The clock for the client.
    instrument_provider : AsterSpotInstrumentProvider
        The instrument provider.
    account_type : AsterAccountType
        The account type for the client.
    base_url_ws : str
        The base URL for the WebSocket client (unused, kept for backward compatibility).
    name : str, optional
        The custom client ID.
    config : AsterExecClientConfig
        The configuration for the client.
    environment : AsterEnvironment
        The resolved Aster environment.
    api_key : str
        The resolved Aster API key.
    api_secret : str
        The resolved Aster API secret.

    Warnings
    --------
    This class should not be used directly, but through a concrete subclass.

    """

    def __init__(
        self,
        loop: asyncio.AbstractEventLoop,
        client: AsterHttpClient,
        account: AsterAccountHttpAPI,
        market: AsterMarketHttpAPI,
        enum_parser: AsterEnumParser,
        msgbus: MessageBus,
        cache: Cache,
        clock: LiveClock,
        instrument_provider: InstrumentProvider,
        account_type: AsterAccountType,
        base_url_ws: str,
        name: str | None,
        config: AsterExecClientConfig,
        environment: AsterEnvironment,
        api_key: str,
        api_secret: str,
    ) -> None:
        super().__init__(
            loop=loop,
            client_id=ClientId(name or config.venue.value),
            venue=config.venue,
            oms_type=OmsType.HEDGING if account_type.is_futures else OmsType.NETTING,
            instrument_provider=instrument_provider,
            account_type=AccountType.CASH if account_type.is_spot else AccountType.MARGIN,
            base_currency=None,
            msgbus=msgbus,
            cache=cache,
            clock=clock,
        )

        # Configuration
        self._aster_account_type: AsterAccountType = account_type
        self._use_gtd: bool = config.use_gtd
        self._use_reduce_only: bool = config.use_reduce_only
        self._use_position_ids: bool = config.use_position_ids
        self._treat_expired_as_canceled: bool = config.treat_expired_as_canceled
        self._log_rejected_due_post_only_as_warning: bool = (
            config.log_rejected_due_post_only_as_warning
        )
        self._recv_window = config.recv_window_ms
        self._max_retries = config.max_retries or 3
        self._log.info(f"Account type: {self._aster_account_type.value}", LogColor.BLUE)
        self._log.info(f"{config.use_gtd=}", LogColor.BLUE)
        self._log.info(f"{config.use_reduce_only=}", LogColor.BLUE)
        self._log.info(f"{config.use_position_ids=}", LogColor.BLUE)
        self._log.info(f"{config.treat_expired_as_canceled=}", LogColor.BLUE)
        self._log.info(f"{config.recv_window_ms=}", LogColor.BLUE)
        self._log.info(f"{config.max_retries=}", LogColor.BLUE)
        self._log.info(f"{config.retry_delay_initial_ms=}", LogColor.BLUE)
        self._log.info(f"{config.retry_delay_max_ms=}", LogColor.BLUE)
        self._log.info(f"{config.log_rejected_due_post_only_as_warning=}", LogColor.BLUE)

        self._is_dual_side_position: bool | None = None  # Initialized on connection
        self._set_account_id(
            AccountId(f"{name or config.venue.value}-{self._aster_account_type.value}-master"),
        )

        self._enum_parser = enum_parser

        # HTTP API
        self._http_client = client
        self._http_account = account
        self._http_market = market

        ws_api_url = config.base_url_ws or get_ws_api_base_url(
            account_type=account_type,
            environment=environment,
            is_us=config.us,
        )

        # Futures events arrive on a separate stream (different endpoint)
        stream_base_url: str | None = None

        if account_type.is_futures:
            stream_base_url = config.base_url_ws_stream or get_ws_base_url(
                account_type=account_type,
                environment=environment,
                is_us=config.us,
            )

        # Force Ed25519 when explicitly configured, otherwise auto-detect
        if config.key_type == AsterKeyType.ED25519:
            is_ed25519 = True
        else:
            is_ed25519 = is_ed25519_private_key(api_secret)

        # Futures + HMAC needs REST listenKey fallback
        # (Aster Futures WS API session.logon only accepts Ed25519)
        http_client_for_ws: AsterHttpClient | None = None
        account_type_for_ws: AsterAccountType | None = None

        if account_type.is_futures and not is_ed25519:
            http_client_for_ws = client
            account_type_for_ws = account_type

        self._ws_client = AsterUserDataWebSocketClient(
            clock=clock,
            base_url=ws_api_url,
            handler=self._handle_user_ws_message,
            api_key=api_key,
            api_secret=api_secret,
            loop=self._loop,
            is_futures=account_type.is_futures,
            stream_base_url=stream_base_url,
            is_ed25519=is_ed25519,
            http_client=http_client_for_ws,
            account_type=account_type_for_ws,
        )

        self._submit_order_method: dict[
            OrderType,
            Callable[[Order, AsterFuturesPositionSide | None, str | None, bool], Awaitable[None]],
        ] = {
            OrderType.MARKET: self._submit_market_order,
            OrderType.LIMIT: self._submit_limit_order,
            OrderType.STOP_LIMIT: self._submit_stop_limit_order,
            OrderType.LIMIT_IF_TOUCHED: self._submit_stop_limit_order,
            OrderType.STOP_MARKET: self._submit_stop_market_order,
            OrderType.MARKET_IF_TOUCHED: self._submit_stop_market_order,
            OrderType.TRAILING_STOP_MARKET: self._submit_trailing_stop_market_order,
        }

        # Hot caches
        self._instrument_ids: dict[str, InstrumentId] = {}
        self._active_symbols_cache: tuple[str | None, set[str], list[AsterOrder]] | None = None
        self._generate_order_status_retries: dict[ClientOrderId, int] = {}
        # Aster DEX does not support algo orders (/fapi/v1/algo returns 404).
        # This set is kept empty for interface compatibility but is never populated.
        self._triggered_algo_order_ids: set[ClientOrderId] = set()

        self._retry_manager_pool = RetryManagerPool[None](
            pool_size=100,
            max_retries=config.max_retries or 0,
            delay_initial_ms=config.retry_delay_initial_ms or 1_000,
            delay_max_ms=config.retry_delay_max_ms or 10_000,
            backoff_factor=2,
            logger=self._log,
            exc_types=(AsterError,),
            retry_check=should_retry,
            error_logger=self._log_retry_error,
        )

        self._log.info(f"Base url HTTP {self._http_client.base_url}", LogColor.BLUE)
        self._log.info(f"Base url WebSocket {ws_api_url}", LogColor.BLUE)

    @property
    def use_position_ids(self) -> bool:
        """
        Whether a `position_id` will be assigned to order events generated by the
        client.

        Returns
        -------
        bool

        """
        return self._use_position_ids

    @property
    def treat_expired_as_canceled(self) -> bool:
        """
        Whether the `EXPIRED` execution type is treated as a `CANCEL`.

        Returns
        -------
        bool

        """
        return self._treat_expired_as_canceled

    def _stop(self) -> None:
        self._retry_manager_pool.shutdown()

    def _log_retry_error(self, message: str, exception: BaseException | None) -> None:
        error_code = get_aster_error_code(exception) if exception else None
        is_post_only = isinstance(exception, AsterError) and _is_post_only_rejection(exception)

        if is_post_only and not self._log_rejected_due_post_only_as_warning:
            self._log.info(message)
        elif is_post_only or error_code in ASTER_RETRY_WARNINGS:
            self._log.warning(message)
        else:
            self._log.error(message)

    async def _connect(self) -> None:
        await self._instrument_provider.initialize()
        await self._update_account_state()
        await self._await_account_registered()
        await self._init_dual_side_position()

        # Check Aster-Nautilus clock sync
        server_time: int = await self._http_market.request_server_time()
        self._log.info(f"Aster server time {server_time} UNIX (ms)")

        nautilus_time: int = self._clock.timestamp_ms()
        self._log.info(f"Nautilus clock time {nautilus_time} UNIX (ms)")

        await self._ws_client.connect()
        await self._ws_client.session_logon()
        await self._ws_client.subscribe_user_data_stream()

    async def _update_account_state(self) -> None:
        # Replace method in child class
        raise NotImplementedError

    async def _init_dual_side_position(self) -> None:
        # Replace method in child class
        raise NotImplementedError

    async def _disconnect(self) -> None:
        try:
            await self._ws_client.unsubscribe_user_data_stream()
        except Exception as e:
            self._log.warning(f"Error unsubscribing from user data stream: {e}")

        await self._ws_client.disconnect()

    async def generate_order_status_report(  # noqa: C901 (too complex)
        self,
        command: GenerateOrderStatusReport,
    ) -> OrderStatusReport | None:
        PyCondition.is_false(
            command.client_order_id is None and command.venue_order_id is None,
            "both `client_order_id` and `venue_order_id` were `None`",
        )

        retries = self._generate_order_status_retries.get(command.client_order_id, 0)
        if retries > self._max_retries:
            self._log.error(
                f"Reached maximum retries {self._max_retries}/{self._max_retries} for generating OrderStatusReport for "
                f"{repr(command.client_order_id) if command.client_order_id else ''} "
                f"{repr(command.venue_order_id) if command.venue_order_id else ''}",
            )

            # Clean up retry counter after max retries exceeded
            if (
                command.client_order_id
                and command.client_order_id in self._generate_order_status_retries
            ):
                del self._generate_order_status_retries[command.client_order_id]

            return None

        self._log.info(
            f"Generating OrderStatusReport for "
            f"{repr(command.client_order_id) if command.client_order_id else ''} "
            f"{repr(command.venue_order_id) if command.venue_order_id else ''}",
        )

        try:
            if command.venue_order_id:
                aster_order = await self._http_account.query_order(
                    symbol=command.instrument_id.symbol.value,
                    order_id=int(command.venue_order_id.value),
                )
            else:
                aster_order = await self._http_account.query_order(
                    symbol=command.instrument_id.symbol.value,
                    orig_client_order_id=(
                        command.client_order_id.value
                        if command.client_order_id is not None
                        else None
                    ),
                )
        except AsterError as e:
            retries += 1
            self._log.error(
                f"Cannot generate order status report for {command.client_order_id!r}: {e.message}. Retry {retries}/{self._max_retries}",
            )

            # Check for None before dictionary operations
            if not command.client_order_id:
                self._log.warning("Cannot retry without a client order ID")
                return None

            self._generate_order_status_retries[command.client_order_id] = retries

            order: Order | None = self._cache.order(command.client_order_id)
            if order is None:
                self._log.warning("Order not found in cache")
                return None
            elif order.is_closed:
                return None  # Nothing else to do

            if retries >= self._max_retries:
                if command.client_order_id in self._generate_order_status_retries:
                    del self._generate_order_status_retries[command.client_order_id]

                # Determine if the rejection was specifically due to a POST-ONLY order
                # that would have executed immediately as a taker (GTX_ORDER_REJECT -5022).
                due_post_only = _is_post_only_rejection(e)

                self.generate_order_rejected(
                    strategy_id=order.strategy_id,
                    instrument_id=command.instrument_id,
                    client_order_id=command.client_order_id,
                    reason=str(e.message),
                    ts_event=self._clock.timestamp_ns(),
                    due_post_only=due_post_only,
                )
            return None  # Error now handled

        if not aster_order or (aster_order.origQty and Decimal(aster_order.origQty) == 0):
            # Cannot proceed to generating report
            self._log.error(
                f"Cannot generate `OrderStatusReport` for {command.client_order_id=!r}, {command.venue_order_id=!r}: "
                "order not found",
            )
            return None

        report: OrderStatusReport = aster_order.parse_to_order_status_report(
            account_id=self.account_id,
            instrument_id=self._get_cached_instrument_id(aster_order.symbol),
            report_id=UUID4(),
            enum_parser=self._enum_parser,
            treat_expired_as_canceled=self._treat_expired_as_canceled,
            ts_init=self._clock.timestamp_ns(),
        )

        # Clean up retry counter on successful report generation
        if (
            command.client_order_id
            and command.client_order_id in self._generate_order_status_retries
        ):
            del self._generate_order_status_retries[command.client_order_id]

        self._log.debug(f"Received {report}")
        return report

    def _get_cache_active_symbols(self) -> set[str]:
        # Check cache for all active symbols
        open_orders: list[Order] = self._cache.orders_open(venue=self.venue)
        open_positions: list[Position] = self._cache.positions_open(venue=self.venue)
        active_symbols: set[str] = set()
        for o in open_orders:
            active_symbols.add(o.instrument_id.symbol.value)
        for p in open_positions:
            active_symbols.add(p.instrument_id.symbol.value)
        return active_symbols

    async def _get_aster_position_status_reports(
        self,
        symbol: str | None = None,
    ) -> list[PositionStatusReport]:
        # Implement in child class
        raise NotImplementedError

    async def _get_aster_active_position_symbols(
        self,
        symbol: str | None = None,
    ) -> set[str]:
        # Implement in child class
        raise NotImplementedError

    async def _build_active_symbols(
        self,
        symbol: str | None,
    ) -> tuple[set[str], list[AsterOrder]]:
        if self._active_symbols_cache is not None and self._active_symbols_cache[0] == symbol:
            return self._active_symbols_cache[1], self._active_symbols_cache[2]

        active_symbols = self._get_cache_active_symbols()
        active_symbols.update(await self._get_aster_active_position_symbols(symbol))
        open_orders = await self._http_account.query_open_orders(symbol)

        for order in open_orders:
            active_symbols.add(order.symbol)

        self._active_symbols_cache = (symbol, active_symbols, open_orders)

        return active_symbols, open_orders

    async def generate_order_status_reports(
        self,
        command: GenerateOrderStatusReports,
    ) -> list[OrderStatusReport]:
        self._log.debug("Requesting OrderStatusReports...")
        self._active_symbols_cache = None

        try:
            symbol = (
                command.instrument_id.symbol.value if command.instrument_id is not None else None
            )
            active_symbols, aster_open_orders = await self._build_active_symbols(symbol)

            # Get all orders for those active symbols
            aster_orders: list[AsterOrder] = []

            if command.open_only:
                aster_orders = aster_open_orders
            else:
                for active_symbol in active_symbols:
                    # Here we don't pass a `start_time` or `end_time` as order reports appear to go
                    # randomly missing when these are specified. We filter on the Nautilus side below.
                    # Explicitly setting limit to the max lookback of 1000, in the future we should
                    # add pagination.
                    response = await self._http_account.query_all_orders(
                        symbol=active_symbol,
                        limit=1_000,
                    )
                    aster_orders.extend(response)
        except AsterError as e:
            self._log.exception(f"Cannot generate OrderStatusReport: {e.message}", e)
            return []

        start_ms = secs_to_millis(command.start.timestamp()) if command.start is not None else None
        end_ms = secs_to_millis(command.end.timestamp()) if command.end is not None else None

        reports = self._parse_order_status_reports(aster_orders, start_ms, end_ms)

        self._log_report_receipt(
            len(reports),
            "OrderStatusReport",
            command.log_receipt_level,
        )

        return reports

    def _parse_order_status_reports(
        self,
        aster_orders: list[AsterOrder],
        start_ms: int | None,
        end_ms: int | None,
    ) -> list[OrderStatusReport]:
        reports: list[OrderStatusReport] = []
        for order in aster_orders:
            if start_ms is not None and order.time < start_ms:
                continue  # Filter start on the Nautilus side
            if end_ms is not None and order.time > end_ms:
                continue  # Filter end on the Nautilus side
            if order.origQty and Decimal(order.origQty) == 0:
                continue  # Cannot parse zero quantity order (filter for Aster)
            report = order.parse_to_order_status_report(
                account_id=self.account_id,
                instrument_id=self._get_cached_instrument_id(order.symbol),
                report_id=UUID4(),
                enum_parser=self._enum_parser,
                treat_expired_as_canceled=self._treat_expired_as_canceled,
                ts_init=self._clock.timestamp_ns(),
            )
            self._log.debug(f"Received {report}")
            reports.append(report)
        return reports

    async def generate_fill_reports(
        self,
        command: GenerateFillReports,
    ) -> list[FillReport]:
        self._log.debug("Requesting FillReports...")

        try:
            # Check Aster for all trades on active symbols
            symbol = (
                command.instrument_id.symbol.value if command.instrument_id is not None else None
            )
            active_symbols = self._get_cache_active_symbols()
            active_symbols.update(await self._get_aster_active_position_symbols(symbol))
            aster_trades: list[AsterUserTrade] = []

            for symbol in active_symbols:
                response = await self._http_account.query_user_trades(
                    symbol=symbol,
                    start_time=(
                        secs_to_millis(command.start.timestamp())
                        if command.start is not None
                        else None
                    ),
                    end_time=(
                        secs_to_millis(command.end.timestamp()) if command.end is not None else None
                    ),
                )
                aster_trades.extend(response)
        except AsterError as e:
            self._log.exception(f"Cannot generate FillReport: {e.message}", e)
            return []

        # Parse all Aster trades
        reports: list[FillReport] = []
        for trade in aster_trades:
            if trade.symbol is None:
                self._log.warning(f"No symbol for trade {trade}")
                continue
            report = trade.parse_to_fill_report(
                account_id=self.account_id,
                instrument_id=self._get_cached_instrument_id(trade.symbol),
                report_id=UUID4(),
                ts_init=self._clock.timestamp_ns(),
                use_position_ids=self._use_position_ids,
            )
            self._log.debug(f"Received {report}")
            reports.append(report)

        # Confirm sorting in ascending order
        reports = sorted(reports, key=lambda x: x.trade_id)

        self._log_report_receipt(len(reports), "FillReport", LogLevel.INFO)

        return reports

    async def generate_position_status_reports(
        self,
        command: GeneratePositionStatusReports,
    ) -> list[PositionStatusReport]:
        try:
            if command.instrument_id:
                self._log.info(f"Requesting PositionStatusReport for {command.instrument_id}")
                symbol = command.instrument_id.symbol.value
                reports = await self._get_aster_position_status_reports(symbol)
                if not reports:
                    now = self._clock.timestamp_ns()
                    report = PositionStatusReport(
                        account_id=self.account_id,
                        instrument_id=command.instrument_id,
                        position_side=PositionSide.FLAT,
                        quantity=Quantity.zero(),
                        report_id=UUID4(),
                        ts_last=now,
                        ts_init=now,
                    )
                    reports = [report]
            else:
                self._log.debug("Requesting PositionStatusReports...")
                reports = await self._get_aster_position_status_reports()
        except AsterError as e:
            self._log.exception(f"Cannot generate PositionStatusReport: {e.message}", e)
            return []

        self._log_report_receipt(
            len(reports),
            "PositionStatusReport",
            command.log_receipt_level,
        )

        return reports

    def _check_order_validity(self, order: Order) -> str | None:
        # Implement in child class
        raise NotImplementedError

    def _determine_time_in_force(self, order: Order) -> AsterTimeInForce:
        # Convert the internal TimeInForce enum to the Aster equivalent
        time_in_force: AsterTimeInForce = self._enum_parser.parse_internal_time_in_force(
            order.time_in_force,
        )

        # When the client is configured *not* to make use of the native GTD
        # (Good-Till-Date) support on Aster we transparently downgrade GTD to
        # GTC. Comparison must be performed against the *Aster* enum; the
        # previous implementation compared against the internal Nautilus enum
        # which would always evaluate to ``False`` and therefore never apply
        # the downgrade.
        if time_in_force == AsterTimeInForce.GTD and not self._use_gtd:
            time_in_force = AsterTimeInForce.GTC
            self._log.info(
                f"Converted GTD `time_in_force` to GTC for {order.client_order_id}",
                LogColor.BLUE,
            )
        return time_in_force

    def _determine_good_till_date(
        self,
        order: Order,
        time_in_force: AsterTimeInForce | None,
    ) -> int | None:
        if time_in_force is None or time_in_force != AsterTimeInForce.GTD:
            return None

        good_till_date = nanos_to_millis(order.expire_time_ns) if order.expire_time_ns else None

        if self._aster_account_type.is_spot_or_margin:
            good_till_date = None
            self._log.warning("Cannot set GTD time in force with `expiry_time` for Aster Spot")
        return good_till_date

    def _determine_reduce_only(self, order: Order) -> bool:
        return order.is_reduce_only if self._use_reduce_only else False

    def _determine_reduce_only_str(self, order: Order) -> str | None:
        # `reduceOnly` Cannot be sent in Futures Hedge Mode
        if self._aster_account_type.is_futures and not self._is_dual_side_position:
            return str(self._determine_reduce_only(order))
        return None

    def _get_position_side_from_position_id(
        self,
        position_id: PositionId | None,
        exec_spawn_id: ClientOrderId | None,
    ) -> AsterFuturesPositionSide | None:
        # Position ID must end with either 'LONG', 'SHORT' or 'BOTH' for Aster Futures Hedge position mode

        position_side = None

        if self._aster_account_type.is_spot_or_margin:  # Spot or Margin mode
            return position_side
        elif not self._is_dual_side_position:  # One-way position mode
            return AsterFuturesPositionSide.BOTH

        if position_id is None and exec_spawn_id is not None:
            position_id = self._cache.position_id(exec_spawn_id)

        # For Aster Futures Hedge mode, the position side must be specified in the position_id
        PyCondition.not_none(position_id, "position_id")
        position_side = self._enum_parser.parse_position_id_to_aster_futures_position_side(
            position_id,
        )
        # Check if the position side is valid
        PyCondition.is_in(
            position_side,
            [AsterFuturesPositionSide.LONG, AsterFuturesPositionSide.SHORT],
            "position_side",
            "HedgeModePositionSides",
        )
        return position_side

    async def _query_account(self, _command: QueryAccount) -> None:
        await self._update_account_state()

    async def _submit_order(self, command: SubmitOrder) -> None:
        position_side = self._get_position_side_from_position_id(
            position_id=command.position_id,
            exec_spawn_id=command.order.exec_spawn_id,
        )
        await self._submit_order_inner(command.order, position_side, command.params)

    async def _submit_order_inner(
        self,
        order: Order,
        position_side: AsterFuturesPositionSide | None,
        params: dict[str, object] | None = None,
    ) -> None:
        if order.is_closed:
            self._log.warning(f"Cannot submit already closed order {order}")
            return

        try:
            price_match = self._extract_price_match(order, params)
            close_position = self._extract_close_position(order, params)
        except ValueError as e:
            self._deny_order_pre_submit(order, str(e))
            return

        validation_error = self._validate_order_pre_submit(order)
        if validation_error:
            self._deny_order_pre_submit(order, validation_error)
            return

        self._log.debug(f"Submitting {order}, position_side={position_side}")

        # Generate event here to ensure correct ordering of events
        self.generate_order_submitted(
            strategy_id=order.strategy_id,
            instrument_id=order.instrument_id,
            client_order_id=order.client_order_id,
            ts_event=self._clock.timestamp_ns(),
        )

        retry_manager = await self._retry_manager_pool.acquire()
        try:
            await retry_manager.run(
                "submit_order",
                [order.client_order_id],
                self._submit_order_method[order.order_type],
                order,
                position_side,
                price_match,
                close_position,
            )

            if not retry_manager.result:
                # Determine if the rejection was specifically due to a POST-ONLY order
                # that would have executed immediately as a taker (GTX_ORDER_REJECT -5022).
                last_exc = retry_manager.last_exception
                due_post_only = (
                    _is_post_only_rejection(last_exc)
                    if isinstance(last_exc, AsterError)
                    else False
                )

                self.generate_order_rejected(
                    strategy_id=order.strategy_id,
                    instrument_id=order.instrument_id,
                    client_order_id=order.client_order_id,
                    reason=retry_manager.message,
                    ts_event=self._clock.timestamp_ns(),
                    due_post_only=due_post_only,
                )
        finally:
            await self._retry_manager_pool.release(retry_manager)

    def _extract_price_match(
        self,
        order: Order,
        params: dict[str, object] | None,
    ) -> str | None:
        if params is None:
            return None

        raw_value = params.get("price_match")
        if raw_value is None:
            return None

        if not self._aster_account_type.is_futures:
            raise ValueError(
                "UNSUPPORTED: `price_match` is only supported for Aster futures accounts",
            )

        if not isinstance(raw_value, str):
            raise ValueError(
                "INVALID_ARG: `price_match` must be provided as a string value",
            )

        value = raw_value.upper()
        if value not in ASTER_PRICE_MATCH_VALUES:
            raise ValueError(
                "INVALID_ARG: `price_match` value "
                f"{raw_value!r} is not one of {sorted(ASTER_PRICE_MATCH_VALUES)}",
            )

        if order.is_post_only:
            raise ValueError(
                "UNSUPPORTED: `price_match` cannot be combined with post-only instructions on Aster",
            )

        display_qty = getattr(order, "display_qty", None)
        if display_qty is not None:
            raise ValueError(
                "UNSUPPORTED: `price_match` cannot be combined with iceberg/display quantities on Aster",
            )

        if order.order_type not in ASTER_PRICE_MATCH_ORDER_TYPES:
            raise ValueError(
                f"UNSUPPORTED: `price_match` is not supported for order type {order.type_string()} on Aster",
            )

        return value

    def _extract_close_position(
        self,
        order: Order,
        params: dict[str, object] | None,
    ) -> bool:
        if params is None:
            return False

        raw_value = params.get("close_position")
        if raw_value is None:
            return False

        if not self._aster_account_type.is_futures:
            raise ValueError(
                "UNSUPPORTED: `close_position` is only supported for Aster futures accounts",
            )

        if not isinstance(raw_value, bool):
            raise ValueError(
                "INVALID_ARG: `close_position` must be provided as a bool value",
            )

        if order.order_type not in (OrderType.STOP_MARKET, OrderType.MARKET_IF_TOUCHED):
            raise ValueError(
                f"UNSUPPORTED: `close_position` is not supported for order type {order.type_string()} on Aster",
            )

        if order.is_reduce_only:
            raise ValueError(
                "INVALID_ARG: `close_position` cannot be combined with `reduce_only` on Aster",
            )

        return raw_value

    def _deny_order_pre_submit(self, order: Order, reason: str) -> None:
        self.generate_order_denied(
            strategy_id=order.strategy_id,
            instrument_id=order.instrument_id,
            client_order_id=order.client_order_id,
            reason=reason,
            ts_event=self._clock.timestamp_ns(),
        )

    def _validate_order_pre_submit(self, order: Order) -> str | None:  # noqa: C901 (too complex)
        # Check order type and time-in-force validity
        validity_error = self._check_order_validity(order)
        if validity_error:
            return validity_error

        # Market order validations
        if isinstance(order, MarketOrder):
            if order.is_quote_quantity and not self._aster_account_type.is_spot_or_margin:
                return "UNSUPPORTED_QUOTE_QUANTITY"

        # Stop limit order validations
        elif isinstance(order, (StopLimitOrder, StopMarketOrder)):
            if not self._aster_account_type.is_spot_or_margin and order.trigger_type not in (
                TriggerType.DEFAULT,
                TriggerType.LAST_PRICE,
                TriggerType.MARK_PRICE,
            ):
                return f"INVALID_TRIGGER_TYPE: {trigger_type_to_str(order.trigger_type)}"

        # Trailing stop market order validations
        elif isinstance(order, TrailingStopMarketOrder):
            if order.trigger_type not in (
                TriggerType.DEFAULT,
                TriggerType.LAST_PRICE,
                TriggerType.MARK_PRICE,
            ):
                return f"INVALID_TRIGGER_TYPE: {trigger_type_to_str(order.trigger_type)}"

            if order.trailing_offset_type != TrailingOffsetType.BASIS_POINTS:
                return f"INVALID_TRAILING_OFFSET_TYPE: {trailing_offset_type_to_str(order.trailing_offset_type)}"

            callback_rate = self._trailing_offset_to_callback_rate(order)

            if (
                callback_rate < ASTER_MIN_CALLBACK_RATE
                or callback_rate > ASTER_MAX_CALLBACK_RATE
            ):
                return f"INVALID_TRAILING_OFFSET: {callback_rate}% not in range [{ASTER_MIN_CALLBACK_RATE}, {ASTER_MAX_CALLBACK_RATE}]"

            if order.trigger_price is not None:
                return "INVALID_TRIGGER_PRICE: use activation_price for trailing stop orders"

        return None  # Valid

    async def _submit_market_order(
        self,
        order: MarketOrder,
        position_side: AsterFuturesPositionSide | None,
        price_match: str | None,
        close_position: bool,
    ) -> None:
        assert price_match is None  # type checking

        if order.is_quote_quantity:
            quantity = None
            quote_order_qty = str(order.quantity)
        else:
            quantity = str(order.quantity)
            quote_order_qty = None

        await self._http_account.new_order(
            symbol=order.instrument_id.symbol.value,
            side=self._enum_parser.parse_internal_order_side(order.side),
            order_type=self._enum_parser.parse_internal_order_type(order),
            quantity=quantity,
            quote_order_qty=quote_order_qty,
            reduce_only=self._determine_reduce_only_str(order),
            new_client_order_id=order.client_order_id.value,
            recv_window=str(self._recv_window),
            position_side=position_side,
        )

    async def _submit_limit_order(
        self,
        order: LimitOrder,
        position_side: AsterFuturesPositionSide | None,
        price_match: str | None,
        close_position: bool,
    ) -> None:
        time_in_force = self._determine_time_in_force(order)
        if order.is_post_only and self._aster_account_type.is_spot_or_margin:
            time_in_force = None
        elif order.is_post_only and self._aster_account_type.is_futures:
            time_in_force = AsterTimeInForce.GTX

        await self._http_account.new_order(
            symbol=order.instrument_id.symbol.value,
            side=self._enum_parser.parse_internal_order_side(order.side),
            order_type=self._enum_parser.parse_internal_order_type(order),
            time_in_force=time_in_force,
            good_till_date=self._determine_good_till_date(order, time_in_force),
            quantity=str(order.quantity),
            price=None if price_match else str(order.price),
            iceberg_qty=str(order.display_qty) if order.display_qty is not None else None,
            reduce_only=self._determine_reduce_only_str(order),
            new_client_order_id=order.client_order_id.value,
            recv_window=str(self._recv_window),
            position_side=position_side,
            price_match=price_match,
        )

    async def _submit_stop_limit_order(
        self,
        order: StopLimitOrder,
        position_side: AsterFuturesPositionSide | None,
        price_match: str | None,
        close_position: bool,
    ) -> None:
        if self._aster_account_type.is_spot_or_margin:
            working_type = None
        elif order.trigger_type in (TriggerType.DEFAULT, TriggerType.LAST_PRICE):
            working_type = "CONTRACT_PRICE"
        elif order.trigger_type == TriggerType.MARK_PRICE:
            working_type = "MARK_PRICE"
        else:
            raise RuntimeError(
                f"Unexpected trigger_type {trigger_type_to_str(order.trigger_type)} for StopLimitOrder - "
                "should have been validated in _validate_order_pre_submit",
            )

        time_in_force = self._determine_time_in_force(order)

        if self._aster_account_type.is_futures:
            await self._http_account.new_order(  # type: ignore [attr-defined]
                symbol=order.instrument_id.symbol.value,
                side=self._enum_parser.parse_internal_order_side(order.side),
                order_type=self._enum_parser.parse_internal_order_type(order),
                position_side=position_side,
                quantity=str(order.quantity),
                price=None if price_match else str(order.price),
                stop_price=str(order.trigger_price),
                time_in_force=time_in_force,
                working_type=working_type,
                price_match=price_match,
                reduce_only=self._determine_reduce_only_str(order),
                new_client_order_id=order.client_order_id.value,
                good_till_date=self._determine_good_till_date(order, time_in_force),
                recv_window=str(self._recv_window),
            )
        else:
            await self._http_account.new_order(
                symbol=order.instrument_id.symbol.value,
                side=self._enum_parser.parse_internal_order_side(order.side),
                order_type=self._enum_parser.parse_internal_order_type(order),
                time_in_force=time_in_force,
                good_till_date=self._determine_good_till_date(order, time_in_force),
                quantity=str(order.quantity),
                price=None if price_match else str(order.price),
                stop_price=str(order.trigger_price),
                working_type=working_type,
                iceberg_qty=str(order.display_qty) if order.display_qty is not None else None,
                reduce_only=self._determine_reduce_only_str(order),
                new_client_order_id=order.client_order_id.value,
                recv_window=str(self._recv_window),
                position_side=position_side,
                price_match=price_match,
            )

    async def _submit_order_list(self, command: SubmitOrderList) -> None:
        position_side = self._get_position_side_from_position_id(
            position_id=command.position_id,
            exec_spawn_id=None,
        )

        for order in command.order_list.orders:
            if order.linked_order_ids:
                # Deny all orders in the list if any have linked orders (OCO not supported)
                for list_order in command.order_list.orders:
                    self._deny_order_pre_submit(
                        list_order,
                        "UNSUPPORTED_OCO_CONDITIONAL_ORDERS",
                    )
                return

            await self._submit_order_inner(order, position_side, command.params)

    async def _submit_stop_market_order(
        self,
        order: StopMarketOrder,
        position_side: AsterFuturesPositionSide | None,
        price_match: str | None,
        close_position: bool,
    ) -> None:
        assert price_match is None  # type checking

        if self._aster_account_type.is_spot_or_margin:
            working_type = None
        elif order.trigger_type in (TriggerType.DEFAULT, TriggerType.LAST_PRICE):
            working_type = "CONTRACT_PRICE"
        elif order.trigger_type == TriggerType.MARK_PRICE:
            working_type = "MARK_PRICE"
        else:
            raise RuntimeError(
                f"Unexpected trigger_type {trigger_type_to_str(order.trigger_type)} for StopMarketOrder - "
                "should have been validated in _validate_order_pre_submit",
            )

        time_in_force = self._determine_time_in_force(order)

        if self._aster_account_type.is_futures:
            if close_position:
                # closePosition is mutually exclusive with quantity and reduceOnly
                await self._http_account.new_order(  # type: ignore [attr-defined]
                    symbol=order.instrument_id.symbol.value,
                    side=self._enum_parser.parse_internal_order_side(order.side),
                    order_type=self._enum_parser.parse_internal_order_type(order),
                    position_side=position_side,
                    close_position="true",
                    stop_price=str(order.trigger_price),
                    time_in_force=time_in_force,
                    working_type=working_type,
                    new_client_order_id=order.client_order_id.value,
                    good_till_date=self._determine_good_till_date(order, time_in_force),
                    recv_window=str(self._recv_window),
                )
            else:
                await self._http_account.new_order(  # type: ignore [attr-defined]
                    symbol=order.instrument_id.symbol.value,
                    side=self._enum_parser.parse_internal_order_side(order.side),
                    order_type=self._enum_parser.parse_internal_order_type(order),
                    position_side=position_side,
                    quantity=str(order.quantity),
                    stop_price=str(order.trigger_price),
                    time_in_force=time_in_force,
                    working_type=working_type,
                    reduce_only=self._determine_reduce_only_str(order),
                    new_client_order_id=order.client_order_id.value,
                    good_till_date=self._determine_good_till_date(order, time_in_force),
                    recv_window=str(self._recv_window),
                )
        else:
            await self._http_account.new_order(
                symbol=order.instrument_id.symbol.value,
                side=self._enum_parser.parse_internal_order_side(order.side),
                order_type=self._enum_parser.parse_internal_order_type(order),
                time_in_force=time_in_force,
                good_till_date=self._determine_good_till_date(order, time_in_force),
                quantity=str(order.quantity),
                stop_price=str(order.trigger_price),
                working_type=working_type,
                reduce_only=self._determine_reduce_only_str(order),
                new_client_order_id=order.client_order_id.value,
                recv_window=str(self._recv_window),
                position_side=position_side,
            )

    async def _submit_trailing_stop_market_order(
        self,
        order: TrailingStopMarketOrder,
        position_side: AsterFuturesPositionSide | None,
        price_match: str | None,
        close_position: bool,
    ) -> None:
        assert price_match is None  # type checking

        if order.trigger_type in (TriggerType.DEFAULT, TriggerType.LAST_PRICE):
            working_type = "CONTRACT_PRICE"
        elif order.trigger_type == TriggerType.MARK_PRICE:
            working_type = "MARK_PRICE"
        else:
            raise RuntimeError(
                f"Unexpected trigger_type {trigger_type_to_str(order.trigger_type)} for TrailingStopMarketOrder - "
                "should have been validated in _validate_order_pre_submit",
            )

        time_in_force = self._determine_time_in_force(order)

        callback_rate = self._trailing_offset_to_callback_rate(order)
        callback_rate_str = self._format_callback_rate(callback_rate)

        activation_price: Price | None = order.activation_price

        # TRAILING_STOP_MARKET is a futures-only order type
        await self._http_account.new_order(  # type: ignore [attr-defined]
            symbol=order.instrument_id.symbol.value,
            side=self._enum_parser.parse_internal_order_side(order.side),
            order_type=self._enum_parser.parse_internal_order_type(order),
            position_side=position_side,
            quantity=str(order.quantity),
            activation_price=str(activation_price) if activation_price is not None else None,
            callback_rate=callback_rate_str,
            time_in_force=time_in_force,
            working_type=working_type,
            reduce_only=self._determine_reduce_only_str(order),
            new_client_order_id=order.client_order_id.value,
            good_till_date=self._determine_good_till_date(order, time_in_force),
            recv_window=str(self._recv_window),
        )

    @staticmethod
    def _trailing_offset_to_callback_rate(order: TrailingStopMarketOrder) -> Decimal:
        return Decimal(order.trailing_offset) / Decimal(100)

    @staticmethod
    def _format_callback_rate(callback_rate: Decimal) -> str:
        if callback_rate == callback_rate.to_integral():
            return format(callback_rate.quantize(Decimal("0.1")), "f")

        return format(callback_rate.normalize(), "f")

    def _get_cached_instrument_id(self, symbol: str) -> InstrumentId:
        nautilus_symbol: str = AsterSymbol(symbol).parse_as_nautilus(
            self._aster_account_type,
        )
        instrument_id: InstrumentId | None = self._instrument_ids.get(nautilus_symbol)
        if not instrument_id:
            instrument_id = InstrumentId(Symbol(nautilus_symbol), self.venue)
            self._instrument_ids[nautilus_symbol] = instrument_id
        return instrument_id

    async def _modify_order(self, command: ModifyOrder) -> None:
        if self._aster_account_type.is_spot_or_margin:
            reason = "only supported for `USDT_FUTURES` and `COIN_FUTURES` account types"
            self._log.error(f"Cannot modify order: {reason}")
            self.generate_order_modify_rejected(
                command.strategy_id,
                command.instrument_id,
                command.client_order_id,
                command.venue_order_id,
                reason,
                self._clock.timestamp_ns(),
            )
            return

        order: Order | None = self._cache.order(command.client_order_id)
        if order is None:
            self._log.error(f"{command.client_order_id!r} not found to modify")
            return

        # Check if order can be modified via regular endpoint
        # Aster DEX does not support algo orders — all conditional orders go through
        # the standard /fapi/v1/order endpoint, so only LIMIT orders can be modified.
        is_limit = order.order_type == OrderType.LIMIT

        if not is_limit:
            reason = f"only LIMIT orders supported by the venue (was {order.type_string()})"
            self._log.error(f"Cannot modify order: {reason}")
            self.generate_order_modify_rejected(
                command.strategy_id,
                command.instrument_id,
                command.client_order_id,
                command.venue_order_id,
                reason,
                self._clock.timestamp_ns(),
            )
            return

        retry_manager = await self._retry_manager_pool.acquire()
        try:
            await retry_manager.run(
                "modify_order",
                [order.client_order_id, order.venue_order_id],
                self._http_account.modify_order,
                symbol=order.instrument_id.symbol.value,
                order_id=int(order.venue_order_id.value) if order.venue_order_id else None,
                side=self._enum_parser.parse_internal_order_side(order.side),
                quantity=str(command.quantity) if command.quantity else str(order.quantity),
                price=str(command.price) if command.price else str(order.price),
            )

            if not retry_manager.result:
                self.generate_order_modify_rejected(
                    command.strategy_id,
                    command.instrument_id,
                    command.client_order_id,
                    command.venue_order_id,
                    retry_manager.message,
                    self._clock.timestamp_ns(),
                )
        finally:
            await self._retry_manager_pool.release(retry_manager)

    async def _cancel_order(self, command: CancelOrder) -> None:
        retry_manager = await self._retry_manager_pool.acquire()
        try:
            await retry_manager.run(
                "cancel_order",
                [command.client_order_id, command.venue_order_id],
                self._cancel_order_single,
                instrument_id=command.instrument_id,
                client_order_id=command.client_order_id,
                venue_order_id=command.venue_order_id,
            )

            if not retry_manager.result:
                self.generate_order_cancel_rejected(
                    command.strategy_id,
                    command.instrument_id,
                    command.client_order_id,
                    command.venue_order_id,
                    retry_manager.message,
                    self._clock.timestamp_ns(),
                )
        finally:
            await self._retry_manager_pool.release(retry_manager)

    async def _cancel_orders_batch(
        self,
        instrument_id: InstrumentId,
        orders: list[Order],
    ) -> None:
        retry_manager = await self._retry_manager_pool.acquire()
        try:
            await retry_manager.run(
                "cancel_all_open_orders",
                [instrument_id],
                self._http_account.cancel_all_open_orders,
                symbol=instrument_id.symbol.value,
            )

            if not retry_manager.result:
                if (
                    retry_manager.message is not None
                    and "Unknown order sent" in retry_manager.message
                ):
                    self._log.info(
                        "No open orders to cancel according to Aster",
                        LogColor.GREEN,
                    )
                else:
                    for order in orders:
                        if order.is_closed:
                            continue
                        self.generate_order_cancel_rejected(
                            order.strategy_id,
                            order.instrument_id,
                            order.client_order_id,
                            order.venue_order_id,
                            retry_manager.message,
                            self._clock.timestamp_ns(),
                        )
        finally:
            await self._retry_manager_pool.release(retry_manager)

    async def _cancel_algo_orders_batch(
        self,
        instrument_id: InstrumentId,
        orders: list[Order],
    ) -> None:
        # Aster DEX does not support algo orders (/fapi/v1/algo returns 404).
        # All conditional orders go through the standard /fapi/v1/order endpoint.
        self._log.warning("Aster does not support algo orders — skipping algo batch cancel")

    async def _cancel_all_orders(self, command: CancelAllOrders) -> None:
        if command.order_side != OrderSide.NO_ORDER_SIDE:
            self._log.warning(
                f"Aster does not support order_side filtering for cancel all orders; "
                f"ignoring order_side={order_side_to_str(command.order_side)} and canceling all orders",
            )

        open_orders_strategy: list[Order] = self._cache.orders_open(
            instrument_id=command.instrument_id,
            strategy_id=command.strategy_id,
        )

        # Filter to only SUBMITTED since PENDING_CANCEL/UPDATE are already in orders_open
        inflight_orders_strategy: list[Order] = [
            o
            for o in self._cache.orders_inflight(
                instrument_id=command.instrument_id,
                strategy_id=command.strategy_id,
            )
            if o.status == OrderStatus.SUBMITTED
        ]

        all_strategy_orders = open_orders_strategy + inflight_orders_strategy

        # Count total orders across all strategies (for multi-strategy safety check)
        open_orders_total_count = self._cache.orders_open_count(
            instrument_id=command.instrument_id,
        )
        submitted_orders_total_count = sum(
            1
            for o in self._cache.orders_inflight(instrument_id=command.instrument_id)
            if o.status == OrderStatus.SUBMITTED
        )
        total_orders_count = open_orders_total_count + submitted_orders_total_count

        # Only use batch cancel if this strategy owns all orders for the instrument
        if total_orders_count == len(all_strategy_orders):
            # Aster DEX does not support algo orders (/fapi/v1/algo returns 404).
            # All conditional orders go through the standard /fapi/v1/order endpoint,
            # so we treat all orders as regular orders for cancellation.
            if all_strategy_orders:
                await self._cancel_orders_batch(command.instrument_id, all_strategy_orders)
            return

        # Not every order belongs to this strategy - cancel individually or in batches
        await self._cancel_orders_for_strategy(all_strategy_orders, command)

    async def _cancel_order_single(
        self,
        instrument_id: InstrumentId,
        client_order_id: ClientOrderId,
        venue_order_id: VenueOrderId | None,
    ) -> None:
        order: Order | None = self._cache.order(client_order_id)
        if order is None:
            # Cannot generate cancel rejected event without order in cache
            self._log.error(f"{client_order_id!r} not found to cancel")
            return

        if order.is_closed:
            self._log.warning(
                f"CancelOrder command for {client_order_id!r} when order already {order.status_string()} "
                "(will not send to exchange)",
            )
            return

        # Aster DEX does not support algo orders (/fapi/v1/algo returns 404).
        # All conditional orders go through the standard /fapi/v1/order endpoint.
        await self._http_account.cancel_order(
            symbol=instrument_id.symbol.value,
            order_id=int(venue_order_id.value) if venue_order_id else None,
            orig_client_order_id=client_order_id.value if client_order_id else None,
        )

    async def _cancel_orders_for_strategy(
        self,
        orders: list[Order],
        command: CancelAllOrders,
    ) -> None:
        await self._cancel_orders_individual(orders)

    async def _cancel_orders_individual(self, orders: list[Order]) -> None:
        for order in orders:
            retry_manager = await self._retry_manager_pool.acquire()
            try:
                await retry_manager.run(
                    "cancel_order",
                    [order.client_order_id, order.venue_order_id],
                    self._cancel_order_single,
                    instrument_id=order.instrument_id,
                    client_order_id=order.client_order_id,
                    venue_order_id=order.venue_order_id,
                )

                if not retry_manager.result:
                    self.generate_order_cancel_rejected(
                        order.strategy_id,
                        order.instrument_id,
                        order.client_order_id,
                        order.venue_order_id,
                        retry_manager.message,
                        self._clock.timestamp_ns(),
                    )
            finally:
                await self._retry_manager_pool.release(retry_manager)

    def _handle_user_ws_message(self, raw: bytes) -> None:
        # Implement in child class
        raise NotImplementedError


def _is_post_only_rejection(error: AsterError) -> bool:
    error_code = get_aster_error_code(error)
    if error_code == AsterErrorCode.GTX_ORDER_REJECT:
        return True
    if error_code == AsterErrorCode.NEW_ORDER_REJECTED:
        msg = _get_error_msg(error)
        return msg == ASTER_POST_ONLY_REJECT_MSG
    return False


def _get_error_msg(error: AsterError) -> str:
    if isinstance(error.message, dict):
        return error.message.get("msg", "")
    if isinstance(error.message, str):
        import json

        try:
            parsed = json.loads(error.message)
            if isinstance(parsed, dict):
                return parsed.get("msg", "")
        except (json.JSONDecodeError, ValueError):
            pass
    return ""
