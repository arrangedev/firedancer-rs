use fd_ed25519::pubkey;
use fd_rpc::json_scan::JsonScan;
use fd_rpc::{BatchEntry, Commitment, RpcError, SolanaRpcClient, DEFAULT_RPC_URL};

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0xf) as usize] as char);
    }
    s
}

fn parse_blockhash(result: &JsonScan<'_>) -> Result<([u8; 32], u64), RpcError> {
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
    let hash =
        fd_ed25519::base58::decode_32(blockhash_str).map_err(|_| RpcError::Base58DecodeFailed)?;
    Ok((hash, last_valid))
}

fn parse_u64(result: &JsonScan<'_>) -> Result<u64, RpcError> {
    result
        .as_f64()
        .map(|n| n as u64)
        .ok_or(RpcError::BadResponse("expected number"))
}

fn parse_version<'a>(result: &'a JsonScan<'a>) -> Result<(&'a str, u64), RpcError> {
    let core_str = result
        .field("solana-core")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let feature_set = result
        .field("feature-set")
        .and_then(|v| v.as_f64())
        .map(|n| n as u64)
        .unwrap_or(0);
    Ok((core_str, feature_set))
}

fn parse_account_info(result: &JsonScan<'_>) -> Result<Option<u64>, RpcError> {
    let value = result
        .field("value")
        .ok_or(RpcError::BadResponse("result.value missing"))?;
    if value.is_null() {
        return Ok(None);
    }
    let lamports = value
        .field("lamports")
        .and_then(|v| v.as_f64())
        .map(|n| n as u64)
        .ok_or(RpcError::BadResponse("missing lamports"))?;
    Ok(Some(lamports))
}

fn main() {
    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_RPC_URL.to_string());

    println!("Connecting to {}...", url);

    let mut client = SolanaRpcClient::connect(&url).expect("Failed to connect");

    let whirlpool = pubkey!("Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE");
    let token_a = pubkey!("EUuUbDcafPrmVTD5M6qoJAoyyNbihBhugADAxRMn5he9");
    let token_b = pubkey!("2WLWEuKDgkDUccTpbwYp1GToYktiSB1cXvreHUwiSUVP");

    let entries = [
        BatchEntry::get_latest_blockhash(Commitment::Finalized),
        BatchEntry::get_slot(Commitment::Finalized),
        BatchEntry::get_block_height(Commitment::Finalized),
        BatchEntry::get_transaction_count(Commitment::Finalized),
        BatchEntry::get_version(),
        BatchEntry::get_account_info(whirlpool, Commitment::Confirmed),
        BatchEntry::get_account_info(token_a, Commitment::Confirmed),
        BatchEntry::get_account_info(token_b, Commitment::Confirmed),
    ];

    println!("Sending batch of {} requests...", entries.len());
    let results = client.batch(&entries).expect("batch call failed");

    let blockhash = results.get(0).expect("getLatestBlockhash failed");
    let (hash, last_valid) = parse_blockhash(&blockhash).expect("parse blockhash");
    println!("Blockhash: {}", hex(&hash));
    println!("Last valid block height: {}", last_valid);

    let slot_scan = results.get(1).expect("getSlot failed");
    println!(
        "\nCurrent slot: {}",
        parse_u64(&slot_scan).expect("parse slot")
    );

    let height_scan = results.get(2).expect("getBlockHeight failed");
    println!(
        "Block height: {}",
        parse_u64(&height_scan).expect("parse block height")
    );

    let tx_count_scan = results.get(3).expect("getTransactionCount failed");
    println!(
        "Transaction count: {}",
        parse_u64(&tx_count_scan).expect("parse tx count")
    );

    let version_scan = results.get(4).expect("getVersion failed");
    let (core_str, feature_set) = parse_version(&version_scan).expect("parse version");
    println!("\nSolana core: {}", core_str);
    println!("Feature set: {}", feature_set);

    let whirlpool_scan = results.get(5).expect("getAccountInfo (whirlpool) failed");
    match parse_account_info(&whirlpool_scan).expect("parse whirlpool") {
        Some(lamports) => println!("\nWhirlpool SOL/USDC lamports: {}", lamports),
        None => println!("\nWhirlpool SOL/USDC: account not found"),
    }

    let token_a_scan = results.get(6).expect("getAccountInfo (token A) failed");
    match parse_account_info(&token_a_scan).expect("parse token A") {
        Some(lamports) => println!("Token A lamports: {}", lamports),
        None => println!("Token A: account not found"),
    }

    let token_b_scan = results.get(7).expect("getAccountInfo (token B) failed");
    match parse_account_info(&token_b_scan).expect("parse token B") {
        Some(lamports) => println!("Token B lamports: {}", lamports),
        None => println!("Token B: account not found"),
    }
}
