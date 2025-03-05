use base64::{prelude::BASE64_URL_SAFE_NO_PAD, Engine};
use p256::ecdsa::{signature::Verifier, Signature, VerifyingKey};
use rsa::{pkcs1v15, BigUint, RsaPublicKey};
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Deserialize, Debug)]
pub enum SigningAlg {
    RS256 = -257,
    ES256 = -7,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct JoyIdIdentity {
    pub key_type: String,
    pub public_key: String,
}

impl JoyIdIdentity {
    pub fn from(data: &str) -> Self {
        serde_json::from_str(data).unwrap()
    }
}

// "{"signature":"","alg":-7,"message":""}"
#[derive(Deserialize, Debug)]
pub struct JoyIdSignature {
    pub signature: String,
    pub message: String,
    pub alg: i16,
}

impl JoyIdSignature {
    pub fn from(data: &str) -> Self {
        serde_json::from_str(data).unwrap()
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct JoyIdData {
    pub signature: String,
    pub identity: String,
    pub sign_type: String,
}

impl JoyIdData {
    pub fn from(data: &str) -> Self {
        serde_json::from_str(data).unwrap()
    }
}

fn decode_base64(input: &str) -> Vec<u8> {
    BASE64_URL_SAFE_NO_PAD.decode(input).expect("Input invalid")
}

fn verify_native_key_signature(
    challenge: &str,
    identity: JoyIdIdentity,
    signature: JoyIdSignature,
) -> bool {
    let mut pub_key_bytes = hex::decode(identity.public_key).expect("Public Key invalid");
    if pub_key_bytes.len() == 64 {
        pub_key_bytes.insert(0, 0x04);
    }

    let message_bytes = decode_base64(&signature.message);
    let sig_bytes = decode_base64(&signature.signature);

    let auth_data = &message_bytes[..37];
    let client_data = &message_bytes[37..];

    let mut hasher = Sha256::new();
    hasher.update(client_data);
    let client_data_hash = hasher.finalize();

    if !client_data
        .windows(challenge.len())
        .any(|w| w == challenge.as_bytes())
    {
        return false;
    }

    let mut signature_base = Vec::new();
    signature_base.extend_from_slice(auth_data);
    signature_base.extend_from_slice(&client_data_hash);
    if signature.alg == SigningAlg::ES256 as i16 {
        let verifying_key = VerifyingKey::from_sec1_bytes(&pub_key_bytes).unwrap();

        let signature = Signature::from_der(&sig_bytes).expect("Convert signature failed.");
        return verifying_key.verify(&signature_base, &signature).is_ok();
    }

    false
}

fn verify_session_key_signature(message: &str, signature: &str, pubkey_hex: &str) -> bool {
    let pub_key_bytes = hex::decode(pubkey_hex).expect("Public Key invalid");
    let message_bytes = decode_base64(message);
    let sig_bytes = decode_base64(signature);
    let e = &pub_key_bytes[..3];
    let n = &pub_key_bytes[4..];

    let rsa_pubkey = RsaPublicKey::new(BigUint::from_bytes_be(n), BigUint::from_bytes_be(e))
        .expect("Faild to create public key rsa");

    let verifying_key = pkcs1v15::VerifyingKey::<Sha256>::new(rsa_pubkey);
    let signature =
        pkcs1v15::Signature::try_from(sig_bytes.as_ref()).expect("Faild to convert signature");
    verifying_key.verify(&message_bytes, &signature).is_ok()
}

pub fn verify_signature(challenge: &str, data: JoyIdData) -> bool {
    let challenge_b64 = BASE64_URL_SAFE_NO_PAD.encode(challenge);
    let identity = JoyIdIdentity::from(&data.identity);
    let joyid_signature = JoyIdSignature::from(&data.signature);

    if identity.key_type == "main_key" || identity.key_type == "sub_key" {
        return verify_native_key_signature(&challenge_b64, identity, joyid_signature);
    }

    verify_session_key_signature(&challenge_b64, &data.signature, &identity.public_key)
}
