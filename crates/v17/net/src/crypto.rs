//! `net.minecraft.network.encryption`, ported: the AES/CFB8/NoPadding stream
//! cipher (IV = the shared secret, one keystream across the whole connection),
//! the SHA-1 server-id digest and its `BigInteger.toString(16)` hex form, and
//! the RSA-1024 key handling (`generateKeyPair`, PKCS#1 v1.5 for the shared
//! secret, X.509 SubjectPublicKeyInfo for the encryption request).

use std::fmt;

use aes::Aes128;
use aes::cipher::{BlockEncrypt, KeyInit, generic_array::GenericArray};
use num_bigint::{BigInt, BigUint};
use num_integer::Integer as _;
use sha1::{Digest, Sha1};
use sunset_1_7_core::RandomSource;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CryptoError {
    /// The X.509 public key did not parse.
    BadPublicKey,
    /// RSA decrypt: the PKCS#1 v1.5 padding did not check.
    BadPadding,
    /// RSA keygen: no inverse of 65537 mod λ(n).
    BadKeyPair,
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadPublicKey => write!(f, "Public key reconstitute failed!"),
            Self::BadPadding => write!(f, "Cipher data failed!"),
            Self::BadKeyPair => write!(f, "Key pair generation failed!"),
        }
    }
}

impl std::error::Error for CryptoError {}

/// The RSA key pair the login flow runs on, 1024 bits like vanilla.
#[derive(Clone, Debug)]
pub struct RsaKeyPair {
    pub n: BigUint,
    pub e: BigUint,
    pub d: BigUint,
}

/// The public half a client parses out of the X.509 encryption request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RsaPublicKey {
    pub n: BigUint,
    pub e: BigUint,
}

impl RsaKeyPair {
    pub const KEY_BITS: usize = 1024;
    pub const PUBLIC_EXPONENT: u64 = 65537;

    /// A key pair from its parts (tests and external key sources).
    pub fn from_parts(n: BigUint, e: BigUint, d: BigUint) -> Self {
        Self { n, e, d }
    }

    /// `EncryptionUtils.generateKeyPair`: RSA, 1024 bits.
    pub fn generate(rng: &mut impl RandomSource) -> Result<Self, CryptoError> {
        Self::generate_with_bits(Self::KEY_BITS, rng)
    }

    /// The keygen mechanism at a given modulus size (the tests run smaller).
    pub fn generate_with_bits(
        bits: usize,
        rng: &mut impl RandomSource,
    ) -> Result<Self, CryptoError> {
        for _ in 0..100 {
            let p = random_prime(bits / 2, rng);
            let q = random_prime(bits / 2, rng);
            if p == q {
                continue;
            }
            let n = &p * &q;
            let phi = (&p - 1u32) * (&q - 1u32);
            let e = BigUint::from(Self::PUBLIC_EXPONENT);
            let Some(d) = mod_inverse(&e, &phi) else {
                continue;
            };
            return Ok(Self { n, e, d });
        }
        Err(CryptoError::BadKeyPair)
    }

    pub fn public_key(&self) -> RsaPublicKey {
        RsaPublicKey {
            n: self.n.clone(),
            e: self.e.clone(),
        }
    }

    /// The X.509 `SubjectPublicKeyInfo` DER the encryption request carries.
    pub fn public_key_der(&self) -> Vec<u8> {
        public_key_der(&self.public_key())
    }

    /// `EncryptionUtils.decrypt` with the JCE default "RSA/ECB/PKCS1Padding".
    pub fn decrypt_pkcs1_v15(&self, ciphertext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let padded = self.raw_private_op(ciphertext)?;
        strip_pkcs1_v15(&padded)
    }

    /// Encrypt with PKCS#1 v1.5 padding (the client's encryption response).
    pub fn encrypt_pkcs1_v15(
        &self,
        message: &[u8],
        rng: &mut impl RandomSource,
    ) -> Result<Vec<u8>, CryptoError> {
        self.public_key().encrypt_pkcs1_v15(message, rng)
    }
    /// The private op a decrypt runs: square-and-multiply with `d`.
    fn raw_private_op(&self, input: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let m = BigUint::from_bytes_be(input);
        if m >= self.n {
            return Err(CryptoError::BadPadding);
        }
        let c = m.modpow(&self.d, &self.n);
        Ok(to_fixed_be(&c, (self.n.bits() as usize).div_ceil(8)))
    }
}

