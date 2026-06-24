use argon2::password_hash::{Error as PasswordHashError, SaltString};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand_core::OsRng;
use sha2::{Digest, Sha256};

use crate::config::DigestAlgorithm;

pub fn hash_password(password: &str) -> Result<String, PasswordHashError> {
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)?
        .to_string())
}

pub fn digest_ha1(
    algorithm: DigestAlgorithm,
    username: &str,
    realm: &str,
    password: &str,
) -> String {
    digest_hex(algorithm, &format!("{username}:{realm}:{password}"))
}

pub fn verify_password_hash(password_hash: &str, password: &str) -> bool {
    let Ok(parsed_hash) = PasswordHash::new(password_hash) else {
        return false;
    };

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

pub fn validate_password_hash(password_hash: &str) -> bool {
    let Ok(parsed_hash) = PasswordHash::new(password_hash) else {
        return false;
    };

    matches!(
        parsed_hash.algorithm.as_str(),
        "argon2d" | "argon2i" | "argon2id"
    )
}

fn digest_hex(algorithm: DigestAlgorithm, input: &str) -> String {
    match algorithm {
        DigestAlgorithm::Md5 => format!("{:x}", md5::compute(input.as_bytes())),
        DigestAlgorithm::Sha256 => format!("{:x}", Sha256::digest(input.as_bytes())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_password_hash_verifies() {
        let password_hash = hash_password("secret").expect("password should hash");

        assert!(validate_password_hash(&password_hash));
        assert!(verify_password_hash(&password_hash, "secret"));
        assert!(!verify_password_hash(&password_hash, "wrong"));
    }

    #[test]
    fn generates_digest_ha1() {
        assert_eq!(
            digest_ha1(DigestAlgorithm::Md5, "admin", "xylos", "secret"),
            "3f107222b3c92793b26ffcc29bd3672d"
        );
    }
}

#[cfg(test)]
pub fn hash_password_for_test(password: &str) -> String {
    let salt = SaltString::from_b64("c29tZS1maXhlZC1zYWx0").expect("test salt should be valid");
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("test password should hash")
        .to_string()
}
