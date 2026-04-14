use core::fmt;
use fd_ed25519::{Pubkey, Signature};
use fd_rpc_sys as sys;

use crate::http;
use crate::io::Connection;
use crate::json_scan::JsonScan;
use crate::jsonrpc;
use crate::utils::{self, BufWriter};

pub const DEFAULT_RPC_URL: &str = "https://api.mainnet-beta.solana.com";

const REQUEST_BUF_SZ: usize = 4096;
const BATCH_REQUEST_BUF_SZ: usize = 8192;
const RESPONSE_BUF_SZ: usize = 65536;
const DEFAULT_TIMEOUT_NS: i64 = 30_000_000_000; // 30s
const MAX_RETRIES_429: u32 = 5;
const BACKOFF_BASE_MS: u64 = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Commitment {
    Processed,
    Confirmed,
    Finalized,
}

impl Commitment {
    fn as_str(self) -> &'static str {
        match self {
            Commitment::Processed => "processed",
            Commitment::Confirmed => "confirmed",
            Commitment::Finalized => "finalized",
        }
    }
}

#[derive(Debug)]
pub enum RpcError {
    Io(crate::io::IoError),
    Http(http::HttpError),
    JsonRpc(jsonrpc::JsonRpcError),
    BadResponse(&'static str),
    Base58DecodeFailed,
    ConnectionNotReady,
    Timeout,
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RpcError::Io(e) => write!(f, "I/O: {}", e),
            RpcError::Http(e) => write!(f, "HTTP: {}", e),
            RpcError::JsonRpc(e) => write!(f, "JSON-RPC: {}", e),
            RpcError::BadResponse(msg) => write!(f, "bad response: {}", msg),
            RpcError::Base58DecodeFailed => write!(f, "base58 decode failed"),
            RpcError::ConnectionNotReady => write!(f, "connection not ready"),
            RpcError::Timeout => write!(f, "request timed out"),
        }
    }
}

impl core::error::Error for RpcError {}

impl From<crate::io::IoError> for RpcError {
    fn from(e: crate::io::IoError) -> Self {
        RpcError::Io(e)
    }
}

impl From<http::HttpError> for RpcError {
    fn from(e: http::HttpError) -> Self {
        RpcError::Http(e)
    }
}

impl From<jsonrpc::JsonRpcError> for RpcError {
    fn from(e: jsonrpc::JsonRpcError) -> Self {
        RpcError::JsonRpc(e)
    }
}

#[derive(Debug, Clone)]
pub struct Blockhash {
    pub hash: [u8; 32],
    pub last_valid_block_height: u64,
}

#[derive(Debug, Clone)]
pub struct Version {
    pub solana_core: [u8; 64],
    pub solana_core_len: usize,
    pub feature_set: u64,
}

#[derive(Clone)]
pub struct AccountInfo {
    pub lamports: u64,
    pub owner: [u8; 32],
    pub data: Vec<u8>,
    pub executable: bool,
    pub rent_epoch: u64,
}

impl fmt::Debug for AccountInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccountInfo")
            .field("lamports", &self.lamports)
            .field("owner", &Pubkey::from_bytes(&self.owner).to_base58())
            .field("data", &self.data)
            .field("executable", &self.executable)
            .field("rent_epoch", &self.rent_epoch)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct TokenBalance {
    pub amount: u64,
    pub decimals: u8,
    pub ui_amount_string: [u8; 64],
    pub ui_amount_string_len: usize,
}

#[derive(Debug, Clone)]
pub struct SendTransactionOpts {
    pub skip_preflight: bool,
    pub preflight_commitment: Commitment,
    pub max_retries: Option<u32>,
}

impl Default for SendTransactionOpts {
    fn default() -> Self {
        Self {
            skip_preflight: false,
            preflight_commitment: Commitment::Finalized,
            max_retries: None,
        }
    }
}

#[derive(Clone, Copy)]
pub struct BatchEntry {
    method: &'static str,
    params: [u8; 256],
    params_len: usize,
}

