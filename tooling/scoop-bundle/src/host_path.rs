use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Value, json};

pub fn absolute(path: &Path) -> Result<PathBuf> {
    Ok(if path.is_absolute() {
        path.into()
    } else {
        std::env::current_dir()?.join(path)
    })
}

pub fn value(path: &Path) -> Value {
    if let Some(text) = path.to_str() {
        return json!(text);
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        json!({"encoding": "UnixBytes", "bytes": path.as_os_str().as_bytes()})
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        json!({"encoding": "WindowsWtf16Le", "bytes": path.as_os_str().encode_wide().flat_map(u16::to_le_bytes).collect::<Vec<_>>()})
    }
}

pub fn decode(value: &Value) -> Result<PathBuf> {
    if let Some(text) = value.as_str() {
        return Ok(text.into());
    }
    let encoding = value["encoding"]
        .as_str()
        .context("missing host path encoding")?;
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        let bytes = match encoding {
            "UnixBytes" => serde_json::from_value::<Vec<u8>>(value["bytes"].clone())?,
            "unix-bytes" => {
                let hex = value["hex"].as_str().context("missing path hex")?;
                if !hex.is_ascii() || hex.len() % 2 != 0 {
                    bail!("invalid path hex");
                }
                (0..hex.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&hex[i..i + 2], 16))
                    .collect::<std::result::Result<Vec<_>, _>>()?
            }
            _ => bail!("incompatible host path encoding {encoding}"),
        };
        if bytes.contains(&0) {
            bail!("NUL in host path");
        }
        Ok(OsString::from_vec(bytes).into())
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        let units = match encoding {
            "WindowsWtf16Le" => {
                let bytes: Vec<u8> = serde_json::from_value(value["bytes"].clone())?;
                if bytes.len() % 2 != 0 {
                    bail!("invalid UTF-16 path");
                }
                bytes
                    .chunks_exact(2)
                    .map(|v| u16::from_le_bytes([v[0], v[1]]))
                    .collect::<Vec<_>>()
            }
            "windows-wide" => serde_json::from_value(value["units"].clone())?,
            _ => bail!("incompatible host path encoding {encoding}"),
        };
        if units.contains(&0) {
            bail!("NUL in host path");
        }
        Ok(OsString::from_wide(&units).into())
    }
}

pub fn serialize<S: Serializer>(path: &Path, serializer: S) -> Result<S::Ok, S::Error> {
    value(path).serialize(serializer)
}

pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<PathBuf, D::Error> {
    decode(&Value::deserialize(deserializer)?).map_err(serde::de::Error::custom)
}
