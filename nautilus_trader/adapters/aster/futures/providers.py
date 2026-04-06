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
from decimal import Decimal
from enum import Enum
from typing import Any

import msgspec

from nautilus_trader.adapters.aster.common.constants import ASTER_VENUE
from nautilus_trader.adapters.aster.common.enums import AsterAccountType
from nautilus_trader.adapters.aster.common.enums import AsterSymbolFilterType
from nautilus_trader.adapters.aster.common.schemas.market import AsterSymbolFilter
from nautilus_trader.adapters.aster.common.symbol import AsterSymbol
from nautilus_trader.adapters.aster.config import AsterInstrumentProviderConfig
from nautilus_trader.adapters.aster.futures.enums import AsterFuturesContractStatus
from nautilus_trader.adapters.aster.futures.enums import AsterFuturesContractType
from nautilus_trader.adapters.aster.futures.http.account import AsterFuturesAccountHttpAPI
from nautilus_trader.adapters.aster.futures.http.market import AsterFuturesMarketHttpAPI
from nautilus_trader.adapters.aster.futures.http.wallet import AsterFuturesWalletHttpAPI
from nautilus_trader.adapters.aster.futures.schemas.account import AsterFuturesFeeRates
from nautilus_trader.adapters.aster.futures.schemas.account import AsterFuturesPositionRisk
from nautilus_trader.adapters.aster.futures.schemas.market import AsterFuturesSymbolInfo
from nautilus_trader.adapters.aster.futures.schemas.wallet import AsterFuturesCommissionRate
from nautilus_trader.adapters.aster.http.client import AsterHttpClient
from nautilus_trader.common.component import LiveClock
from nautilus_trader.common.providers import InstrumentProvider
from nautilus_trader.config import InstrumentProviderConfig
from nautilus_trader.core.correctness import PyCondition
from nautilus_trader.core.datetime import millis_to_nanos
from nautilus_trader.model.identifiers import InstrumentId
from nautilus_trader.model.identifiers import Symbol
from nautilus_trader.model.identifiers import Venue
from nautilus_trader.model.instruments.crypto_future import CryptoFuture
from nautilus_trader.model.instruments.crypto_perpetual import CryptoPerpetual
from nautilus_trader.model.objects import PRICE_MAX
from nautilus_trader.model.objects import PRICE_MIN
from nautilus_trader.model.objects import QUANTITY_MAX
from nautilus_trader.model.objects import QUANTITY_MIN
from nautilus_trader.model.objects import Money
from nautilus_trader.model.objects import Price
from nautilus_trader.model.objects import Quantity


def _symbol_info_to_dict(symbol_info: AsterFuturesSymbolInfo) -> dict:
    """
    Convert symbol info to dict with all enums and nested structs converted to
    primitives.

    This ensures the info dict contains only JSON-serializable primitives.

    """

    def _convert_value(value: Any) -> Any:
        # Recursively convert enums and structs to primitives
        if isinstance(value, Enum):
            return value.value
        elif hasattr(value, "__struct_fields__"):
            return _convert_dict(msgspec.structs.asdict(value))
        elif isinstance(value, list):
            return [_convert_value(item) for item in value]
        elif isinstance(value, dict):
            return _convert_dict(value)
        return value

    def _convert_dict(d: dict) -> dict:
        return {key: _convert_value(val) for key, val in d.items()}

    return _convert_dict(msgspec.structs.asdict(symbol_info))


