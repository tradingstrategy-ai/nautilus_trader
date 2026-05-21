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

from nautilus_trader.adapters.aster.common.constants import ASTER_VENUE
from nautilus_trader.adapters.aster.common.enums import AsterAccountType
from nautilus_trader.adapters.aster.common.enums import AsterEnvironment
from nautilus_trader.adapters.aster.common.enums import AsterKeyType
from nautilus_trader.adapters.aster.common.symbol import AsterSymbol
from nautilus_trader.adapters.aster.futures.enums import AsterFuturesMarginType
from nautilus_trader.config import InstrumentProviderConfig
from nautilus_trader.config import LiveDataClientConfig
from nautilus_trader.config import LiveExecClientConfig
from nautilus_trader.config import PositiveInt
from nautilus_trader.model.identifiers import Venue


class AsterInstrumentProviderConfig(InstrumentProviderConfig, frozen=True):
    """
    Configuration for ``AsterInstrumentProvider`` instances.

    Parameters
    ----------
    load_all : bool, default False
        If all venue instruments should be loaded on start.
    load_ids : frozenset[InstrumentId], optional
        The list of instrument IDs to be loaded on start (if `load_all` is False).
    filters : frozendict or dict[str, Any], optional
        The venue specific instrument loading filters to apply.
    filter_callable: str, optional
        A fully qualified path to a callable that takes a single argument, `instrument` and returns a bool, indicating
        whether the instrument should be loaded
    log_warnings : bool, default True
        If parser warnings should be logged.
    query_commission_rates : bool, default False
        If commission rates should be queried per symbol from the exchange.
        When False, uses venue fee tier tables as fallback.
        Recommended for market maker accounts with negative maker fees.

    """

    def __eq__(self, other: object) -> bool:
        if other is None:
            return False
        if not isinstance(other, AsterInstrumentProviderConfig):
            return False
        return (
            self.load_all == other.load_all
            and self.load_ids == other.load_ids
            and self.filters == other.filters
            and self.query_commission_rates == other.query_commission_rates
        )

    def __hash__(self) -> int:
        filters = frozenset(self.filters.items()) if self.filters else None
        return hash((self.load_all, self.load_ids, filters, self.query_commission_rates))

    query_commission_rates: bool = False


class AsterDataClientConfig(LiveDataClientConfig, frozen=True):
    """
    Configuration for ``AsterDataClient`` instances.

    Parameters
    ----------
    venue : Venue, default ASTER_VENUE
        The venue for the client.
    api_key : str, optional
        The Aster API public key.
        If ``None``, the client will work for public market data only.
        Providing an API key may improve rate limits.
    api_secret : str, optional
        The Aster API secret key.
        If ``None``, the client will work for public market data only.
    key_type : AsterKeyType, default 'HMAC'
        Deprecated: key type is now auto-detected from the api_secret format.
        Only needed for RSA keys (set explicitly to ``AsterKeyType.RSA``).
    account_type : AsterAccountType, default AsterAccountType.SPOT
        The account type for the client.
    base_url_http : str, optional
        The HTTP client custom endpoint override.
    base_url_ws : str, optional
        The WebSocket client custom endpoint override.
    proxy_url : str, optional
        The proxy URL for HTTP requests.
    environment : AsterEnvironment, optional
        The Aster environment (LIVE, TESTNET, or DEMO). Defaults to LIVE.
    us : bool, default False
        If client is connecting to Aster US.
    testnet : bool, default False
        Deprecated: use ``environment`` instead.
    update_instruments_interval_mins: PositiveInt or None, default 60
        The interval (minutes) between reloading instruments from the venue.
    use_agg_trade_ticks : bool, default False
        Whether to use aggregated trade tick endpoints instead of raw trades.
        TradeId of ticks will be the Aggregate tradeId returned by Aster.
    local_addr : str, optional
        Optional local source IP address to bind outbound TCP connections to (REST + WS).
        When set, every outbound socket from this client originates from the given local
        IP — used to pin a single bot process to a specific source IP. Must be a valid
        IPv4 or IPv6 literal (e.g. ``"51.89.47.201"``). When ``None`` (default), the
        kernel selects the source IP from the routing table.

    """

    venue: Venue = ASTER_VENUE
    api_key: str | None = None
    api_secret: str | None = None
    key_type: AsterKeyType = AsterKeyType.HMAC
    account_type: AsterAccountType = AsterAccountType.USDT_FUTURES  # Aster is futures-only
    base_url_http: str | None = None
    base_url_ws: str | None = None
    proxy_url: str | None = None
    environment: AsterEnvironment | None = None
    us: bool = False
    testnet: bool = False
    update_instruments_interval_mins: PositiveInt | None = 60
    use_agg_trade_ticks: bool = False
    local_addr: str | None = None


