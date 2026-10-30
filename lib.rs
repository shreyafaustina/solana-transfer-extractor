#[allow(unused)]
mod pb;

use pb::{mydata::v1 as mydata, sf::substreams::solana::v1::Transactions};

const SYSTEM_PROGRAM_ID: &str = "11111111111111111111111111111111";

#[substreams::handlers::map]
fn map_my_data(transactions: Transactions) -> mydata::MyData {
    let mut output = mydata::MyData::default();

    for confirmed_tx in transactions.transactions {
        let Some(meta) = confirmed_tx.meta.as_option() else {
            continue;
        };

        // Ignore failed transactions.
        if meta.err.as_option().is_some_and(|err| !err.err.is_empty()) {
            continue;
        }

        let Some(transaction) = confirmed_tx.transaction.as_option() else {
            continue;
        };

        let Some(message) = transaction.message.as_option() else {
            continue;
        };

        // Include static and address-table account keys.
        let mut account_keys = message.account_keys.clone();
        account_keys.extend(meta.loaded_writable_addresses.clone());
        account_keys.extend(meta.loaded_readonly_addresses.clone());

        // Regular instructions.
        for instruction in &message.instructions {
            extract_transfer(
                instruction.program_id_index,
                &instruction.accounts,
                &instruction.data,
                &account_keys,
                &mut output,
            );
        }

        // Inner instructions (instructions called by other programs).
        for group in &meta.inner_instructions {
            for instruction in &group.instructions {
                extract_transfer(
                    instruction.program_id_index,
                    &instruction.accounts,
                    &instruction.data,
                    &account_keys,
                    &mut output,
                );
            }
        }
    }

    output
}

fn extract_transfer(
    program_id_index: u32,
    accounts: &[u8],
    data: &[u8],
    account_keys: &[Vec<u8>],
    output: &mut mydata::MyData,
) {
    // A System Program Transfer instruction is exactly 12 bytes.
    if data.len() != 12 || accounts.len() < 2 {
        return;
    }

    // Instruction type 2 = Transfer.
    let instruction_type =
        u32::from_le_bytes([data[0], data[1], data[2], data[3]]);

    if instruction_type != 2 {
        return;
    }

    // Verify the instruction's program ID.
    let Some(program_id) = account_keys.get(program_id_index as usize) else {
        return;
    };

    if bs58::encode(program_id).into_string() != SYSTEM_PROGRAM_ID {
        return;
    }

    let from_index = accounts[0] as usize;
    let to_index = accounts[1] as usize;

    let (Some(from), Some(to)) = (
        account_keys.get(from_index),
        account_keys.get(to_index),
    ) else {
        return;
    };

    // Decode the amount in lamports.
    let amount = u64::from_le_bytes([
        data[4], data[5], data[6], data[7],
        data[8], data[9], data[10], data[11],
    ]);

    output.transfers.push(mydata::Transfer {
        from: bs58::encode(from).into_string(),
        to: bs58::encode(to).into_string(),
        amount,
    });
}
