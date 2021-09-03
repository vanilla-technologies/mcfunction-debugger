use std::{
    env,
    ffi::OsStr,
    fmt::Display,
    fs::{copy, create_dir, create_dir_all, read_dir, write, File},
    io::{self, BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};
use vergen::{vergen, Config};
use walkdir::WalkDir;

fn main() -> io::Result<()> {
    let out_dir = env::var_os("OUT_DIR").unwrap();

    vergen(Config::default()).unwrap();

    set_build_env()?;

    remove_license_header_from_templates(&out_dir);

    generate_tests(out_dir)?;

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

fn remove_license_header_from_templates(out_dir: impl AsRef<Path>) {
    let in_dir = "src/datapack_template";
    println!("cargo:rerun-if-changed={}", in_dir);

    for entry in WalkDir::new(&in_dir) {
        let entry = entry.unwrap();
        let in_path = entry.path();
        let out_path = out_dir.as_ref().join(in_path);
        let file_type = entry.file_type();
        if file_type.is_dir() {
            println!("Creating dir  {}", out_path.display());
            create_dir_all(out_path).unwrap();
        } else if file_type.is_file() {
            println!("Creating file {}", out_path.display());
            if in_path.extension() == Some(OsStr::new("mcfunction")) {
                let reader = BufReader::new(File::open(in_path).unwrap());
                let mut writer = BufWriter::new(File::create(out_path).unwrap());
                for line in reader
                    .lines()
                    .skip_while(|line| line.as_ref().ok().filter(|l| l.starts_with('#')).is_some())
                    .skip_while(|line| line.as_ref().ok().filter(|l| l.is_empty()).is_some())
                {
                    writer.write_all(line.unwrap().as_bytes()).unwrap();
                    writer.write_all(&[b'\n']).unwrap();
                }
            } else {
                copy(in_path, out_path).unwrap();
            }
        }
    }
}

const DATAPACKS_PATH: &str = "src/tests/datapacks";

fn generate_tests(out_dir: impl AsRef<Path>) -> io::Result<()> {
    let out_dir = out_dir.as_ref().join("tests");
    create_dir(&out_dir)?;
    generate_tests_for_category("test", &out_dir)?;
    generate_tests_for_category("test_before_age_increment", &out_dir)?;
    generate_tests_for_category("test_after_age_increment", &out_dir)?;
    Ok(())
}

fn generate_tests_for_category(category: &str, out_dir: impl AsRef<Path>) -> io::Result<()> {
    let path = out_dir.as_ref().join(category).with_extension("rs");
    let mut contents = find_test_cases(DATAPACKS_PATH, category)?
        .iter()
        .map(|test| test.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    contents.push('\n');
    write(&path, contents).unwrap();
    Ok(())
}

fn find_test_cases(datapacks_path: impl AsRef<Path>, category: &str) -> io::Result<Vec<TestCase>> {
    let path = datapacks_path
        .as_ref()
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
                                .strip_prefix(&datapacks_path)
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
                            .strip_prefix(&datapacks_path)
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
            "test!({}, {}, \"{}\"",
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
