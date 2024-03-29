use clap::{App, Arg};
use failure::{Error, ResultExt};
use std::io::Write;
use std::path::{Path, PathBuf};

use failure::bail;

mod build;
mod constants;

fn main() -> Result<(), Error> {
    let version = format!(
        "{}.{}.{}{}",
        env!("CARGO_PKG_VERSION_MAJOR"),
        env!("CARGO_PKG_VERSION_MINOR"),
        env!("CARGO_PKG_VERSION_PATCH"),
        option_env!("CARGO_PKG_VERSION_PRE").unwrap_or("")
    );
    let matches = App::new("bithumb-chain-wasm-build")
        .about("does awesome things")
        .version(version.as_str())
        .arg(
            Arg::with_name("input")
                .index(1)
                .required(true)
                .help("Wasm file generated by rustc compiler"),
        )
        .arg(Arg::with_name("output").index(2).required(false).help("Output wasm/wast file name"))
        .arg(
            Arg::with_name("keepcustom")
                .long("keep-custom")
                .help("Keep custom section in output wasm file"),
        )
        .get_matches();

    let input = matches.value_of("input").expect("required arg can not be None");
    if !Path::new(input).exists() {
        bail!("file does not exist: {}", input);
    }
    let (save_str, output) = match matches.value_of("output") {
        Some(output) => (None, output.to_string()),
        None => {
            let mut path = PathBuf::from(input);
            let mut file_stem = path.file_stem().unwrap().to_os_string();
            file_stem.push("_optimized.wasm");
            path.pop();
            path.push(file_stem);
            let wasm = path.to_str().unwrap().to_string();
            let str_file = wasm.clone() + ".str";

            (Some(str_file), wasm)
        }
    };

    let keep_custom = matches.is_present("keepcustom");

    let module =
        parity_wasm::deserialize_file(input).context("could not deserialize input wasm file")?;

    let module = build::build(module, !keep_custom)?;
    let buf = parity_wasm::serialize(module)?;
    if buf.len() > constants::MAX_WASM_SIZE {
        bail!("finial wasm file size exceed 512KB");
    }

    if let Some(str_file) = save_str {
        let mut io = ::std::fs::File::create(str_file)?;
        io.write_all(hex::encode(&buf).as_bytes())?;
    }

    let buf: Vec<u8> = match Path::new(&output).extension() {
        Some(ext) if ext == "wat" || ext == "wast" => {
            let wat = wabt::wasm2wat(buf)?;
            wat.into_bytes()
        }
        Some(ext) if ext == "str" => hex::encode(&buf).as_bytes().to_vec(),
        _ => buf,
    };

    let mut io = ::std::fs::File::create(output)?;
    io.write_all(&buf)?;
    return Ok(());
}

#[cfg(test)]
mod test;
