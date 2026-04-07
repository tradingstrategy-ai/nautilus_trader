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

//! Python bindings for the Aster adapter.

#![allow(
    clippy::missing_errors_doc,
    reason = "errors documented on underlying Rust methods"
)]

pub mod config;
pub mod enums;
pub mod factories;
pub mod types;

use nautilus_core::python::{to_pyruntime_err, to_pyvalue_err};
use nautilus_system::{
    factories::{ClientConfig, DataClientFactory, ExecutionClientFactory},
    get_global_pyo3_registry,
};
use pyo3::prelude::*;

use crate::{
    common::{
        bar::AsterBar,
        consts::ASTER_NAUTILUS_FUTURES_BROKER_ID,
        encoder::decode_broker_id,
        enums::{AsterEnvironment, AsterMarginType, AsterPositionSide, AsterProductType},
    },
    config::{AsterDataClientConfig, AsterExecClientConfig},
    factories::{AsterDataClientFactory, AsterExecutionClientFactory},
};

#[allow(clippy::needless_pass_by_value)]
fn extract_aster_data_factory(
    py: Python<'_>,
    factory: Py<PyAny>,
) -> PyResult<Box<dyn DataClientFactory>> {
    match factory.extract::<AsterDataClientFactory>(py) {
        Ok(f) => Ok(Box::new(f)),
        Err(e) => Err(to_pyvalue_err(format!(
            "Failed to extract AsterDataClientFactory: {e}"
        ))),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn extract_aster_exec_factory(
    py: Python<'_>,
    factory: Py<PyAny>,
) -> PyResult<Box<dyn ExecutionClientFactory>> {
    match factory.extract::<AsterExecutionClientFactory>(py) {
        Ok(f) => Ok(Box::new(f)),
        Err(e) => Err(to_pyvalue_err(format!(
            "Failed to extract AsterExecutionClientFactory: {e}"
        ))),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn extract_aster_data_config(
    py: Python<'_>,
    config: Py<PyAny>,
) -> PyResult<Box<dyn ClientConfig>> {
    match config.extract::<AsterDataClientConfig>(py) {
        Ok(c) => Ok(Box::new(c)),
        Err(e) => Err(to_pyvalue_err(format!(
            "Failed to extract AsterDataClientConfig: {e}"
        ))),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn extract_aster_exec_config(
    py: Python<'_>,
    config: Py<PyAny>,
) -> PyResult<Box<dyn ClientConfig>> {
    match config.extract::<AsterExecClientConfig>(py) {
        Ok(c) => Ok(Box::new(c)),
        Err(e) => Err(to_pyvalue_err(format!(
            "Failed to extract AsterExecClientConfig: {e}"
        ))),
    }
}

/// Decodes a Aster Futures encoded `clientOrderId` back to the original value.
///
/// Aster Futures orders placed through the Rust execution client have their
/// `ClientOrderId` encoded with a broker ID prefix for Link and Trade
/// attribution. This function reverses that encoding.
///
/// Strings without the broker prefix are returned unchanged.
#[pyfunction]
#[pyo3(name = "decode_aster_futures_client_order_id")]
fn py_decode_aster_futures_client_order_id(encoded: &str) -> String {
    decode_broker_id(encoded, ASTER_NAUTILUS_FUTURES_BROKER_ID)
}

/// Aster adapter Python module.
///
/// Loaded as `nautilus_pyo3.aster`.
///
/// # Errors
///
/// Returns an error if module initialization fails.
#[pymodule]
pub fn aster(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<AsterProductType>()?;
    m.add_class::<AsterEnvironment>()?;
    m.add_class::<AsterMarginType>()?;
    m.add_class::<AsterPositionSide>()?;
    m.add_class::<AsterBar>()?;
    m.add_class::<AsterDataClientConfig>()?;
    m.add_class::<AsterExecClientConfig>()?;
    m.add_class::<AsterDataClientFactory>()?;
    m.add_class::<AsterExecutionClientFactory>()?;
    m.add_function(wrap_pyfunction!(
        py_decode_aster_futures_client_order_id,
        m
    )?)?;

    // NOTE: AsterBar Arrow encode/decode traits not yet implemented.
    // Skipping ensure_custom_data_registered::<AsterBar>() — Aster-specific
    // bars use Python-side serialization via __init__.py for now.

    let registry = get_global_pyo3_registry();

    if let Err(e) =
        registry.register_factory_extractor("ASTER".to_string(), extract_aster_data_factory)
    {
        return Err(to_pyruntime_err(format!(
            "Failed to register Aster data factory extractor: {e}"
        )));
    }

    if let Err(e) = registry
        .register_exec_factory_extractor("ASTER".to_string(), extract_aster_exec_factory)
    {
        return Err(to_pyruntime_err(format!(
            "Failed to register Aster exec factory extractor: {e}"
        )));
    }

    if let Err(e) = registry.register_config_extractor(
        "AsterDataClientConfig".to_string(),
        extract_aster_data_config,
    ) {
        return Err(to_pyruntime_err(format!(
            "Failed to register Aster data config extractor: {e}"
        )));
    }

    if let Err(e) = registry.register_config_extractor(
        "AsterExecClientConfig".to_string(),
        extract_aster_exec_config,
    ) {
        return Err(to_pyruntime_err(format!(
            "Failed to register Aster exec config extractor: {e}"
        )));
    }

    Ok(())
}
