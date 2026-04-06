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
from nautilus_trader.adapters.aster.common.schemas.user import AsterListenKey
from nautilus_trader.adapters.aster.common.symbol import AsterSymbol
from nautilus_trader.adapters.aster.http.client import AsterHttpClient
from nautilus_trader.adapters.aster.http.endpoint import AsterHttpEndpoint
from nautilus_trader.core.correctness import PyCondition
from nautilus_trader.core.nautilus_pyo3 import HttpMethod


class AsterListenKeyHttp(AsterHttpEndpoint):
    """
    Endpoint for managing user data streams (listenKey).

    `POST /fapi/v1/listenKey`
    `PUT /fapi/v1/listenKey`
    `DELETE /fapi/v1/listenKey`

    References
    ----------
    https://developers.aster.com/docs/derivatives/usds-margined-futures/user-data-streams

    """

    def __init__(
        self,
        client: AsterHttpClient,
        url_path: str,
    ):
        methods = {
            HttpMethod.POST: AsterSecurityType.USER_STREAM,
            HttpMethod.PUT: AsterSecurityType.USER_STREAM,
            HttpMethod.DELETE: AsterSecurityType.USER_STREAM,
        }
        super().__init__(
            client,
            methods,
            url_path,
        )
        self._post_resp_decoder = msgspec.json.Decoder(AsterListenKey)
        self._put_resp_decoder = msgspec.json.Decoder()
        self._delete_resp_decoder = msgspec.json.Decoder()

    class PostParameters(msgspec.Struct, omit_defaults=True, frozen=True):
        """
        POST parameters for creating listen keys.

        Parameters
        ----------
        symbol : AsterSymbol, optional
            The trading pair. Only required for ISOLATED MARGIN accounts.

        """

        symbol: AsterSymbol | None = None

    class PutDeleteParameters(msgspec.Struct, omit_defaults=True, frozen=True):
        """
        PUT & DELETE parameters for managing listen keys.

        Parameters
        ----------
        symbol : AsterSymbol, optional
            The trading pair. Only required for ISOLATED MARGIN accounts.
        listenKey : str, optional
            The listen key to manage. Only required for SPOT/MARGIN accounts.

        """

        symbol: AsterSymbol | None = None
        listenKey: str | None = None

    async def _post(self, params: PostParameters | None = None) -> AsterListenKey:
        method_type = HttpMethod.POST
        raw = await self._method(method_type, params)
        return self._post_resp_decoder.decode(raw)

    async def _put(self, params: PutDeleteParameters | None = None) -> dict:
        method_type = HttpMethod.PUT
        raw = await self._method(method_type, params)
        return self._put_resp_decoder.decode(raw)

    async def _delete(self, params: PutDeleteParameters | None = None) -> dict:
        method_type = HttpMethod.DELETE
        raw = await self._method(method_type, params)
        return self._delete_resp_decoder.decode(raw)


class AsterUserDataHttpAPI:
    """
    Provides access to the Aster User Data Stream HTTP REST API.

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
        account_type: AsterAccountType,
    ):
        PyCondition.not_none(client, "client")
        self.client = client
        self.account_type = account_type

        if account_type == AsterAccountType.SPOT:
            listen_key_url = "/api/v3/userDataStream"
        elif account_type == AsterAccountType.MARGIN:
            listen_key_url = "/sapi/v1/userDataStream"
        elif account_type == AsterAccountType.ISOLATED_MARGIN:
            listen_key_url = "/sapi/v1/userDataStream/isolated"
        elif account_type == AsterAccountType.USDT_FUTURES:
            listen_key_url = "/fapi/v1/listenKey"
        elif account_type == AsterAccountType.COIN_FUTURES:
            listen_key_url = "/dapi/v1/listenKey"
        else:
            raise RuntimeError(
                f"invalid `AsterAccountType`, was {account_type}",
            )

        self._endpoint_listenkey = AsterListenKeyHttp(client, listen_key_url)

    async def create_listen_key(
        self,
        symbol: str | None = None,
    ) -> AsterListenKey:
        """
        Create a new Aster listenKey.
        """
        key = await self._endpoint_listenkey._post(
            params=self._endpoint_listenkey.PostParameters(
                symbol=AsterSymbol(symbol) if symbol else None,
            ),
        )
        return key

    async def keepalive_listen_key(
        self,
        symbol: str | None = None,
        listen_key: str | None = None,
    ):
        """
        Keepalive an existing Aster listenKey.
        """
        await self._endpoint_listenkey._put(
            params=self._endpoint_listenkey.PutDeleteParameters(
                symbol=AsterSymbol(symbol) if symbol else None,
                listenKey=listen_key,
            ),
        )

    async def close_listen_key(
        self,
        symbol: str | None = None,
        listen_key: str | None = None,
    ):
        """
        Close an existing Aster listenKey.
        """
        await self._endpoint_listenkey._delete(
            params=self._endpoint_listenkey.PutDeleteParameters(
                symbol=AsterSymbol(symbol) if symbol else None,
                listenKey=listen_key,
            ),
        )
