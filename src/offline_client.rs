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

//! An offline version of the client that is suitable for use on air gapped
//! machines.

use frame_metadata::RuntimeMetadataPrefixed;
use::core::{ marker::PhantomData, convert::TryInto};
use sp_version::RuntimeVersion;
use sp_runtime::traits::SignedExtension;
use codec::Decode;
use sp_core::Bytes;

use crate::{
    error::Error,
    extrinsic::{self, SignedExtra, Signer, UncheckedExtrinsic},
    runtimes::Runtime,
    metadata::Metadata,
    rpc::SystemProperties,
    frame::Call,
    Encoded
};
/// OfflineClientBuilder for constructing a client on an air gapped device
#[derive(Default)]
pub struct OfflineClientBuilder<T: Runtime> {
    _marker: std::marker::PhantomData<T>,
    page_size: Option<u32>,
}

/// Required options for building `OfflineClient`.
pub struct OfflineClientOptions<T: Runtime> {
    genesis_hash: T::Hash,
    // TODO figure out how to read in from file
    metadata: Bytes,
    // DEV NOTE properties and runtime_version can probs just be hardcoded in a constants file
    properties: SystemProperties,
    runtime_version: RuntimeVersion,
}


impl<T: Runtime> OfflineClientBuilder<T> {
    /// Create a new `OfflineClientBuilder`
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
            page_size: None,
        }
    }

    /// Set the page size.
    pub fn set_page_size(mut self, size: u32) -> Self {
        self.page_size = Some(size);
        self
    }

    /// Create a new `OfflineClient`
    pub fn build(
        self,
        opts: OfflineClientOptions<T>,
    ) -> Result<OfflineClient<T>, Error> {
        let metadata_prefixed: RuntimeMetadataPrefixed = Decode::decode(&mut &opts.metadata[..])?;
        let metadata: Metadata = metadata_prefixed.try_into()?;

        Ok(OfflineClient {
            genesis_hash: opts.genesis_hash,
            metadata,
            properties: opts.properties,
            runtime_version: opts.runtime_version,
            _marker: PhantomData,
            page_size: self.page_size.unwrap_or(10),
        })
    }
}

/// Client for creating and signing transactions on an air gapped device
pub struct OfflineClient<T: Runtime> {
    genesis_hash: T::Hash,
    metadata: Metadata,
    properties: SystemProperties,
    runtime_version: RuntimeVersion,
    _marker: PhantomData<(fn() -> T::Signature, T::Extra)>,
    page_size: u32,
}

impl<T: Runtime> Clone for OfflineClient<T> {
    fn clone(&self) -> Self {
        Self {
            genesis_hash: self.genesis_hash,
            metadata: self.metadata.clone(),
            properties: self.properties.clone(),
            runtime_version: self.runtime_version.clone(),
            _marker: PhantomData,
            page_size: self.page_size,
        }
    }
}

impl<T: Runtime> OfflineClient<T> {
    /// Returns the genesis hash.
    pub fn genesis(&self) -> &T::Hash {
        &self.genesis_hash
    }

    /// Returns the chain metadata.
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// Returns the system properties
    pub fn properties(&self) -> &SystemProperties {
        &self.properties
    }

    /// Encodes a call.
    pub fn encode<C: Call<T>>(&self, call: C) -> Result<Encoded, Error> {
        Ok(self
            .metadata()
            .module_with_calls(C::MODULE)
            .and_then(|module| module.call(C::FUNCTION, call))?)
    }

    /// Creates an unsigned extrinsic.
    pub fn create_unsigned<C: Call<T> + Send + Sync>(
        &self,
        call: C,
    ) -> Result<UncheckedExtrinsic<T>, Error> {
        let call = self.encode(call)?;
        Ok(extrinsic::create_unsigned::<T>(call))
    }

    /// Creates a signed extrinsic.
    pub async fn create_signed<C: Call<T> + Send + Sync>(
        &self,
        call: C,
        signer: &(dyn Signer<T> + Send + Sync),
    ) -> Result<UncheckedExtrinsic<T>, Error>
    where
        <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned:
            Send + Sync,
    {
        if signer.nonce().is_none() {
            return Err(Error::from("Signer needs a nonce set for air gapped extrinsic construction."));
        }
        let account_nonce = signer.nonce().unwrap();

        let call = self.encode(call)?;
        let signed = extrinsic::create_signed(
            &self.runtime_version,
            self.genesis_hash,
            account_nonce,
            call,
            signer,
        )
        .await?;
        Ok(signed)
    }
}

pub mod util {
    //! Utilities for using the offline client

    use super::*;
    use std::fs::File;
    use std::io::prelude::*;
    use serde::{Deserialize, Serialize};
    use hex;

    #[derive(Serialize, Deserialize)]
    struct RPCResponse {
        jsonrpc: String,
        result: String
    }

    /// Read in runtime metadata from the JSON response to an RPC.
    ///
    /// The is expected to contain a JSON object with the form:
    ///
    /// ```no_run
    /// {"jsonrpc":"2.0","result":"0xddb9934d1ef19d9b1cb1e10857b6e4a24fe6c495d7a8632288235c1412538b84","id":1}
    /// ```
    ///
    /// where `result` is the the field to return as `Bytes`.
    pub fn rpc_response_to_bytes(file_name: &str) -> Result<Vec<u8>, Error> {
        let mut file = File::open(file_name)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let rpc_response: RPCResponse = serde_json::from_str(&contents)?;
        let bytes = hex::decode(rpc_response.result)?;

        Ok(bytes)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//      #[test]
//     fn test result_to_bytes() {

//     }

// }