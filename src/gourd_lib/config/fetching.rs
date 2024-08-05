use std::fmt;
use std::fmt::Debug;
use std::fmt::Display;
use std::marker::PhantomData;
use std::ops::Deref;
use std::path::PathBuf;
use std::str::FromStr;

use serde::de;
use serde::de::Visitor;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;

use crate::constants::URL_ESCAPE;
use crate::file_system::FileOperations;

#[allow(rustdoc::bare_urls)]
/// A wrapper around [PathBuf] to allow serde expansion of globs.
///
/// This allows for a user to specify a fetched resource in place of a local
/// one just how they would specify an actual path.
///
/// The PERMISSIONS generic argument specifies the permissions that the
/// downloaded file is set to after saving it.
///
/// # Examples
///
/// "/test/test.in" deserializes into the path "/test/test.in"
/// "fetch|http://example.com | /test/test.in" deserializes into
/// the same exact path "/test/test.in" but the file is fetched using a
/// GET request to the URL before the |.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, Serialize)]
pub struct FetchedPath<const PERMISSIONS: u32>(pub PathBuf);

// impl<'de, const PERM: u32> Deserialize<'de> for FetchedPath<PERM> {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         /// The custom map visitor for inputs.
//         struct FetchedVisitor<const PERM: u32> {
//             /// Phantom marker.
//             marker: PhantomData<()>,
//         }
//
//         impl<'de, const PERM: u32> Visitor<'de> for FetchedVisitor<PERM> {
//             type Value = FetchedPath<PERM>;
//
//             fn expecting(&self, formatter: &mut fmt::Formatter) ->
// fmt::Result {                 formatter.write_str("a valid path, in the form
// of a string")             }
//
//             #[inline]
//             fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
//             where
//                 E: serde::de::Error,
//             {
//                 // because of the cfg lower...
//                 #[allow(unused_variables)]
//                 if let Some(actual) = v.strip_prefix(URL_ESCAPE) {
//                     #[cfg(feature = "fetching")]
//                     {
//                         // use crate::config::maps::DeserState;
//                         // use crate::config::maps::IS_USER_FACING;
//                         use crate::network::download_file;
//
//                         let errmap = "invalid url syntax, expected '[url] |
// [file]'";
//
//                         if let DeserState::User(fs) =
// IS_USER_FACING.with_borrow(|x| x.clone()) {                             let
// mut iter = actual.split('|');
//
//                             let url =
// iter.next().ok_or(de::Error::custom(errmap))?.trim();
// let filename = PathBuf::from_str(
// iter.next().ok_or(de::Error::custom(errmap))?.trim(),
// )                             .map_err(|x| de::Error::custom(format!("invalid
// path {x}")))?;
//
//                             if iter.next().is_some() {
//                                 return Err(de::Error::custom(errmap));
//                             }
//
//                             if !filename.exists() {
//                                 download_file(url, &filename,
// &fs).map_err(|x| {
// de::Error::custom(format!(                                         "could not
// download file {v}...\n{x}"                                     ))
//                                 })?;
//
//                                 fs.set_permissions(&filename,
// PERM).map_err(|x| {
// de::Error::custom(format!(                                         "could set
// permissions for file {v}...\n{x}"                                     ))
//                                 })?;
//                             }
//
//                             Ok(FetchedPath(filename))
//                         } else {
//                             Err(de::Error::custom(format!("url not allowed in
// path: {v}")))                         }
//                     }
//                     #[cfg(not(feature = "fetching"))]
//                     {
//                         Err(de::Error::custom(
//                             "this version of gourd was built without fetching
// support, do not use urls",                         ))
//                     }
//                 } else {
//                     Ok(FetchedPath(PathBuf::from_str(v).map_err(|x| {
//                         de::Error::custom(format!("could not include the
// path: {x}"))                     })?))
//                 }
//             }
//         }
//
//         let visitor = FetchedVisitor {
//             marker: PhantomData,
//         };
//
//         deserializer.deserialize_str(visitor)
//     }
// }
//
// impl<const PERM: u32> Display for FetchedPath<PERM> {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{:?}", self.0)
//     }
// }
//
// impl<const PERM: u32> Deref for FetchedPath<PERM> {
//     type Target = PathBuf;
//
//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }
//
// impl<const PERM: u32> From<PathBuf> for FetchedPath<PERM> {
//     fn from(value: PathBuf) -> Self {
//         FetchedPath(value)
//     }
// }
