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

/// Prior to running the following example, you will need to start up a polkadot
/// `--dev` node to get the genesis hash and runtime metadata. (To do this, go
/// to paritytech/polkadot on github, follow the instructions to obtain the desired
/// binary, and then run `polkadot --dev`) Once the node is running `cd` into
/// into this repo's `examples` directory. (We assume the nodes http RPC port is
/// accessible via default `http://localhost:9933`)
///
/// 1) Get the runtime metadata by running:
///
/// ```bash
/// curl -X POST -H 'Content-Type: application/json' \
/// -d '{"jsonrpc":"2.0","id": 1, "method":"state_getMetadata"}' \
/// -o state_getMetadata_res.json http://localhost:9933
/// ```
///
/// 2) Get the genesis hash by running:
///
/// ```bash
/// curl -X POST -H 'Content-Type: application/json' \
/// -d '{"jsonrpc":"2.0","id": 1, "method":"chain_getBlockHash", "params": [0]}' \
/// -o chain_getBlockHash_res.json http://localhost:9933
/// ```
///
/// 3) Get the runtime version info by running:
///
/// curl -X POST -H 'Content-Type: application/json' \
/// -d '{"jsonrpc":"2.0","id": 1, "method":"chain_getRuntimeVersion" }' \
/// -o chain_getBlockHash_res.json http://localhost:9933
///
/// Then to run this example, go to the root directory and run:
///
/// ```bash
/// cargo run --example offline_balance_transfer
/// ```
#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Create the client
    let (metadata, genesis_hash, runtime_version, properties) = gather_inputs()?;
    let options = OfflineClientOptions {
        genesis_hash,
        metadata,
        properties,
        runtime_version,
    };
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
    signer.set_nonce(0);

    // Create the signed extrinsic
    let signed_extrinsic = client.create_signed_encoded(call, &signer).await?;

    println!("Transaction to submit: {:#?}", signed_extrinsic);

    Ok(())
}


fn gather_inputs() -> Result<(Vec<u8>, Vec<u8>, RuntimeVersion, SystemProperties), Box<dyn std::error::Error>> {
    // Path to the directory where the RPC responses reside
    let base_path = env::current_dir()?.join("examples");

    // `state_getMetadata_res.json` contains the output from
    // ```bash
    // curl -X POST -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","id": 1, "method":"state_getMetadata"}' -o state_getMetadata_res.json http://localhost:9933
    // ```
    // where `http://localhost:9933` is the address of a polkadot node run with `polkadot --dev`
    let path_to_metadata = base_path.clone().join("state_getMetadata_res.json");
    let metadata = util::rpc_to_bytes(path_to_metadata)?;

    let path_to_genesis_hash = base_path.clone().join("chain_getBlockHash_res.json");
    let genesis_hash = util::rpc_to_bytes(path_to_genesis_hash)?;

    let path_to_runtime_version = base_path.clone().join("chain_getRuntimeVersion_res.json");
    let runtime_version = util::rpc_to_runtime_version(path_to_runtime_version)?;

    // let path_to_properties = base_path.clone().join("system_properties_res.json");
    // let properties = util::rpc_to_properties(path_to_properties)?;
    // In this case, system_properties from a --dev node returns an empty object
    let properties = SystemProperties {
        ss58_format: 42,
        token_decimals: 12,
        token_symbol: "UNIT".to_string(),
    };

    Ok((metadata, genesis_hash, runtime_version, properties))
}