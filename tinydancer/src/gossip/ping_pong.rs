use {
    bincode::{serialize, Error},
    lru::LruCache,
    rand::{CryptoRng, Fill, Rng},
    serde::{Deserialize, Serialize},
    solana_sdk::{
        hash::{self, Hash},
        pubkey::Pubkey,
        sanitize::{Sanitize, SanitizeError},
        signature::{Keypair, Signable, Signature, Signer},
    },
    std::{
        borrow::Cow,
        net::SocketAddr,
        time::{Duration, Instant},
    },
};

const PING_PONG_HASH_PREFIX: &[u8] = "SOLANA_PING_PONG".as_bytes();

#[derive(Debug, Deserialize, Serialize)]
pub struct Ping<T> {
    from: Pubkey,
    token: T,
    signature: Signature,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Pong {
    from: Pubkey,
    hash: Hash, // Hash of received ping token.
    signature: Signature,
}

/// Maintains records of remote nodes which have returned a valid response to a
/// ping message, and on-the-fly ping messages pending a pong response from the
/// remote node.
pub struct PingCache {
    // Time-to-live of received pong messages.
    ttl: Duration,
    // Rate limit delay to generate pings for a given address
    rate_limit_delay: Duration,
    // Timestamp of last ping message sent to a remote node.
    // Used to rate limit pings to remote nodes.
    pings: LruCache<(Pubkey, SocketAddr), Instant>,
    // Verified pong responses from remote nodes.
    pongs: LruCache<(Pubkey, SocketAddr), Instant>,
    // Hash of ping tokens sent out to remote nodes,
    // pending a pong response back.
    pending_cache: LruCache<Hash, (Pubkey, SocketAddr)>,
}

impl<T: Serialize> Ping<T> {
    pub fn new(token: T, keypair: &Keypair) -> Result<Self, Error> {
        let signature = keypair.sign_message(&serialize(&token)?);
        let ping = Ping {
            from: keypair.pubkey(),
            token,
            signature,
        };
        Ok(ping)
    }
}

impl<T> Ping<T>
where
    T: Serialize + Fill + Default,
{
    pub fn new_rand<R>(rng: &mut R, keypair: &Keypair) -> Result<Self, Error>
    where
        R: Rng + CryptoRng,
    {
        let mut token = T::default();
        rng.fill(&mut token);
        Ping::new(token, keypair)
    }
}

impl<T> Sanitize for Ping<T> {
    fn sanitize(&self) -> Result<(), SanitizeError> {
        self.from.sanitize()?;
        // TODO Add self.token.sanitize()?; when rust's
        // specialization feature becomes stable.
        self.signature.sanitize()
    }
}

impl<T: Serialize> Signable for Ping<T> {
    fn pubkey(&self) -> Pubkey {
        self.from
    }

    fn signable_data(&self) -> Cow<[u8]> {
        Cow::Owned(serialize(&self.token).unwrap())
    }

    fn get_signature(&self) -> Signature {
        self.signature
    }

    fn set_signature(&mut self, signature: Signature) {
        self.signature = signature;
    }
}

impl Pong {
    pub fn new<T: Serialize>(ping: &Ping<T>, keypair: &Keypair) -> Result<Self, Error> {
        let token = serialize(&ping.token)?;
        let hash = hash::hashv(&[PING_PONG_HASH_PREFIX, &token]);
        let pong = Pong {
            from: keypair.pubkey(),
            hash,
            signature: keypair.sign_message(hash.as_ref()),
        };
        Ok(pong)
    }

    pub fn from(&self) -> &Pubkey {
        &self.from
    }
}

impl Sanitize for Pong {
    fn sanitize(&self) -> Result<(), SanitizeError> {
        self.from.sanitize()?;
        self.hash.sanitize()?;
        self.signature.sanitize()
    }
}

impl Signable for Pong {
    fn pubkey(&self) -> Pubkey {
        self.from
    }

    fn signable_data(&self) -> Cow<[u8]> {
        Cow::Owned(self.hash.as_ref().into())
    }

    fn get_signature(&self) -> Signature {
        self.signature
    }

    fn set_signature(&mut self, signature: Signature) {
        self.signature = signature;
    }
}
