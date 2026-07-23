//! Sealed envelope wire format. See `docs/ROUTING-PROTOCOL.md`.
//!
//! An envelope's ID is never trusted from the wire — every node derives it itself as
//! `BLAKE3(header || sealed_payload)`, so dedup cannot be spoofed by a malicious sender
//! supplying a colliding or bogus ID.

use crate::error::{MeshError, Result};

pub const WIRE_VERSION: u8 = 1;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct EnvelopeId(pub [u8; 32]);

impl EnvelopeId {
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Addressing {
    /// "Everyone nearby" — e.g. disaster bulletins.
    Broadcast,
    /// Passphrase-derived shared channel key; no owner, no server.
    Channel([u8; 32]),
    /// Bounded membership group (sender seals one copy per member at a higher layer).
    Group([u8; 32]),
    /// One-to-one, addressed by the recipient's identity fingerprint.
    Direct([u8; 32]),
}

impl Addressing {
    fn tag(&self) -> u8 {
        match self {
            Addressing::Broadcast => 0,
            Addressing::Channel(_) => 1,
            Addressing::Group(_) => 2,
            Addressing::Direct(_) => 3,
        }
    }

    fn target(&self) -> Option<&[u8; 32]> {
        match self {
            Addressing::Broadcast => None,
            Addressing::Channel(id) | Addressing::Group(id) | Addressing::Direct(id) => Some(id),
        }
    }
}

/// Emergency traffic is prioritized by the routing engine over ordinary messaging
/// (`docs/ROUTING-PROTOCOL.md` §7.2). Lower ordinal = higher priority = evicted last.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub enum Priority {
    Sos = 0,
    Bulletin = 1,
    Normal = 2,
    Low = 3,
}

impl Priority {
    fn from_tag(tag: u8) -> Result<Self> {
        match tag {
            0 => Ok(Priority::Sos),
            1 => Ok(Priority::Bulletin),
            2 => Ok(Priority::Normal),
            3 => Ok(Priority::Low),
            _ => Err(MeshError::Malformed("unknown priority tag")),
        }
    }
}

/// A sealed envelope as it travels the mesh. `sealed` is opaque ciphertext produced by the
/// crypto layer (AEAD / Double Ratchet / channel key) — the routing engine never inspects it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Envelope {
    pub id: EnvelopeId,
    pub addressing: Addressing,
    pub priority: Priority,
    pub ttl_hops: u8,
    pub expires_at: u64,
    pub sealed: Vec<u8>,
}

impl Envelope {
    /// Construct a new envelope; the ID is derived from the content, not chosen by the caller.
    pub fn new(
        addressing: Addressing,
        priority: Priority,
        ttl_hops: u8,
        expires_at: u64,
        sealed: Vec<u8>,
    ) -> Self {
        let id = Self::compute_id(addressing, priority, expires_at, &sealed);
        Envelope {
            id,
            addressing,
            priority,
            ttl_hops,
            expires_at,
            sealed,
        }
    }

    /// The envelope ID deliberately excludes `ttl_hops`: it is decremented at every relay hop
    /// (`crate::engine::decrement_ttl`), so if it were part of the identity hash, the *same*
    /// logical message would get a *different* ID at each hop — silently defeating dedup for
    /// anything beyond a single hop. `expires_at`, by contrast, is fixed at creation and never
    /// mutated in transit, so it safely stays part of identity.
    fn compute_id(addressing: Addressing, priority: Priority, expires_at: u64, sealed: &[u8]) -> EnvelopeId {
        let mut buf = Vec::with_capacity(1 + 1 + 32 + 1 + 8 + 4 + sealed.len());
        buf.push(WIRE_VERSION);
        buf.push(addressing.tag());
        if let Some(target) = addressing.target() {
            buf.extend_from_slice(target);
        }
        buf.push(priority as u8);
        buf.extend_from_slice(&expires_at.to_le_bytes());
        buf.extend_from_slice(&(sealed.len() as u32).to_le_bytes());
        buf.extend_from_slice(sealed);
        EnvelopeId(blake3::hash(&buf).into())
    }

    fn encode_body(
        addressing: Addressing,
        priority: Priority,
        ttl_hops: u8,
        expires_at: u64,
        sealed: &[u8],
    ) -> Vec<u8> {
        let mut buf = Vec::with_capacity(1 + 1 + 32 + 1 + 1 + 8 + 4 + sealed.len());
        buf.push(WIRE_VERSION);
        buf.push(addressing.tag());
        if let Some(target) = addressing.target() {
            buf.extend_from_slice(target);
        }
        buf.push(priority as u8);
        buf.push(ttl_hops);
        buf.extend_from_slice(&expires_at.to_le_bytes());
        buf.extend_from_slice(&(sealed.len() as u32).to_le_bytes());
        buf.extend_from_slice(sealed);
        buf
    }

