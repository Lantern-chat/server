use std::{io, path::Path};

use preprocessor::{Context, Error as PreprocessorError, ErrorKind, Preprocessor};

fn preprocess_file(pre: &mut Preprocessor, path: &Path) -> Result<(), PreprocessorError> {
    let out = pre.process_file(path)?;

    let err_ctx = Context::new(path);

    let Some(parent) = path.parent() else {
        return Err(err_ctx.error(ErrorKind::IoError(io::Error::new(
            io::ErrorKind::NotFound,
            "Parent directory not found",
        ))));
    };

    let out_path = parent.join("out");

    _ = std::fs::create_dir(&out_path);

    match std::fs::write(out_path.join(path.file_name().unwrap()), out) {
        Ok(_) => Ok(()),
        Err(e) => Err(err_ctx.error(ErrorKind::IoError(e))),
    }
}

fn main() {
    let mut pre = Preprocessor::new(Vec::new());

    pre.single_line_comment("--"); // SQL single line comment

    preprocess_file(&mut pre, "./sql/seed.sql".as_ref()).unwrap();
    pre.clear();

    // TODO: More SQL files
}
