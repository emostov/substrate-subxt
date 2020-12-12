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

use ::core::{convert::TryInto, marker::PhantomData};
use codec::Decode;
use frame_metadata::RuntimeMetadataPrefixed;
use sp_runtime::traits::SignedExtension;
pub use sp_version::RuntimeVersion;
use codec::Encode;

use crate::{
    error::Error,
    extrinsic::{self, SignedExtra, Signer, UncheckedExtrinsic},
    frame::Call,
    metadata::Metadata,
    rpc::SystemProperties,
    runtimes::Runtime,
    Encoded,
};
/// OfflineClientBuilder for constructing a client on an air gapped device
#[derive(Default)]
pub struct OfflineClientBuilder<T: Runtime> {
    _marker: std::marker::PhantomData<T>,
}

/// Required options for building `OfflineClient`.
pub struct OfflineClientOptions {
    /// Scale encoded genesis hash
    pub genesis_hash: Vec<u8>, // TODO: should this offer the option to pass in `T::Hash`?
    /// Scale encoded metadata with prefix
    pub metadata: Vec<u8>, // TODO: should this offer the option to pass in `Metadata`?
    /// SystemProperties
    pub properties: SystemProperties,
    /// RuntimeVersion
    pub runtime_version: RuntimeVersion,
}

impl<T: Runtime> OfflineClientBuilder<T> {
    /// Create a new `OfflineClientBuilder`
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }

    /// Create a new `OfflineClient`
    pub fn build(self, opts: OfflineClientOptions) -> Result<OfflineClient<T>, Error> {
        let metadata_prefixed: RuntimeMetadataPrefixed = Decode::decode(&mut &opts.metadata[..])?;
        let metadata: Metadata = metadata_prefixed.try_into()?;

        let genesis_hash: T::Hash = Decode::decode(&mut &opts.genesis_hash[..])?;

        Ok(OfflineClient {
            genesis_hash,
            metadata,
            properties: opts.properties,
            runtime_version: opts.runtime_version,
            _marker: PhantomData,
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
}

impl<T: Runtime> Clone for OfflineClient<T> {
    fn clone(&self) -> Self {
        Self {
            genesis_hash: self.genesis_hash,
            metadata: self.metadata.clone(),
            properties: self.properties.clone(),
            runtime_version: self.runtime_version.clone(),
            _marker: PhantomData,
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
            return Err(
                "Signer needs a nonce set for air gapped extrinsic construction.".into(),
            );
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

    /// Created an encoded, signed extrinsic that is ready to broadcast.
    pub async fn create_signed_encoded<C: Call<T> + Send + Sync>(
        &self,
        call: C,
        signer: &(dyn Signer<T> + Send + Sync),
    ) -> Result<String, Error>
    where
    <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    {
        let signed_extrinsic = self.create_signed(call, signer).await?.encode();
        let hex = format!("{}{}", "0x", hex::encode(signed_extrinsic));

        Ok(hex)
    }
}

pub mod util {
    //! Utilities for using the offline client
    // TODO: Should these utils be moved out of repo to examples to reduce Api
    // maintenance concern with breaking changes?

    use super::*;
    use std::path::PathBuf;
    use hex;
    use serde::{Deserialize, Serialize};
    use sp_runtime::DeserializeOwned;
    use std::fs::File;
    use std::io::prelude::*;

    /// The shape of an RPC JSON response object
    #[derive(Serialize, Deserialize)]
    struct RpcRes<T> {
        jsonrpc: String,
        result: T,
    }

    /// Read in a scale encoded hex `result` from the response to a RPC call.
    ///
    /// The file expected to contain a JSON object with the form:
    ///
    /// ```no_run
    /// {"jsonrpc":"2.0","result":"0xff","id":1}
    /// ```
    ///
    /// where `result` is a field representing scale encoded bytes.
    pub fn rpc_to_bytes(path: PathBuf) -> Result<Vec<u8>, Error> {
        let contents = file_to_string(path)?;

        let rpc_response: RpcRes<String> = serde_json::from_str(&contents)?;
        // remove `0x` from the hex string.
        let hex = &rpc_response.result[2..];
        let bytes = hex::decode(hex)?;

        Ok(bytes)
    }

    /// Deserialize a struct from the `result` in a JSON response to the
    /// RPC `system_properties`. (Relevant structs to deserialize include
    /// `SystemProperties` and `RuntimeVersion`.)
    ///
    /// The file expected to contain a JSON object with the form:
    ///
    /// ```no_run
    /// {"jsonrpc":"2.0","result":"...","id":1}
    /// ```
    ///
    /// where `result` is a field representing a struct in JSON.
    pub fn rpc_to_struct<T:  DeserializeOwned>(path: PathBuf) -> Result<T, Error> {
        let contents = file_to_string(path)?;

        let rpc_response: RpcRes<T> = serde_json::from_str(&contents)?;

        Ok(rpc_response.result)
    }

    /// Read a file to a string (non-buffered).
    fn file_to_string(path: PathBuf) -> Result<String, Error> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        Ok(contents)
    }
}
