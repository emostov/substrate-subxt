// Copyright 2019-2020 Parity Technologies (UK) Ltd.
// This file is part of substrate-subxt.
//
// subxt is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// subxt is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with substrate-subxt.  If not, see <http://www.gnu.org/licenses/>.

use sp_keyring::AccountKeyring;
use substrate_subxt::{KusamaRuntime, PairSigner, SystemProperties, balances, offline_client::{
    OfflineClientBuilder,
    OfflineClientOptions,
    util,
    RuntimeVersion
}};
use std::env;

/// **N.B.** [At the time of writing all transactions default to being immortal.
/// Please learn more about best practices with transaction mortality before
/// continuing.](https://wiki.polkadot.network/docs/en/build-protocol-info#transaction-mortality)
///
/// We use a `--dev` node for this example because it easily gives us access to
/// the canonical Alice and Bob accounts which have pre-seeded funds from genesis.
///
/// To get a local development node started, follow the instructions in the
/// paritytech/polkadot README and then start the dev node with the command
/// described [here](https://github.com/paritytech/polkadot#development).
///
/// For this example, we assume the nodes http RPC port is accessible via
///`http://localhost:9933`, which is the default.
///
/// Prior to running the following example, you will need to start up a polkadot
/// `--dev` node to get the genesis hash, runtime metadata, and runtime version.
/// Note that you will also need system properties, which we hard code for this
/// example due to the setup of the `--dev` node. In most cases, system properties
/// and genesis hash can be hardcoded for a network while runtime metadata and
/// runtime version will need to be updated after a runtime upgrade.
///
/// Below are `curl` commands to fetch the aforementioned information and write
/// to .json files. Note the files the curl command outputs correspond to
/// to the file names the example code expects. For air gapped signing, these
/// would need to be fetched with an online machine and transferred to the air
/// gapped machine.
///
/// Make sure to run the following in the root directory of this project.
///
/// 1) Get the runtime metadata by running:
///
/// ```bash
/// curl -X POST -H 'Content-Type: application/json' \
/// -d '{"jsonrpc":"2.0","id": 1, "method":"state_getMetadata"}' \
/// -o metadata.json http://localhost:9933
/// ```
///
/// 2) Get the genesis hash by running:
///
/// ```bash
/// curl -X POST -H 'Content-Type: application/json' \
/// -d '{"jsonrpc":"2.0","id": 1, "method":"chain_getBlockHash", "params": [0]}' \
/// -o genesis_hash.json http://localhost:9933
/// ```
///
/// 3) Get the runtime version info by running:
///
/// curl -X POST -H 'Content-Type: application/json' \
/// -d '{"jsonrpc":"2.0","id": 1, "method":"chain_getRuntimeVersion" }' \
/// -o runtime_version.json http://localhost:9933
///
/// Then to run this example, go to the root directory and run:
///
/// ```bash
/// cargo run --example offline_balance_transfer
/// ```
#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Gather RPC derived inputs. This wold be done on an online device
    let (metadata, genesis_hash, runtime_version, properties) = gather_inputs()?;

    // Create the client
    let options = OfflineClientOptions {
        genesis_hash,
        metadata,
        properties,
        runtime_version,
    };
    // We use `KusamaRuntime` here, which (at the time of writing) works with Polkadot, Kusama, and
    // Westend, among others. If types used in KusamaRuntime change in a network upgrade this may
    // no longer be compatible.
    let client = OfflineClientBuilder::<KusamaRuntime>::new().build(options)?;

    // Declare the extrinsic arguments. Note: You can find structs for declaring
    // arguments in the subxt modules corresponding to pallet name.
    let dest = AccountKeyring::Bob.to_account_id();
    let call = balances::TransferCall {
        to: &dest,
        amount: 12_345,
    };

    let mut signer = PairSigner::new(AccountKeyring::Alice.pair());
    // N.B. The signer must have a nonce set. On a related note, remember to increment the nonce.
    signer.set_nonce(0); // Assume this is Alice's first transaction

    // Create the signed extrinsic, which can be copy + pasted as is from the terminal and broadcasted
    let signed_extrinsic = client.create_signed_encoded(call, &signer).await?;

    println!("Transaction to submit: {:#?}", signed_extrinsic);

    Ok(())
}


fn gather_inputs() -> Result<(Vec<u8>, Vec<u8>, RuntimeVersion, SystemProperties), Box<dyn std::error::Error>> {
    // Path to the directory where the RPC responses reside
    let base_path = env::current_dir()?;

    let path_to_metadata = base_path.clone().join("metadata.json");
    let metadata = util::rpc_to_bytes(path_to_metadata)?;

    let path_to_genesis_hash = base_path.clone().join("genesis_hash.json");
    let genesis_hash = util::rpc_to_bytes(path_to_genesis_hash)?;

    let path_to_runtime_version = base_path.clone().join("runtime_version.json");
    let runtime_version = util::rpc_to_struct::<RuntimeVersion>(path_to_runtime_version)?;

    // let path_to_properties = base_path.clone().join("properties.json");
    // let properties = util::rpc_to_properties(path_to_properties)?;
    // In this case, system_properties from a --dev node returns an empty object.
    let properties = SystemProperties {
        ss58_format: 42,
        token_decimals: 12,
        token_symbol: "UNIT".to_string(),
    };

    Ok((metadata, genesis_hash, runtime_version, properties))
}