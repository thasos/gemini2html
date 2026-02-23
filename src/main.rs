#![forbid(unsafe_code)]
use gemini2html::Gemini2HtmlError;
use gemini2html::convert_gemini_file;

use log::{debug, error, info};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

/// Walks through a tree directory, recreate arborescence, convert gemini files, and copy the rest
/// it's a recursive function, but we always need the original ancestor (it's ugly, I know...)
fn convert_tree(
    // TODO find a way to retrieve ancestor
    // with `ancestors()` ? or `components()` ?
    // see https://doc.rust-lang.org/std/path/struct.Path.html#method.ancestors
    ancestor: &Path,
    source_tree_directory: &Path,
    target_tree_directory: &Path,
) -> Result<(), Gemini2HtmlError> {
    let tree = fs::read_dir(source_tree_directory).map_err(|e| {
        error!("unable to read source directory : {e:?}");
        Gemini2HtmlError
    })?;
    // used further, should be in args
    let gemini_extension = OsStr::new("gmi");
    // start looping on dir entries
    for direntry in tree {
        match direntry {
            Ok(direntry) => {
                // we only need path
                let direntry = direntry.path();
                // recreate directory tree
                if direntry.is_dir() {
                    info!("- 🗃️ found directory {:?}", &direntry);
                    let target_directory =
                        replace_ancestor(ancestor, target_tree_directory, &direntry)?;
                    if !target_directory.is_dir() {
                        info!("- 🦢 need to create {:?}", &target_directory);
                        fs::create_dir(&target_directory).map_err(|e| {
                            error!("unable to create directory : {e:?}");
                            Gemini2HtmlError
                        })?;
                    }
                    convert_tree(ancestor, &direntry, target_tree_directory)?;
                } else {
                    info!("- 🗒 found file {:?}", direntry);
                    // convert gemini file (`.gmi` extension)
                    if direntry.extension() == Some(gemini_extension) {
                        info!("- ⏩ convert to html");
                        // we need to change file extension
                        let mut target_html_file_path = direntry.clone();
                        let _ = target_html_file_path.set_extension("html");
                        let target_file = replace_ancestor(
                            ancestor,
                            target_tree_directory,
                            &target_html_file_path,
                        )?;
                        convert_gemini_file(&direntry, &target_file)?;
                    // copy other files (images...)
                    } else {
                        let target_file =
                            replace_ancestor(ancestor, target_tree_directory, &direntry)?;
                        info!("- 🍝 copy non gemini file : {:?}", target_file);
                        fs::copy(direntry, target_file).map_err(|e| {
                            error!("unable to create directory : {e:?}");
                            Gemini2HtmlError
                        })?;
                    }
                }
            }
            Err(e) => {
                error!("unable to list all files in directory {e}");
                return Err(Gemini2HtmlError);
            }
        }
    }
    Ok(())
}

/// Replace ancestor directory by another in a Path
fn replace_ancestor(
    source_ancestor: &Path,
    target_ancestor: &Path,
    path: &Path,
) -> Result<PathBuf, Gemini2HtmlError> {
    let target_without_ancestor = path.strip_prefix(source_ancestor).map_err(|e| {
        error!("unable to read source directory : {e:?}");
        Gemini2HtmlError
    })?;
    let final_target = Path::new(target_ancestor).join(target_without_ancestor);
    Ok(final_target)
}

/// Very simple args parser
fn parse_args(args: &[String]) -> Result<(&Path, &Path), Gemini2HtmlError> {
    if args.len() < 3 {
        error!(
            "not enough arguments, usage : gemini2html <source directory> <destination directory>"
        );
        return Err(Gemini2HtmlError);
    }
    let source = &args[1];
    let target = &args[2];
    debug!("source directory : {:?}", source);
    debug!("target directory :  {:?}", target);
    let source_tree_directory = Path::new(source);
    let target_tree_directory = Path::new(target);
    Ok((source_tree_directory, target_tree_directory))
}

