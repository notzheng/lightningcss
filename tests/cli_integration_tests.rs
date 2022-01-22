use assert_cmd::prelude::*;
use assert_fs::fixture::FixtureError;
use assert_fs::prelude::*;
use indoc::indoc;
use parcel_css::css_modules::CssModuleExport;
use predicates::prelude::*;
use std::collections::HashMap;
use std::process::Command;

fn test_file() -> Result<assert_fs::NamedTempFile, FixtureError> {
  let file = assert_fs::NamedTempFile::new("test.css")?;
  file.write_str(
    r#"
      .foo {
        border: none;
      }
    "#,
  )?;
  Ok(file)
}

fn css_module_test_vals() -> (String, String, String) {
  let exports: HashMap<&str, CssModuleExport> = HashMap::from([
    (
      "fade",
      CssModuleExport {
        name: "fade_EgL3uq".into(),
        composes: vec![],
        is_referenced: false,
      },
    ),
    (
      "foo",
      CssModuleExport {
        name: "foo_EgL3uq".into(),
        composes: vec![],
        is_referenced: false,
      },
    ),
    (
      "circles",
      CssModuleExport {
        name: "circles_EgL3uq".into(),
        composes: vec![],
        is_referenced: true,
      },
    ),
    (
      "id",
      CssModuleExport {
        name: "id_EgL3uq".into(),
        composes: vec![],
        is_referenced: false,
      },
    ),
    (
      "test",
      CssModuleExport {
        name: "test_EgL3uq".into(),
        composes: vec![],
        is_referenced: true,
      },
    ),
  ]);
  (
    r#"
      .foo {
        color: red;
      }
      
      #id {
        animation: 2s test;
      }

      @keyframes test {
        from { color: red }
        to { color: yellow }
      }

      @counter-style circles {
        symbols: Ⓐ Ⓑ Ⓒ;
      }

      ul {
        list-style: circles;
      }

      @keyframes fade {
        from { opacity: 0 }
        to { opacity: 1 }
      }
    "#
    .into(),
    indoc! {r#"
      .foo_EgL3uq {
        color: red;
      }

      #id_EgL3uq {
        animation: test_EgL3uq 2s;
      }

      @keyframes test_EgL3uq {
        from {
          color: red;
        }

        to {
          color: #ff0;
        }
      }

      @counter-style circles_EgL3uq {
        symbols: Ⓐ Ⓑ Ⓒ;
      }

      ul {
        list-style: circles_EgL3uq;
      }

      @keyframes fade_EgL3uq {
        from {
          opacity: 0;
        }

        to {
          opacity: 1;
        }
      }
    "#}
    .into(),
    serde_json::to_string(&exports).unwrap(),
  )
}