impl BatchEntry {
    #[inline]
    pub fn raw(method: &'static str, params: &[u8]) -> Self {
        let mut entry = Self {
            method,
            params: [0u8; 256],
            params_len: params.len().min(256),
        };
        entry.params[..entry.params_len].copy_from_slice(&params[..entry.params_len]);
        entry
    }

    #[inline]
    pub fn get_slot(commitment: Commitment) -> Self {
        let mut params = [0u8; 256];
        let len = fmt_commitment_params(&mut params, commitment);
        Self {
            method: "getSlot",
            params,
            params_len: len,
        }
    }

    #[inline]
    pub fn get_block_height(commitment: Commitment) -> Self {
        let mut params = [0u8; 256];
        let len = fmt_commitment_params(&mut params, commitment);
        Self {
            method: "getBlockHeight",
            params,
            params_len: len,
        }
    }

    #[inline]
    pub fn get_latest_blockhash(commitment: Commitment) -> Self {
        let mut params = [0u8; 256];
        let len = fmt_commitment_params(&mut params, commitment);
        Self {
            method: "getLatestBlockhash",
            params,
            params_len: len,
        }
    }

    #[inline]
    pub fn get_transaction_count(commitment: Commitment) -> Self {
        let mut params = [0u8; 256];
        let len = fmt_commitment_params(&mut params, commitment);
        Self {
            method: "getTransactionCount",
            params,
            params_len: len,
        }
    }

    #[inline]
    pub fn get_balance(pubkey: &[u8; 32], commitment: Commitment) -> Self {
        let pubkey_b58 = fd_ed25519::base58::encode_32(pubkey);
        let mut params = [0u8; 256];
        let len = fmt_pubkey_commitment_params(&mut params, &pubkey_b58, commitment);
        Self {
            method: "getBalance",
            params,
            params_len: len,
        }
    }

    #[inline]
    pub fn get_health() -> Self {
        Self {
            method: "getHealth",
            params: [0u8; 256],
            params_len: 2,
        }
    }

    #[inline]
    pub fn get_version() -> Self {
        Self {
            method: "getVersion",
            params: [0u8; 256],
            params_len: 2,
        }
    }

    #[inline]
    pub fn get_account_info(pubkey: Pubkey, commitment: Commitment) -> Self {
        let pubkey_b58 = pubkey.to_base58();
        let mut params = [0u8; 256];
        let len = fmt_account_info_params(&mut params, &pubkey_b58, commitment);
        Self {
            method: "getAccountInfo",
            params,
            params_len: len,
        }
    }

    #[inline]
    pub fn get_multiple_accounts(pubkeys: &[Pubkey], commitment: Commitment) -> Self {
        let pubkeys_b58 = pubkeys.iter().map(|p| p.to_base58()).collect::<Vec<_>>();
        let mut params = [0u8; 256];
        let len = fmt_multiple_accounts_params(&mut params, &pubkeys_b58, commitment);
        Self {
            method: "getMultipleAccounts",
            params,
            params_len: len,
        }
    }

    #[inline]
    pub fn get_program_accounts(program_id: Pubkey, commitment: Commitment) -> Self {
        let pubkey_b58 = program_id.to_base58();
        let mut params = [0u8; 256];
        let len = fmt_account_info_params(&mut params, &pubkey_b58, commitment);
        Self {
            method: "getProgramAccounts",
            params,
            params_len: len,
        }
    }

    #[inline]
    pub fn get_token_account_balance(pubkey: &[u8; 32], commitment: Commitment) -> Self {
        let pubkey_b58 = fd_ed25519::base58::encode_32(pubkey);
        let mut params = [0u8; 256];
        let len = fmt_pubkey_commitment_params(&mut params, &pubkey_b58, commitment);
        Self {
            method: "getTokenAccountBalance",
            params,
            params_len: len,
        }
    }

