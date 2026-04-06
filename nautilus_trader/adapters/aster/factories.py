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
import warnings
from functools import lru_cache

from nautilus_trader.adapters.aster.common.credentials import get_api_key
from nautilus_trader.adapters.aster.common.credentials import get_api_secret
from nautilus_trader.adapters.aster.common.credentials import is_ed25519_private_key
from nautilus_trader.adapters.aster.common.enums import AsterAccountType
from nautilus_trader.adapters.aster.common.enums import AsterEnvironment
from nautilus_trader.adapters.aster.common.enums import AsterKeyType
from nautilus_trader.adapters.aster.common.urls import get_http_base_url
from nautilus_trader.adapters.aster.common.urls import get_ws_base_url
from nautilus_trader.adapters.aster.config import AsterDataClientConfig
from nautilus_trader.adapters.aster.config import AsterExecClientConfig
from nautilus_trader.adapters.aster.config import AsterInstrumentProviderConfig
from nautilus_trader.adapters.aster.futures.data import AsterFuturesDataClient
from nautilus_trader.adapters.aster.futures.execution import AsterFuturesExecutionClient
from nautilus_trader.adapters.aster.futures.providers import AsterFuturesInstrumentProvider
from nautilus_trader.adapters.aster.http.client import AsterHttpClient
from nautilus_trader.adapters.aster.spot.data import AsterSpotDataClient
from nautilus_trader.adapters.aster.spot.execution import AsterSpotExecutionClient
from nautilus_trader.adapters.aster.spot.providers import AsterSpotInstrumentProvider
from nautilus_trader.cache.cache import Cache
from nautilus_trader.common.component import LiveClock
from nautilus_trader.common.component import MessageBus
from nautilus_trader.config import InstrumentProviderConfig
from nautilus_trader.core.nautilus_pyo3 import Quota
from nautilus_trader.live.factories import LiveDataClientFactory
from nautilus_trader.live.factories import LiveExecClientFactory
from nautilus_trader.model.identifiers import Venue


def _resolve_environment(
    environment: AsterEnvironment | None,
    testnet: bool,
) -> AsterEnvironment:
    if environment is not None and testnet:
        raise ValueError(
            "Cannot set both `environment` and `testnet`. "
            "Use `environment` only (`testnet` is deprecated).",
        )

    if testnet:
        warnings.warn(
            "`testnet` is deprecated, use `environment=AsterEnvironment.TESTNET` instead.",
            DeprecationWarning,
            stacklevel=3,
        )
        return AsterEnvironment.TESTNET

    return environment or AsterEnvironment.LIVE


@lru_cache(1)
def get_cached_aster_http_client(
    clock: LiveClock,
    account_type: AsterAccountType,
    api_key: str | None = None,
    api_secret: str | None = None,
    key_type: AsterKeyType = AsterKeyType.HMAC,
    base_url: str | None = None,
    environment: AsterEnvironment = AsterEnvironment.LIVE,
    is_us: bool = False,
    proxy_url: str | None = None,
) -> AsterHttpClient:
    """
    Cache and return a Aster HTTP client with the given key and secret.

    If a cached client with matching parameters already exists, the cached client will be returned.

    Parameters
    ----------
    clock : LiveClock
        The clock for the client.
    account_type : AsterAccountType
        The account type for the client.
    api_key : str, optional
        The API key for the client.
        If ``None``, the client will work for public market data only.
    api_secret : str, optional
        The API secret for the client.
        If ``None``, the client will work for public market data only.
    key_type : AsterKeyType, default 'HMAC'
        The private key cryptographic algorithm type.
    base_url : str, optional
        The base URL for the API endpoints.
    environment : AsterEnvironment, default LIVE
        The Aster environment.
    is_us : bool, default False
        If the client is connecting to Aster US.
    proxy_url : str, optional
        The proxy URL for HTTP requests.

    Returns
    -------
    AsterHttpClient

    """
    default_http_base_url = get_http_base_url(account_type, environment, is_us)

    # Determine key type: honor explicit RSA/ED25519, otherwise auto-detect
    rsa_private_key = None
    ed25519_private_key = None

    if key_type == AsterKeyType.RSA:
        rsa_private_key = api_secret
    elif key_type == AsterKeyType.ED25519 or (api_secret and is_ed25519_private_key(api_secret)):
        ed25519_private_key = api_secret

    # Set up rate limit quotas
    global_key = "aster:global"

    if account_type.is_spot:
        # Spot
        global_quota = Quota.rate_per_minute(6000)
        ratelimiter_default_quota = global_quota
        ratelimiter_quotas: list[tuple[str, Quota]] = [
            (global_key, global_quota),
            ("aster:api/v3/order", Quota.rate_per_minute(3000)),
            ("aster:api/v3/allOrders", Quota.rate_per_minute(int(3000 / 20))),
            ("aster:api/v3/klines", Quota.rate_per_minute(600)),
        ]
    else:
        # Futures
        global_quota = Quota.rate_per_minute(2400)
        ratelimiter_default_quota = global_quota
        ratelimiter_quotas = [
            (global_key, global_quota),
            ("aster:fapi/v1/order", Quota.rate_per_minute(1200)),
            ("aster:fapi/v1/allOrders", Quota.rate_per_minute(int(1200 / 20))),
            ("aster:fapi/v1/commissionRate", Quota.rate_per_minute(int(2400 / 20))),
            ("aster:fapi/v1/klines", Quota.rate_per_minute(600)),
        ]

    return AsterHttpClient(
        clock=clock,
        api_key=api_key,
        api_secret=api_secret,
        rsa_private_key=rsa_private_key,
        ed25519_private_key=ed25519_private_key,
        base_url=base_url or default_http_base_url,
        ratelimiter_quotas=ratelimiter_quotas,
        ratelimiter_default_quota=ratelimiter_default_quota,
        proxy_url=proxy_url,
    )


