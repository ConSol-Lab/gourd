use std::ops::Deref;

use clap::builder::PossibleValuesParser;
use clap::builder::Str;
use clap::builder::ValueParser;
use clap::ValueHint;
use flate2::write::GzEncoder;
use flate2::Compression;
use tar::Archive;
use tar::Builder;

const GOURD_INIT_EXAMPLE_FOLDERS: &str = "src/resources/gourd_init_examples";

/// Creates tarballs for examples that should be included in the `gourd`
/// runtime.
///
/// Also returns the updated CLI command while including completions of the
/// example.
fn build_builtin_examples(out_folder: &Path, completions_command: Command) -> Result<Command> {
    let _ = fs::create_dir(out_folder);

    println!("cargo::rerun-if-changed={GOURD_INIT_EXAMPLE_FOLDERS}");
    println!("cargo::rerun-if-changed=src/resources/build_builtin_examples.rs");

    let mut possible_ids: Vec<Str> = vec![];

    for e in PathBuf::from(GOURD_INIT_EXAMPLE_FOLDERS)
        .read_dir()
        .context(format!(
            "Could not find the '{GOURD_INIT_EXAMPLE_FOLDERS}' directory."
        ))?
    {
        let path = e?.path();

        if (path.is_dir()) {
            println!("Generating example tarball for {path:?}");

            let mut tar_path = PathBuf::from(out_folder);
            let file_name = path
                .file_name()
                .context("Could not get the directory name")?;

            tar_path.push(file_name);
            tar_path.set_extension("tar.gz");

            let id_str = Str::from(
                &file_name
                    .to_str()
                    .context("Invalid characters in example subfolder name")?
                    .to_owned()
                    .replace([' ', '_'], "-"),
            );

            if (id_str.contains('.')) {
                println!(
                    "cargo:warning=The '.' character is invalid for a folder name \
                in \"resources/gourd_init_examples\": {id_str}."
                );
                continue;
            }

            if possible_ids.contains(&id_str) {
                println!(
                    "cargo:warning=There are two subfolders matching the \"{id_str}\" example ID."
                );
                continue;
            }

            println!("The output file is {tar_path:?}");
            generate_example_tarball(&path, &tar_path)?;

            possible_ids.push(id_str);
        }
    }

    Ok(
        completions_command.mut_subcommand("init", |init_subcommand| {
            init_subcommand.mut_arg("example", |example_arg| {
                example_arg.value_parser(ValueParser::from(possible_ids))
            })
        }),
    )
}

/// Creates a `gourd init` example at the specified path.
///
/// This function accepts a path to a subfolder containing a valid `gourd.toml`
/// and other experiment resources.
/// It compresses the folder contents into a `.tar.gz` archive (excluding the
/// folder itself), while also compiling `.rs` filesinto platform-native
/// binaries. The archive will be created at the provided 'tarball' path.
fn generate_example_tarball(subfolder_path: &Path, tarball_output_path: &Path) -> Result<()> {
    if !subfolder_path.is_dir() {
        bail!("The subfolder path {subfolder_path:?} is not a directory.");
    }

    if !tarball_output_path
        .parent()
        .expect("The tarball output path has no parent.")
        .is_dir()
    {
        bail!("The tarball output path {tarball_output_path:?} is not a directory.");
    }

    let mut file = File::create(tarball_output_path)?;

    let mut gz = GzEncoder::new(file, Compression::default());
    let mut tar = tar::Builder::new(gz);

    println!("Writing the folder contents to {tarball_output_path:?}");
    append_files_to_tarball(&mut tar, PathBuf::from("."), subfolder_path)?;

    println!("Finalizing the archive.");
    tar.finish();
    Ok(())
}

/// Appends experiment files to the given tarball builder.
///
/// This function recursively searches the provided directory, adding all
/// normal files and a `rustc`-compiled version of each `.rs` file to the tar
/// archive.
fn append_files_to_tarball(
    tar: &mut Builder<GzEncoder<File>>,
    path_in_subfolder: PathBuf,
    subfolder_root: &Path,
) -> Result<()> {
    let mut fs_path = subfolder_root.to_path_buf();
    fs_path.push(&path_in_subfolder);

    if fs_path.is_file() {
        println!("Inclding file: {fs_path:?}");

        if is_a_rust_file(&fs_path) {
            compile_rust_file(&fs_path)
                .context(format!("Could not compile a Rust example: {fs_path:?}"))?;

            let compiled_fs_path = &fs_path.with_extension("");
            let compiled_subfolder_path = &path_in_subfolder.with_extension("");

            tar.append_path_with_name(compiled_fs_path, compiled_subfolder_path)
                .context(format!(
                    "Could not add a compiled Rust file to the tarball: {compiled_fs_path:?}"
                ))?;

            fs::remove_file(compiled_fs_path).context(format!(
                "Could not remove the compiled file: {compiled_fs_path:?}"
            ));
        } else {
            tar.append_path_with_name(&fs_path, &path_in_subfolder)
                .context(format!("Could not add a file to the tarball: {fs_path:?}"))?;
        }

        Ok(())
    } else if fs_path.is_dir() {
        for e in fs::read_dir(&fs_path)
            .context(format!("Could not read the directory at {fs_path:?}"))?
        {
            let entry_name = e
                .context(format!(
                    "Could not unwrap directory entry in entry {fs_path:?}"
                ))?
                .file_name();

            let mut new_path_in_subfolder = path_in_subfolder.clone();
            new_path_in_subfolder.push(entry_name);

            append_files_to_tarball(tar, new_path_in_subfolder, subfolder_root)?
        }

        Ok(())
    } else {
        Ok(())
    }
}

/// Checks whether the path is a Rust file.
///
/// Returns true if the provided path links to a file,
/// and the file has the `.rs` extension.
fn is_a_rust_file(path: &Path) -> bool {
    path.is_file() && path.extension().is_some_and(|ext| ext == "rs")
}

/// Returns the path of the file after it has been compiled with `rustc`.
fn compile_rust_file(path: &Path) -> Result<()> {
    let canon_path =
        canonicalize(path).context(format!("Could not canonicalize the path: {path:?}"))?;

    let str_path = canon_path.to_str().ok_or_else(|| anyhow!(":("))?;

    let compiled_path = canon_path.with_extension("");

    let str_compiled_path = compiled_path.to_str().ok_or_else(|| anyhow!(":("))?;

    // on linux, we statically link the examples to musl in order to prevent glibc
    // compatibility errors.
    let target_triple = format!("{}-unknown-linux-musl", std::env::consts::ARCH);
    let compile_args = if std::env::consts::OS == "linux" {
        std::process::Command::new("rustup")
            .arg("target")
            .arg("add")
            .arg(&target_triple)
            .spawn()?
            .wait()?;

        vec![
            "--target",
            &target_triple,
            "-C",
            "target-feature=+crt-static",
            "-O",
            str_path,
            "-o",
            str_compiled_path,
        ]
    } else {
        vec!["-O", str_path, "-o", str_compiled_path]
    };

    let output = run_command(
        "rustc",
        &compile_args,
        Some(canon_path.parent().ok_or_else(|| anyhow!(":("))?.to_owned()),
    )?;

    if !compiled_path.is_file() {
        Err(anyhow!("Rustc output: {output}")
            .context(format!("No rust file generated at {compiled_path:?}")))
    } else {
        Ok(())
    }
}