    #[inline]
    fn params(&self) -> &[u8] {
        if self.method == "getHealth" || self.method == "getVersion" {
            b"[]"
        } else {
            &self.params[..self.params_len]
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BatchResults<'a> {
    data: &'a [u8],
}

impl<'a> BatchResults<'a> {
    pub fn get(&self, index: usize) -> Result<JsonScan<'a>, jsonrpc::JsonRpcError> {
        let scan = JsonScan::new(self.data);
        let item = scan
            .index(index)
            .ok_or(jsonrpc::JsonRpcError::MissingField("batch index"))?;
        parse_batch_item(item)
    }

    pub fn iter(&self) -> impl Iterator<Item = Result<JsonScan<'a>, jsonrpc::JsonRpcError>> + 'a {
        let scan = JsonScan::new(self.data);
        scan.array_iter()
            .into_iter()
            .flatten()
            .map(parse_batch_item)
    }
}

fn parse_batch_item(item: JsonScan<'_>) -> Result<JsonScan<'_>, jsonrpc::JsonRpcError> {
    if let Some(error_obj) = item.field("error") {
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
            return Err(jsonrpc::JsonRpcError::RpcError { code, message });
        }
    }
    item.field("result")
        .ok_or(jsonrpc::JsonRpcError::MissingField("result"))
}

struct UrlParts {
    host: [u8; 256],
    host_len: usize,
    port: u16,
    path: [u8; 256],
    path_len: usize,
    use_tls: bool,
}

pub struct SolanaRpcClient {
    conn: Connection,
    next_id: u64,
    url: UrlParts,
    addr: u32,
    timeout_ns: i64,
    scratch: Vec<u8>,
}

impl SolanaRpcClient {
    pub fn connect(url_str: &str) -> Result<Self, RpcError> {
        let mut url_parsed: sys::fd_url_t = unsafe { core::mem::zeroed() };
        let mut err: libc::c_int = 0;
        let result = unsafe {
            sys::fd_url_parse_cstr(
                &mut url_parsed,
                url_str.as_ptr() as *const libc::c_char,
                url_str.len() as u64,
                &mut err,
            )
        };
        if result.is_null() {
            return Err(RpcError::BadResponse("invalid URL"));
        }

        let scheme = unsafe {
            core::slice::from_raw_parts(
                url_parsed.scheme as *const u8,
                url_parsed.scheme_len as usize,
            )
        };
        let use_tls = scheme.len() >= 5 && scheme[..5].eq_ignore_ascii_case(b"https");

        let host_slice = unsafe {
            core::slice::from_raw_parts(url_parsed.host as *const u8, url_parsed.host_len as usize)
        };
        let mut url = UrlParts {
            host: [0u8; 256],
            host_len: host_slice.len().min(255),
            port: 0,
            path: [0u8; 256],
            path_len: 0,
            use_tls,
        };
        url.host[..url.host_len].copy_from_slice(&host_slice[..url.host_len]);

        if url_parsed.port_len > 0 {
            let port_slice = unsafe {
                core::slice::from_raw_parts(
                    url_parsed.port as *const u8,
                    url_parsed.port_len as usize,
                )
            };
            if let Ok(s) = core::str::from_utf8(port_slice) {
                url.port = s.parse().unwrap_or(if use_tls { 443 } else { 80 });
            }
        } else {
            url.port = if use_tls { 443 } else { 80 };
        }

        if url_parsed.tail_len > 0 {
            let tail = unsafe {
                core::slice::from_raw_parts(
                    url_parsed.tail as *const u8,
                    url_parsed.tail_len as usize,
                )
            };
            let path_len = tail.len().min(255);
            url.path[..path_len].copy_from_slice(&tail[..path_len]);
            url.path_len = path_len;
        } else {
            url.path[0] = b'/';
            url.path_len = 1;
        }

        let addr =
            utils::resolve_host(core::str::from_utf8(&url.host[..url.host_len]).unwrap_or(""))
                .ok_or(RpcError::BadResponse("DNS resolution failed"))?;

        let mut conn = Connection::new();
        let hostname = core::str::from_utf8(&url.host[..url.host_len]).ok();
        conn.connect(addr, url.port, use_tls, hostname)?;

        let mut client = Self {
            conn,
            next_id: 1,
            url,
            addr,
            timeout_ns: DEFAULT_TIMEOUT_NS,
            scratch: vec![0u8; RESPONSE_BUF_SZ],
        };

        client.wait_ready()?;
        Ok(client)
    }

