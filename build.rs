//! The building process.
//!
//! This script does two steps when building `gourd`.
//! 1. The shell completions are compiled and placed in
//!    `[output_dir]/completions/`.
//! 2. If the feature `documentation` is on, user and maintainer documentation
//!    will be compiled into `[output_dir]/manpages/`.

#![allow(unused)]
#![allow(clippy::missing_docs_in_private_items)]

use std::env;
use std::fmt::format;
use std::fs;
use std::fs::canonicalize;
use std::fs::File;
use std::fs::Permissions;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command as StdCommand;
use std::ptr::copy;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Context;
use anyhow::Result;
use clap::Command;
use clap::CommandFactory;
use clap_complete::generate_to;
use clap_complete::shells::Bash;
use clap_complete::shells::Fish;
use clap_complete::shells::PowerShell;
use clap_complete::shells::Zsh;

#[cfg(feature = "builtin-examples")]
include!("src/resources/build_builtin_examples.rs");

include!("src/gourd/cli/def.rs");

const GOURD_MANPAGE: &str = "docs/user/gourd.1.tex";
const GOURD_TOML_MANPAGE: &str = "docs/user/gourd.toml.5.tex";
const GOURD_TUTORIAL_MANPAGE: &str = "docs/user/gourd-tutorial.7.tex";
const MAINTAINER_DOCS: &str = "./maintainer.tex";
const MAINTAINER_DOCS_WORKDIR: &str = "docs/maintainer/";

const PREAMBLE: &str = include_str!("docs/user/html/preamble.html");
const POSTAMBLE: &str = include_str!("docs/user/html/postamble.html");
const STYLE: &str = include_str!("docs/user/html/manpage.css");

const INSTALLER: &str = include_str!("src/resources/install.sh");

const XETEX_OPTS: [&str; 3] = [
    "-halt-on-error",
    "-shell-escape",
    "-interaction=nonstopmode",
];

const MANDOC_OPTS: [&str; 5] = [
    "-I",
    "os=\"rendered by mandoc\"",
    "-Kutf-8",
    "-Ofragment,toc",
    "-Thtml",
];

fn main() -> Result<()> {
    let outdir: PathBuf = match env::var_os("OUT_DIR") {
        None => return Ok(()),
        Some(outdir) => outdir,
    }
    .into();

    let triple: String = match env::var_os("TARGET") {
        None => return Ok(()),
        Some(outdir) => outdir.to_str().map(|x| x.to_string()).unwrap(),
    };

    let target_dir = outdir.parent().unwrap().parent().unwrap().parent().unwrap();

    let completions = target_dir.join("completions/");
    let docs = target_dir.join("manpages/");

    let _ = fs::create_dir(&completions);

    let mut completions_command = Cli::command();

    #[cfg(feature = "builtin-examples")]
    {
        let tars = target_dir.join("tarballs/");
        completions_command = build_builtin_examples(&tars, completions_command)?;
    }

    generate_to(Bash, &mut completions_command, "gourd", &completions)?;
    generate_to(Fish, &mut completions_command, "gourd", &completions)?;
    generate_to(PowerShell, &mut completions_command, "gourd", &completions)?;
    generate_to(Zsh, &mut completions_command, "gourd", &completions)?;

    #[cfg(feature = "documentation")]
    {
        let _ = fs::create_dir(&docs);

        println!("cargo::rerun-if-changed=docs/");
        println!("cargo::rerun-if-changed=src/resources/install.sh");
        println!("cargo::rerun-if-changed=src/resources/uninstall.sh");

        let gourd = generate_man(GOURD_MANPAGE.parse()?, &docs)?;
        generate_pdf(GOURD_MANPAGE.parse()?, &docs)?;
        generate_html(gourd, &docs)?;

        let gourd_toml = generate_man(GOURD_TOML_MANPAGE.parse()?, &docs)?;
        generate_pdf(GOURD_TOML_MANPAGE.parse()?, &docs)?;
        generate_html(gourd_toml, &docs)?;

        let gourd_tutorial = generate_man(GOURD_TUTORIAL_MANPAGE.parse()?, &docs)?;
        generate_pdf(GOURD_TUTORIAL_MANPAGE.parse()?, &docs)?;
        generate_html(gourd_tutorial, &docs)?;

        generate_latex(
            MAINTAINER_DOCS.parse()?,
            &docs,
            Some(MAINTAINER_DOCS_WORKDIR.parse()?),
        )?;

        let installer = target_dir.join("generate-installer.sh");
        let uninstaller = target_dir.join("uninstall.sh");

        fs::write(&installer, INSTALLER.replace("{{triple}}", &triple));

        #[cfg(unix)]
        fs::set_permissions(&installer, Permissions::from_mode(0o755));
    }

    Ok(())
}