impl RsaPublicKey {
    /// PKCS#1 v1.5 encrypt: `00 02 || random nonzero || 00 || message`.
    pub fn encrypt_pkcs1_v15(
        &self,
        message: &[u8],
        rng: &mut impl RandomSource,
    ) -> Result<Vec<u8>, CryptoError> {
        let modulus_bytes = (self.n.bits() as usize).div_ceil(8);
        if message.len() + 11 > modulus_bytes {
            return Err(CryptoError::BadPadding);
        }
        let mut padded = vec![0u8; modulus_bytes];
        padded[0] = 0x00;
        padded[1] = 0x02;
        let pad_len = modulus_bytes - message.len() - 3;
        let mut filled = 0;
        while filled < pad_len {
            let mut chunk = [0u8; 4];
            random_bytes(rng, &mut chunk);
            for &byte in &chunk {
                if filled == pad_len {
                    break;
                }
                if byte != 0 {
                    padded[2 + filled] = byte;
                    filled += 1;
                }
            }
        }
        padded[2 + pad_len] = 0x00;
        padded[3 + pad_len..].copy_from_slice(message);
        Ok(self.raw_public_op(&padded))
    }

    /// The public op: square-and-multiply with the (small) public exponent.
    fn raw_public_op(&self, input: &[u8]) -> Vec<u8> {
        let m = BigUint::from_bytes_be(input);
        let c = m.modpow(&self.e, &self.n);
        to_fixed_be(&c, (self.n.bits() as usize).div_ceil(8))
    }
}

fn strip_pkcs1_v15(padded: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if padded.len() < 11 || padded[0] != 0x00 || padded[1] != 0x02 {
        return Err(CryptoError::BadPadding);
    }
    let Some(separator) = padded[2..].iter().position(|&b| b == 0) else {
        return Err(CryptoError::BadPadding);
    };
    if separator < 8 {
        return Err(CryptoError::BadPadding);
    }
    Ok(padded[3 + separator..].to_vec())
}

/// `EncryptionUtils.crypt`: AES/CFB8/NoPadding with the IV equal to the key.
/// One instance per direction; the keystream runs across the whole connection.
pub struct AesCfb8 {
    cipher: Aes128,
    shift: [u8; 16],
}

impl AesCfb8 {
    pub fn new(key: &[u8; 16]) -> Self {
        Self {
            cipher: Aes128::new(GenericArray::from_slice(key)),
            shift: *key,
        }
    }

    pub fn encrypt(&mut self, data: &mut [u8]) {
        for byte in data.iter_mut() {
            let keystream = self.next_keystream();
            let cipher_byte = *byte ^ keystream;
            self.shift.copy_within(1.., 0);
            self.shift[15] = cipher_byte;
            *byte = cipher_byte;
        }
    }

    pub fn decrypt(&mut self, data: &mut [u8]) {
        for byte in data.iter_mut() {
            let keystream = self.next_keystream();
            let cipher_byte = *byte ^ keystream;
            self.shift.copy_within(1.., 0);
            self.shift[15] = *byte;
            *byte = cipher_byte;
        }
    }

    fn next_keystream(&mut self) -> u8 {
        let mut block = GenericArray::clone_from_slice(&self.shift);
        self.cipher.encrypt_block(&mut block);
        block[0]
    }
}

/// `EncryptionUtils.digest`: SHA-1 over the server id (ISO-8859-1), the X.509
/// public key and the shared secret, in that order.
pub fn server_id_hash(server_id: &str, public_key_der: &[u8], shared_secret: &[u8]) -> [u8; 20] {
    let mut digest = Sha1::new();
    digest.update(latin1_bytes(server_id));
    digest.update(public_key_der);
    digest.update(shared_secret);
    digest.finalize().into()
}

/// `String.getBytes("ISO_8859_1")`: chars past 0xFF become '?'.
fn latin1_bytes(text: &str) -> Vec<u8> {
    text.chars()
        .map(|c| if (c as u32) < 0x100 { c as u8 } else { b'?' })
        .collect()
}

/// `new BigInteger(bytes).toString(16)`: signed, lowercase, no leading zeros.
pub fn java_hex_bigint(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "0".to_string();
    }
    if bytes[0] & 0x80 == 0 {
        return hex_magnitude(bytes);
    }
    // Two's complement negate, then the magnitude in hex behind a '-'.
    let mut magnitude: Vec<u8> = bytes.iter().map(|b| !b).collect();
    for byte in magnitude.iter_mut().rev() {
        let (sum, carry) = byte.overflowing_add(1);
        *byte = sum;
        if !carry {
            break;
        }
    }
    format!("-{}", hex_magnitude(&magnitude))
}

fn hex_magnitude(bytes: &[u8]) -> String {
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let stripped = hex.trim_start_matches('0');
    if stripped.is_empty() {
        "0".to_string()
    } else {
        stripped.to_string()
    }
}

