use std::collections::hash_map::DefaultHasher;
use std::convert::{TryFrom, TryInto};
use std::fmt::Display;
use std::hash::{Hash, Hasher};

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use prost::Message;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::generated::types;
use crate::types::key::*;
use crate::types::time::DateTimeUtc;
use crate::types::transaction::hash_tx;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error decoding a transaction from bytes: {0}")]
    TxDecodingError(prost::DecodeError),
    #[error("Error decoding an IntentGossipMessage from bytes: {0}")]
    IntentDecodingError(prost::DecodeError),
    #[error("Error decoding an DkgGossipMessage from bytes: {0}")]
    DkgDecodingError(prost::DecodeError),
    #[error("Intent is empty")]
    NoIntentError,
    #[error("Dkg is empty")]
    NoDkgError,
    #[error("Timestamp is empty")]
    NoTimestampError,
    #[error("Timestamp is invalid: {0}")]
    InvalidTimestamp(prost_types::TimestampOutOfSystemRangeError),
}

pub type Result<T> = std::result::Result<T, Error>;

/// This can be used to sign an arbitrary tx. The signature is produced and
/// verified on the tx data concatenated with the tx code, however the tx code
/// itself is not part of this structure.
///
/// Because the signature is not checked by the ledger, we don't inline it into
/// the `Tx` type directly. Instead, the signature is attached to the `tx.data`,
/// which is can then be checked by a validity predicate wasm.
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, BorshSchema)]
pub struct SignedTxData {
    /// The original tx data bytes, if any
    pub data: Option<Vec<u8>>,
    /// The signature is produced on the tx data concatenated with the tx code
    /// and the timestamp.
    pub sig: common::Signature,
}

/// A generic signed data wrapper for Borsh encode-able data.
#[derive(
Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize,
)]
pub struct Signed<T: BorshSerialize + BorshDeserialize> {
    /// Arbitrary data to be signed
    pub data: T,
    /// The signature of the data
    pub sig: common::Signature,
}

impl<T> PartialEq for Signed<T>
    where
        T: BorshSerialize + BorshDeserialize + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data && self.sig == other.sig
    }
}

impl<T> Eq for Signed<T> where
    T: BorshSerialize + BorshDeserialize + Eq + PartialEq
{
}

impl<T> Hash for Signed<T>
    where
        T: BorshSerialize + BorshDeserialize + Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.data.hash(state);
        self.sig.hash(state);
    }
}

impl<T> PartialOrd for Signed<T>
    where
        T: BorshSerialize + BorshDeserialize + PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.data.partial_cmp(&other.data)
    }
}

impl<T> Signed<T>
    where
        T: BorshSerialize + BorshDeserialize,
{
    /// Initialize a new signed data.
    pub fn new(keypair: &common::SecretKey, data: T) -> Self {
        let to_sign = data
            .try_to_vec()
            .expect("Encoding data for signing shouldn't fail");
        let sig = common::SigScheme::sign(keypair, &to_sign);
        Self { data, sig }
    }

    /// Verify that the data has been signed by the secret key
    /// counterpart of the given public key.
    pub fn verify(
        &self,
        pk: &common::PublicKey,
    ) -> std::result::Result<(), VerifySigError> {
        let bytes = self
            .data
            .try_to_vec()
            .expect("Encoding data for verifying signature shouldn't fail");
        common::SigScheme::verify_signature_raw(pk, &bytes, &self.sig)
    }
}

#[derive(
Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize, BorshSchema, Hash,
)]
pub struct Tx {
    pub code: Vec<u8>,
    pub data: Option<Vec<u8>>,
    pub timestamp: DateTimeUtc,
}

impl TryFrom<&[u8]> for Tx {
    type Error = Error;

    fn try_from(tx_bytes: &[u8]) -> Result<Self> {
        let tx = types::Tx::decode(tx_bytes).map_err(Error::TxDecodingError)?;
        let timestamp = match tx.timestamp {
            Some(t) => t.try_into().map_err(Error::InvalidTimestamp)?,
            None => return Err(Error::NoTimestampError),
        };
        Ok(Tx {
            code: tx.code,
            data: tx.data,
            timestamp,
        })
    }
}