class AsterExecClientConfig(LiveExecClientConfig, frozen=True):
    """
    Configuration for ``AsterExecutionClient`` instances.

    Parameters
    ----------
    venue : Venue, default ASTER_VENUE
        The venue for the client.
    api_key : str, optional
        The Aster API public key.
        If ``None`` then will source from `ASTER_API_KEY` (or testnet equivalent).
    api_secret : str, optional
        The Aster API secret key.
        If ``None`` then will source from `ASTER_API_SECRET` (or testnet equivalent).
    key_type : AsterKeyType, default 'HMAC'
        Deprecated: key type is now auto-detected from the api_secret format.
        Only needed for RSA keys (set explicitly to ``AsterKeyType.RSA``).
    account_type : AsterAccountType, default AsterAccountType.SPOT
        The account type for the client.
    base_url_http : str, optional
        The HTTP client custom endpoint override.
    base_url_ws : str, optional
        The WebSocket API client custom endpoint override.
    base_url_ws_stream : str, optional
        The WebSocket stream custom endpoint override for futures user data event delivery.
        Only applicable to futures account types. When ``None``, derived from the environment.
    proxy_url : str, optional
        The proxy URL for HTTP requests.
    environment : AsterEnvironment, optional
        The Aster environment (LIVE, TESTNET, or DEMO). Defaults to LIVE.
    us : bool, default False
        If client is connecting to Aster US.
    testnet : bool, default False
        Deprecated: use ``environment`` instead.
    use_gtd : bool, default False
        If GTD orders will use the Aster GTD TIF option.
        Aster does NOT support GTD, so this defaults to False (GTD remapped to GTC).
    use_reduce_only : bool, default True
        If the `reduce_only` execution instruction on orders is sent through to the exchange.
        If True, then will assign the value on orders sent to the exchange, otherwise will always be False.
    use_position_ids: bool, default True
        If Aster Futures hedging position IDs should be used.
        If False, then order event `position_id`(s) from the execution client will be `None`, which
        allows *virtual* positions with `OmsType.HEDGING`.
    use_trade_lite: bool, default False
        If TRADE_LITE events should be used.
        If True, commissions will be calculated based on the instrument's details.
    treat_expired_as_canceled : bool, default False
        If the `EXPIRED` execution type is semantically treated as `CANCELED`.
        Aster treats cancels with certain combinations of order type and time in force as expired
        events. This config option allows you to treat these uniformally as cancels.
    recv_window_ms : PositiveInt, default 5000
        The receive window (milliseconds) for Aster HTTP requests.
    max_retries : PositiveInt, optional
        The maximum number of times a submit, cancel or modify order request will be retried.
    retry_delay_initial_ms : PositiveInt, optional
        The initial delay (milliseconds) between retries. Short delays with frequent retries may result in account bans.
    retry_delay_max_ms : PositiveInt, optional
        The maximum delay (milliseconds) between retries.
    futures_leverages : dict[AsterSymbol, PositiveInt], optional
        The initial leverage to be used for each symbol. It's applicable to futures only.
    futures_margin_types : dict[AsterSymbol, AsterFuturesMarginType], optional
        Margin type (isolated or cross) to be used for each symbol. It's applicable to futures only.
    log_rejected_due_post_only_as_warning : bool, default True
        If order rejected events where `due_post_only` is True should be logged as warnings.

    Warnings
    --------
    A short `retry_delay` with frequent retries may result in account bans.

    """

    venue: Venue = ASTER_VENUE
    api_key: str | None = None
    api_secret: str | None = None
    key_type: AsterKeyType = AsterKeyType.HMAC
    account_type: AsterAccountType = AsterAccountType.USDT_FUTURES  # Aster is futures-only
    base_url_http: str | None = None
    base_url_ws: str | None = None
    base_url_ws_stream: str | None = None
    proxy_url: str | None = None
    environment: AsterEnvironment | None = None
    us: bool = False
    testnet: bool = False
    use_gtd: bool = False  # Aster does not support GTD
    use_reduce_only: bool = True
    use_position_ids: bool = True
    use_trade_lite: bool = False
    treat_expired_as_canceled: bool = False
    recv_window_ms: PositiveInt = 5_000
    max_retries: PositiveInt | None = None
    retry_delay_initial_ms: PositiveInt | None = None
    retry_delay_max_ms: PositiveInt | None = None
    futures_leverages: dict[AsterSymbol, PositiveInt] | None = None
    futures_margin_types: dict[AsterSymbol, AsterFuturesMarginType] | None = None
    log_rejected_due_post_only_as_warning: bool = True
    local_addr: str | None = None
