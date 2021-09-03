use std::{
    collections::BTreeMap,
    env,
    ffi::OsStr,
    fmt::Display,
    fs::{copy, create_dir_all, read_dir, write, File},
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
};
use vergen::{vergen, Config};
use walkdir::WalkDir;

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();

    vergen(Config::default()).unwrap();

    set_build_env();

    remove_license_header_from_templates(&out_dir);

    generate_tests(out_dir);
}

fn set_build_env() {
    let path = "build.env";
    println!("cargo:rerun-if-changed={}", path);
    if let Ok(file) = File::open(path) {
        for (key, value) in BufReader::new(file)
            .lines()
            .map(|line| line.unwrap())
            .collect::<Vec<_>>()
            .iter()
            .flat_map(|line| line.split_once('='))
        {
            println!("cargo:rustc-env={}={}", key, value);
        }
    }
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

fn generate_tests(out_dir: impl AsRef<Path>) {
    let out_dir = out_dir.as_ref().join("tests");
    create_dir_all(&out_dir).unwrap();

    let datapack_path = Path::new(DATAPACKS_PATH).join("mcfd_test");
    for (namespace, tests) in find_tests(&datapack_path) {
        let path = out_dir.join(namespace).with_extension("rs");
        let mut contents = tests
            .into_iter()
            .map(|test| test.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        contents.push('\n');
        write(&path, contents).unwrap();
    }

    let mut writer =
        BufWriter::new(File::create(out_dir.join("expand_test_templates.rs")).unwrap());
    writer.write_all("{\n".as_bytes()).unwrap();
    for entry in WalkDir::new(&datapack_path).sort_by_file_name() {
        let entry = entry.unwrap();
        if entry.file_type().is_file() {
            let relative_path = entry.path().strip_prefix(DATAPACKS_PATH).unwrap();
            writeln!(
                writer,
                "    expand_test_template!(\"{}\").await?;",
                relative_path.display()
            )
            .unwrap();
        }
    }
    writer.write_all("}\n".as_bytes()).unwrap();
}

fn find_tests(datapack_path: impl AsRef<Path>) -> BTreeMap<String, Vec<TestCase>> {
    let datapack_path = datapack_path.as_ref();
    println!("cargo:rerun-if-changed={}", datapack_path.display());
    let data_path = datapack_path.join("data");

    let mut tests: BTreeMap<String, Vec<TestCase>> = BTreeMap::new();
    for namespace_entry in read_dir(&data_path).unwrap() {
        let namespace_entry = namespace_entry.unwrap();
        if !namespace_entry.file_type().unwrap().is_dir() {
            continue;
        }
        let namespace = namespace_entry.file_name();
        let namespace = namespace.to_str().unwrap();
        tests.insert(
            namespace.to_string(),
            find_tests_in_namespace(datapack_path, namespace),
        );
    }
    tests
}

fn find_tests_in_namespace(datapack_path: impl AsRef<Path>, namespace: &str) -> Vec<TestCase> {
    let functions_path = datapack_path
        .as_ref()
        .join("data")
        .join(namespace)
        .join("functions");
    let mut tests = Vec::new();
    for entry in WalkDir::new(&functions_path) {
        let entry = entry.unwrap();
        if entry.file_type().is_file() && entry.file_name() == OsStr::new("test.mcfunction") {
            let parent_file_name = entry.path().parent().unwrap().file_name().unwrap();
            tests.push(TestCase {
                namespace: namespace.to_string(),
                name: parent_file_name.to_str().unwrap().to_string(),
            });
        }
    }
    tests
}

struct TestCase {
    namespace: String,
    name: String,
}

impl Display for TestCase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "test!({}, {});", self.namespace, self.name)
    }
}