/// `Random.nextBytes` on top of the shared `java.util.Random` port: whole
/// ints, low byte first.
pub fn random_bytes(rng: &mut impl RandomSource, out: &mut [u8]) {
    let mut index = 0;
    while index < out.len() {
        let draw = rng.next_int() as u32;
        let group = (out.len() - index).min(4);
        for (offset, byte) in out[index..index + group].iter_mut().enumerate() {
            *byte = (draw >> (offset * 8)) as u8;
        }
        index += group;
    }
}

/// X.509 `SubjectPublicKeyInfo` DER for an RSA public key, the bytes the
/// encryption request carries and the server-id digest hashes.
pub fn public_key_der(key: &RsaPublicKey) -> Vec<u8> {
    let mut integers = Vec::new();
    der_integer_into(&mut integers, &key.n);
    der_integer_into(&mut integers, &key.e);
    let public_key = der_tlv(0x30, &integers);

    let mut bit_string = vec![0x00];
    bit_string.extend_from_slice(&public_key);
    let bit_string = der_tlv(0x03, &bit_string);

    let mut algorithm = Vec::new();
    algorithm.extend_from_slice(&der_tlv(
        0x06,
        &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01],
    ));
    algorithm.extend_from_slice(&der_tlv(0x05, &[]));
    let algorithm = der_tlv(0x30, &algorithm);

    let mut spki = algorithm;
    spki.extend_from_slice(&bit_string);
    der_tlv(0x30, &spki)
}

/// `EncryptionUtils.generatePublicKey`: parse the X.509 DER back out.
pub fn parse_public_key_der(der: &[u8]) -> Result<RsaPublicKey, CryptoError> {
    let (spki, _) = der_expect(der, 0x30)?;
    let (algorithm, rest) = der_expect(spki, 0x30)?;
    // `algorithm` is the content: the OID, then the NULL.
    let (oid, oid_rest) = der_expect(algorithm, 0x06)?;
    if oid != [0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01].as_slice() {
        return Err(CryptoError::BadPublicKey);
    }
    let (null, tail) = der_expect(oid_rest, 0x05)?;
    if !null.is_empty() || !tail.is_empty() {
        return Err(CryptoError::BadPublicKey);
    }
    let (bit_string, _) = der_expect(rest, 0x03)?;
    if bit_string.first() != Some(&0x00) {
        return Err(CryptoError::BadPublicKey);
    }
    let (public_key, _) = der_expect(&bit_string[1..], 0x30)?;
    let (n, rest) = der_expect(public_key, 0x02)?;
    let (e, tail) = der_expect(rest, 0x02)?;
    if !tail.is_empty() {
        return Err(CryptoError::BadPublicKey);
    }
    Ok(RsaPublicKey {
        n: BigUint::from_bytes_be(n),
        e: BigUint::from_bytes_be(e),
    })
}

fn der_tlv(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    let len = content.len();
    if len < 0x80 {
        out.push(len as u8);
    } else if len <= 0xff {
        out.push(0x81);
        out.push(len as u8);
    } else {
        out.push(0x82);
        out.push((len >> 8) as u8);
        out.push(len as u8);
    }
    out.extend_from_slice(content);
    out
}

fn der_integer_into(out: &mut Vec<u8>, value: &BigUint) {
    let mut bytes = value.to_bytes_be();
    if bytes.first().is_some_and(|b| b & 0x80 != 0) {
        bytes.insert(0, 0x00);
    }
    out.extend_from_slice(&der_tlv(0x02, &bytes));
}

fn der_expect(der: &[u8], tag: u8) -> Result<(&[u8], &[u8]), CryptoError> {
    let (&first, rest) = der.split_first().ok_or(CryptoError::BadPublicKey)?;
    if first != tag {
        return Err(CryptoError::BadPublicKey);
    }
    let (&first_len, content) = rest.split_first().ok_or(CryptoError::BadPublicKey)?;
    // `content` holds the length bytes for the long form, then the value.
    let (len, header) = if first_len & 0x80 == 0 {
        (first_len as usize, 0usize)
    } else {
        let count = (first_len & 0x7f) as usize;
        if count == 0 || count > 2 || content.len() < count {
            return Err(CryptoError::BadPublicKey);
        }
        let mut len = 0usize;
        for &byte in &content[..count] {
            len = (len << 8) | byte as usize;
        }
        (len, count)
    };
    if content.len() < header + len {
        return Err(CryptoError::BadPublicKey);
    }
    Ok((&content[header..header + len], &content[header + len..]))
}

