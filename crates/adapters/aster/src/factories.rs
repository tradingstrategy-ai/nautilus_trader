// -------------------------------------------------------------------------------------------------
//  Copyright (C) 2015-2026 Nautech Systems Pty Ltd. All rights reserved.
//  https://nautechsystems.io
//
//  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
//  You may not use this file except in compliance with the License.
//  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
// -------------------------------------------------------------------------------------------------

//! Factory functions for creating Aster clients and components.

use std::{cell::RefCell, rc::Rc};

use nautilus_common::{
    cache::Cache,
    clients::{DataClient, ExecutionClient},
    clock::Clock,
};
use nautilus_live::ExecutionClientCore;
use nautilus_model::{
    enums::{AccountType, OmsType},
    identifiers::ClientId,
};
use nautilus_system::factories::{ClientConfig, DataClientFactory, ExecutionClientFactory};

use crate::{
    common::{
        consts::{ASTER, ASTER_VENUE},
        enums::AsterProductType,
    },
    config::{AsterDataClientConfig, AsterExecClientConfig},
    futures::{data::AsterFuturesDataClient, execution::AsterFuturesExecutionClient},
};

/// Factory for creating Aster data clients.
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.core.nautilus_pyo3.aster", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.aster")
)]
pub struct AsterDataClientFactory;

impl AsterDataClientFactory {
    /// Creates a new [`AsterDataClientFactory`] instance.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for AsterDataClientFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl DataClientFactory for AsterDataClientFactory {
    fn create(
        &self,
        name: &str,
        config: &dyn ClientConfig,
        _cache: Rc<RefCell<Cache>>,
        _clock: Rc<RefCell<dyn Clock>>,
    ) -> anyhow::Result<Box<dyn DataClient>> {
        let aster_config = config
            .as_any()
            .downcast_ref::<AsterDataClientConfig>()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Invalid config type for AsterDataClientFactory. Expected AsterDataClientConfig, was {config:?}",
                )
            })?
            .clone();

        let client_id = ClientId::from(name);

        let product_type = aster_config
            .product_types
            .first()
            .copied()
            .unwrap_or(AsterProductType::UsdM);

        match product_type {
            AsterProductType::UsdM | AsterProductType::CoinM => {
                let client =
                    AsterFuturesDataClient::new(client_id, aster_config, product_type)?;
                Ok(Box::new(client))
            }
            _ => {
                anyhow::bail!("Unsupported product type for Aster data client: {product_type:?}")
            }
        }
    }

    fn name(&self) -> &'static str {
        ASTER
    }

    fn config_type(&self) -> &'static str {
        stringify!(AsterDataClientConfig)
    }
}

/// Factory for creating Aster Spot execution clients.
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.core.nautilus_pyo3.aster", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.aster")
)]
pub struct AsterExecutionClientFactory;

impl AsterExecutionClientFactory {
    /// Creates a new [`AsterExecutionClientFactory`] instance.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for AsterExecutionClientFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionClientFactory for AsterExecutionClientFactory {
    fn create(
        &self,
        name: &str,
        config: &dyn ClientConfig,
        cache: Rc<RefCell<Cache>>,
    ) -> anyhow::Result<Box<dyn ExecutionClient>> {
        let aster_config = config
            .as_any()
            .downcast_ref::<AsterExecClientConfig>()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Invalid config type for AsterExecutionClientFactory. Expected AsterExecClientConfig, was {config:?}",
                )
            })?
            .clone();

        let product_type = aster_config
            .product_types
            .first()
            .copied()
            .unwrap_or(AsterProductType::UsdM);

        match product_type {
            AsterProductType::UsdM | AsterProductType::CoinM => {
                // Futures uses margin account type and netting OMS
                let account_type = AccountType::Margin;
                let oms_type = OmsType::Netting;

                let core = ExecutionClientCore::new(
                    aster_config.trader_id,
                    ClientId::from(name),
                    *ASTER_VENUE,
                    oms_type,
                    aster_config.account_id,
                    account_type,
                    None, // base_currency
                    cache,
                );

                let client = AsterFuturesExecutionClient::new(core, aster_config)?;
                Ok(Box::new(client))
            }
            _ => {
                anyhow::bail!(
                    "Unsupported product type for Aster execution client: {product_type:?}"
                )
            }
        }
    }

    fn name(&self) -> &'static str {
        ASTER
    }

    fn config_type(&self) -> &'static str {
        stringify!(AsterExecClientConfig)
    }
}

#[cfg(test)]
mod tests {
    use nautilus_system::factories::DataClientFactory;
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_aster_data_client_factory_creation() {
        let factory = AsterDataClientFactory::new();
        assert_eq!(factory.name(), "ASTER");
        assert_eq!(factory.config_type(), "AsterDataClientConfig");
    }

    #[rstest]
    fn test_aster_data_client_factory_default() {
        let factory = AsterDataClientFactory;
        assert_eq!(factory.name(), "ASTER");
    }
}