@lru_cache(1)
def get_cached_aster_spot_instrument_provider(
    client: AsterHttpClient,
    clock: LiveClock,
    account_type: AsterAccountType,
    environment: AsterEnvironment,
    config: InstrumentProviderConfig,
    venue: Venue,
) -> AsterSpotInstrumentProvider:
    """
    Cache and return an instrument provider for the Aster Spot/Margin exchange.

    If a cached provider already exists, then that provider will be returned.

    Parameters
    ----------
    client : AsterHttpClient
        The client for the instrument provider.
    clock : LiveClock
        The clock for the instrument provider.
    account_type : AsterAccountType
        The Aster account type for the instrument provider.
    environment : AsterEnvironment
        The Aster environment.
    config : InstrumentProviderConfig
        The configuration for the instrument provider.
    venue : Venue
        The venue for the instrument provider.

    Returns
    -------
    AsterSpotInstrumentProvider

    """
    return AsterSpotInstrumentProvider(
        client=client,
        clock=clock,
        account_type=account_type,
        environment=environment,
        config=config,
        venue=venue,
    )


@lru_cache(1)
def get_cached_aster_futures_instrument_provider(
    client: AsterHttpClient,
    clock: LiveClock,
    account_type: AsterAccountType,
    config: InstrumentProviderConfig | AsterInstrumentProviderConfig,
    venue: Venue,
) -> AsterFuturesInstrumentProvider:
    """
    Cache and return an instrument provider for the Aster Futures exchange.

    If a cached provider already exists, then that provider will be returned.

    Parameters
    ----------
    client : AsterHttpClient
        The client for the instrument provider.
    clock : LiveClock
        The clock for the instrument provider.
    account_type : AsterAccountType
        The Aster account type for the instrument provider.
    config : InstrumentProviderConfig | AsterInstrumentProviderConfig
        The configuration for the instrument provider.
    venue : Venue
        The venue for the instrument provider.

    Returns
    -------
    AsterFuturesInstrumentProvider

    """
    return AsterFuturesInstrumentProvider(
        client=client,
        clock=clock,
        account_type=account_type,
        config=config,
        venue=venue,
    )