    #[inline]
    pub fn set_timeout_ns(&mut self, ns: i64) {
        self.timeout_ns = ns;
    }

    fn wait_ready(&mut self) -> Result<(), RpcError> {
        let start = utils::monotonic_ns();
        loop {
            let r = self.conn.pump();
            if self.conn.is_ready() {
                return Ok(());
            }
            if r.error || r.closed {
                return Err(RpcError::ConnectionNotReady);
            }
            if (utils::monotonic_ns() - start) as i64 >= self.timeout_ns {
                return Err(RpcError::Timeout);
            }
            std::thread::sleep(core::time::Duration::from_micros(100));
        }
    }

    fn reconnect(&mut self) -> Result<(), RpcError> {
        let mut conn = Connection::new();
        let hostname = core::str::from_utf8(&self.url.host[..self.url.host_len]).ok();
        conn.connect(self.addr, self.url.port, self.url.use_tls, hostname)?;
        self.conn = conn;
        self.wait_ready()
    }

    #[inline]
    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        id
    }

    fn call(
        &mut self,
        method: &str,
        params: &[u8],
    ) -> Result<jsonrpc::JsonRpcResponse<'_>, RpcError> {
        for attempt in 0..=MAX_RETRIES_429 {
            let result = match self.send_and_recv(method, params) {
                Err(RpcError::Http(http::HttpError::ConnectionClosed)) => {
                    self.reconnect()?;
                    self.send_and_recv(method, params)
                }
                other => other,
            };
            match result {
                Err(RpcError::Http(http::HttpError::BadStatus(429)))
                    if attempt < MAX_RETRIES_429 =>
                {
                    let delay_ms = BACKOFF_BASE_MS << attempt.min(4);
                    std::thread::sleep(core::time::Duration::from_millis(delay_ms));
                    continue;
                }
                Err(e) => return Err(e),
                Ok((offset, len)) => {
                    return jsonrpc::JsonRpcResponse::parse(&self.scratch[offset..offset + len])
                        .map_err(RpcError::JsonRpc);
                }
            }
        }
        unreachable!()
    }

    fn send_and_recv(&mut self, method: &str, params: &[u8]) -> Result<(usize, usize), RpcError> {
        let id = self.next_id();
        let mut req_buf = [0u8; REQUEST_BUF_SZ];
        let req_len = jsonrpc::serialize_request(&mut req_buf, method, params, id)?;
        let req = &req_buf[..req_len];

        let mut path_buf = [0u8; 256];
        let path_len = self.url.path_len;
        path_buf[..path_len].copy_from_slice(&self.url.path[..path_len]);
        let path = core::str::from_utf8(&path_buf[..path_len]).unwrap_or("/");

        let mut host_buf = [0u8; 256];
        let host_len = self.url.host_len;
        host_buf[..host_len].copy_from_slice(&self.url.host[..host_len]);
        let host = core::str::from_utf8(&host_buf[..host_len]).unwrap_or("localhost");

        http::write_request(&mut self.conn, "POST", path, host, "application/json", req)?;

        let timeout = self.timeout_ns;
        let scratch_base = self.scratch.as_ptr() as usize;
        let resp = http::read_response(&mut self.scratch, &mut self.conn, timeout)?;

        if resp.status != 200 {
            return Err(RpcError::Http(http::HttpError::BadStatus(resp.status)));
        }

        let body_offset = resp.body.as_ptr() as usize - scratch_base;
        let body_len = resp.body.len();
        Ok((body_offset, body_len))
    }

    pub fn get_latest_blockhash(&mut self, commitment: Commitment) -> Result<Blockhash, RpcError> {
        let mut params = [0u8; 128];
        let params_len = fmt_commitment_params(&mut params, commitment);

        let resp = self.call("getLatestBlockhash", &params[..params_len])?;
        let result = resp.result();

        let value = result
            .field("value")
            .ok_or(RpcError::BadResponse("result.value missing"))?;

        let blockhash_str = value
            .field("blockhash")
            .and_then(|v| v.as_str())
            .ok_or(RpcError::BadResponse("missing blockhash"))?;

        let last_valid = value
            .field("lastValidBlockHeight")
            .and_then(|v| v.as_f64())
            .map(|n| n as u64)
            .ok_or(RpcError::BadResponse("missing lastValidBlockHeight"))?;

        let hash = fd_ed25519::base58::decode_32(blockhash_str)
            .map_err(|_| RpcError::Base58DecodeFailed)?;

        Ok(Blockhash {
            hash,
            last_valid_block_height: last_valid,
        })
    }

    pub fn get_balance(
        &mut self,
        pubkey: &[u8; 32],
        commitment: Commitment,
    ) -> Result<u64, RpcError> {
        let pubkey_b58 = fd_ed25519::base58::encode_32(pubkey);

        let mut params = [0u8; 256];
        let params_len = fmt_pubkey_commitment_params(&mut params, &pubkey_b58, commitment);

        let resp = self.call("getBalance", &params[..params_len])?;
        let result = resp.result();

        if result.is_object() {
            result
                .field("value")
                .and_then(|v| v.as_f64())
                .map(|n| n as u64)
                .ok_or(RpcError::BadResponse("missing value"))
        } else {
            resp.result_u64().map_err(|e| e.into())
        }
    }

    pub fn send_transaction(
        &mut self,
        tx_bytes: &[u8],
        opts: &SendTransactionOpts,
    ) -> Result<Signature, RpcError> {
        let encoded = fd_base64::encode(tx_bytes);
        let encoded_str = core::str::from_utf8(&encoded)
            .map_err(|_| RpcError::BadResponse("base64 encode produced invalid utf8"))?;

        let mut params = [0u8; 2048];
        let params_len = fmt_send_tx_params(&mut params, encoded_str, opts);

        let resp = self.call("sendTransaction", &params[..params_len])?;
        let sig_str = resp.result_string()?;
        let sig =
            fd_ed25519::base58::decode_64(sig_str).map_err(|_| RpcError::Base58DecodeFailed)?;
        Ok(Signature::from_bytes(&sig))
    }

    #[inline]
    pub fn get_transaction_count(&mut self, commitment: Commitment) -> Result<u64, RpcError> {
        let mut params = [0u8; 128];
        let params_len = fmt_commitment_params(&mut params, commitment);
        let resp = self.call("getTransactionCount", &params[..params_len])?;
        resp.result_u64().map_err(|e| e.into())
    }

    #[inline]
    pub fn get_slot(&mut self, commitment: Commitment) -> Result<u64, RpcError> {
        let mut params = [0u8; 128];
        let params_len = fmt_commitment_params(&mut params, commitment);
        let resp = self.call("getSlot", &params[..params_len])?;
        resp.result_u64().map_err(|e| e.into())
    }

    #[inline]
    pub fn get_block_height(&mut self, commitment: Commitment) -> Result<u64, RpcError> {
        let mut params = [0u8; 128];
        let params_len = fmt_commitment_params(&mut params, commitment);
        let resp = self.call("getBlockHeight", &params[..params_len])?;
        resp.result_u64().map_err(|e| e.into())
    }

    #[inline]
    pub fn get_health(&mut self) -> Result<(), RpcError> {
        let resp = self.call("getHealth", b"[]")?;
        let _ = resp.result_string()?;
        Ok(())
    }

    #[inline]
    pub fn get_version(&mut self) -> Result<Version, RpcError> {
        let resp = self.call("getVersion", b"[]")?;
        let result = resp.result();

        if !result.is_object() {
            return Err(RpcError::BadResponse("result is not an object"));
        }

        let core_str = result
            .field("solana-core")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let feature_set = result
            .field("feature-set")
            .and_then(|v| v.as_f64())
            .map(|n| n as u64)
            .unwrap_or(0);

        let mut version = Version {
            solana_core: [0u8; 64],
            solana_core_len: core_str.len().min(63),
            feature_set,
        };
        version.solana_core[..version.solana_core_len]
            .copy_from_slice(&core_str.as_bytes()[..version.solana_core_len]);

        Ok(version)
    }

    #[inline]
    pub fn get_account_info(
        &mut self,
        pubkey: Pubkey,
        commitment: Commitment,
    ) -> Result<Option<AccountInfo>, RpcError> {
        let pubkey_b58 = pubkey.to_base58();
        let mut params = [0u8; 256];
        let params_len = fmt_account_info_params(&mut params, &pubkey_b58, commitment);

        let resp = self.call("getAccountInfo", &params[..params_len])?;
        let result = resp.result();

        let value = result
            .field("value")
            .ok_or(RpcError::BadResponse("result.value missing"))?;

        if value.is_null() {
            return Ok(None);
        }

        parse_account_info(&value).map(Some)
    }

    #[inline]
    pub fn get_multiple_accounts(
        &mut self,
        pubkeys: &[Pubkey],
        commitment: Commitment,
    ) -> Result<Vec<Option<AccountInfo>>, RpcError> {
        let pubkeys_b58 = pubkeys.iter().map(|p| p.to_base58()).collect::<Vec<_>>();
        let mut params = vec![0u8; 128 + pubkeys_b58.len() * 64];
        let params_len = fmt_multiple_accounts_params(&mut params, &pubkeys_b58, commitment);

        let resp = self.call("getMultipleAccounts", &params[..params_len])?;
        let result = resp.result();

        let values = result
            .field("value")
            .ok_or(RpcError::BadResponse("result.value missing"))?;

        let iter = values
            .array_iter()
            .ok_or(RpcError::BadResponse("result.value is not an array"))?;

        let mut out = Vec::with_capacity(16);
        for item in iter {
            if item.is_null() {
                out.push(None);
            } else {
                out.push(Some(parse_account_info(&item)?));
            }
        }
        Ok(out)
    }

    #[inline]
    pub fn get_program_accounts(
        &mut self,
        program_id: Pubkey,
        commitment: Commitment,
    ) -> Result<Vec<([u8; 32], AccountInfo)>, RpcError> {
        let program_b58 = program_id.to_base58();
        let mut params = [0u8; 256];
        let params_len = fmt_account_info_params(&mut params, &program_b58, commitment);

        let resp = self.call("getProgramAccounts", &params[..params_len])?;
        let result = resp.result();

        let iter = result
            .array_iter()
            .ok_or(RpcError::BadResponse("result is not an array"))?;

        let mut out = Vec::with_capacity(16);
        for entry in iter {
            let pubkey_str = entry
                .field("pubkey")
                .and_then(|v| v.as_str())
                .ok_or(RpcError::BadResponse("missing pubkey in entry"))?;
            let pubkey = fd_ed25519::base58::decode_32(pubkey_str)
                .map_err(|_| RpcError::Base58DecodeFailed)?;

            let account = entry
                .field("account")
                .ok_or(RpcError::BadResponse("missing account in entry"))?;
            let info = parse_account_info(&account)?;
            out.push((pubkey, info));
        }
        Ok(out)
    }

    #[inline]
    pub fn get_token_account_balance(
        &mut self,
        pubkey: &[u8; 32],
        commitment: Commitment,
    ) -> Result<TokenBalance, RpcError> {
        let pubkey_b58 = fd_ed25519::base58::encode_32(pubkey);
        let mut params = [0u8; 256];
        let params_len = fmt_pubkey_commitment_params(&mut params, &pubkey_b58, commitment);

        let resp = self.call("getTokenAccountBalance", &params[..params_len])?;
        let result = resp.result();

        let value = result
            .field("value")
            .ok_or(RpcError::BadResponse("result.value missing"))?;

        let amount_str = value
            .field("amount")
            .and_then(|v| v.as_str())
            .ok_or(RpcError::BadResponse("missing amount"))?;
        let amount: u64 = amount_str
            .parse()
            .map_err(|_| RpcError::BadResponse("amount is not a u64"))?;

        let decimals = value
            .field("decimals")
            .and_then(|v| v.as_f64())
            .map(|n| n as u8)
            .ok_or(RpcError::BadResponse("missing decimals"))?;

        let ui_str = value
            .field("uiAmountString")
            .and_then(|v| v.as_str())
            .unwrap_or("0");

        let mut tb = TokenBalance {
            amount,
            decimals,
            ui_amount_string: [0u8; 64],
            ui_amount_string_len: ui_str.len().min(63),
        };
        tb.ui_amount_string[..tb.ui_amount_string_len]
            .copy_from_slice(&ui_str.as_bytes()[..tb.ui_amount_string_len]);

        Ok(tb)
    }

    pub fn batch(&mut self, entries: &[BatchEntry]) -> Result<BatchResults<'_>, RpcError> {
        let mut req_buf = [0u8; BATCH_REQUEST_BUF_SZ];
        let req_len = self.serialize_batch(&mut req_buf, entries)?;

        let mut path_buf = [0u8; 256];
        let path_len = self.url.path_len;
        path_buf[..path_len].copy_from_slice(&self.url.path[..path_len]);
        let path = core::str::from_utf8(&path_buf[..path_len]).unwrap_or("/");

        let mut host_buf = [0u8; 256];
        let host_len = self.url.host_len;
        host_buf[..host_len].copy_from_slice(&self.url.host[..host_len]);
        let host = core::str::from_utf8(&host_buf[..host_len]).unwrap_or("localhost");

        http::write_request(
            &mut self.conn,
            "POST",
            path,
            host,
            "application/json",
            &req_buf[..req_len],
        )?;

        let timeout = self.timeout_ns;
        let scratch_base = self.scratch.as_ptr() as usize;
        let resp = http::read_response(&mut self.scratch, &mut self.conn, timeout)?;

        if resp.status != 200 {
            return Err(RpcError::Http(http::HttpError::BadStatus(resp.status)));
        }

        let body_offset = resp.body.as_ptr() as usize - scratch_base;
        let body_len = resp.body.len();

        Ok(BatchResults {
            data: &self.scratch[body_offset..body_offset + body_len],
        })
    }

    fn serialize_batch(
        &mut self,
        buf: &mut [u8],
        entries: &[BatchEntry],
    ) -> Result<usize, RpcError> {
        let mut w = BufWriter::new(buf);

        macro_rules! push {
            ($d:expr) => {
                if !w.write($d) {
                    return Err(RpcError::JsonRpc(jsonrpc::JsonRpcError::SerializeTooLarge));
                }
            };
        }

        push!(b"[");
        for (i, entry) in entries.iter().enumerate() {
            if i > 0 {
                push!(b",");
            }
            let id = self.next_id();
            push!(b"{\"jsonrpc\":\"2.0\",\"id\":");
            let mut itoa_buf = [0u8; 20];
            push!(utils::fmt_u64(id, &mut itoa_buf));
            push!(b",\"method\":\"");
            push!(entry.method.as_bytes());
            push!(b"\",\"params\":");
            push!(entry.params());
            push!(b"}");
        }
        push!(b"]");

        Ok(w.pos())
    }
}