impl From<Tx> for types::Tx {
    fn from(tx: Tx) -> Self {
        let timestamp = Some(tx.timestamp.into());
        types::Tx {
            code: tx.code,
            data: tx.data,
            timestamp,
        }
    }
}

impl Tx {
    pub fn new(code: Vec<u8>, data: Option<Vec<u8>>) -> Self {
        Tx {
            code,
            data,
            timestamp: DateTimeUtc::now(),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        let tx: types::Tx = self.clone().into();
        tx.encode(&mut bytes)
            .expect("encoding a transaction failed");
        bytes
    }

    pub fn hash(&self) -> [u8; 32] {
        hash_tx(&self.to_bytes()).0
    }

    pub fn code_hash(&self) -> [u8; 32] {
        hash_tx(&self.code).0
    }

    pub fn tx_to_encrypt(&self) -> (Vec<u8>,Vec<u8>,Vec<u8>,Vec<u8>) {
        let tx_bytes = self.to_bytes().clone();

        let mut code_byte_size : usize = 0;
        if self.code.len() != 0 {
            code_byte_size = self.code.len() as usize;
            code_byte_size+=2;
        }
        let code_bytes = (&tx_bytes[0..code_byte_size]).to_vec();

        // get the bytes of data field from bytes of tx
        let mut data_byte_size : usize = 0;
        if self.data.is_some(){
            data_byte_size = self.data.clone().expect("Getting transaction data shouldn't fail").len() as usize;
            data_byte_size += 2;
        }
        let timestamp_shift :usize =2;
        // get the bytes of timestamp field from bytes of tx
        let mut timestamp_byte_size = tx_bytes[code_byte_size + data_byte_size + 1] as usize;
        let tx_len = tx_bytes.len();
        while timestamp_byte_size + data_byte_size + code_byte_size + timestamp_shift != tx_len
        {
            data_byte_size+=1;
            timestamp_byte_size=tx_bytes[code_byte_size + data_byte_size + 1] as usize;
        }
        timestamp_byte_size+=2;
        let data_bytes  = (&tx_bytes[code_byte_size..
            code_byte_size + data_byte_size]).to_vec();
        let timestamp_bytes =
            (&tx_bytes[code_byte_size + data_byte_size ..
                code_byte_size + data_byte_size + timestamp_byte_size
                ]).to_vec();
        let code_hash = (&hash_tx(&code_bytes).0).to_vec();

        (code_hash, code_bytes, data_bytes, timestamp_bytes)
    }

    pub fn tx_to_sign(&self) -> [u8; 32] {
        let tx_bytes = self.to_bytes();
        let mut code_byte_size : usize = 0;
        if self.code.len() != 0 {
            code_byte_size = self.code.len() as usize;
            code_byte_size+=2;
        }
        let code_bytes :Vec<u8> = (&tx_bytes[0..code_byte_size]).to_vec();

        // get the bytes of data field from bytes of tx
        let mut data_byte_size : usize = 0;
        if self.data.is_some(){
            data_byte_size = self.data.clone().expect("Getting transaction data shouldn't fail").len() as usize;
            data_byte_size += 2;
        }
        let timestamp_shift :usize =2;
        // get the bytes of timestamp field from bytes of tx
        let mut timestamp_byte_size = tx_bytes[code_byte_size + data_byte_size + 1] as usize;
        let tx_len = tx_bytes.len();
        while timestamp_byte_size + data_byte_size + code_byte_size + timestamp_shift != tx_len
        {
            data_byte_size+=1;
            timestamp_byte_size=tx_bytes[code_byte_size + data_byte_size + 1] as usize;
        }
        timestamp_byte_size+=2;
        let data_bytes :&[u8] = &tx_bytes[code_byte_size..
            code_byte_size + data_byte_size];
        let timestamp_bytes :&[u8] =
            &tx_bytes[code_byte_size + data_byte_size ..
                code_byte_size + data_byte_size + timestamp_byte_size
                ];

        let code_hash :&[u8] = &hash_tx(&code_bytes).0;
        hash_tx(&([code_hash,data_bytes,timestamp_bytes].concat())).0
    }

    /// Sign a transaction using [`SignedTxData`].
    pub fn sign(self, keypair: &common::SecretKey) -> Self {
        let to_sign = self.tx_to_sign();
        let sig = common::SigScheme::sign(keypair, &to_sign);
        let signed = SignedTxData {
            data: self.data,
            sig,
        }
            .try_to_vec()
            .expect("Encoding transaction data shouldn't fail");
        Tx {
            code: self.code,
            data: Some(signed),
            timestamp: self.timestamp,
        }
    }

    /// Verify that the transaction has been signed by the secret key
    /// counterpart of the given public key.
    pub fn verify_sig(
        &self,
        pk: &common::PublicKey,
        sig: &common::Signature,
    ) -> std::result::Result<(), VerifySigError> {
        // Try to get the transaction data from decoded `SignedTxData`
        let tx_data = self.data.clone().ok_or(VerifySigError::MissingData)?;
        let signed_tx_data = SignedTxData::try_from_slice(&tx_data[..])
            .expect("Decoding transaction data shouldn't fail");
        let data = signed_tx_data.data;
        let tx = Tx {
            code: self.code.clone(),
            data,
            timestamp: self.timestamp,
        };
        let signed_data = tx.tx_to_sign();
        common::SigScheme::verify_signature_raw(pk, &signed_data, sig)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntentGossipMessage {
    pub intent: Intent,
}

impl TryFrom<&[u8]> for IntentGossipMessage {
    type Error = Error;

    fn try_from(intent_bytes: &[u8]) -> Result<Self> {
        let intent = types::IntentGossipMessage::decode(intent_bytes)
            .map_err(Error::IntentDecodingError)?;
        match &intent.msg {
            Some(types::intent_gossip_message::Msg::Intent(intent)) => {
                Ok(IntentGossipMessage {
                    intent: intent.clone().try_into()?,
                })
            }
            None => Err(Error::NoIntentError),
        }
    }
}

impl From<IntentGossipMessage> for types::IntentGossipMessage {
    fn from(message: IntentGossipMessage) -> Self {
        types::IntentGossipMessage {
            msg: Some(types::intent_gossip_message::Msg::Intent(
                message.intent.into(),
            )),
        }
    }
}

impl IntentGossipMessage {
    pub fn new(intent: Intent) -> Self {
        IntentGossipMessage { intent }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        let message: types::IntentGossipMessage = self.clone().into();
        message
            .encode(&mut bytes)
            .expect("encoding an intent gossip message failed");
        bytes
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub struct DkgGossipMessage {
    pub dkg: Dkg,
}

impl TryFrom<&[u8]> for DkgGossipMessage {
    type Error = Error;

    fn try_from(dkg_bytes: &[u8]) -> Result<Self> {
        let message = types::DkgGossipMessage::decode(dkg_bytes)
            .map_err(Error::DkgDecodingError)?;
        match &message.dkg_message {
            Some(types::dkg_gossip_message::DkgMessage::Dkg(dkg)) => {
                Ok(DkgGossipMessage {
                    dkg: dkg.clone().into(),
                })
            }
            None => Err(Error::NoDkgError),
        }
    }
}

impl From<DkgGossipMessage> for types::DkgGossipMessage {
    fn from(message: DkgGossipMessage) -> Self {
        types::DkgGossipMessage {
            dkg_message: Some(types::dkg_gossip_message::DkgMessage::Dkg(
                message.dkg.into(),
            )),
        }
    }
}

#[allow(dead_code)]
impl DkgGossipMessage {
    pub fn new(dkg: Dkg) -> Self {
        DkgGossipMessage { dkg }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        let message: types::DkgGossipMessage = self.clone().into();
        message
            .encode(&mut bytes)
            .expect("encoding a DKG gossip message failed");
        bytes
    }
}

#[derive(Clone, Debug, PartialEq, Hash, Eq)]
pub struct Intent {
    pub data: Vec<u8>,
    pub timestamp: DateTimeUtc,
}

impl TryFrom<types::Intent> for Intent {
    type Error = Error;

    fn try_from(intent: types::Intent) -> Result<Self> {
        let timestamp = match intent.timestamp {
            Some(t) => t.try_into().map_err(Error::InvalidTimestamp)?,
            None => return Err(Error::NoTimestampError),
        };
        Ok(Intent {
            data: intent.data,
            timestamp,
        })
    }
}

impl From<Intent> for types::Intent {
    fn from(intent: Intent) -> Self {
        let timestamp = Some(intent.timestamp.into());
        types::Intent {
            data: intent.data,
            timestamp,
        }
    }
}

impl Intent {
    pub fn new(data: Vec<u8>) -> Self {
        Intent {
            data,
            timestamp: DateTimeUtc::now(),
        }
    }

    pub fn id(&self) -> IntentId {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        IntentId::from(hasher.finish().to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IntentId(pub Vec<u8>);

impl<T: Into<Vec<u8>>> From<T> for IntentId {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl Display for IntentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(&self.0))
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub struct Dkg {
    pub data: String,
}

impl From<types::Dkg> for Dkg {
    fn from(dkg: types::Dkg) -> Self {
        Dkg { data: dkg.data }
    }
}

impl From<Dkg> for types::Dkg {
    fn from(dkg: Dkg) -> Self {
        types::Dkg { data: dkg.data }
    }
}

#[allow(dead_code)]
impl Dkg {
    pub fn new(data: String) -> Self {
        Dkg { data }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tx() {
        let code = "wasm code".as_bytes().to_owned();
        let data = "arbitrary data".as_bytes().to_owned();
        let tx = Tx::new(code.clone(), Some(data.clone()));

        let bytes = tx.to_bytes();
        let tx_from_bytes =
            Tx::try_from(bytes.as_ref()).expect("decoding failed");
        assert_eq!(tx_from_bytes, tx);

        let types_tx = types::Tx {
            code,
            data: Some(data),
            timestamp: None,
        };
        let mut bytes = vec![];
        types_tx.encode(&mut bytes).expect("encoding failed");
        match Tx::try_from(bytes.as_ref()) {
            Err(Error::NoTimestampError) => {}
            _ => panic!("unexpected result"),
        }
    }

    #[test]
    fn test_serialize_tx() {
        let code = "wasm code".as_bytes().to_owned();
        let data = "arbitrary data".as_bytes().to_owned();
        let tx = Tx::new(code.clone(), Some(data.clone()));

        let encoded_tx = tx.try_to_vec().unwrap();
        let decoded_tx = Tx::try_from_slice(&encoded_tx).unwrap();
        assert_eq!(tx, decoded_tx);
    }

    #[test]
    fn test_intent_gossip_message() {
        let data = "arbitrary data".as_bytes().to_owned();
        let intent = Intent::new(data);
        let message = IntentGossipMessage::new(intent);

        let bytes = message.to_bytes();
        let message_from_bytes = IntentGossipMessage::try_from(bytes.as_ref())
            .expect("decoding failed");
        assert_eq!(message_from_bytes, message);
    }

    #[test]
    fn test_dkg_gossip_message() {
        let data = "arbitrary string".to_owned();
        let dkg = Dkg::new(data);
        let message = DkgGossipMessage::new(dkg);

        let bytes = message.to_bytes();
        let message_from_bytes = DkgGossipMessage::try_from(bytes.as_ref())
            .expect("decoding failed");
        assert_eq!(message_from_bytes, message);
    }

    #[test]
    fn test_intent() {
        let data = "arbitrary data".as_bytes().to_owned();
        let intent = Intent::new(data.clone());

        let types_intent: types::Intent = intent.clone().into();
        let intent_from_types =
            Intent::try_from(types_intent).expect("no timestamp");
        assert_eq!(intent_from_types, intent);

        let types_intent = types::Intent {
            data,
            timestamp: None,
        };
        match Intent::try_from(types_intent) {
            Err(Error::NoTimestampError) => {}
            _ => panic!("unexpected result"),
        }
    }

    #[test]
    fn test_dkg() {
        let data = "arbitrary string".to_owned();
        let dkg = Dkg::new(data);

        let types_dkg: types::Dkg = dkg.clone().into();
        let dkg_from_types = Dkg::from(types_dkg);
        assert_eq!(dkg_from_types, dkg);
    }
}
