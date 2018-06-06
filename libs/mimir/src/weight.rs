use serde::de::{Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// The Weight is an object used to control the elasticsearch sort.
///
/// A Weight's value must be a float between 0 and 1.
///
/// Since we need interior mutability for the weight and we need it to be std::sync,
/// it emulates an AtomicFloat.
/// The main idea is to store the value in an AtomicUSize and convert it to a float with the
/// `get_val` method.
///
/// The weight needs to be normalized before used.
/// The normalization works by giving the maximum value to the normalize method,
/// this way we can compute a weight between 0 and 1.
#[derive(Debug)]
pub struct Weight {
    val: AtomicUsize,
    normalized: AtomicBool,
}

// the max value is u32 to be able to keep the prevision while converting it to f64
const MAX_VAL: f64 = u32::max_value() as f64;

impl Weight {
    // Note we limit val to a u32 even if we store a usize because we'll loose precision while converting it to float
    pub fn new(val: u32) -> Weight {
        Weight {
            val: AtomicUsize::new(val as usize),
            normalized: AtomicBool::new(false),
        }
    }

    fn from_normalized_float(val: f64) -> Weight {
        debug_assert!(0f64 <= val);
        debug_assert!(val <= 1f64);
        Weight {
            val: AtomicUsize::new((val * MAX_VAL) as usize),
            normalized: AtomicBool::new(true),
        }
    }

    /// get the float value of the weight.
    /// If the weight has not yet been normlized returns `None` else returns the float
    pub fn value(&self) -> Option<f64> {
        match self.normalized.load(Ordering::Relaxed) {
            true => Some(self.unnormalized_value() as f64 / MAX_VAL),
            false => None,
        }
    }

    pub fn unnormalized_value(&self) -> u32 {
        // we can read the atomic without any ordering constraints as in practice
        // we don't change the value in multi thread context
        self.val.load(Ordering::Relaxed) as u32
    }

    pub fn normalize(&self, max_value: u32) {
        let mut val: f64 = self.val.load(Ordering::Relaxed) as f64 / max_value as f64;
        debug_assert!(0f64 <= val);
        debug_assert!(val <= 1f64);
        val *= MAX_VAL;
        self.val.store(val as usize, Ordering::Relaxed);
        self.normalized.store(true, Ordering::Relaxed);
    }
}

impl Clone for Weight {
    fn clone(&self) -> Weight {
        Weight {
            val: AtomicUsize::new(self.val.load(Ordering::Relaxed)),
            normalized: AtomicBool::new(self.normalized.load(Ordering::Relaxed)),
        }
    }
}

impl Serialize for Weight {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // unwrap() here to ensure that the weight is always normalized before serialization
        serializer.serialize_f64(self.value().unwrap())
    }
}

impl Default for Weight {
    fn default() -> Self {
        Weight::new(0u32)
    }
}

impl<'de> Deserialize<'de> for Weight {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = Weight;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("float representing a weight")
            }

            fn visit_f64<E>(self, value: f64) -> Result<Weight, E>
            where
                E: ::serde::de::Error,
            {
                Ok(Weight::from_normalized_float(value))
            }
        }

        // Deserialize the weight as f64
        deserializer.deserialize_f64(Visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::Weight;
    extern crate serde_json;

    #[test]
    pub fn test_basic_uses() {
        let weight = Weight::new(12u32);

        assert!(weight.value().is_none());
    }

    #[test]
    pub fn test_zero() {
        let weight = Weight::new(0u32);
        weight.normalize(12u32);

        abs_diff_eq!(0.0f64, weight.value().unwrap());
    }

    #[test]
    pub fn test_max_val() {
        let weight = Weight::new(12u32);
        weight.normalize(12u32);

        abs_diff_eq!(1.0f64, weight.value().unwrap());
    }

    proptest! {

        #[test]
        fn test_normalize_for_any_value(val in 0u32..1000000u32) {
            let weight = Weight::new(val);
            weight.normalize(1000000u32);
            abs_diff_eq!(weight.value().unwrap(), val as f64 / 1000000f64);
        }
    }

    #[test]
    pub fn test_normalize() {
        let weight = Weight::new(12u32);
        weight.normalize(24u32);

        abs_diff_eq!(0.5f64, weight.value().unwrap());
    }

    #[test]
    pub fn test_weight_serialization() {
        let weight = Weight::new(12u32);
        weight.normalize(24u32);

        let as_json = serde_json::to_string(&weight).unwrap();
        let from_json: Weight = serde_json::from_str(&as_json).unwrap();
        abs_diff_eq!(0.5f64, from_json.value().unwrap());
    }
}
