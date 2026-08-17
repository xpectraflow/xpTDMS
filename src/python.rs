#![allow(unsafe_op_in_unsafe_fn)]
#![allow(non_snake_case)]

use crate::defrag::Defragmenter;
use crate::model::file::TdmsFile;
use crate::model::property::PropertyValue;
use crate::writer::TdmsWriter;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::collections::HashMap;

fn property_to_py<'py>(py: Python<'py>, val: &PropertyValue) -> PyResult<Bound<'py, PyAny>> {
    match val {
        PropertyValue::Void => Ok(py.None().into_bound(py)),
        PropertyValue::I8(v) => Ok(v.into_py(py).into_bound(py)),
        PropertyValue::I16(v) => Ok(v.into_py(py).into_bound(py)),
        PropertyValue::I32(v) => Ok(v.into_py(py).into_bound(py)),
        PropertyValue::I64(v) => Ok(v.into_py(py).into_bound(py)),
        PropertyValue::U8(v) => Ok(v.into_py(py).into_bound(py)),
        PropertyValue::U16(v) => Ok(v.into_py(py).into_bound(py)),
        PropertyValue::U32(v) => Ok(v.into_py(py).into_bound(py)),
        PropertyValue::U64(v) => Ok(v.into_py(py).into_bound(py)),
        PropertyValue::SingleFloat(v) => Ok(v.into_py(py).into_bound(py)),
        PropertyValue::DoubleFloat(v) => Ok(v.into_py(py).into_bound(py)),
        PropertyValue::String(s) => Ok(s.into_py(py).into_bound(py)),
        PropertyValue::Boolean(b) => Ok(b.into_py(py).into_bound(py)),
        PropertyValue::Timestamp(ts) => Ok(ts.unix_seconds().into_py(py).into_bound(py)),
    }
}

fn map_properties_to_py<'py>(
    py: Python<'py>,
    props: &HashMap<String, PropertyValue>,
) -> PyResult<HashMap<String, PyObject>> {
    let mut map = HashMap::new();
    for (k, v) in props {
        map.insert(k.clone(), property_to_py(py, v)?.unbind());
    }
    Ok(map)
}

#[pyclass(name = "TdmsFile")]
pub struct PyTdmsFile {
    inner: TdmsFile,
}

#[pymethods]
impl PyTdmsFile {
    #[staticmethod]
    pub fn open(path: &str) -> PyResult<Self> {
        let inner = TdmsFile::open(path)
            .map_err(|e| PyValueError::new_err(format!("Failed to open TDMS file: {}", e)))?;
        Ok(Self { inner })
    }

    pub fn group_names(&self) -> Vec<String> {
        self.inner.groups.keys().cloned().collect()
    }

    pub fn channel_names(&self, group_name: &str) -> PyResult<Vec<String>> {
        let group = self
            .inner
            .group(group_name)
            .ok_or_else(|| PyValueError::new_err(format!("Group '{}' not found", group_name)))?;
        Ok(group.channels.keys().cloned().collect())
    }

    pub fn properties(&self, py: Python<'_>) -> PyResult<HashMap<String, PyObject>> {
        map_properties_to_py(py, &self.inner.properties)
    }

    pub fn group_properties(&self, py: Python<'_>, group_name: &str) -> PyResult<HashMap<String, PyObject>> {
        let group = self
            .inner
            .group(group_name)
            .ok_or_else(|| PyValueError::new_err(format!("Group '{}' not found", group_name)))?;
        map_properties_to_py(py, &group.properties)
    }

    pub fn channel_properties(
        &self,
        py: Python<'_>,
        group_name: &str,
        channel_name: &str,
    ) -> PyResult<HashMap<String, PyObject>> {
        let channel = self
            .inner
            .channel(group_name, channel_name)
            .ok_or_else(|| PyValueError::new_err(format!("Channel '{}/{}' not found", group_name, channel_name)))?;
        map_properties_to_py(py, &channel.properties)
    }

    pub fn read_channel_f64(&self, group_name: &str, channel_name: &str) -> PyResult<Vec<f64>> {
        self.inner
            .read_channel_data::<f64>(group_name, channel_name)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    pub fn read_channel_f32(&self, group_name: &str, channel_name: &str) -> PyResult<Vec<f32>> {
        self.inner
            .read_channel_data::<f32>(group_name, channel_name)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    pub fn read_channel_i8(&self, group_name: &str, channel_name: &str) -> PyResult<Vec<i8>> {
        self.inner
            .read_channel_data::<i8>(group_name, channel_name)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    pub fn read_channel_i16(&self, group_name: &str, channel_name: &str) -> PyResult<Vec<i16>> {
        self.inner
            .read_channel_data::<i16>(group_name, channel_name)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    pub fn read_channel_i32(&self, group_name: &str, channel_name: &str) -> PyResult<Vec<i32>> {
        self.inner
            .read_channel_data::<i32>(group_name, channel_name)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    pub fn read_channel_i64(&self, group_name: &str, channel_name: &str) -> PyResult<Vec<i64>> {
        self.inner
            .read_channel_data::<i64>(group_name, channel_name)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    pub fn read_channel_u8(&self, group_name: &str, channel_name: &str) -> PyResult<Vec<u8>> {
        self.inner
            .read_channel_data::<u8>(group_name, channel_name)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    pub fn read_channel_u16(&self, group_name: &str, channel_name: &str) -> PyResult<Vec<u16>> {
        self.inner
            .read_channel_data::<u16>(group_name, channel_name)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    pub fn read_channel_u32(&self, group_name: &str, channel_name: &str) -> PyResult<Vec<u32>> {
        self.inner
            .read_channel_data::<u32>(group_name, channel_name)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    pub fn read_channel_u64(&self, group_name: &str, channel_name: &str) -> PyResult<Vec<u64>> {
        self.inner
            .read_channel_data::<u64>(group_name, channel_name)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    pub fn read_channel_bool(&self, group_name: &str, channel_name: &str) -> PyResult<Vec<bool>> {
        self.inner
            .read_channel_data::<bool>(group_name, channel_name)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }
}

#[pyfunction]
pub fn defragment(input_path: &str, output_path: &str) -> PyResult<()> {
    Defragmenter::defragment(input_path, output_path)
        .map_err(|e| PyValueError::new_err(format!("Defragmentation failed: {}", e)))
}

#[pyfunction]
pub fn write_channel_f64(file_path: &str, group_name: &str, channel_name: &str, data: Vec<f64>) -> PyResult<()> {
    let mut writer = TdmsWriter::create(file_path)
        .map_err(|e| PyValueError::new_err(format!("Failed to create writer: {}", e)))?;
    writer
        .write_channel(group_name, channel_name, &data)
        .map_err(|e| PyValueError::new_err(format!("Failed to write channel: {}", e)))
}

#[pymodule]
fn xpTDMS(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyTdmsFile>()?;
    m.add_function(wrap_pyfunction!(defragment, m)?)?;
    m.add_function(wrap_pyfunction!(write_channel_f64, m)?)?;
    Ok(())
}
