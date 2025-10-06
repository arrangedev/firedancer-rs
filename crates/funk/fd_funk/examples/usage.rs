use fd_funk::{Funk, FunkBuilder, RecordKey, Result, TransactionId};

fn main() -> Result<()> {
    let funk = FunkBuilder::new()
        .with_max_transactions(10)
        .with_max_records(100)
        .build_with_alloc()?;

    println!("[created_instance]: max_txns=10, backing=heap");

    initial(&funk)?;
    branching(&funk)?;

    show_state(&funk)?;

    let metrics = funk.metrics();
    println!(
        "[final_metrics]: workspace_backed={}, transaction_full={}",
        metrics.workspace_backed, metrics.transaction_full
    );

    Ok(())
}

fn initial(funk: &Funk) -> Result<()> {
    let txn_id = TransactionId::generate();
    let txn = funk.prepare_transaction(None, &txn_id)?;

    let records = [
        ("user:alice", "active"),
        ("user:bob", "inactive"),
        ("counter", "42"),
        ("config:debug", "true"),
    ];

    for (key_str, value_str) in records.iter() {
        let key = RecordKey::from_str(key_str)?;
        funk.insert_record(&txn, &key, value_str.as_bytes())?;
        println!(
            "[inserted_record]: key={}, data_len={}",
            key_str,
            value_str.len()
        );
    }

    funk.publish_transaction(&txn)?;
    println!("[published_transaction]: id={}", txn_id);

    Ok(())
}

fn branching(funk: &Funk) -> Result<()> {
    let shared_key = RecordKey::from_str("shared_data")?;

    let txn1_id = TransactionId::generate();
    let txn1 = funk.prepare_transaction(None, &txn1_id)?;
    funk.insert_record(&txn1, &shared_key, b"version_1")?;
    println!("[branch_1]: set shared_data=version_1");

    let txn2_id = TransactionId::generate();
    let txn2 = funk.prepare_transaction(Some(&txn1), &txn2_id)?;
    funk.insert_record(&txn2, &shared_key, b"version_2")?;
    println!("[branch_2]: set shared_data=version_2");

    let txn3_id = TransactionId::generate();
    let txn3 = funk.prepare_transaction(Some(&txn1), &txn3_id)?;
    funk.insert_record(&txn3, &shared_key, b"version_3")?;
    println!("[branch_3]: set shared_data=version_3");

    let record2 = funk.query_record(&txn2, &shared_key)?;
    let record3 = funk.query_record(&txn3, &shared_key)?;
    println!(
        "[query_branch_2]: shared_data={}",
        String::from_utf8_lossy(record2.value())
    );
    println!(
        "[query_branch_3]: shared_data={}",
        String::from_utf8_lossy(record3.value())
    );

    funk.publish_transaction(&txn2)?;
    println!("[published_winner]: branch_2 published, branch_3 cancelled");

    Ok(())
}

fn show_state(funk: &Funk) -> Result<()> {
    let root = funk.root_transaction();

    let keys_to_check = [
        "user:alice",
        "user:bob",
        "counter",
        "config:debug",
        "shared_data",
    ];

    for key_str in keys_to_check.iter() {
        let key = RecordKey::from_str(key_str)?;
        match funk.query_record(&root, &key) {
            Ok(record) => {
                let value_str = String::from_utf8_lossy(record.value());
                println!("[final_state]: {}={}", key_str, value_str);
            }
            Err(_) => {
                println!("[final_state]: {}=<not found>", key_str);
            }
        }
    }

    Ok(())
}
