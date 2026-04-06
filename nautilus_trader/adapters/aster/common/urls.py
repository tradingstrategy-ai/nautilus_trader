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

from nautilus_trader.adapters.aster.common.enums import AsterAccountType
from nautilus_trader.adapters.aster.common.enums import AsterEnvironment


def get_http_base_url(  # noqa: C901 (URL dispatch)
    account_type: AsterAccountType,
    environment: AsterEnvironment,
    is_us: bool,
) -> str:
    if environment == AsterEnvironment.TESTNET:
        if account_type.is_spot_or_margin:
            return "https://testnet.aster.vision"
        elif (
            account_type == AsterAccountType.USDT_FUTURES
            or account_type == AsterAccountType.COIN_FUTURES
        ):
            return "https://testnet.asterfuture.com"
        else:
            raise RuntimeError(  # pragma: no cover (design-time error)
                f"invalid `AsterAccountType`, was {account_type}",  # pragma: no cover
            )

    if environment == AsterEnvironment.DEMO:
        if account_type.is_spot_or_margin:
            return "https://demo-api.aster.com"
        elif account_type == AsterAccountType.USDT_FUTURES:
            return "https://demo-fapi.asterdex.com"
        elif account_type == AsterAccountType.COIN_FUTURES:
            return "https://testnet.asterfuture.com"
        else:
            raise RuntimeError(  # pragma: no cover (design-time error)
                f"invalid `AsterAccountType`, was {account_type}",  # pragma: no cover
            )

    top_level_domain: str = "us" if is_us else "com"

    if account_type.is_spot:
        return f"https://api.aster.{top_level_domain}"
    elif account_type.is_margin:
        return f"https://sapi.aster.{top_level_domain}"
    elif account_type == AsterAccountType.USDT_FUTURES:
        return f"https://fapi.aster.{top_level_domain}"
    elif account_type == AsterAccountType.COIN_FUTURES:
        return f"https://dapi.aster.{top_level_domain}"
    else:
        raise RuntimeError(  # pragma: no cover (design-time error)
            f"invalid `AsterAccountType`, was {account_type}",  # pragma: no cover
        )


def get_ws_api_base_url(  # noqa: C901 (URL dispatch)
    account_type: AsterAccountType,
    environment: AsterEnvironment,
    is_us: bool,
) -> str:
    """
    Return the WebSocket API base URL for user data streams.

    This is the new authenticated WebSocket API endpoint that replaces listenKey.

    """
    if environment == AsterEnvironment.TESTNET:
        if account_type.is_spot_or_margin:
            return "wss://ws-api.testnet.aster.vision/ws-api/v3"
        elif account_type == AsterAccountType.USDT_FUTURES:
            return "wss://testnet.asterfuture.com/ws-fapi/v1"
        elif account_type == AsterAccountType.COIN_FUTURES:
            raise ValueError("no WS API testnet for COIN-M futures")
        else:
            raise RuntimeError(
                f"invalid `AsterAccountType`, was {account_type}",
            )

    if environment == AsterEnvironment.DEMO:
        if account_type.is_spot_or_margin:
            return "wss://demo-ws-api.aster.com/ws-api/v3"
        elif account_type == AsterAccountType.USDT_FUTURES:
            # Futures demo uses same WS API as futures testnet
            return "wss://testnet.asterfuture.com/ws-fapi/v1"
        elif account_type == AsterAccountType.COIN_FUTURES:
            raise ValueError("no WS API demo for COIN-M futures")
        else:
            raise RuntimeError(
                f"invalid `AsterAccountType`, was {account_type}",
            )

    top_level_domain: str = "us" if is_us else "com"

    if account_type.is_spot_or_margin:
        return f"wss://ws-api.aster.{top_level_domain}:443/ws-api/v3"
    elif account_type == AsterAccountType.USDT_FUTURES:
        return f"wss://ws-fapi.aster.{top_level_domain}/ws-fapi/v1"
    elif account_type == AsterAccountType.COIN_FUTURES:
        return f"wss://ws-dapi.aster.{top_level_domain}/ws-dapi/v1"
    else:
        raise RuntimeError(
            f"invalid `AsterAccountType`, was {account_type}",
        )


def get_ws_base_url(  # noqa: C901 (URL dispatch)
    account_type: AsterAccountType,
    environment: AsterEnvironment,
    is_us: bool,
) -> str:
    if environment == AsterEnvironment.TESTNET:
        if account_type.is_spot_or_margin:
            return "wss://stream.testnet.aster.vision"
        elif account_type == AsterAccountType.USDT_FUTURES:
            return "wss://stream.asterfuture.com"
        elif account_type == AsterAccountType.COIN_FUTURES:
            return "wss://dstream.asterfuture.com"
        else:
            raise RuntimeError(  # pragma: no cover (design-time error)
                f"invalid `AsterAccountType`, was {account_type}",  # pragma: no cover
            )

    if environment == AsterEnvironment.DEMO:
        if account_type.is_spot_or_margin:
            return "wss://demo-stream.aster.com"
        elif account_type == AsterAccountType.USDT_FUTURES:
            # Futures demo uses same WS URLs as futures testnet
            return "wss://stream.asterfuture.com"
        elif account_type == AsterAccountType.COIN_FUTURES:
            return "wss://dstream.asterfuture.com"
        else:
            raise RuntimeError(  # pragma: no cover (design-time error)
                f"invalid `AsterAccountType`, was {account_type}",  # pragma: no cover
            )

    top_level_domain: str = "us" if is_us else "com"

    if account_type.is_spot_or_margin:
        return f"wss://stream.aster.{top_level_domain}:9443"
    elif account_type == AsterAccountType.USDT_FUTURES:
        return f"wss://fstream.aster.{top_level_domain}"
    elif account_type == AsterAccountType.COIN_FUTURES:
        return f"wss://dstream.aster.{top_level_domain}"
    else:
        raise RuntimeError(
            f"invalid `AsterAccountType`, was {account_type}",
        )  # pragma: no cover (design-time error)
