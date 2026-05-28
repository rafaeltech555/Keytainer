use hmac::{Hmac, Mac};
use sha1::Sha1;
use sha2::{Sha256, Sha512};

use crate::error::{AppError, AppResult};
use crate::vault::{TotpAlg, TotpEntry};

/// Decode a base32 (RFC 4648, uppercase, no padding) secret.
pub fn decode_secret(secret: &str) -> AppResult<Vec<u8>> {
    // Strip whitespace and uppercase since users often paste pretty-printed secrets.
    let cleaned: String = secret.chars().filter(|c| !c.is_whitespace()).collect();
    let cleaned = cleaned.to_ascii_uppercase();
    base32::decode(base32::Alphabet::Rfc4648 { padding: false }, &cleaned)
        .ok_or(AppError::InvalidTotpSecret)
}

/// Compute the TOTP code for the given moment.
pub fn code_at(entry: &TotpEntry, unix_seconds: u64) -> AppResult<String> {
    let secret = decode_secret(&entry.secret)?;
    let counter = unix_seconds / entry.period.max(1) as u64;
    let counter_be = counter.to_be_bytes();

    let raw_code = match entry.algorithm {
        TotpAlg::Sha1 => hotp_sha1(&secret, &counter_be)?,
        TotpAlg::Sha256 => hotp_sha256(&secret, &counter_be)?,
        TotpAlg::Sha512 => hotp_sha512(&secret, &counter_be)?,
    };

    let digits = entry.digits.clamp(6, 10) as u32;
    let modulus = 10u32.pow(digits);
    Ok(format!(
        "{:0>width$}",
        raw_code % modulus,
        width = digits as usize
    ))
}

/// Seconds remaining in the current TOTP period.
pub fn remaining_seconds(entry: &TotpEntry, unix_seconds: u64) -> u32 {
    let period = entry.period.max(1);
    period - (unix_seconds % period as u64) as u32
}

fn truncate(bytes: &[u8]) -> u32 {
    let offset = (bytes[bytes.len() - 1] & 0x0f) as usize;
    ((bytes[offset] & 0x7f) as u32) << 24
        | (bytes[offset + 1] as u32) << 16
        | (bytes[offset + 2] as u32) << 8
        | (bytes[offset + 3] as u32)
}

fn hotp_sha1(secret: &[u8], counter_be: &[u8; 8]) -> AppResult<u32> {
    let mut mac = <Hmac<Sha1> as Mac>::new_from_slice(secret)
        .map_err(|e| AppError::Crypto(format!("hmac key: {e}")))?;
    mac.update(counter_be);
    Ok(truncate(&mac.finalize().into_bytes()))
}

fn hotp_sha256(secret: &[u8], counter_be: &[u8; 8]) -> AppResult<u32> {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(secret)
        .map_err(|e| AppError::Crypto(format!("hmac key: {e}")))?;
    mac.update(counter_be);
    Ok(truncate(&mac.finalize().into_bytes()))
}

fn hotp_sha512(secret: &[u8], counter_be: &[u8; 8]) -> AppResult<u32> {
    let mut mac = <Hmac<Sha512> as Mac>::new_from_slice(secret)
        .map_err(|e| AppError::Crypto(format!("hmac key: {e}")))?;
    mac.update(counter_be);
    Ok(truncate(&mac.finalize().into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 6238 reference test vectors. Secret is the ASCII bytes of:
    //   SHA-1   -> "12345678901234567890"                                (20 bytes)
    //   SHA-256 -> "12345678901234567890123456789012"                    (32 bytes)
    //   SHA-512 -> "1234567890..." repeated to 64 bytes                  (64 bytes)
    fn b32(bytes: &[u8]) -> String {
        base32::encode(base32::Alphabet::Rfc4648 { padding: false }, bytes)
    }

    fn entry(secret_bytes: &[u8], alg: TotpAlg, digits: u8) -> TotpEntry {
        TotpEntry {
            secret: b32(secret_bytes),
            algorithm: alg,
            digits,
            period: 30,
            issuer: None,
        }
    }

    #[test]
    fn rfc6238_sha1_8digits() {
        let secret = b"12345678901234567890";
        let e = entry(secret, TotpAlg::Sha1, 8);
        assert_eq!(code_at(&e, 59).unwrap(), "94287082");
        assert_eq!(code_at(&e, 1111111109).unwrap(), "07081804");
        assert_eq!(code_at(&e, 1111111111).unwrap(), "14050471");
        assert_eq!(code_at(&e, 1234567890).unwrap(), "89005924");
        assert_eq!(code_at(&e, 2000000000).unwrap(), "69279037");
    }

    #[test]
    fn rfc6238_sha256_8digits() {
        let secret = b"12345678901234567890123456789012";
        let e = entry(secret, TotpAlg::Sha256, 8);
        assert_eq!(code_at(&e, 59).unwrap(), "46119246");
        assert_eq!(code_at(&e, 1111111109).unwrap(), "68084774");
        assert_eq!(code_at(&e, 1234567890).unwrap(), "91819424");
    }

    #[test]
    fn rfc6238_sha512_8digits() {
        let secret = b"1234567890123456789012345678901234567890123456789012345678901234";
        let e = entry(secret, TotpAlg::Sha512, 8);
        assert_eq!(code_at(&e, 59).unwrap(), "90693936");
        assert_eq!(code_at(&e, 1111111109).unwrap(), "25091201");
        assert_eq!(code_at(&e, 1234567890).unwrap(), "93441116");
    }

    #[test]
    fn default_6_digit_code() {
        let secret = b"12345678901234567890";
        let e = entry(secret, TotpAlg::Sha1, 6);
        // Last 6 of "94287082" is "287082"
        assert_eq!(code_at(&e, 59).unwrap(), "287082");
    }

    #[test]
    fn handles_pretty_printed_secret() {
        let secret = b"12345678901234567890";
        let pretty = b32(secret)
            .as_bytes()
            .chunks(4)
            .map(std::str::from_utf8)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join(" ")
            .to_ascii_lowercase();

        let e = TotpEntry {
            secret: pretty,
            algorithm: TotpAlg::Sha1,
            digits: 8,
            period: 30,
            issuer: None,
        };
        assert_eq!(code_at(&e, 59).unwrap(), "94287082");
    }

    #[test]
    fn invalid_base32_errors() {
        let e = TotpEntry {
            secret: "this is not base32!!!".into(),
            algorithm: TotpAlg::Sha1,
            digits: 6,
            period: 30,
            issuer: None,
        };
        let err = code_at(&e, 0).unwrap_err();
        assert!(matches!(err, AppError::InvalidTotpSecret));
    }

    #[test]
    fn remaining_seconds_wraps_within_period() {
        let e = entry(b"12345678901234567890", TotpAlg::Sha1, 6);
        assert_eq!(remaining_seconds(&e, 0), 30);
        assert_eq!(remaining_seconds(&e, 1), 29);
        assert_eq!(remaining_seconds(&e, 29), 1);
        assert_eq!(remaining_seconds(&e, 30), 30);
    }
}
