//! Double Ratchet session: forward secrecy + post-compromise security for 1:1 and tiny-group
//! sessions. See `docs/CRYPTOGRAPHY.md` §5. Groups above the tiny-group size use MLS
//! (RFC 9420) instead — not implemented in this crate yet, tracked separately.
//!
//! Reference: Perrin & Marlinspike, "The Double Ratchet Algorithm", Signal, Rev 4.

use std::collections::HashMap;

use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use rand_core::OsRng;
use sha2::Sha256;
use x25519_dalek::{PublicKey as XPublicKey, StaticSecret};

use crate::crypto::{aead_open_once, aead_seal_once};
use crate::error::{MeshError, Result};

/// Bound on stored out-of-order message keys — a flood/memory-exhaustion guard, per
/// `docs/CRYPTOGRAPHY.md` §5 ("skipped-message keys (bounded)").
const MAX_SKIP: usize = 1000;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Header {
    pub dh_pub: [u8; 32],
    /// Length of the previous sending chain, so the receiver knows how many messages to skip
    /// in the old chain before ratcheting.
    pub pn: u32,
    /// Index of this message within the current sending chain.
    pub n: u32,
}

impl Header {
    pub fn to_bytes(&self) -> [u8; 40] {
        let mut buf = [0u8; 40];
        buf[..32].copy_from_slice(&self.dh_pub);
        buf[32..36].copy_from_slice(&self.pn.to_le_bytes());
        buf[36..40].copy_from_slice(&self.n.to_le_bytes());
        buf
    }
}

pub struct DoubleRatchet {
    dh_self: StaticSecret,
    dh_self_pub: XPublicKey,
    dh_remote: Option<XPublicKey>,
    root_key: [u8; 32],
    send_chain: Option<[u8; 32]>,
    recv_chain: Option<[u8; 32]>,
    send_n: u32,
    recv_n: u32,
    prev_send_n: u32,
    skipped: HashMap<([u8; 32], u32), [u8; 32]>,
}

impl DoubleRatchet {
    /// The party who completed the handshake as initiator (Alice): already knows the peer's
    /// ratchet public key, so she can ratchet immediately and has a sending chain from the
    /// start.
    pub fn init_initiator(shared_secret: [u8; 32], remote_ratchet_pub: XPublicKey) -> Self {
        let dh_self = StaticSecret::random_from_rng(OsRng);
        let dh_self_pub = XPublicKey::from(&dh_self);
        let mut this = DoubleRatchet {
            dh_self,
            dh_self_pub,
            dh_remote: None,
            root_key: shared_secret,
            send_chain: None,
            recv_chain: None,
            send_n: 0,
            recv_n: 0,
            prev_send_n: 0,
            skipped: HashMap::new(),
        };
        let dh_out = this.dh_self.diffie_hellman(&remote_ratchet_pub);
        let (new_root, send_chain) = kdf_rk(this.root_key, dh_out.as_bytes());
        this.root_key = new_root;
        this.send_chain = Some(send_chain);
        this.dh_remote = Some(remote_ratchet_pub);
        this
    }

    /// The party who completed the handshake as responder (Bob): has no sending chain until
    /// the first inbound message triggers a ratchet.
    pub fn init_responder(shared_secret: [u8; 32], own_ratchet_keypair: StaticSecret) -> Self {
        let dh_self_pub = XPublicKey::from(&own_ratchet_keypair);
        DoubleRatchet {
            dh_self: own_ratchet_keypair,
            dh_self_pub,
            dh_remote: None,
            root_key: shared_secret,
            send_chain: None,
            recv_chain: None,
            send_n: 0,
            recv_n: 0,
            prev_send_n: 0,
            skipped: HashMap::new(),
        }
    }

    pub fn ratchet_public(&self) -> XPublicKey {
        self.dh_self_pub
    }

    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<(Header, Vec<u8>)> {
        let chain = self
            .send_chain
            .ok_or(MeshError::Ratchet("no sending chain yet (waiting on first inbound message)"))?;
        let (new_chain, mk) = kdf_ck(chain);
        self.send_chain = Some(new_chain);

        let header = Header {
            dh_pub: self.dh_self_pub.to_bytes(),
            pn: self.prev_send_n,
            n: self.send_n,
        };
        self.send_n += 1;

        let ct = aead_seal_once(&mk, &header.to_bytes(), plaintext)?;
        Ok((header, ct))
    }

