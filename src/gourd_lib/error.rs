use std::fmt::Display;

use crate::constants::ERROR_STYLE;
use crate::constants::HELP_STYLE;

/// The error context structure, provides an explanation and help.
///
/// The first element of the structure is the errors "context".
/// The second element is the help message displayed to the user.
///
/// Both have to implement [Display], and will be displayed when the error is
/// printed. # Example
///
/// You can use this for example with two [String]s.
///
/// ```should_panic
/// # use gourd_lib::error::Ctx;
/// # use anyhow::anyhow;
/// # use anyhow::Result;
/// # use anyhow::Context;
/// # fn main() -> Result<()> {
/// Err(anyhow!("Any struct implementing std::error::Error")).context(Ctx("context", "help"))
/// # }
/// ```
#[derive(Debug)]
pub struct Ctx<A, B>(pub A, pub B)
where
    A: Display,
    B: Display;

impl<A: Display, B: Display> Display for Ctx<A, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !format!("{}", self.0).is_empty() {
            writeln!(f, "{}caused by:{:#} {}", ERROR_STYLE, ERROR_STYLE, self.0)?;
        }

        if !format!("{}", self.1).is_empty() {
            writeln!(f, "\n{}help:{:#} {}", HELP_STYLE, HELP_STYLE, self.1)?;
        }

        Ok(())
    }
}

/// This is a shorthand for returning the context of a error.
///
/// This macro takes a context message, a help message and formats them.
///
/// A macro invocation like so:
/// ```ignore
/// ctx!([context], [context arg 1], [context arg 2], ...; [help], [help args], ...)
/// ```
/// Will desugar to:
/// ```ignore
/// || Ctx(format!([context], [context args]), format!([help], [help args]))
/// ```
///
/// Note the placement of the `;` and `,`. They are required and otherwise the
/// macro will not parse.
///
/// # Example
///
/// Assume that we want to run [std::fs::read] and add context to the error
/// message.
///
/// This can be done as follows:
///
/// ```no_run
/// # #[macro_use]
/// # use gourd_lib::error::Ctx;
/// # use gourd_lib::ctx;
/// # use std::path::PathBuf;
/// # use anyhow::Context;
/// # let path: PathBuf = "/".parse().unwrap();
/// std::fs::read(&path).with_context(ctx!(
///   "Could not read the file {path:?}", ;
///   "Ensure that the file exists and you have permissions to access it",
/// ));
/// ```
///
/// If one does not want to print a help message this can be easily done by
/// leaving it empty:
///
/// ```no_run
/// # #[macro_use]
/// # use gourd_lib::error::Ctx;
/// # use gourd_lib::ctx;
/// # use std::path::PathBuf;
/// # use anyhow::Context;
/// # let path: PathBuf = "/".parse().unwrap();
/// std::fs::read(&path).with_context(ctx!(
///   "Could not read the file {path:?}", ;
///   "",
/// ));
/// ```
#[macro_export]
macro_rules! ctx {
    {$cause: expr,  $($arg_cause: expr)*; $help: expr, $($arg_help: tt)*} => {
      || {$crate::error::Ctx(format!($cause, $($arg_cause)*), format!($help, $($arg_help)*))}
    };
}

/// This is a shorthand for the [anyhow::bail] macro, now with context.
///
/// # Example
///
/// Instead of doing:
/// ```no_run
/// # #[macro_use]
/// # use gourd_lib::error::Ctx;
/// # use gourd_lib::ctx;
/// # use std::path::PathBuf;
/// # use anyhow::Context;
/// # use anyhow::anyhow;
/// # use anyhow::Result;
/// # fn main() -> Result<()> {
/// return Err(anyhow!("Something")).with_context(ctx!("Something", ; "Help", ));
/// # Ok(())
/// # }
/// ```
///
/// Do:
/// ```no_run
/// # #[macro_use]
/// # use gourd_lib::error::Ctx;
/// # use gourd_lib::ctx;
/// # use gourd_lib::bailc;
/// # use std::path::PathBuf;
/// # use anyhow::anyhow;
/// # use anyhow::Context;
/// # use anyhow::Result;
/// # fn main() -> Result<()> {
/// bailc!("Something", ; "Something", ; "Help", );
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! bailc {
    {$text: expr,  $($arg_text: expr)*; $cause: expr,  $($arg_cause: expr)*; $help: expr, $($arg_help: tt)*} => {
        return Err(anyhow::anyhow!($text, $($arg_text)*)).with_context($crate::error::ctx!($cause, $($arg_cause)*; $help, $($arg_help)*));
    };
    {$text: expr $(,$arg_text: expr)*} => {
        return Err(anyhow::anyhow!($text, $($arg_text)*)).with_context($crate::error::ctx!("",;"",));
    };
}

pub use ctx;
