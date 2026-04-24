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
    Cache and return an Aster HTTP client with the given key and secret.

    If a cached client with matching parameters already exists, the cached client will be returned.

    """
    default_http_base_url = get_http_base_url(account_type, environment, is_us)

    # Determine key type: honor explicit RSA/ED25519, otherwise auto-detect
    rsa_private_key = None
    ed25519_private_key = None

    if key_type == AsterKeyType.RSA:
        rsa_private_key = api_secret
    elif key_type == AsterKeyType.ED25519 or (api_secret and is_ed25519_private_key(api_secret)):
        ed25519_private_key = api_secret

    # Aster Futures rate limits (from spike validation)
    global_key = "aster:global"
    global_quota = Quota.rate_per_minute(2400)
    ratelimiter_default_quota = global_quota
    ratelimiter_quotas: list[tuple[str, Quota]] = [
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
    Provides an Aster live data client factory.

    Aster is futures-only — no spot support.
    """

    @staticmethod
    def create(  # type: ignore
        loop: asyncio.AbstractEventLoop,
        name: str,
        config: AsterDataClientConfig,
        msgbus: MessageBus,
        cache: Cache,
        clock: LiveClock,
    ) -> AsterFuturesDataClient:
        environment = _resolve_environment(config.environment, config.testnet)

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
    Provides an Aster live execution client factory.

    Aster is futures-only — no spot support.
    """

    @staticmethod
    def create(  # type: ignore
        loop: asyncio.AbstractEventLoop,
        name: str,
        config: AsterExecClientConfig,
        msgbus: MessageBus,
        cache: Cache,
        clock: LiveClock,
    ) -> AsterFuturesExecutionClient:
        if config.key_type == AsterKeyType.RSA:
            raise ValueError(
                "RSA keys are not supported for Aster execution clients. "
                "Use Ed25519 or HMAC keys instead.",
            )

        environment = _resolve_environment(config.environment, config.testnet)

        api_key = config.api_key or get_api_key(config.account_type, environment)
        api_secret = config.api_secret or get_api_secret(config.account_type, environment)

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