    pub fn decrypt(&mut self, header: &Header, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let remote_pub = XPublicKey::from(header.dh_pub);

        if self.dh_remote != Some(remote_pub) {
            if let (Some(chain), Some(old_remote)) = (self.recv_chain.take(), self.dh_remote) {
                self.skip_chain(chain, old_remote.to_bytes(), header.pn)?;
            }
            self.dh_ratchet(remote_pub);
        }

        let mk = self.message_key_for(header.n)?;
        aead_open_once(&mk, &header.to_bytes(), ciphertext)
    }

    fn dh_ratchet(&mut self, remote_pub: XPublicKey) {
        self.prev_send_n = self.send_n;
        self.send_n = 0;
        self.recv_n = 0;
        self.dh_remote = Some(remote_pub);

        let dh_out = self.dh_self.diffie_hellman(&remote_pub);
        let (new_root, recv_chain) = kdf_rk(self.root_key, dh_out.as_bytes());
        self.root_key = new_root;
        self.recv_chain = Some(recv_chain);

        self.dh_self = StaticSecret::random_from_rng(OsRng);
        self.dh_self_pub = XPublicKey::from(&self.dh_self);
        let dh_out2 = self.dh_self.diffie_hellman(&remote_pub);
        let (new_root2, send_chain) = kdf_rk(self.root_key, dh_out2.as_bytes());
        self.root_key = new_root2;
        self.send_chain = Some(send_chain);
    }

    fn skip_chain(&mut self, mut chain: [u8; 32], remote_bytes: [u8; 32], upto: u32) -> Result<()> {
        while self.recv_n < upto {
            let (new_chain, mk) = kdf_ck(chain);
            chain = new_chain;
            self.store_skipped((remote_bytes, self.recv_n), mk)?;
            self.recv_n += 1;
        }
        Ok(())
    }

    fn message_key_for(&mut self, n: u32) -> Result<[u8; 32]> {
        let remote_bytes = self
            .dh_remote
            .ok_or(MeshError::Ratchet("no remote ratchet key established"))?
            .to_bytes();

        if let Some(mk) = self.skipped.remove(&(remote_bytes, n)) {
            return Ok(mk);
        }
        if n < self.recv_n {
            return Err(MeshError::Ratchet("message key already consumed (replay or out-of-window)"));
        }

        let mut chain = self
            .recv_chain
            .take()
            .ok_or(MeshError::Ratchet("no receiving chain established"))?;

        while self.recv_n < n {
            let (new_chain, mk) = kdf_ck(chain);
            chain = new_chain;
            self.store_skipped((remote_bytes, self.recv_n), mk)?;
            self.recv_n += 1;
        }

        let (new_chain, mk) = kdf_ck(chain);
        self.recv_chain = Some(new_chain);
        self.recv_n += 1;
        Ok(mk)
    }

    fn store_skipped(&mut self, key: ([u8; 32], u32), mk: [u8; 32]) -> Result<()> {
        if self.skipped.len() >= MAX_SKIP {
            return Err(MeshError::Ratchet("too many skipped messages (bounded flood guard)"));
        }
        self.skipped.insert(key, mk);
        Ok(())
    }
}

/// `KDF_RK`: advances the root key given fresh DH output, yielding a new root key and a new
/// chain key. HKDF-SHA256(salt = root_key, ikm = dh_out, info = "MESH_DR_RK") -> 64 bytes.
fn kdf_rk(root_key: [u8; 32], dh_out: &[u8]) -> ([u8; 32], [u8; 32]) {
    let hk = Hkdf::<Sha256>::new(Some(&root_key), dh_out);
    let mut okm = [0u8; 64];
    hk.expand(b"MESH_DR_RK", &mut okm).expect("64 <= 255*32");
    let mut new_root = [0u8; 32];
    let mut chain_key = [0u8; 32];
    new_root.copy_from_slice(&okm[..32]);
    chain_key.copy_from_slice(&okm[32..]);
    (new_root, chain_key)
}