#[test]
fn valid_input_file() -> Result<(), Box<dyn std::error::Error>> {
  let file = assert_fs::NamedTempFile::new("test.css")?;
  file.write_str(
    r#"
      .foo {
        border: none;
      }
    "#,
  )?;

  let mut cmd = Command::cargo_bin("parcel_css")?;
  cmd.arg(file.path());
  cmd.assert().success().stdout(predicate::str::contains(indoc! {r#"
        .foo {
          border: none;
        }"#}));

  Ok(())
}

#[test]
fn no_input_file() -> Result<(), Box<dyn std::error::Error>> {
  let mut cmd = Command::cargo_bin("parcel_css")?;
  cmd.assert().failure().stderr(predicate::str::contains(
    "The following required arguments were not provided:\n    <INPUT_FILE>",
  ));

  Ok(())
}

#[test]
fn empty_input_file() -> Result<(), Box<dyn std::error::Error>> {
  let file = assert_fs::NamedTempFile::new("test.css")?;
  file.write_str("")?;

  let mut cmd = Command::cargo_bin("parcel_css")?;
  cmd.arg(file.path());
  cmd.assert().success();

  Ok(())
}

#[test]
fn output_file_option() -> Result<(), Box<dyn std::error::Error>> {
  let infile = test_file()?;
  let outfile = assert_fs::NamedTempFile::new("test.out")?;
  let mut cmd = Command::cargo_bin("parcel_css")?;
  cmd.arg(infile.path());
  cmd.arg("--output-file").arg(outfile.path());
  cmd.assert().success();
  outfile.assert(predicate::str::contains(indoc! {r#"
        .foo {
          border: none;
        }"#}));

  Ok(())
}

#[test]
fn minify_option() -> Result<(), Box<dyn std::error::Error>> {
  let infile = test_file()?;
  let mut cmd = Command::cargo_bin("parcel_css")?;
  cmd.arg(infile.path());
  cmd.arg("--minify");
  cmd
    .assert()
    .success()
    .stdout(predicate::str::contains(indoc! {r#".foo{border:0}"#}));

  Ok(())
}

#[test]
// nesting doesn't do anything with the default targets. until cli supports more targets, this option is a noop
#[ignore]
fn nesting_option() -> Result<(), Box<dyn std::error::Error>> {
  let infile = assert_fs::NamedTempFile::new("test.css")?;
  infile.write_str(
    r#"
        .foo {
          color: blue;
          & > .bar { color: red; }
        }
      "#,
  )?;

  let mut cmd = Command::cargo_bin("parcel_css")?;
  cmd.arg(infile.path());
  cmd.arg("--nesting");
  cmd.assert().success().stdout(predicate::str::contains(indoc! {r#"
        .foo {
          color: #00f;
        }

        .foo > .bar {
          color: red;
        }
      "#}));

  Ok(())
}

#[test]
fn css_modules_stdout() -> Result<(), Box<dyn std::error::Error>> {
  let (input, output, exports) = css_module_test_vals();
  let infile = assert_fs::NamedTempFile::new("test.css")?;
  infile.write_str(&input)?;
  let mut cmd = Command::cargo_bin("parcel_css")?;
  cmd.current_dir(infile.path().parent().unwrap());
  cmd.arg(infile.path());
  cmd.arg("--css-modules");
  cmd.assert().success().stdout(predicate::str::contains(output));

  let assert = cmd.assert();
  let output = String::from_utf8(assert.get_output().stdout.clone())?;
  let module_json_line = output.lines().next_back().unwrap();
  let expected: serde_json::Value = serde_json::from_str(&exports)?;
  let actual: serde_json::Value = serde_json::from_str(&module_json_line)?;
  assert_eq!(expected, actual);

  Ok(())
}

#[test]
fn css_modules_infer_output_file() -> Result<(), Box<dyn std::error::Error>> {
  let (input, _, exports) = css_module_test_vals();
  let infile = assert_fs::NamedTempFile::new("test.css")?;
  let outfile = assert_fs::NamedTempFile::new("out.css")?;
  infile.write_str(&input)?;
  let mut cmd = Command::cargo_bin("parcel_css")?;
  cmd.current_dir(infile.path().parent().unwrap());
  cmd.arg(infile.path());
  cmd.arg("--css-modules");
  cmd.arg("-o").arg(outfile.path());
  cmd.assert().success();

  let expected: serde_json::Value = serde_json::from_str(&exports)?;
  let actual: serde_json::Value =
    serde_json::from_str(&std::fs::read_to_string(outfile.path().with_extension("json"))?)?;
  assert_eq!(expected, actual);

  Ok(())
}

#[test]
fn css_modules_output_target_option() -> Result<(), Box<dyn std::error::Error>> {
  let (input, _, exports) = css_module_test_vals();
  let infile = assert_fs::NamedTempFile::new("test.css")?;
  let outfile = assert_fs::NamedTempFile::new("out.css")?;
  let modules_file = assert_fs::NamedTempFile::new("module.json")?;
  infile.write_str(&input)?;
  let mut cmd = Command::cargo_bin("parcel_css")?;
  cmd.current_dir(infile.path().parent().unwrap());
  cmd.arg(infile.path());
  cmd.arg("-o").arg(outfile.path());
  cmd.arg("--css-modules");
  cmd.arg("--css-modules-output-file").arg(modules_file.path());
  cmd.assert().success();

  let expected: serde_json::Value = serde_json::from_str(&exports)?;
  let actual: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(modules_file.path())?)?;
  assert_eq!(expected, actual);

  Ok(())
}

#[test]
fn sourcemap() -> Result<(), Box<dyn std::error::Error>> {
  let (input, _, _) = css_module_test_vals();
  let infile = assert_fs::NamedTempFile::new("test.css")?;
  let outdir = assert_fs::TempDir::new()?;
  let outfile = outdir.child("out.css");
  infile.write_str(&input)?;
  let mut cmd = Command::cargo_bin("parcel_css")?;
  cmd.current_dir(infile.path().parent().unwrap());
  cmd.arg(infile.path());
  cmd.arg("-o").arg(outfile.path());
  cmd.arg("--sourcemap");
  cmd.assert().success();

  outfile.assert(predicate::str::contains(&format!(
    "/*# sourceMappingURL={}.map */",
    outfile.path().to_str().unwrap()
  )));
  let mapfile = outdir.child("out.css.map");
  mapfile.assert(predicate::str::contains(r#""version":3"#));
  mapfile.assert(predicate::str::contains(r#""sources":["test.css"]"#));
  mapfile.assert(predicate::str::contains(
    r#""mappings":"AACM;;;;AAIA;;;;AAIA;;;;;;;;;;AAKA;;;;AAIA;;;;AAIA""#,
  ));

  Ok(())
}