fn parse_account_info(value: &JsonScan<'_>) -> Result<AccountInfo, RpcError> {
    let lamports = value
        .field("lamports")
        .and_then(|v| v.as_f64())
        .map(|n| n as u64)
        .ok_or(RpcError::BadResponse("missing lamports"))?;

    let owner_str = value
        .field("owner")
        .and_then(|v| v.as_str())
        .ok_or(RpcError::BadResponse("missing owner"))?;
    let owner =
        fd_ed25519::base58::decode_32(owner_str).map_err(|_| RpcError::Base58DecodeFailed)?;

    let executable = value
        .field("executable")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let rent_epoch = value
        .field("rentEpoch")
        .and_then(|v| v.as_f64())
        .map(|n| n as u64)
        .unwrap_or(0);

    let data = value
        .field("data")
        .ok_or(RpcError::BadResponse("missing data"))?;

    let data_bytes = if let Some(mut iter) = data.array_iter() {
        let encoded = iter
            .next()
            .and_then(|v| v.as_str())
            .ok_or(RpcError::BadResponse("data[0] is not a string"))?;
        let encoding = iter.next().and_then(|v| v.as_str()).unwrap_or("base64");
        if encoding == "base58" {
            return Err(RpcError::BadResponse(
                "base58 data encoding not supported; request base64",
            ));
        }
        fd_base64::decode(encoded.as_bytes())
            .map_err(|_| RpcError::BadResponse("base64 decode failed"))?
    } else if data.is_string() {
        Vec::new()
    } else {
        Vec::new()
    };

    Ok(AccountInfo {
        lamports,
        owner,
        data: data_bytes,
        executable,
        rent_epoch,
    })
}

