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

from nautilus_trader.adapters.aster.common.constants import ASTER_RETRY_ERRORS
from nautilus_trader.adapters.aster.common.enums import AsterErrorCode


class AsterError(Exception):
    """
    The base class for all Aster specific errors.
    """

    def __init__(self, status, message, headers):
        super().__init__(message)
        self.status = status
        self.message = message
        self.headers = headers


class AsterServerError(AsterError):
    """
    Represents an Aster specific 500 series HTTP error.
    """

    def __init__(self, status, message, headers):
        super().__init__(status, message, headers)


class AsterClientError(AsterError):
    """
    Represents an Aster specific 400 series HTTP error.
    """

    def __init__(self, status, message, headers):
        super().__init__(status, message, headers)


def get_aster_error_code(error: BaseException) -> AsterErrorCode | None:
    """
    Extract the Aster error code from an exception.

    Parameters
    ----------
    error : BaseException
        The error to extract the code from.

    Returns
    -------
    AsterErrorCode | None
        The error code if it can be extracted, otherwise None.

    """
    if isinstance(error, AsterError):
        try:
            # Handle case where message might be a dict, string, or missing 'code' key
            if isinstance(error.message, dict) and "code" in error.message:
                return AsterErrorCode(int(error.message["code"]))
            elif isinstance(error.message, str):
                # Try to parse error code from string format like '{"code":-1021,"msg":"..."}'
                import json

                try:
                    parsed_message = json.loads(error.message)
                    if isinstance(parsed_message, dict) and "code" in parsed_message:
                        return AsterErrorCode(int(parsed_message["code"]))
                except (json.JSONDecodeError, ValueError, KeyError):
                    pass
        except (ValueError, KeyError, TypeError):
            pass  # If any parsing fails, return None

    return None


def should_retry(error: BaseException) -> bool:
    """
    Determine if a retry should be attempted based on the error code.

    Parameters
    ----------
    error : BaseException
        The error to check.

    Returns
    -------
    bool
        True if should retry, otherwise False.

    """
    error_code = get_aster_error_code(error)
    return error_code in ASTER_RETRY_ERRORS if error_code else False
