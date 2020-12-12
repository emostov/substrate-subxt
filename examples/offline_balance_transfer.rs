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
use substrate_subxt::{KusamaRuntime, PairSigner, SystemProperties, balances::*, offline_client::{
    OfflineClientBuilder,
    OfflineClientOptions,
    util,
    RuntimeVersion
}};

/// Prior to running the following example, you will need to start up a polkadot
/// `--dev` node to get the genesis hash and runtime metadata. (To do this, go
/// to paritytech/polkadot on github, follow the instructions to obtain the desired
/// binary, and then run `polkadot --dev`) Once the node is running `cd` into
/// into this repo's `examples` directory.
///
/// 1) Get the runtime metadata by running
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


#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let signer = PairSigner::new(AccountKeyring::Alice.pair());
    let dest = AccountKeyring::Bob.to_account_id().into();


    let (metadata, genesis_hash, runtime_version, properties) = gather_inputs()?;

    let options = OfflineClientOptions {
        genesis_hash,
        metadata,
        properties,
        runtime_version,
    };

    let client = OfflineClientBuilder::<KusamaRuntime>::new().build(options)?;

    let hash = client.transfer(&signer, &dest, 10_000).await?;

    println!("Balance transfer extrinsic submitted: {}", hash);

    Ok(())
}


fn gather_inputs() -> Result<(Vec<u8>, Vec<u8>, RuntimeVersion, SystemProperties), Box<dyn std::error::Error>> {
    // `state_getMetadata_res.json` contains the output from
    // ```bash
    // curl -X POST -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","id": 1, "method":"state_getMetadata"}' -o state_getMetadata_res.json http://localhost:9933
    // ```
    // where `http://localhost:9933` is the address of a polkadot node run with `polkadot --dev`
    let metadata = util::rpc_to_bytes("./state_getMetadata_res.json")?;

    // `chain_getBlockHash_res.json` contains the output from
    // ```bash
    // curl -X POST -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","id": 1, "method":"chain_getBlockHash", "params": [0]}' -o chain_getBlockHash_res.json http://localhost:9933
    // ```
    let genesis_hash = util::rpc_to_bytes("./chain_getBlockHash_res.json")?;

    let runtime_version = util::rpc_to_runtime_version("./chain_getRuntimeVersion_res.json")?;

    let properties = util::rpc_to_properties("./system_properties_res.json")?;

    Ok((metadata, genesis_hash, runtime_version, properties))
}