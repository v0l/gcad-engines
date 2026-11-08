use gcad_bevy::{Centre, GcadSettings, bake};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage: gcad-bevy-bake [--check] [--height H | --scale K] [--centre no|yes|unrigged] [--detail N] [--set NAME=VALUE]... ASSET_DIR
Builds every .gcad and .gasm under ASSET_DIR into ASSET_DIR/gcad-baked/<key>.bin and index.json.
--check builds nothing and fails when a model has no baked entry or a stale one.";

fn parse(args: &[String]) -> Result<(bool, GcadSettings, PathBuf), String> {
    let mut check = false;
    let mut settings = GcadSettings::default();
    let mut dir = None;
    let mut args = args.iter();
    let number = |v: Option<&String>, flag: &str| -> Result<f64, String> {
        v.and_then(|v| v.parse().ok()).ok_or(format!("{flag} takes a number"))
    };
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => check = true,
            "--height" => settings.height = Some(number(args.next(), arg)?),
            "--scale" => settings.scale = Some(number(args.next(), arg)?),
            "--detail" => settings.detail = number(args.next(), arg)?,
            "--centre" => {
                settings.centre = match args.next().map(String::as_str) {
                    Some("no") => Centre::No,
                    Some("yes") => Centre::Yes,
                    Some("unrigged") => Centre::Unrigged,
                    _ => return Err("--centre takes no, yes or unrigged".into()),
                }
            }
            "--set" => {
                let pair = args.next().ok_or("--set takes NAME=VALUE")?;
                let (name, value) = pair.split_once('=').ok_or("--set takes NAME=VALUE")?;
                settings = settings.set(name, value.parse().map_err(|_| "--set takes NAME=VALUE")?);
            }
            "-h" | "--help" => return Err(String::new()),
            flag if flag.starts_with("--") => return Err(format!("unknown option {flag}")),
            _ if dir.is_none() => dir = Some(PathBuf::from(arg)),
            _ => return Err("one asset folder".into()),
        }
    }
    Ok((check, settings, dir.ok_or("needs an asset folder")?))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (check, settings, dir) = match parse(&args) {
        Ok(parsed) => parsed,
        Err(problem) => {
            if !problem.is_empty() {
                eprintln!("{problem}");
            }
            eprintln!("{USAGE}");
            return if problem.is_empty() { ExitCode::SUCCESS } else { ExitCode::FAILURE };
        }
    };
    let settings_for = |_: &str| Some(settings.clone());
    if check {
        let problems = bake::check(&dir, settings_for);
        for problem in &problems {
            eprintln!("{problem}");
        }
        return if problems.is_empty() { ExitCode::SUCCESS } else { ExitCode::FAILURE };
    }
    match bake::bake(&dir, settings_for) {
        Ok(baked) => {
            eprintln!(
                "baked {} model(s) into {}: {} built, {} unchanged, {} old removed, {} bytes",
                baked.index.entries.len(),
                bake::baked_dir(&dir).display(),
                baked.built.len(),
                baked.reused.len(),
                baked.removed,
                baked.bytes
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