class AsterLiveDataClientFactory(LiveDataClientFactory):
    """
    Provides a Aster live data client factory.
    """

    @staticmethod
    def create(  # type: ignore
        loop: asyncio.AbstractEventLoop,
        name: str,
        config: AsterDataClientConfig,
        msgbus: MessageBus,
        cache: Cache,
        clock: LiveClock,
    ) -> AsterSpotDataClient | AsterFuturesDataClient:
        """
        Create a new Aster data client.

        Parameters
        ----------
        loop : asyncio.AbstractEventLoop
            The event loop for the client.
        name : str
            The custom client ID.
        config : AsterDataClientConfig
            The client configuration.
        msgbus : MessageBus
            The message bus for the client.
        cache : Cache
            The cache for the client.
        clock : LiveClock
            The clock for the client.

        Returns
        -------
        AsterSpotDataClient or AsterFuturesDataClient

        Raises
        ------
        ValueError
            If `config.account_type` is not a valid `AsterAccountType`.

        """
        environment = _resolve_environment(config.environment, config.testnet)

        # Get HTTP client singleton
        client: AsterHttpClient = get_cached_aster_http_client(
            clock=clock,
            account_type=config.account_type,
            api_key=config.api_key,
            api_secret=config.api_secret,
            key_type=config.key_type,
            base_url=config.base_url_http,
            environment=environment,
            is_us=config.us,
            proxy_url=config.proxy_url,
        )

        default_base_url_ws: str = get_ws_base_url(
            account_type=config.account_type,
            environment=environment,
            is_us=config.us,
        )

        provider: AsterSpotInstrumentProvider | AsterFuturesInstrumentProvider

        if config.account_type.is_spot_or_margin:
            # Get instrument provider singleton
            provider = get_cached_aster_spot_instrument_provider(
                client=client,
                clock=clock,
                account_type=config.account_type,
                environment=environment,
                config=config.instrument_provider,
                venue=config.venue,
            )

            return AsterSpotDataClient(
                loop=loop,
                client=client,
                msgbus=msgbus,
                cache=cache,
                clock=clock,
                instrument_provider=provider,
                account_type=config.account_type,
                base_url_ws=config.base_url_ws or default_base_url_ws,
                name=name,
                config=config,
            )
        else:
            # Get instrument provider singleton
            provider = get_cached_aster_futures_instrument_provider(
                client=client,
                clock=clock,
                account_type=config.account_type,
                config=config.instrument_provider,
                venue=config.venue,
            )

            return AsterFuturesDataClient(
                loop=loop,
                client=client,
                msgbus=msgbus,
                cache=cache,
                clock=clock,
                instrument_provider=provider,
                account_type=config.account_type,
                base_url_ws=config.base_url_ws or default_base_url_ws,
                name=name,
                config=config,
            )


class AsterLiveExecClientFactory(LiveExecClientFactory):
    """
    Provides a Aster live execution client factory.
    """

    @staticmethod
    def create(  # type: ignore
        loop: asyncio.AbstractEventLoop,
        name: str,
        config: AsterExecClientConfig,
        msgbus: MessageBus,
        cache: Cache,
        clock: LiveClock,
    ) -> AsterSpotExecutionClient | AsterFuturesExecutionClient:
        """
        Create a new Aster execution client.

        Parameters
        ----------
        loop : asyncio.AbstractEventLoop
            The event loop for the client.
        name : str
            The custom client ID.
        config : AsterExecClientConfig
            The configuration for the client.
        msgbus : MessageBus
            The message bus for the client.
        cache : Cache
            The cache for the client.
        clock : LiveClock
            The clock for the client.

        Returns
        -------
        AsterExecutionClient

        Raises
        ------
        ValueError
            If `config.account_type` is not a valid `AsterAccountType`.

        """
        if config.key_type == AsterKeyType.RSA:
            raise ValueError(
                "RSA keys are not supported for Aster execution clients. "
                "Use Ed25519 or HMAC keys instead.",
            )

        environment = _resolve_environment(config.environment, config.testnet)

        api_key = config.api_key or get_api_key(config.account_type, environment)
        api_secret = config.api_secret or get_api_secret(config.account_type, environment)

        # Get HTTP client singleton
        client: AsterHttpClient = get_cached_aster_http_client(
            clock=clock,
            account_type=config.account_type,
            api_key=api_key,
            api_secret=api_secret,
            key_type=config.key_type,
            base_url=config.base_url_http,
            environment=environment,
            is_us=config.us,
            proxy_url=config.proxy_url,
        )

        default_base_url_ws: str = get_ws_base_url(
            account_type=config.account_type,
            environment=environment,
            is_us=config.us,
        )

        provider: AsterSpotInstrumentProvider | AsterFuturesInstrumentProvider

        if config.account_type.is_spot or config.account_type.is_margin:
            # Get instrument provider singleton
            provider = get_cached_aster_spot_instrument_provider(
                client=client,
                clock=clock,
                account_type=config.account_type,
                environment=environment,
                config=config.instrument_provider,
                venue=config.venue,
            )

            return AsterSpotExecutionClient(
                loop=loop,
                client=client,
                msgbus=msgbus,
                cache=cache,
                clock=clock,
                instrument_provider=provider,
                base_url_ws=config.base_url_ws or default_base_url_ws,
                account_type=config.account_type,
                name=name,
                config=config,
                environment=environment,
                api_key=api_key,
                api_secret=api_secret,
            )
        else:
            # Get instrument provider singleton
            provider = get_cached_aster_futures_instrument_provider(
                client=client,
                clock=clock,
                account_type=config.account_type,
                config=config.instrument_provider,
                venue=config.venue,
            )

            return AsterFuturesExecutionClient(
                loop=loop,
                client=client,
                msgbus=msgbus,
                cache=cache,
                clock=clock,
                instrument_provider=provider,
                base_url_ws=config.base_url_ws or default_base_url_ws,
                account_type=config.account_type,
                name=name,
                config=config,
                environment=environment,
                api_key=api_key,
                api_secret=api_secret,
            )