#[inline]
fn fmt_commitment_params(buf: &mut [u8], commitment: Commitment) -> usize {
    let s = match commitment {
        Commitment::Processed => b"[{\"commitment\":\"processed\"}]" as &[u8],
        Commitment::Confirmed => b"[{\"commitment\":\"confirmed\"}]",
        Commitment::Finalized => b"[{\"commitment\":\"finalized\"}]",
    };
    let len = s.len().min(buf.len());
    buf[..len].copy_from_slice(&s[..len]);
    len
}

#[inline]
fn fmt_pubkey_commitment_params(buf: &mut [u8], pubkey: &str, commitment: Commitment) -> usize {
    let mut w = BufWriter::new(buf);
    w.write(b"[\"");
    w.write(pubkey.as_bytes());
    w.write(b"\",{\"commitment\":\"");
    w.write(commitment.as_str().as_bytes());
    w.write(b"\"}]");
    w.pos()
}

#[inline]
fn fmt_account_info_params(buf: &mut [u8], pubkey: &str, commitment: Commitment) -> usize {
    let mut w = BufWriter::new(buf);
    w.write(b"[\"");
    w.write(pubkey.as_bytes());
    w.write(b"\",{\"encoding\":\"base64\",\"commitment\":\"");
    w.write(commitment.as_str().as_bytes());
    w.write(b"\"}]");
    w.pos()
}