    /// Serialize to the wire format (header + opaque sealed payload). No ID is included; the
    /// receiver recomputes it from these bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        Self::encode_body(
            self.addressing,
            self.priority,
            self.ttl_hops,
            self.expires_at,
            &self.sealed,
        )
    }

    /// Parse untrusted wire bytes. Rejects malformed input; never panics on attacker input.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let mut cursor = bytes;
        let version = *cursor.first().ok_or(MeshError::Malformed("empty envelope"))?;
        if version != WIRE_VERSION {
            return Err(MeshError::Malformed("unsupported wire version"));
        }
        cursor = &cursor[1..];

        let tag = *cursor.first().ok_or(MeshError::Malformed("missing addressing tag"))?;
        cursor = &cursor[1..];

        let addressing = match tag {
            0 => Addressing::Broadcast,
            1 | 2 | 3 => {
                if cursor.len() < 32 {
                    return Err(MeshError::Malformed("truncated addressing target"));
                }
                let mut target = [0u8; 32];
                target.copy_from_slice(&cursor[..32]);
                cursor = &cursor[32..];
                match tag {
                    1 => Addressing::Channel(target),
                    2 => Addressing::Group(target),
                    _ => Addressing::Direct(target),
                }
            }
            _ => return Err(MeshError::Malformed("unknown addressing tag")),
        };

        let priority_tag = *cursor.first().ok_or(MeshError::Malformed("missing priority"))?;
        let priority = Priority::from_tag(priority_tag)?;
        cursor = &cursor[1..];

        let ttl_hops = *cursor.first().ok_or(MeshError::Malformed("missing ttl"))?;
        cursor = &cursor[1..];

        if cursor.len() < 8 {
            return Err(MeshError::Malformed("truncated expiry"));
        }
        let expires_at = u64::from_le_bytes(cursor[..8].try_into().unwrap());
        cursor = &cursor[8..];

        if cursor.len() < 4 {
            return Err(MeshError::Malformed("truncated length prefix"));
        }
        let sealed_len = u32::from_le_bytes(cursor[..4].try_into().unwrap()) as usize;
        cursor = &cursor[4..];

        if cursor.len() != sealed_len {
            return Err(MeshError::Malformed("sealed payload length mismatch"));
        }
        let sealed = cursor.to_vec();

        // Recomputed the same hop-stable way as `new` (see `compute_id`) — deliberately not a
        // hash of the raw wire `bytes`, since those include `ttl_hops`, which changes per hop.
        let id = Self::compute_id(addressing, priority, expires_at, &sealed);

        Ok(Envelope {
            id,
            addressing,
            priority,
            ttl_hops,
            expires_at,
            sealed,
        })
    }

    pub fn is_expired(&self, now: u64) -> bool {
        now >= self.expires_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Envelope {
        Envelope::new(
            Addressing::Direct([7u8; 32]),
            Priority::Normal,
            8,
            1_800_000_000,
            b"opaque-ciphertext".to_vec(),
        )
    }

    #[test]
    fn roundtrip_preserves_fields_and_id() {
        let env = sample();
        let bytes = env.to_bytes();
        let parsed = Envelope::from_bytes(&bytes).expect("parse");
        assert_eq!(parsed, env);
        assert_eq!(parsed.id, env.id);
    }

    #[test]
    fn id_is_content_derived_not_wire_supplied() {
        let a = sample();
        let mut b = sample();
        b.sealed = b"different-ciphertext".to_vec();
        // recompute via constructor since `sealed` differs
        let b = Envelope::new(b.addressing, b.priority, b.ttl_hops, b.expires_at, b.sealed);
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn rejects_truncated_input() {
        let env = sample();
        let bytes = env.to_bytes();
        for cut in 0..bytes.len() {
            assert!(Envelope::from_bytes(&bytes[..cut]).is_err());
        }
    }

    #[test]
    fn broadcast_addressing_has_no_target_bytes() {
        let env = Envelope::new(Addressing::Broadcast, Priority::Sos, 4, 100, vec![1, 2, 3]);
        let bytes = env.to_bytes();
        // version(1) + tag(1) + priority(1) + ttl(1) + expiry(8) + len(4) + payload(3)
        assert_eq!(bytes.len(), 1 + 1 + 1 + 1 + 8 + 4 + 3);
        let parsed = Envelope::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.addressing, Addressing::Broadcast);
    }

    #[test]
    fn priority_ordering_matches_docs_sos_highest() {
        assert!(Priority::Sos < Priority::Bulletin);
        assert!(Priority::Bulletin < Priority::Normal);
        assert!(Priority::Normal < Priority::Low);
    }

    #[test]
    fn expiry_check() {
        let env = sample();
        assert!(!env.is_expired(1_000_000_000));
        assert!(env.is_expired(1_800_000_000));
        assert!(env.is_expired(1_900_000_000));
    }

    #[test]
    fn id_is_stable_across_ttl_decrement_at_each_relay_hop() {
        // Regression test: ttl_hops is decremented at every relay hop
        // (`crate::engine::decrement_ttl`). If it were part of the ID's content hash, the same
        // logical message would get a different ID at every hop, silently breaking dedup for
        // anything beyond one hop.
        let addressing = Addressing::Broadcast;
        let sealed = b"same-message".to_vec();
        let hop0 = Envelope::new(addressing, Priority::Normal, 8, 9_999_999_999, sealed.clone());
        let hop1 = Envelope::new(addressing, Priority::Normal, 7, 9_999_999_999, sealed.clone());
        let hop2 = Envelope::new(addressing, Priority::Normal, 1, 9_999_999_999, sealed);
        assert_eq!(hop0.id, hop1.id);
        assert_eq!(hop0.id, hop2.id);

        // And the same holds after a real decrement_ttl + wire roundtrip, not just construction.
        let relayed = crate::engine::decrement_ttl(hop0.clone()).expect("hops remain");
        let parsed = Envelope::from_bytes(&relayed.to_bytes()).unwrap();
        assert_eq!(parsed.id, hop0.id);
    }
}
