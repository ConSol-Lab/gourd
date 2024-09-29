use core::fmt;
use std::hash::Hash;
use std::ops::Deref;
use std::ops::DerefMut;

use serde::de::Visitor;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;

/// A wrapper around [regex_lite::Regex] to allow serde.
#[derive(Debug, Clone)]
pub struct Regex(regex_lite::Regex);

impl Eq for Regex {}

impl PartialEq for Regex {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_string() == other.0.to_string()
    }
}

impl Hash for Regex {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_string().hash(state)
    }
}

impl Serialize for Regex {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for Regex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        /// The visitor for regex values.
        struct RegexVisitor;

        impl Visitor<'_> for RegexVisitor {
            // see: https://serde.rs/impl-deserialize.html
            type Value = Regex;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid regex string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let regex = regex_lite::Regex::new(v);

                regex
                    .map_err(|parse_err| {
                        serde::de::Error::custom(format!("This is not a valid regex: {parse_err}"))
                    })
                    .map(Regex)
            }
        }

        deserializer.deserialize_str(RegexVisitor)
    }
}

impl Deref for Regex {
    type Target = regex_lite::Regex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Regex {
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        &mut self.0
    }
}

impl From<regex_lite::Regex> for Regex {
    fn from(regex: regex_lite::Regex) -> Self {
        Self(regex)
    }
}
