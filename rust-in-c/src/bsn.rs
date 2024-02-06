use std::fmt::Display;

pub use ffi::*;
use serde::{de::Visitor, Deserialize, Serialize};

// Errors are pretty vague
#[diplomat::bridge]
pub mod ffi {
    use diplomat_runtime::DiplomatWriteable;
    use std::fmt::Write;

    // Clone does not get exposed, even though it could
    #[derive(Debug, Clone)]
    pub enum Error {
        InvalidBsn,
    }

    impl Error {
        // For some reason can't receive &self?
        pub fn fmt_display(this: &Self, w: &mut DiplomatWriteable) -> Result<(), ()> {
            write!(w, "{}", this).map_err(|_| ())
        }
    }

    #[derive(Debug, PartialEq, Eq, Clone)]
    #[diplomat::opaque]
    pub struct Bsn {
        pub(crate) inner: String,
    }

    impl Bsn {
        // Can only return boxed refs of Self
        pub fn try_new_boxed(bsn: &str) -> Result<Box<Self>, Error> {
            Self::try_new(bsn).map(Box::new)
        }

        pub fn validate(bsn: &str) -> bool {
            if !matches!(bsn.len(), 8 | 9) {
                return false;
            }
            let sum = [9, 8, 7, 6, 5, 4, 3, 2, -1]
                .iter()
                .zip(bsn.chars())
                .try_fold(0, |sum, (multiplier, digit)| {
                    let Some(digit) = digit.to_digit(10) else {
                        return Err(Error::InvalidBsn);
                    };
                    Ok(sum + (multiplier * digit as i32))
                });

            let Ok(sum) = sum else {
                return false;
            };

            sum % 11 == 0
        }
    }
}
// Need to define trait impls outside bridge block

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidBsn => write!(f, "Invalid BSN number"),
        }
    }
}

impl Bsn {
    pub fn try_new(bsn: &str) -> Result<Self, Error> {
        let bsn = bsn.to_string();
        if Self::validate(&bsn) {
            Ok(Self { inner: bsn })
        } else {
            Err(Error::InvalidBsn)
        }
    }
}

impl Serialize for Bsn {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.inner)
    }
}

impl<'de> Deserialize<'de> for Bsn {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct BsnVisitor;

        impl<'d> Visitor<'d> for BsnVisitor {
            type Value = Bsn;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "A string representing a valid BSN")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Bsn::try_new(v).map_err(serde::de::Error::custom)
            }
        }

        deserializer.deserialize_string(BsnVisitor)
    }
}
