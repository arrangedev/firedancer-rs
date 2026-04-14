use core::fmt;

use crate::json_scan::JsonScan;
use crate::utils::{self, BufWriter};

#[derive(Debug)]
pub enum JsonRpcError {
    SerializeTooLarge,
    ParseFailed,
    MissingField(&'static str),
    RpcError { code: i64, message: String },
}

impl fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonRpcError::SerializeTooLarge => write!(f, "JSON-RPC request too large"),
            JsonRpcError::ParseFailed => write!(f, "failed to parse JSON-RPC response"),
            JsonRpcError::MissingField(name) => write!(f, "missing field: {}", name),
            JsonRpcError::RpcError { code, message } => {
                write!(f, "RPC error {}: {}", code, message)
            }
        }
    }
}

impl core::error::Error for JsonRpcError {}

// shorthand macro for writing to a buffer with sz safety check
macro_rules! push {
    ($w:expr, $d:expr) => {
        if !$w.write($d) {
            return Err(JsonRpcError::SerializeTooLarge);
        }
    };
}

pub fn serialize_request(
    buf: &mut [u8],
    method: &str,
    params: &[u8],
    id: u64,
) -> Result<usize, JsonRpcError> {
    let mut w = BufWriter::new(buf);

    push!(w, b"{\"jsonrpc\":\"2.0\",\"id\":");
    let mut itoa_buf = [0u8; 20];
    push!(w, utils::fmt_u64(id, &mut itoa_buf));
    push!(w, b",\"method\":\"");
    push!(w, method.as_bytes());
    push!(w, b"\",\"params\":");
    push!(w, params);
    push!(w, b"}");

    Ok(w.pos())
}

pub struct JsonRpcResponse<'a> {
    result: JsonScan<'a>,
}

impl<'a> JsonRpcResponse<'a> {
    pub fn parse(body: &'a [u8]) -> Result<Self, JsonRpcError> {
        let scan = JsonScan::new(body);

        if let Some(error_obj) = scan.field("error") {
            if error_obj.is_object() {
                let code = error_obj
                    .field("code")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(-1);
                let message = error_obj
                    .field("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown error")
                    .to_string();
                return Err(JsonRpcError::RpcError { code, message });
            }
        }

        let result = scan
            .field("result")
            .ok_or(JsonRpcError::MissingField("result"))?;
        Ok(Self { result })
    }

    #[inline]
    pub fn result(&self) -> &JsonScan<'a> {
        &self.result
    }

    #[inline]
    pub fn result_u64(&self) -> Result<u64, JsonRpcError> {
        self.result
            .as_f64()
            .map(|n| n as u64)
            .ok_or(JsonRpcError::MissingField("result (number)"))
    }

    #[inline]
    pub fn result_string(&self) -> Result<&'a str, JsonRpcError> {
        self.result
            .as_str()
            .ok_or(JsonRpcError::MissingField("result (string)"))
    }
}