class AsterFuturesInstrumentProvider(InstrumentProvider):
    """
    Provides a means of loading instruments from the Aster Futures exchange.

    Parameters
    ----------
    client : APIClient
        The client for the provider.
    config : InstrumentProviderConfig, optional
        The configuration for the provider.

    """

    def __init__(
        self,
        client: AsterHttpClient,
        clock: LiveClock,
        account_type: AsterAccountType = AsterAccountType.USDT_FUTURES,
        config: InstrumentProviderConfig | AsterInstrumentProviderConfig | None = None,
        venue: Venue = ASTER_VENUE,
    ) -> None:
        super().__init__(config=config)

        self._clock = clock
        self._client = client
        self._account_type = account_type
        self._venue = venue

        self._http_account = AsterFuturesAccountHttpAPI(
            self._client,
            clock=self._clock,
            account_type=account_type,
        )
        self._http_wallet = AsterFuturesWalletHttpAPI(
            self._client,
            clock=self._clock,
            account_type=account_type,
        )
        self._http_market = AsterFuturesMarketHttpAPI(self._client, account_type=account_type)

        self._log_warnings = config.log_warnings if config else True

        self._decoder = msgspec.json.Decoder()
        self._encoder = msgspec.json.Encoder()

        # This fee rates map is only applicable for backtesting, as live trading will utilise
        # real-time account update messages provided by Aster.
        # These fee rates assume USD-M Futures Trading without the 10% off for using BNB.
        # The next step is to enable users to pass their own fee rates map via the config.
        # In the future, we aim to represent this fee model with greater accuracy for backtesting.
        # https://www.aster.com/en/fee/futureFee
        # Last verified: 2025-01-22
        self._fee_rates = {
            0: AsterFuturesFeeRates(feeTier=0, maker="0.000200", taker="0.000500"),
            1: AsterFuturesFeeRates(feeTier=1, maker="0.000160", taker="0.000400"),
            2: AsterFuturesFeeRates(feeTier=2, maker="0.000140", taker="0.000350"),
            3: AsterFuturesFeeRates(feeTier=3, maker="0.000120", taker="0.000320"),
            4: AsterFuturesFeeRates(feeTier=4, maker="0.000100", taker="0.000300"),
            5: AsterFuturesFeeRates(feeTier=5, maker="0.000080", taker="0.000270"),
            6: AsterFuturesFeeRates(feeTier=6, maker="0.000060", taker="0.000250"),
            7: AsterFuturesFeeRates(feeTier=7, maker="0.000040", taker="0.000220"),
            8: AsterFuturesFeeRates(feeTier=8, maker="0.000020", taker="0.000200"),
            9: AsterFuturesFeeRates(feeTier=9, maker="0.000000", taker="0.000170"),
        }

    async def load_all_async(self, filters: dict | None = None) -> None:
        filters_str = "..." if not filters else f" with filters {filters}..."
        self._log.info(f"Loading all instruments{filters_str}")

        # Get exchange info for all assets
        exchange_info = await self._http_market.query_futures_exchange_info()

        # Get fee tier from account info (requires authentication)
        if self._client.api_key is None or self._client._secret is None:
            self._log.info(
                "API credentials not configured; using default fee tier 0",
            )
            fee_rates = self._fee_rates[0]
        else:
            account_info = await self._http_account.query_futures_account_info()
            fee_rates = self._fee_rates[account_info.feeTier]

        if (
            isinstance(self._config, AsterInstrumentProviderConfig)
            and self._config.query_commission_rates
            and self._client.api_key is not None
            and self._client._secret is not None
        ):
            self._log.info("Querying commission rates per symbol (parallel requests)")

            async def _query_fee(symbol: str) -> AsterFuturesCommissionRate:
                try:
                    return await self._http_wallet.query_futures_commission_rate(symbol=symbol)
                except Exception as e:
                    self._log.warning(
                        f"Failed to query commission rate for {symbol}: {e}, falling back to fee tier table",
                    )
                    return AsterFuturesCommissionRate(
                        symbol=symbol,
                        makerCommissionRate=fee_rates.maker,
                        takerCommissionRate=fee_rates.taker,
                    )

            tasks = [_query_fee(symbol_info.symbol) for symbol_info in exchange_info.symbols]
            fees = await asyncio.gather(*tasks)

            for symbol_info, fee in zip(exchange_info.symbols, fees, strict=True):
                self._parse_instrument(
                    symbol_info=symbol_info,
                    fee=fee,
                    ts_event=millis_to_nanos(exchange_info.serverTime),
                )
        else:
            for symbol_info in exchange_info.symbols:
                fee = AsterFuturesCommissionRate(
                    symbol=symbol_info.symbol,
                    makerCommissionRate=fee_rates.maker,
                    takerCommissionRate=fee_rates.taker,
                )
                self._parse_instrument(
                    symbol_info=symbol_info,
                    fee=fee,
                    ts_event=millis_to_nanos(exchange_info.serverTime),
                )

    async def load_ids_async(
        self,
        instrument_ids: list[InstrumentId],
        filters: dict | None = None,
    ) -> None:
        if not instrument_ids:
            self._log.warning("No instrument IDs given for loading.")
            return

        # Check all instrument IDs
        for instrument_id in instrument_ids:
            PyCondition.equal(instrument_id.venue, self._venue, "instrument_id.venue", "ASTER")

        # Extract all symbol strings
        symbols = [
            str(AsterSymbol(instrument_id.symbol.value)) for instrument_id in instrument_ids
        ]

        # Get exchange info for all assets
        exchange_info = await self._http_market.query_futures_exchange_info()
        symbol_info_dict: dict[str, AsterFuturesSymbolInfo] = {
            info.symbol: info for info in exchange_info.symbols
        }

        # Get fee tier and position risk (requires authentication)
        if self._client.api_key is None or self._client._secret is None:
            fee_rates = self._fee_rates[0]
            position_risk = {}
        else:
            account_info = await self._http_account.query_futures_account_info()
            fee_rates = self._fee_rates[account_info.feeTier]
            position_risk_resp = await self._http_account.query_futures_position_risk()
            position_risk = {risk.symbol: risk for risk in position_risk_resp}

        if (
            isinstance(self._config, AsterInstrumentProviderConfig)
            and self._config.query_commission_rates
            and self._client.api_key is not None
            and self._client._secret is not None
        ):

            async def _query_fee(symbol: str) -> AsterFuturesCommissionRate:
                try:
                    return await self._http_wallet.query_futures_commission_rate(symbol=symbol)
                except Exception as e:
                    self._log.warning(
                        f"Failed to query commission rate for {symbol}: {e}. Falling back to fee tier table.",
                    )
                    return AsterFuturesCommissionRate(
                        symbol=symbol,
                        makerCommissionRate=fee_rates.maker,
                        takerCommissionRate=fee_rates.taker,
                    )

            tasks = [_query_fee(symbol) for symbol in symbols]
            fees = await asyncio.gather(*tasks)

            for symbol, fee in zip(symbols, fees, strict=True):
                self._parse_instrument(
                    symbol_info=symbol_info_dict[symbol],
                    fee=fee,
                    ts_event=millis_to_nanos(exchange_info.serverTime),
                    position_risk=position_risk.get(symbol),
                )
        else:
            for symbol in symbols:
                fee = AsterFuturesCommissionRate(
                    symbol=symbol,
                    makerCommissionRate=fee_rates.maker,
                    takerCommissionRate=fee_rates.taker,
                )
                self._parse_instrument(
                    symbol_info=symbol_info_dict[symbol],
                    fee=fee,
                    ts_event=millis_to_nanos(exchange_info.serverTime),
                    position_risk=position_risk.get(symbol),
                )

    async def load_async(self, instrument_id: InstrumentId, filters: dict | None = None) -> None:
        PyCondition.not_none(instrument_id, "instrument_id")
        PyCondition.equal(instrument_id.venue, self._venue, "instrument_id.venue", "ASTER")

        filters_str = "..." if not filters else f" with filters {filters}..."
        self._log.debug(f"Loading instrument {instrument_id}{filters_str}.")

        symbol = str(AsterSymbol(instrument_id.symbol.value))

        # Get exchange info for all assets
        exchange_info = await self._http_market.query_futures_exchange_info()
        symbol_info_dict: dict[str, AsterFuturesSymbolInfo] = {
            info.symbol: info for info in exchange_info.symbols
        }

        # Get fee tier from account info (requires authentication)
        if self._client.api_key is None or self._client._secret is None:
            fee_rates = self._fee_rates[0]
        else:
            account_info = await self._http_account.query_futures_account_info()
            fee_rates = self._fee_rates[account_info.feeTier]

        if (
            isinstance(self._config, AsterInstrumentProviderConfig)
            and self._config.query_commission_rates
            and self._client.api_key is not None
            and self._client._secret is not None
        ):
            try:
                fee = await self._http_wallet.query_futures_commission_rate(symbol=symbol)
            except Exception as e:
                self._log.warning(
                    f"Failed to query commission rate for {symbol}: {e}. Falling back to fee tier table.",
                )
                fee = AsterFuturesCommissionRate(
                    symbol=symbol,
                    makerCommissionRate=fee_rates.maker,
                    takerCommissionRate=fee_rates.taker,
                )
        else:
            fee = AsterFuturesCommissionRate(
                symbol=symbol,
                makerCommissionRate=fee_rates.maker,
                takerCommissionRate=fee_rates.taker,
            )

        self._parse_instrument(
            symbol_info=symbol_info_dict[symbol],
            ts_event=millis_to_nanos(exchange_info.serverTime),
            fee=fee,
        )

    def _parse_instrument(
        self,
        symbol_info: AsterFuturesSymbolInfo,
        ts_event: int,
        position_risk: AsterFuturesPositionRisk | None = None,
        fee: AsterFuturesCommissionRate | None = None,
    ) -> None:
        contract_type_str = symbol_info.contractType

        if (
            contract_type_str == ""
            or symbol_info.status == AsterFuturesContractStatus.PENDING_TRADING
        ):
            self._log.debug(f"Instrument not yet defined: {symbol_info.symbol}")
            return  # Not yet defined

        ts_init = self._clock.timestamp_ns()
        try:
            # Create quote and base assets
            base_currency = symbol_info.parse_to_base_currency()
            quote_currency = symbol_info.parse_to_quote_currency()

            raw_symbol = Symbol(symbol_info.symbol)
            parsed_symbol = AsterSymbol(raw_symbol.value).parse_as_nautilus(
                self._account_type,
            )
            nautilus_symbol = Symbol(parsed_symbol)
            instrument_id = InstrumentId(symbol=nautilus_symbol, venue=self._venue)

            # Parse instrument filters
            filters: dict[AsterSymbolFilterType, AsterSymbolFilter] = {
                f.filterType: f for f in symbol_info.filters
            }
            price_filter: AsterSymbolFilter = filters.get(AsterSymbolFilterType.PRICE_FILTER)
            lot_size_filter: AsterSymbolFilter = filters.get(AsterSymbolFilterType.LOT_SIZE)
            min_notional_filter: AsterSymbolFilter = filters.get(
                AsterSymbolFilterType.MIN_NOTIONAL,
            )

            tick_size = price_filter.tickSize
            step_size = lot_size_filter.stepSize
            PyCondition.in_range(float(tick_size), PRICE_MIN, PRICE_MAX, "tick_size")
            PyCondition.in_range(float(step_size), QUANTITY_MIN, QUANTITY_MAX, "step_size")

            price_precision = abs(int(Decimal(tick_size).as_tuple().exponent))
            size_precision = abs(int(Decimal(step_size).as_tuple().exponent))
            price_increment = Price.from_str(tick_size)
            size_increment = Quantity.from_str(step_size)
            max_quantity = Quantity(float(lot_size_filter.maxQty), precision=size_precision)
            min_quantity = Quantity(float(lot_size_filter.minQty), precision=size_precision)
            min_notional = None

            if filters.get(AsterSymbolFilterType.MIN_NOTIONAL):
                min_notional = Money(min_notional_filter.notional, currency=quote_currency)
            max_notional = (
                Money(position_risk.maxNotionalValue, currency=quote_currency)
                if position_risk and position_risk.maxNotionalValue is not None
                else None
            )
            max_price = Price(float(price_filter.maxPrice), precision=price_precision)
            min_price = Price(float(price_filter.minPrice), precision=price_precision)

            # Futures commissions
            maker_fee = Decimal(0)
            taker_fee = Decimal(0)

            if fee:
                assert fee.symbol == symbol_info.symbol
                maker_fee = Decimal(fee.makerCommissionRate)
                taker_fee = Decimal(fee.takerCommissionRate)

            if symbol_info.marginAsset == symbol_info.baseAsset:
                settlement_currency = base_currency
            elif symbol_info.marginAsset == symbol_info.quoteAsset:
                settlement_currency = quote_currency
            else:
                raise ValueError(f"Unrecognized margin asset {symbol_info.marginAsset}")

            contract_type = AsterFuturesContractType(contract_type_str)
            if contract_type in (
                AsterFuturesContractType.PERPETUAL,
                AsterFuturesContractType.PERPETUAL_DELIVERING,
                AsterFuturesContractType.TRADIFI_PERPETUAL,
            ):
                instrument = CryptoPerpetual(
                    instrument_id=instrument_id,
                    raw_symbol=raw_symbol,
                    base_currency=base_currency,
                    quote_currency=quote_currency,
                    settlement_currency=settlement_currency,
                    is_inverse=False,  # No inverse instruments trade on Aster
                    price_precision=price_precision,
                    size_precision=size_precision,
                    price_increment=price_increment,
                    size_increment=size_increment,
                    max_quantity=max_quantity,
                    min_quantity=min_quantity,
                    max_notional=max_notional,
                    min_notional=min_notional,
                    max_price=max_price,
                    min_price=min_price,
                    margin_init=Decimal(1),  # Aster docs: ignore API values
                    margin_maint=Decimal(1),  # Aster docs: ignore API values
                    maker_fee=maker_fee,
                    taker_fee=taker_fee,
                    ts_event=ts_event,
                    ts_init=ts_init,
                    info=_symbol_info_to_dict(symbol_info),
                )
                self.add_currency(currency=instrument.base_currency)
            elif contract_type in (
                AsterFuturesContractType.CURRENT_MONTH,
                AsterFuturesContractType.CURRENT_QUARTER,
                AsterFuturesContractType.CURRENT_QUARTER_DELIVERING,
                AsterFuturesContractType.NEXT_MONTH,
                AsterFuturesContractType.NEXT_QUARTER,
            ):
                instrument = CryptoFuture(
                    instrument_id=instrument_id,
                    raw_symbol=raw_symbol,
                    underlying=base_currency,
                    quote_currency=quote_currency,
                    settlement_currency=settlement_currency,
                    is_inverse=False,  # No inverse instruments trade on Aster
                    activation_ns=millis_to_nanos(symbol_info.onboardDate),
                    expiration_ns=millis_to_nanos(symbol_info.deliveryDate),
                    price_precision=price_precision,
                    size_precision=size_precision,
                    price_increment=price_increment,
                    size_increment=size_increment,
                    max_quantity=max_quantity,
                    min_quantity=min_quantity,
                    max_notional=None,
                    min_notional=min_notional,
                    max_price=max_price,
                    min_price=min_price,
                    margin_init=Decimal(1),  # Aster docs: ignore API values
                    margin_maint=Decimal(1),  # Aster docs: ignore API values
                    maker_fee=maker_fee,
                    taker_fee=taker_fee,
                    ts_event=ts_event,
                    ts_init=ts_init,
                    info=_symbol_info_to_dict(symbol_info),
                )
                self.add_currency(currency=instrument.underlying)
            else:
                raise RuntimeError(  # pragma: no cover (design-time error)
                    f"invalid `AsterFuturesContractType`, was {contract_type}",  # pragma: no cover
                )

            self.add_currency(currency=instrument.quote_currency)
            self.add(instrument=instrument)

            self._log.debug(f"Added instrument {instrument.id}.")
        except ValueError as e:
            if self._log_warnings:
                self._log.warning(f"Unable to parse instrument {symbol_info.symbol}: {e}.")