/// `KDF_CK`: advances a symmetric chain key, yielding the next chain key and a message key.
/// `ck' = HMAC-SHA256(ck, 0x02)`, `mk = HMAC-SHA256(ck, 0x01)` — the standard Double Ratchet
/// chain-key construction.
fn kdf_ck(chain_key: [u8; 32]) -> ([u8; 32], [u8; 32]) {
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(&chain_key).expect("HMAC accepts any key length");
    mac.update(&[0x01]);
    let mk_full = mac.finalize().into_bytes();
    let mut message_key = [0u8; 32];
    message_key.copy_from_slice(&mk_full[..32]);

    let mut mac = HmacSha256::new_from_slice(&chain_key).expect("HMAC accepts any key length");
    mac.update(&[0x02]);
    let ck_full = mac.finalize().into_bytes();
    let mut next_chain = [0u8; 32];
    next_chain.copy_from_slice(&ck_full[..32]);

    (next_chain, message_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paired_ratchets() -> (DoubleRatchet, DoubleRatchet) {
        let shared_secret = [42u8; 32];
        let bob_ratchet_key = StaticSecret::random_from_rng(OsRng);
        let bob_ratchet_pub = XPublicKey::from(&bob_ratchet_key);

        let alice = DoubleRatchet::init_initiator(shared_secret, bob_ratchet_pub);
        let bob = DoubleRatchet::init_responder(shared_secret, bob_ratchet_key);
        (alice, bob)
    }

    #[test]
    fn basic_in_order_exchange() {
        let (mut alice, mut bob) = paired_ratchets();

        let (h1, ct1) = alice.encrypt(b"hi bob").unwrap();
        assert_eq!(bob.decrypt(&h1, &ct1).unwrap(), b"hi bob");

        let (h2, ct2) = bob.encrypt(b"hi alice").unwrap();
        assert_eq!(alice.decrypt(&h2, &ct2).unwrap(), b"hi alice");

        let (h3, ct3) = alice.encrypt(b"second message").unwrap();
        assert_eq!(bob.decrypt(&h3, &ct3).unwrap(), b"second message");
    }

    #[test]
    fn out_of_order_delivery_within_same_chain() {
        let (mut alice, mut bob) = paired_ratchets();

        let (h1, ct1) = alice.encrypt(b"one").unwrap();
        let (h2, ct2) = alice.encrypt(b"two").unwrap();
        let (h3, ct3) = alice.encrypt(b"three").unwrap();

        // Bob receives them out of order: 3, 1, 2.
        assert_eq!(bob.decrypt(&h3, &ct3).unwrap(), b"three");
        assert_eq!(bob.decrypt(&h1, &ct1).unwrap(), b"one");
        assert_eq!(bob.decrypt(&h2, &ct2).unwrap(), b"two");
    }

    #[test]
    fn forward_secrecy_message_keys_are_not_reused() {
        let (mut alice, mut bob) = paired_ratchets();
        let (h1, ct1) = alice.encrypt(b"one").unwrap();
        bob.decrypt(&h1, &ct1).unwrap();
        // Replaying the same header+ciphertext must fail: the message key was consumed.
        assert!(bob.decrypt(&h1, &ct1).is_err());
    }

    #[test]
    fn post_compromise_security_self_heals_after_dh_ratchet() {
        let (mut alice, mut bob) = paired_ratchets();

        let (h1, ct1) = alice.encrypt(b"one").unwrap();
        bob.decrypt(&h1, &ct1).unwrap();

        // Bob replies, forcing a DH ratchet step on Alice's side with fresh key material —
        // this is the self-healing property: even if a prior key had leaked, this exchange is
        // independent of it because it derives from a freshly generated ratchet keypair.
        let (h2, ct2) = bob.encrypt(b"reply").unwrap();
        assert_eq!(alice.decrypt(&h2, &ct2).unwrap(), b"reply");

        let (h3, ct3) = alice.encrypt(b"post-heal").unwrap();
        assert_eq!(bob.decrypt(&h3, &ct3).unwrap(), b"post-heal");
    }

    #[test]
    fn tampered_ciphertext_is_rejected() {
        let (mut alice, mut bob) = paired_ratchets();
        let (h1, mut ct1) = alice.encrypt(b"one").unwrap();
        let last = ct1.len() - 1;
        ct1[last] ^= 0x01;
        assert!(bob.decrypt(&h1, &ct1).is_err());
    }

    #[test]
    fn many_out_of_order_messages_across_multiple_ratchets() {
        let (mut alice, mut bob) = paired_ratchets();

        let a1 = alice.encrypt(b"a1").unwrap();
        let a2 = alice.encrypt(b"a2").unwrap();
        assert_eq!(bob.decrypt(&a1.0, &a1.1).unwrap(), b"a1");

        let b1 = bob.encrypt(b"b1").unwrap(); // ratchets bob's send chain into existence
        assert_eq!(alice.decrypt(&b1.0, &b1.1).unwrap(), b"b1");

        // a2 arrives late, after alice has already ratcheted forward via bob's reply.
        assert_eq!(bob.decrypt(&a2.0, &a2.1).unwrap(), b"a2");

        let a3 = alice.encrypt(b"a3").unwrap();
        assert_eq!(bob.decrypt(&a3.0, &a3.1).unwrap(), b"a3");
    }
}
