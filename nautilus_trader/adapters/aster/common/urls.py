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
Aster DEX API URL resolution.

Aster is futures-only. Validated URLs from spike (2026-04-06):
- REST:  https://fapi.asterdex.com
- WS:    wss://fstream.asterdex.com
- Testnet REST: https://fapi-testnet.asterdex.com (requires whitelist)
- Testnet WS:   wss://fstream-testnet.asterdex.com (requires whitelist)

"""

from nautilus_trader.adapters.aster.common.enums import AsterAccountType
from nautilus_trader.adapters.aster.common.enums import AsterEnvironment


def get_http_base_url(
    account_type: AsterAccountType,
    environment: AsterEnvironment,
    is_us: bool,
) -> str:
    # Aster is futures-only — reject spot/margin
    if account_type.is_spot_or_margin:
        raise ValueError(
            f"Aster does not support spot/margin trading, got account_type={account_type}",
        )

    if environment == AsterEnvironment.TESTNET:
        return "https://fapi-testnet.asterdex.com"

    if environment == AsterEnvironment.DEMO:
        return "https://fapi-testnet.asterdex.com"  # Demo uses testnet

    return "https://fapi.asterdex.com"


def get_ws_api_base_url(
    account_type: AsterAccountType,
    environment: AsterEnvironment,
    is_us: bool,
) -> str:
    """Return the WebSocket API base URL for user data streams."""
    if account_type.is_spot_or_margin:
        raise ValueError(
            f"Aster does not support spot/margin trading, got account_type={account_type}",
        )

    if environment == AsterEnvironment.TESTNET:
        return "wss://fstream-testnet.asterdex.com/ws-fapi/v1"

    if environment == AsterEnvironment.DEMO:
        return "wss://fstream-testnet.asterdex.com/ws-fapi/v1"

    return "wss://fstream.asterdex.com/ws-fapi/v1"


def get_ws_base_url(
    account_type: AsterAccountType,
    environment: AsterEnvironment,
    is_us: bool,
) -> str:
    if account_type.is_spot_or_margin:
        raise ValueError(
            f"Aster does not support spot/margin trading, got account_type={account_type}",
        )

    if environment == AsterEnvironment.TESTNET:
        return "wss://fstream-testnet.asterdex.com"

    if environment == AsterEnvironment.DEMO:
        return "wss://fstream-testnet.asterdex.com"

    return "wss://fstream.asterdex.com"