#[inline]
fn fmt_multiple_accounts_params(
    buf: &mut [u8],
    pubkeys: &[String],
    commitment: Commitment,
) -> usize {
    let mut w = BufWriter::new(buf);
    w.write(b"[[");
    for (i, pk) in pubkeys.iter().enumerate() {
        if i > 0 {
            w.write(b",");
        }
        w.write(b"\"");
        w.write(pk.as_bytes());
        w.write(b"\"");
    }
    w.write(b"],{\"encoding\":\"base64\",\"commitment\":\"");
    w.write(commitment.as_str().as_bytes());
    w.write(b"\"}]");
    w.pos()
}

#[inline]
fn fmt_send_tx_params(buf: &mut [u8], encoded_tx: &str, opts: &SendTransactionOpts) -> usize {
    let mut w = BufWriter::new(buf);

    w.write(b"[\"");
    w.write(encoded_tx.as_bytes());
    w.write(b"\",{\"encoding\":\"base64\"");

    if opts.skip_preflight {
        w.write(b",\"skipPreflight\":true");
    }

    w.write(b",\"preflightCommitment\":\"");
    w.write(opts.preflight_commitment.as_str().as_bytes());
    w.write(b"\"");

    if let Some(retries) = opts.max_retries {
        w.write(b",\"maxRetries\":");
        let mut itoa = [0u8; 20];
        w.write(utils::fmt_u32(retries, &mut itoa));
    }

    w.write(b"}]");
    w.pos()
}