fn to_fixed_be(value: &BigUint, width: usize) -> Vec<u8> {
    let mut bytes = value.to_bytes_be();
    while bytes.len() < width {
        bytes.insert(0, 0x00);
    }
    bytes
}

fn mod_inverse(a: &BigUint, m: &BigUint) -> Option<BigUint> {
    let a = BigInt::from(a.clone());
    let m = BigInt::from(m.clone());
    let egcd = a.extended_gcd(&m);
    let (gcd, x) = (egcd.gcd, egcd.x);
    if gcd != BigInt::from(1u32) {
        return None;
    }
    let inverse = x.mod_floor(&m);
    inverse.to_biguint()
}

/// Miller-Rabin over the shared `java.util.Random` port: deterministic for a
/// given seed.
fn random_prime(bits: usize, rng: &mut impl RandomSource) -> BigUint {
    loop {
        let mut bytes = vec![0u8; bits / 8];
        random_bytes(rng, &mut bytes);
        bytes[0] |= 0x80;
        bytes[bits / 8 - 1] |= 0x01;
        let candidate = BigUint::from_bytes_be(&bytes);
        if is_probable_prime(&candidate, rng) {
            return candidate;
        }
    }
}

const MILLER_RABIN_ROUNDS: usize = 40;
const SMALL_PRIMES: [u32; 29] = [
    2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97,
    101, 103, 107, 109,
];

