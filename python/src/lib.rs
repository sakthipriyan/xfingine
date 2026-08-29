//! Python bindings for [`xfingine`].
//!
//! Each engine accepts a `dict` and returns a `dict`, with a `*_json` variant
//! that takes and returns a JSON string.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pythonize::{depythonize_bound, pythonize};

/// Generate the dict and JSON entry points for one engine.
macro_rules! bind_engine {
    ($value_fn:ident, $json_fn:ident, $request:ty, $engine:path) => {
        #[pyfunction]
        fn $value_fn(py: Python<'_>, request: &Bound<'_, PyAny>) -> PyResult<PyObject> {
            let request: $request = depythonize_bound(request.clone())
                .map_err(|e| PyValueError::new_err(format!("invalid request: {e}")))?;
            let result = $engine(&request).map_err(|e| PyValueError::new_err(e.to_string()))?;
            pythonize(py, &result)
                .map(|bound| bound.into())
                .map_err(|e| PyValueError::new_err(format!("could not build result: {e}")))
        }

        #[pyfunction]
        fn $json_fn(request_json: &str) -> PyResult<String> {
            let request: $request = serde_json::from_str(request_json)
                .map_err(|e| PyValueError::new_err(format!("invalid request: {e}")))?;
            let result = $engine(&request).map_err(|e| PyValueError::new_err(e.to_string()))?;
            serde_json::to_string(&result).map_err(|e| PyValueError::new_err(e.to_string()))
        }
    };
}

#[cfg(feature = "emi")]
bind_engine!(
    compute_emi,
    compute_emi_json,
    ::xfingine::emi::EmiRequest,
    ::xfingine::emi::compute
);

#[cfg(feature = "categorizer")]
bind_engine!(
    categorize_transactions,
    categorize_transactions_json,
    ::xfingine::categorizer::CategorizeRequest,
    ::xfingine::categorizer::categorize_transactions
);

#[cfg(feature = "categorizer")]
bind_engine!(
    derive_rules,
    derive_rules_json,
    ::xfingine::categorizer::DeriveRulesRequest,
    ::xfingine::categorizer::derive_rules
);

#[pymodule]
fn xfingine(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    #[cfg(feature = "emi")]
    {
        m.add_function(wrap_pyfunction!(compute_emi, m)?)?;
        m.add_function(wrap_pyfunction!(compute_emi_json, m)?)?;
    }

    #[cfg(feature = "categorizer")]
    {
        m.add_function(wrap_pyfunction!(categorize_transactions, m)?)?;
        m.add_function(wrap_pyfunction!(categorize_transactions_json, m)?)?;
        m.add_function(wrap_pyfunction!(derive_rules, m)?)?;
        m.add_function(wrap_pyfunction!(derive_rules_json, m)?)?;
    }

    Ok(())
}
