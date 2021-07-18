use std::env;
use std::ffi::OsStr;
use std::fmt::Display;
use std::fmt::Write;
use std::fs;
use std::fs::read_dir;
use std::fs::File;
use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;
use walkdir::WalkDir;

fn main() -> io::Result<()> {
    let out_dir = env::var_os("OUT_DIR").unwrap();

    set_env()?;

    let path = Path::new(&out_dir).join("tests.rs");
    let contents = find_tests()?
        .iter()
        .map(|test| test.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&path, contents).unwrap();

    Ok(())
}

fn set_env() -> io::Result<()> {
    let path = "build.properties";
    for (key, value) in BufReader::new(File::open(path)?)
        .lines()
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .flat_map(|line| line.split_once('='))
    {
        println!("cargo:rustc-env={}={}", key, value);
    }
    Ok(())
}

fn find_tests() -> io::Result<Vec<TestCase>> {
    let datapack_path = Path::new("test/datapack_template");

    println!("cargo:rerun-if-changed={}", datapack_path.display());

    let mut tests = Vec::new();
    for test_entry in read_dir(datapack_path.join("data/test/functions"))? {
        let test_entry = test_entry?;
        if test_entry.file_type()?.is_dir() {
            let test_dir = test_entry.path();
            let test_file = test_dir.join("test.mcfunction");
            if test_file.is_file() {
                let mut util_files = Vec::new();
                for util_entry in WalkDir::new(&test_dir) {
                    let util_entry = util_entry?;
                    let util_file = util_entry.path();
                    if util_file != test_file
                        && util_file.extension() == Some(OsStr::new("mcfunction"))
                        && util_entry.file_type().is_file()
                    {
                        util_files
                            .push(util_file.strip_prefix(datapack_path).unwrap().to_path_buf());
                    }
                }
                if let Some(name) = test_dir.file_name().and_then(OsStr::to_str) {
                    tests.push(TestCase {
                        name: name.to_string(),
                        util_files,
                    });
                }
            }
        }
    }
    Ok(tests)
}

struct TestCase {
    name: String,
    util_files: Vec<PathBuf>,
}

impl Display for TestCase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("test!(")?;
        f.write_str(&self.name)?;
        for util_file in &self.util_files {
            f.write_str(", \"")?;
            f.write_str(&util_file.display().to_string())?;
            f.write_char('"')?;
        }
        f.write_str(");\n")
    }
}