fn is_probable_prime(n: &BigUint, rng: &mut impl RandomSource) -> bool {
    for &small in &SMALL_PRIMES {
        let small = BigUint::from(small);
        if *n == small {
            return true;
        }
        if n % &small == BigUint::ZERO {
            return false;
        }
    }
    let one = BigUint::from(1u32);
    let n_minus_1 = n - &one;
    let mut exponent = n_minus_1.clone();
    let mut twos: usize = 0;
    while !exponent.bit(0) {
        exponent >>= 1;
        twos += 1;
    }
    let top = n - &BigUint::from(2u32);
    for _ in 0..MILLER_RABIN_ROUNDS {
        let mut bytes = [0u8; 8];
        random_bytes(rng, &mut bytes);
        let base = BigUint::from_bytes_be(&bytes) % &top + &BigUint::from(2u32);
        let mut x = base.modpow(&exponent, n);
        if x == one || x == n_minus_1 {
            continue;
        }
        let mut witness = false;
        for _ in 0..twos.saturating_sub(1) {
            x = (&x * &x) % n;
            if x == n_minus_1 {
                witness = true;
                break;
            }
        }
        if !witness {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use sunset_1_7_core::LegacyRandom;

    /// An RSA-1024 key pair generated with openssl, the independent oracle for
    /// the key handling.
    const N_HEX: &str = "c724b3f551cc4e7a822f8822e40f845469da54c633e1e494f6a0fb1c02718dac40cf5c855d9edb1e652724d4c194de5a684e780505ed5a6aa1f8c6e79b57001498e4ac037b6b7e9a9c2deb4306d1ff0782f600ade4bb7bf403a2ec774d2f472cbe1d1af28ab1440f7384764c90d5f91d158f3667ebd1e72d35c527255965e539";
    const D_HEX: &str = "9c901a7b4fe7d3fb34c6b1bcdc97e66d871b03731fef048aa8ab3f40721b5c4a8459825fc78ed0ae40d059807b00d31f38ca1849bab334568928d117dd64b4c6196253014409f0d31c0e426a075861c9efd2fff34a9cbb3c780c7da2672d123e0f2f0ad261088a1278092fe85bccd3c70927972e80f884bb5ffd77b3e295b541";
    const PUB_DER_HEX: &str = "30819f300d06092a864886f70d010101050003818d0030818902818100c724b3f551cc4e7a822f8822e40f845469da54c633e1e494f6a0fb1c02718dac40cf5c855d9edb1e652724d4c194de5a684e780505ed5a6aa1f8c6e79b57001498e4ac037b6b7e9a9c2deb4306d1ff0782f600ade4bb7bf403a2ec774d2f472cbe1d1af28ab1440f7384764c90d5f91d158f3667ebd1e72d35c527255965e5390203010001";
    /// openssl rsautl -pkcs encryption of 000102..0f under the same key.
    const OPENSSL_CT_HEX: &str = "6248308b8dee0a23d39aa29466296307615edf6fbdd5e614122ec760921827de93a61ee5acaed008302861ba1c5d61c606ff00b5932b00f4042631ee1a2dc3319750b81ee01a41d7bb1def33731bac2c3b111cf1ef45ac857198915cae7a76b809fc55f7d9cf5a02cbfea66b63b298034f6f3ce7081ee5eb8763f56a45298cbc";

    fn openssl_key() -> RsaKeyPair {
        RsaKeyPair::from_parts(
            BigUint::from_bytes_be(&hex_bytes(N_HEX)),
            BigUint::from(65537u64),
            BigUint::from_bytes_be(&hex_bytes(D_HEX)),
        )
    }

    fn hex_bytes(hex: &str) -> Vec<u8> {
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn cfb8_matches_openssl() {
        // AES-128-CFB8 with the IV equal to the key (vanilla's
        // `IvParameterSpec(key.getEncoded())`), verified against openssl
        // `enc -aes-128-cfb8 -K KEY -iv KEY -nopad`.
        let key = [
            0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, 0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf,
            0x4f, 0x3c,
        ];
        let plaintext: Vec<u8> = [
            0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96, 0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93,
            0x17, 0x2a,
        ]
        .to_vec();
        let mut encryptor = AesCfb8::new(&key);
        let mut encrypted = plaintext.clone();
        encryptor.encrypt(&mut encrypted);
        assert_eq!(
            encrypted
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
            "14118e0710eb1b4ca30ae2de24747efb"
        );
        let mut decryptor = AesCfb8::new(&key);
        decryptor.decrypt(&mut encrypted);
        assert_eq!(encrypted, plaintext);
    }

    #[test]
    fn cfb8_keystream_runs_across_calls() {
        // The vanilla cipher is one stream across packets: splitting the same
        // plaintext into two updates must not change the ciphertext.
        let key = [7u8; 16];
        let plaintext: Vec<u8> = (0..40u8).collect();
        let mut whole = plaintext.clone();
        AesCfb8::new(&key).encrypt(&mut whole);
        let mut parts = plaintext.clone();
        let mut cipher = AesCfb8::new(&key);
        cipher.encrypt(&mut parts[..16]);
        cipher.encrypt(&mut parts[16..]);
        assert_eq!(whole, parts);
    }

    #[test]
    fn server_id_hash_matches_sha1() {
        let key: Vec<u8> = (0u8..16).collect();
        let secret = [0xde, 0xad, 0xbe, 0xef].repeat(4);
        let hash = server_id_hash("", &key, &secret);
        // hashlib.sha1(key + secret) with the ISO-8859-1 server id.
        assert_eq!(
            java_hex_bigint(&hash),
            "6d4978479567c32129d6cee773b801a57fdd22bc"
        );
    }

    #[test]
    fn java_hex_handles_negatives() {
        assert_eq!(java_hex_bigint(&[]), "0");
        assert_eq!(java_hex_bigint(&[0x00, 0x01]), "1");
        assert_eq!(java_hex_bigint(&[0x0f]), "f");
        assert_eq!(java_hex_bigint(&[0x80, 0x00]), "-8000");
        assert_eq!(java_hex_bigint(&[0xff]), "-1");
    }

    #[test]
    fn public_key_der_round_trips() {
        let key = openssl_key();
        let der = public_key_der(&key.public_key());
        assert_eq!(der, hex_bytes(PUB_DER_HEX));
        let parsed = parse_public_key_der(&der).unwrap();
        assert_eq!(parsed.n, key.n);
        assert_eq!(parsed.e, key.e);
    }

    #[test]
    fn decrypts_openssl_ciphertext() {
        let key = openssl_key();
        let plaintext = key.decrypt_pkcs1_v15(&hex_bytes(OPENSSL_CT_HEX)).unwrap();
        assert_eq!(plaintext, (0u8..16).collect::<Vec<u8>>());
    }

    #[test]
    fn pkcs1_round_trip_with_openssl_key() {
        let key = openssl_key();
        let mut rng = LegacyRandom::new(42);
        let message = b"secret shared";
        let ciphertext = key.encrypt_pkcs1_v15(message, &mut rng).unwrap();
        assert_eq!(key.decrypt_pkcs1_v15(&ciphertext).unwrap(), message);
    }

    #[test]
    fn keygen_produces_working_pairs() {
        // The mechanism test runs at 128 bits; the login flow uses 1024.
        let mut rng = LegacyRandom::new(7);
        let key = RsaKeyPair::generate_with_bits(128, &mut rng).unwrap();
        let message = b"m";
        let ciphertext = key.encrypt_pkcs1_v15(message, &mut rng).unwrap();
        assert_eq!(key.decrypt_pkcs1_v15(&ciphertext).unwrap(), message);
        let parsed = parse_public_key_der(&public_key_der(&key.public_key())).unwrap();
        assert_eq!(parsed.n, key.n);
    }
}
