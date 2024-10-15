use jwt_simple::prelude::*;

use crate::User;

const JWT_DURATION: u64 = 60 * 60 * 24 * 7;
const JWT_ISSUER: &str = "chat_server";
const JWT_AUDIENCE: &str = "chat_web";

pub struct EncodingKey(Ed25519KeyPair);

#[allow(unused)]
pub struct DecodingKey(Ed25519PublicKey);

impl EncodingKey {
    pub fn load(pem: &str) -> Result<Self, jwt_simple::Error> {
        let key = Ed25519KeyPair::from_pem(pem)?;
        Ok(Self(key))
    }

    pub fn sign(&self, user: impl Into<User>) -> Result<String, jwt_simple::Error> {
        let claims = Claims::with_custom_claims(user.into(), Duration::from_secs(JWT_DURATION))
            .with_issuer(JWT_ISSUER)
            .with_audience(JWT_AUDIENCE);
        self.0.sign(claims)
    }
}

impl DecodingKey {
    pub fn load(pem: &str) -> Result<Self, jwt_simple::Error> {
        let key = Ed25519PublicKey::from_pem(pem)?;
        Ok(Self(key))
    }

    #[allow(unused)]
    pub fn verify(&self, token: &str) -> Result<User, jwt_simple::Error> {
        // let mut options = VerificationOptions::default();
        // options.allowed_issuers = Some(HashSet::from_strings(&[JWT_ISSUER]));
        // options.allowed_audiences = Some(HashSet::from_strings(&[JWT_AUDIENCE]));

        let options = VerificationOptions {
            allowed_issuers: Some(HashSet::from_strings(&[JWT_ISSUER])),
            allowed_audiences: Some(HashSet::from_strings(&[JWT_AUDIENCE])),
            ..Default::default()
        };

        let claims = self.0.verify_token::<User>(token, Some(options))?;
        Ok(claims.custom)
    }
}

#[cfg(test)]
mod tests {

    use anyhow::Result;

    use super::*;

    #[test]
    fn jwt_sign_verify_should_work() -> Result<()> {
        // openssl genpkey -algorithm ed25519 -out private.pem
        let encoding_pem = include_str!("../../fixtures/private.pem");
        // openssl pkey -in private.pem -pubout -out public.pem
        let decoding_pem = include_str!("../../fixtures/public.pem");
        let ek = EncodingKey::load(encoding_pem)?;
        let dk = DecodingKey::load(decoding_pem)?;

        let user = User::new(1, "alon", "alon@gmail.com");

        let token = ek.sign(user.clone())?;
        // assert_eq!(token, "");
        let user2 = dk.verify(&token)?;
        assert_eq!(user, user2);

        Ok(())
    }
}