/// Here is the magic
fn main() -> Result<(), Gemini2HtmlError> {
    env_logger::Builder::default()
        .filter_level(log::LevelFilter::Info)
        .init();
    info!("🚀 start gemini2html");
    info!("---------------------");

    // read directory from args
    let args: Vec<String> = env::args().collect();
    let (source_tree_directory, target_tree_directory) = parse_args(&args)?;

    // source directory exists ?
    if source_tree_directory.is_dir() {
        // create target directory if not present
        if !target_tree_directory.is_dir() {
            info!("ℹ️  target directory not found, create it");
            fs::create_dir(target_tree_directory).map_err(|e| {
                error!("unable to create target directory : {e:?}");
                Gemini2HtmlError
            })?;
        }
        // let's go hike
        info!(
            "🚶 walking source directory {:?} and create tree 🌳",
            source_tree_directory
        );
        convert_tree(
            source_tree_directory,
            source_tree_directory,
            target_tree_directory,
        )?;

        // let gemini_file = Path::new("./tests/gemini_file.gmi");
        // let html_file = Path::new("./tests/html_file.html");
        // // TODO handle result
        // convert_gemini_file(gemini_file, html_file)?;
        info!("---------------------");
        info!("💤 end gemini2html");
        Ok(())
    } else {
        error!("source directory {:?} not found", source_tree_directory);
        info!("---------------------");
        info!("💤 end gemini2html");
        Err(Gemini2HtmlError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    #[test]
    fn test_replace_ancestor() {
        let origin_ancestor = Path::new("./origin");
        let target = Path::new("./target");
        let path = Path::new("./origin/some_entry");
        let replaced_path = replace_ancestor(origin_ancestor, target, path).unwrap();
        assert_eq!(replaced_path, Path::new("./target/some_entry"));
    }
    #[test]
    fn test_convert_tree() {
        let source_tree_directory = Path::new("./tests");
        let target_tree_directory = Path::new("./output_tests");
        if target_tree_directory.is_dir() {
            fs::remove_dir_all(target_tree_directory)
                .expect("unable to purge old test dir ./output_tests");
        }
        fs::create_dir(target_tree_directory).expect("unable to create ./output_tests");
        let _ = convert_tree(
            source_tree_directory,
            source_tree_directory,
            target_tree_directory,
        );
        let files: Vec<&Path> = vec![
            // ⛔ ⬇️
            Path::new("./output_tests/gemini_file.html"),
            Path::new("./output_tests/non_gemini_file.txt"),
            Path::new("./output_tests/subdir/subfile.html"),
            Path::new("./output_tests/subdir/subsubdir/subfile.html"),
            // ⬇️  always append here, or it will break cargo insta snapshots order
            // 🟢
        ];
        for file in files {
            let content = fs::read_to_string(file).expect("unable to read file in ./output_tests");
            assert_snapshot!(content);
        }
        fs::remove_dir_all(target_tree_directory)
            .expect("unable to purge old test dir ./output_tests");
    }
    #[test]
    fn test_parse_args() {
        let args = [
            "gemini2html".to_string(),
            "path1".to_string(),
            "path2".to_string(),
        ];
        let (path1, path2) = parse_args(&args).unwrap();
        assert_eq!(path1, "path1");
        assert_eq!(path2, "path2");
    }
    // #[test]
    // fn test_main() {
    //     let source_tree_directory = Path::new("./tests");
    //     let target_tree_directory = Path::new("./output_tests");
    //     fs::remove_dir_all("./output_tests").expect("unable to purge old test dir ./output_tests");
    //     fs::create_dir(target_tree_directory).expect("unable to create ./output_tests");
    //     let _ = convert_tree(
    //         source_tree_directory,
    //         source_tree_directory,
    //         target_tree_directory,
    //     );
    //     let files: Vec<&Path> = vec![
    //         // ⛔ ⬇️
    //         Path::new("./output_tests/gemini_file.html"),
    //         Path::new("./output_tests/non_gemini_file.txt"),
    //         Path::new("./output_tests/subdir/subfile.html"),
    //         Path::new("./output_tests/subdir/subsubdir/subfile.html"),
    //         // ⬇️  always append here, or it will break cargo insta snapshots order
    //         // 🟢
    //     ];
    //     for file in files {
    //         let content = fs::read_to_string(file).expect("unable to read file in ./output_tests");
    //         assert_snapshot!(content);
    //     }
    // }
}
