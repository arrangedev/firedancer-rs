use fd_ed25519::pubkey;
use fd_rpc::{Commitment, SolanaRpcClient, DEFAULT_RPC_URL};

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

    println!("Connected. Fetching latest blockhash...");

    let blockhash = client
        .get_latest_blockhash(Commitment::Finalized)
        .expect("getLatestBlockhash failed");

    println!("Blockhash: {}", hex(&blockhash.hash));
    println!(
        "Last valid block height: {}",
        blockhash.last_valid_block_height
    );

    println!("\nFetching slot...");
    let slot = client
        .get_slot(Commitment::Finalized)
        .expect("getSlot failed");
    println!("Current slot: {}", slot);

    println!("\nFetching block height...");
    let height = client
        .get_block_height(Commitment::Finalized)
        .expect("getBlockHeight failed");
    println!("Block height: {}", height);

    println!("\nFetching transaction count...");
    let tx_count = client
        .get_transaction_count(Commitment::Finalized)
        .expect("getTransactionCount failed");
    println!("Transaction count: {}", tx_count);

    println!("\nFetching version...");
    let version = client.get_version().expect("getVersion failed");
    let core_str =
        core::str::from_utf8(&version.solana_core[..version.solana_core_len]).unwrap_or("?");
    println!("Solana core: {}", core_str);
    println!("Feature set: {}", version.feature_set);

    println!("\nFetching Whirlpool SOL/USDC...");
    let account = pubkey!("Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE");
    let account_info = client
        .get_account_info(account, Commitment::Confirmed)
        .expect("getAccountInfo failed");
    println!("Account info: {:?}", account_info);

    let token_accounts = &[
        pubkey!("EUuUbDcafPrmVTD5M6qoJAoyyNbihBhugADAxRMn5he9"),
        pubkey!("2WLWEuKDgkDUccTpbwYp1GToYktiSB1cXvreHUwiSUVP"),
    ];
    let token_accounts_info = client
        .get_multiple_accounts(token_accounts, Commitment::Confirmed)
        .expect("getMultipleAccounts failed");
    println!("Token accounts info: {:?}", token_accounts_info);
}
