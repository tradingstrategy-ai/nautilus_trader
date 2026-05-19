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
Integration tests for the local_addr (source-IP pinning) feature on the HL adapter.

These tests require the compiled pyo3 extension. They run as part of the normal
pytest suite once the wheel builds. The corresponding Rust unit tests live in:

- crates/network/src/http/client.rs       (HttpClient + reqwest local_addr)
- crates/network/src/websocket/config.rs  (WebSocketConfig.local_addr field)
- crates/adapters/hyperliquid/src/http/client.rs  (HL adapter plumbing)
- crates/adapters/aster/src/futures/http/client.rs  (Aster adapter plumbing)

The Rust tests prove the mechanism works end-to-end (real HTTP requests with
the bound source IP, both success on loopback and failure on unbindable IPs).
These Python tests prove the pyo3 layer surfaces the right errors and accepts
the right inputs.
"""

from __future__ import annotations

import pytest

# These imports require the compiled wheel. They will raise ImportError when run
# from an editable install where build.py hasn't been run yet — that's expected.
pytest.importorskip("nautilus_trader.core.nautilus_pyo3")

from nautilus_trader.adapters.hyperliquid.config import HyperliquidDataClientConfig
from nautilus_trader.adapters.hyperliquid.config import HyperliquidExecClientConfig


class TestHyperliquidConfigLocalAddr:
    """The config dataclass must accept the new local_addr field with a None default."""

    def test_data_client_config_default_local_addr_is_none(self):
        config = HyperliquidDataClientConfig()
        assert config.local_addr is None

    def test_data_client_config_accepts_valid_ipv4_string(self):
        config = HyperliquidDataClientConfig(local_addr="127.0.0.1")
        assert config.local_addr == "127.0.0.1"

    def test_data_client_config_accepts_valid_ipv6_string(self):
        config = HyperliquidDataClientConfig(local_addr="::1")
        assert config.local_addr == "::1"

    def test_data_client_config_accepts_none_explicitly(self):
        config = HyperliquidDataClientConfig(local_addr=None)
        assert config.local_addr is None

    def test_exec_client_config_default_local_addr_is_none(self):
        config = HyperliquidExecClientConfig()
        assert config.local_addr is None

    def test_exec_client_config_accepts_valid_ipv4_string(self):
        config = HyperliquidExecClientConfig(local_addr="10.0.0.5")
        assert config.local_addr == "10.0.0.5"


class TestHyperliquidHttpClientLocalAddr:
    """The pyo3 HyperliquidHttpClient constructor must accept local_addr and validate it."""

    def test_construction_with_no_local_addr(self):
        from nautilus_trader.core.nautilus_pyo3 import HyperliquidHttpClient

        # Should not raise.
        client = HyperliquidHttpClient(is_testnet=True, timeout_secs=60)
        assert client is not None

    def test_construction_with_loopback_local_addr(self):
        from nautilus_trader.core.nautilus_pyo3 import HyperliquidHttpClient

        client = HyperliquidHttpClient(
            is_testnet=True,
            timeout_secs=60,
            local_addr="127.0.0.1",
        )
        assert client is not None

    def test_construction_with_invalid_local_addr_raises_value_error(self):
        from nautilus_trader.core.nautilus_pyo3 import HyperliquidHttpClient

        with pytest.raises(ValueError, match="Invalid local_addr"):
            HyperliquidHttpClient(
                is_testnet=True,
                timeout_secs=60,
                local_addr="not.an.ip",
            )

    def test_construction_with_empty_local_addr_raises_value_error(self):
        from nautilus_trader.core.nautilus_pyo3 import HyperliquidHttpClient

        with pytest.raises(ValueError, match="Invalid local_addr"):
            HyperliquidHttpClient(
                is_testnet=True,
                timeout_secs=60,
                local_addr="",
            )

    @pytest.mark.parametrize(
        "bad_input",
        [
            "999.999.999.999",
            "256.0.0.1",
            "127.0.0",
            "127.0.0.1:80",
            "[::1]",
            "::g",
            "localhost",
        ],
    )
    def test_construction_rejects_malformed_ips(self, bad_input):
        from nautilus_trader.core.nautilus_pyo3 import HyperliquidHttpClient

        with pytest.raises(ValueError, match="Invalid local_addr"):
            HyperliquidHttpClient(
                is_testnet=True,
                timeout_secs=60,
                local_addr=bad_input,
            )


class TestHyperliquidWebSocketClientLocalAddr:
    """The pyo3 HyperliquidWebSocketClient constructor must accept local_addr."""

    def test_ws_construction_with_no_local_addr(self):
        from nautilus_trader.core.nautilus_pyo3 import HyperliquidWebSocketClient

        client = HyperliquidWebSocketClient(testnet=True)
        assert client is not None

    def test_ws_construction_with_loopback_local_addr(self):
        from nautilus_trader.core.nautilus_pyo3 import HyperliquidWebSocketClient

        client = HyperliquidWebSocketClient(testnet=True, local_addr="127.0.0.1")
        assert client is not None

    def test_ws_construction_with_invalid_local_addr_raises_value_error(self):
        from nautilus_trader.core.nautilus_pyo3 import HyperliquidWebSocketClient

        with pytest.raises(ValueError, match="Invalid local_addr"):
            HyperliquidWebSocketClient(testnet=True, local_addr="garbage")


class TestEnvVarPattern:
    """The pattern used by consumer-side live runners: os.environ.get(...) or None.

    Empty string from env should normalise to None (not raise on construction).
    """

    def test_empty_env_value_becomes_none(self):
        import os

        os.environ["TEST_LOCAL_ADDR_EMPTY"] = ""
        try:
            value = os.environ.get("TEST_LOCAL_ADDR_EMPTY") or None
            assert value is None
            # And passing None to the config must not raise.
            config = HyperliquidDataClientConfig(local_addr=value)
            assert config.local_addr is None
        finally:
            del os.environ["TEST_LOCAL_ADDR_EMPTY"]

    def test_unset_env_var_becomes_none(self):
        import os

        # Ensure unset.
        os.environ.pop("TEST_LOCAL_ADDR_UNSET", None)
        value = os.environ.get("TEST_LOCAL_ADDR_UNSET") or None
        assert value is None
        config = HyperliquidDataClientConfig(local_addr=value)
        assert config.local_addr is None

    def test_set_env_var_passes_through(self):
        import os

        os.environ["TEST_LOCAL_ADDR_SET"] = "127.0.0.1"
        try:
            value = os.environ.get("TEST_LOCAL_ADDR_SET") or None
            assert value == "127.0.0.1"
            config = HyperliquidDataClientConfig(local_addr=value)
            assert config.local_addr == "127.0.0.1"
        finally:
            del os.environ["TEST_LOCAL_ADDR_SET"]
