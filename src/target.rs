use core::cmp::Ordering;
use num_bigint::BigUint;
use std::fmt;
use std::fmt::Write as _;

pub fn u256_from_compact_target(bits: u32) -> Uint256 {
    // This is a floating-point "compact" encoding originally used by
    // OpenSSL, which satoshi put into consensus code, so we're stuck
    // with it. The exponent needs to have 3 subtracted from it, hence
    // this goofy decoding code:
    let (mant, expt) = {
        let unshifted_expt = bits >> 24;
        if unshifted_expt <= 3 {
            ((bits & 0xFFFFFF) >> (8 * (3 - unshifted_expt as usize)), 0)
        } else {
            (bits & 0xFFFFFF, 8 * ((bits >> 24) - 3))
        }
    };

    // The mantissa is signed but may not be negative
    if mant > 0x7FFFFF {
        Default::default()
    } else {
        Uint256::from_u64(mant as u64) << (expt as usize)
    }
}

pub fn u256_from_stratum_difficulty_str(difficulty: &str) -> Option<Uint256> {
    let difficulty = difficulty.trim();
    if difficulty.is_empty() || difficulty.starts_with('-') {
        return None;
    }

    let (whole, frac, scale) = if let Some((whole, frac)) = difficulty.split_once('.') {
        let frac = frac.trim_end_matches('0');
        if frac.is_empty() {
            (whole, "", BigUint::from(1u8))
        } else {
            let scale = BigUint::from(10u8).pow(frac.len() as u32);
            (whole, frac, scale)
        }
    } else {
        (difficulty, "", BigUint::from(1u8))
    };

    let whole_value = if whole.is_empty() { BigUint::from(0u8) } else { BigUint::parse_bytes(whole.as_bytes(), 10)? };
    let frac_value = if frac.is_empty() { BigUint::from(0u8) } else { BigUint::parse_bytes(frac.as_bytes(), 10)? };
    let difficulty_scaled = &whole_value * &scale + frac_value;

    if difficulty_scaled == BigUint::from(0u8) {
        return None;
    }

    let diff1 = BigUint::parse_bytes(b"00ffff0000000000000000000000000000000000000000000000000000000000", 16)?;
    let target = diff1 * scale / difficulty_scaled;
    let bytes = target.to_bytes_le();
    let mut target_bytes = [0u8; 32];
    let len = bytes.len().min(target_bytes.len());
    target_bytes[..len].copy_from_slice(&bytes[..len]);
    Some(Uint256::from_le_bytes(target_bytes))
}

/// Little-endian large integer type
#[derive(Copy, Clone, PartialEq, Eq, Hash, Default, Debug)]
pub struct Uint256(pub [u64; 4]);

impl Uint256 {
    #[inline(always)]
    pub fn new(v: [u64; 4]) -> Self {
        Self(v)
    }
    /// Create an object from a given unsigned 64-bit integer
    #[inline]
    pub fn from_u64(init: u64) -> Uint256 {
        let mut ret = [0; 4];
        ret[0] = init;
        Uint256(ret)
    }

    /// Creates big integer value from a byte slice using
    /// little-endian encoding
    #[inline(always)]
    pub fn from_le_bytes(bytes: [u8; 32]) -> Uint256 {
        let mut out = [0u64; 4];
        // This should optimize to basically a transmute.
        out.iter_mut()
            .zip(bytes.chunks_exact(8))
            .for_each(|(word, bytes)| *word = u64::from_le_bytes(bytes.try_into().unwrap()));
        Self(out)
    }

    #[inline(always)]
    pub fn to_le_bytes(self) -> [u8; 32] {
        let mut out = [0u8; 32];
        // This should optimize to basically a transmute.
        out.chunks_exact_mut(8).zip(self.0).for_each(|(bytes, word)| bytes.copy_from_slice(&word.to_le_bytes()));
        out
    }

    #[inline(always)]
    pub fn to_le_u64(self) -> [u64; 4] {
        self.0
    }

    #[inline(always)]
    pub fn from_le_u64(arr: [u64; 4]) -> Uint256 {
        Uint256(arr)
    }

    #[inline]
    pub fn to_be_hex(self) -> String {
        let bytes = self.to_le_bytes();
        let mut out = String::with_capacity(64);
        for byte in bytes.iter().rev() {
            let _ = write!(&mut out, "{:02x}", byte);
        }
        out
    }
}

impl fmt::LowerHex for Uint256 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.to_le_bytes().iter().try_for_each(|&c| write!(f, "{:02x}", c))
    }
}

impl PartialOrd for Uint256 {
    #[inline(always)]
    fn partial_cmp(&self, other: &Uint256) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Uint256 {
    #[inline(always)]
    fn cmp(&self, other: &Uint256) -> Ordering {
        // We need to manually implement ordering because we use little-endian
        // and the auto derive is a lexicographic ordering(i.e. memcmp)
        // which with numbers is equivalent to big-endian
        Iterator::cmp(self.0.iter().rev(), other.0.iter().rev())
    }
}

impl core::ops::Shl<usize> for Uint256 {
    type Output = Uint256;

    fn shl(self, shift: usize) -> Uint256 {
        let Uint256(ref original) = self;
        let mut ret = [0u64; 4];
        let word_shift = shift / 64;
        let bit_shift = shift % 64;
        for i in 0..4 {
            // Shift
            if bit_shift < 64 && i + word_shift < 4 {
                ret[i + word_shift] += original[i] << bit_shift;
            }
            // Carry
            if bit_shift > 0 && i + word_shift + 1 < 4 {
                ret[i + word_shift + 1] += original[i] >> (64 - bit_shift);
            }
        }
        Uint256(ret)
    }
}
