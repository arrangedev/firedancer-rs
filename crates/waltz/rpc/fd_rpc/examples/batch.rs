use fd_ed25519::pubkey;
use fd_rpc::{BatchEntry, Commitment, SolanaRpcClient, DEFAULT_RPC_URL};

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0xf) as usize] as char);
    }
    s
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
        BatchEntry::get_token_account_balance(token_a, Commitment::Confirmed),
        BatchEntry::get_token_account_balance(token_b, Commitment::Confirmed),
    ];

    println!("Sending batch of {} requests...", entries.len());
    let results = client.batch(&entries).expect("batch call failed");

    let blockhash = results.get_blockhash(0).expect("getLatestBlockhash failed");
    println!("Blockhash: {}", hex(&blockhash.hash));
    println!(
        "Last valid block height: {}",
        blockhash.last_valid_block_height
    );

    println!(
        "\nCurrent slot: {}",
        results.get_u64(1).expect("getSlot failed")
    );
    println!(
        "Block height: {}",
        results.get_u64(2).expect("getBlockHeight failed")
    );
    println!(
        "Transaction count: {}",
        results.get_u64(3).expect("getTransactionCount failed")
    );

    let version = results.get_version(4).expect("getVersion failed");
    let core_str =
        core::str::from_utf8(&version.solana_core[..version.solana_core_len]).unwrap_or("unknown");
    println!("\nSolana core: {}", core_str);
    println!("Feature set: {}", version.feature_set);

    match results
        .get_account_info(5)
        .expect("getAccountInfo (whirlpool) failed")
    {
        Some(info) => println!("\nWhirlpool SOL/USDC lamports: {}", info.lamports),
        None => println!("\nWhirlpool SOL/USDC: account not found"),
    }

    let token_a_bal = results
        .get_token_balance(6)
        .expect("getTokenAccountBalance (token A) failed");
    let ui_a =
        core::str::from_utf8(&token_a_bal.ui_amount_string[..token_a_bal.ui_amount_string_len])
            .unwrap_or("?");
    println!("Token A: {} (decimals: {})", ui_a, token_a_bal.decimals);

    let token_b_bal = results
        .get_token_balance(7)
        .expect("getTokenAccountBalance (token B) failed");
    let ui_b =
        core::str::from_utf8(&token_b_bal.ui_amount_string[..token_b_bal.ui_amount_string_len])
            .unwrap_or("?");
    println!("Token B: {} (decimals: {})", ui_b, token_b_bal.decimals);
}