fn generate_man(doc_path: PathBuf, out_folder: &Path) -> Result<PathBuf> {
    let output = out_folder.join(doc_path.with_extension("man").file_name().unwrap());

    run_command(
        "latex2man",
        &vec![
            "-t./docs/user/latex2man.trans",
            "-M",
            doc_path.to_str().unwrap(),
            output.to_str().unwrap(),
        ],
        None,
    )?;

    Ok(output)
}

fn generate_latex(
    doc_path: PathBuf,
    out_folder: &Path,
    workdir: Option<PathBuf>,
) -> Result<PathBuf> {
    let xetex_workdir = out_folder.join("xetex/");
    let _ = fs::create_dir(&xetex_workdir);

    let output_expected = xetex_workdir.join(doc_path.with_extension("pdf").file_name().unwrap());
    let output_actual = out_folder.join(doc_path.with_extension("pdf").file_name().unwrap());

    let mut opts = XETEX_OPTS.to_vec();

    let output_dir_arg = format!("-output-directory={}", xetex_workdir.to_str().unwrap());

    opts.push(&output_dir_arg);
    opts.push(doc_path.to_str().unwrap());

    run_command("xelatex", &opts, workdir.clone())?;
    run_command("xelatex", &opts, workdir)?;

    fs::copy(output_expected, &output_actual)?;

    let _ = fs::remove_dir_all(&xetex_workdir);

    Ok(output_actual)
}

fn generate_pdf(doc_path: PathBuf, out_folder: &Path) -> Result<PathBuf> {
    let xetex_workdir = out_folder.join("xetex/");

    let output_intr = xetex_workdir.join(doc_path.with_extension("tex").file_name().unwrap());

    let _ = fs::create_dir(&xetex_workdir);

    run_command(
        "latex2man",
        &vec![
            "-L",
            doc_path.to_str().unwrap(),
            output_intr.to_str().unwrap(),
        ],
        None,
    )?;

    generate_latex(output_intr, out_folder, None)
}

fn generate_html(man_path: PathBuf, out_folder: &Path) -> Result<PathBuf> {
    let mut opts = MANDOC_OPTS.to_vec();
    opts.push(man_path.to_str().unwrap());

    let mut html = run_command("mandoc", &opts, None)?;

    html = html.replace(
        "gourd-tutorial(7)",
        "<a class=\"manref\" href=\"./gourd-tutorial.7.html\">gourd-tutorial(7)</a>",
    );
    html = html.replace(
        "gourd(1)",
        "<a class=\"manref\" href=\"./gourd.1.html\">gourd(1)</a>",
    );
    html = html.replace(
        "gourd.toml(5)",
        "<a class=\"manref\" href=\"./gourd.toml.5.html\">gourd.toml(5)</a>",
    );

    let out_path = out_folder.join(man_path.with_extension("html").file_name().unwrap());
    let style_path = out_folder.join("manpage.css");

    fs::write(&out_path, format!("{}{}{}", PREAMBLE, html, POSTAMBLE))?;
    fs::write(style_path, STYLE)?;

    Ok(out_path)
}

fn run_command(cmd: &str, args: &Vec<&str>, workdir: Option<PathBuf>) -> Result<String> {
    let mut actual = StdCommand::new(cmd);
    if let Some(direr) = workdir {
        actual.current_dir(direr);
    }
    actual.args(args);

    println!("running {actual:?}");

    let out = actual.output()?;

    println!("command output: {}", String::from_utf8(out.stdout.clone())?);

    if !out.status.success() {
        panic!(
            "Running {actual:?} failed, \nerr: {}",
            String::from_utf8(out.stderr)?
        );
    }

    String::from_utf8(out.stdout).context("")
}
