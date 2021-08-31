use std::{
    env,
    ffi::OsStr,
    fmt::Display,
    fs::{read_dir, write, File},
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf},
};
use vergen::{vergen, Config};
use walkdir::WalkDir;

fn main() -> io::Result<()> {
    let out_dir = env::var_os("OUT_DIR").unwrap();

    vergen(Config::default()).unwrap();

    set_build_env()?;

    let path = Path::new(&out_dir).join("tests.rs");
    let mut contents = find_tests()?
        .iter()
        .map(|test| test.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    contents.push('\n');
    write(&path, contents).unwrap();

    Ok(())
}

fn set_build_env() -> io::Result<()> {
    let path = "build.env";
    println!("cargo:rerun-if-changed={}", path);
    if let Ok(file) = File::open(path) {
        for (key, value) in BufReader::new(file)
            .lines()
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .flat_map(|line| line.split_once('='))
        {
            println!("cargo:rustc-env={}={}", key, value);
        }
    }
    Ok(())
}

fn find_tests() -> io::Result<Vec<TestCase>> {
    let datapacks_path = Path::new("src/tests/datapacks");

    let mut tests = Vec::new();
    tests.extend(find_test_cases(datapacks_path, "test")?);
    tests.extend(find_test_cases(
        datapacks_path,
        "test_before_age_increment",
    )?);
    tests.extend(find_test_cases(datapacks_path, "test_after_age_increment")?);
    Ok(tests)
}

fn find_test_cases(datapacks_path: &Path, category: &str) -> io::Result<Vec<TestCase>> {
    let path = datapacks_path
        .join("mcfd_test/data")
        .join(category)
        .join("functions");
    println!("cargo:rerun-if-changed={}", path.display());

    let mut tests = Vec::new();
    for test_entry in read_dir(path)? {
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
                        util_files.push(
                            util_file
                                .strip_prefix(datapacks_path)
                                .unwrap()
                                .to_path_buf(),
                        );
                    }
                }
                if let Some(name) = test_dir.file_name().and_then(OsStr::to_str) {
                    tests.push(TestCase {
                        category: category.to_string(),
                        name: name.to_string(),
                        test_file: test_file
                            .strip_prefix(datapacks_path)
                            .unwrap()
                            .to_path_buf(),
                        util_files,
                    });
                }
            }
        }
    }
    Ok(tests)
}

struct TestCase {
    category: String,
    name: String,
    test_file: PathBuf,
    util_files: Vec<PathBuf>,
}

impl Display for TestCase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}!({}, {}, \"{}\"",
            self.category,
            self.category,
            self.name,
            self.test_file.display()
        )?;
        for util_file in &self.util_files {
            write!(f, ", \"{}\"", util_file.display())?;
        }
        write!(f, ");")
    }
}
