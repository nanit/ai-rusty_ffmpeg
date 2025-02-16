use std::{
    env, fs, path::{Path, PathBuf}, process::Command, sync::LazyLock
};

const POSSIBLE_BUILD_COMPONENTS: &[&str] = &["x264"];
static DEFAULT_BUILD_COMPONENTS: LazyLock<Vec<String>> = LazyLock::new(|| vec!["x264".into()]);

fn get_build_components() -> Result<Vec<String>> {
    let env_components: Vec<String> = match env::var("AI_FFMPEG_BUILD_COMPONENTS") {
        Ok(s) => s.split(",").map(String::from).collect(),
        Err(_) => return Ok(DEFAULT_BUILD_COMPONENTS.clone()),
    };

    for comp in env_components.iter() {
        if !POSSIBLE_BUILD_COMPONENTS.contains(&comp.as_str()) {
            return Err(format!("component list unknown component {comp}").into());
        }
    }

    Ok(env_components)
}

fn run_build(out_dir: &Path) -> Result<()> {
    let source_dir = out_dir.join("source");
    fs::create_dir(&source_dir)?;

    let build_dir = out_dir.join("build");
    fs::create_dir(&build_dir)?;

    let install_dir = out_dir.join("install");
    fs::create_dir(&install_dir)?;

    let build_components = get_build_components()?;
    for bc in build_components {
        if bc == "x264" {
            build_x264(&source_dir, &build_dir)?;
        }
    }

    build_ffmpeg(&source_dir, &build_dir, &install_dir)?;

    fs::remove_dir_all(&source_dir)?;
    fs::remove_dir_all(&build_dir)?;

    Ok(())
}

fn build_ffmpeg(source_dir: &Path, build_dir: &Path, install_dir: &Path) -> Result<()> {
    let ffmpeg_version = ffmpeg_version()?;

    let archive_path = source_dir.join("ffmpeg-snapshot.tar.bz2");

    if !Command::new("wget").args(
        [
            "-O",
            archive_path.to_str().unwrap(),
            format!("https://ffmpeg.org/releases/ffmpeg-{ffmpeg_version}.tar.bz2").as_str()
        ]
    ).status()?.success() {
        return Err("error downloading ffmpeg sources".into());
    }

    if !Command::new("tar").arg("xjvf").arg(&archive_path)
    .current_dir(source_dir.to_str().unwrap()).status()?.success() {
        return Err("error untarring sources".into())
    }

    let ffmpeg_source_dir = source_dir.join(format!("ffmpeg-{ffmpeg_version}"));

    if !Command::new("./configure").current_dir(&ffmpeg_source_dir).env(
        "PKG_CONFIG_PATH",
        build_dir.join("lib").join("pkgconfig").to_str().unwrap(),
    ).args([
        format!("--prefix={}", install_dir.to_str().unwrap()),
        format!("--extra-cflags=-I{}", build_dir.join("include").to_str().unwrap()),
        format!("--extra-ldflags=-L{}", build_dir.join("lib").to_str().unwrap())
    ]).args([
        "--disable-shared",
        "--enable-static",
        "--extra-libs=-lpthread",
        "--extra-libs=-lm",
        "--ld=g++",
        "--disable-libxcb",
        "--disable-securetransport",
        "--disable-debug",
        "--disable-programs",
        "--disable-doc",
        "--disable-bsfs",
        "--disable-indevs",
        "--disable-outdevs",
        "--disable-devices",
        "--disable-hwaccels",
        "--enable-hwaccel=h264_videotoolbox",
        "--enable-decoder=h264",
        "--disable-encoders",
        "--enable-gpl",
        "--enable-libx264",
        "--enable-pic",
    ]).status()?.success() {
        return Err("error running ./configure for ffmpeg".into())
    }

    if !Command::new("make").args(["-j", "10"]).current_dir(&ffmpeg_source_dir).status()?.success() {
        return Err("error running make for ffmpeg".into())
    }

    if !Command::new("make").arg("install").current_dir(&ffmpeg_source_dir).status()?.success() {
        return Err("error running make install for ffmpeg".into())
    }

    Ok(())
}

fn build_x264(source_dir: &Path, build_dir: &Path) -> Result<()> {
    let source_dir = source_dir.join("x264");
    fs::create_dir(&source_dir)?;

    let status = Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            "https://code.videolan.org/videolan/x264.git",
            source_dir.to_str().unwrap(),
        ])
        .status()?;
    if !status.success() {
        return Err("error cloning x264 repo".into());
    }

    if !Command::new("./configure")
        .current_dir(&source_dir)
        .env(
            "PKG_CONFIG_PATH",
            build_dir.join("lib").join("pkgconfig").to_str().unwrap(),
        )
        .args([
            format!("--prefix={}", build_dir.to_str().unwrap()).as_str(),
            "--disable-shared",
            "--enable-static",
            "--enable-pic",
        ])
        .status()?
        .success()
    {
        return Err("error configuring x264 library".into());
    }

    if !Command::new("make").current_dir(&source_dir).status()?.success() {
        return Err("error running make for x264 library".into());
    }

    if !Command::new("make").arg("install").current_dir(&source_dir).status()?.success() {
        return Err("error running make install for x264 library".into());
    }

    Ok(())
}

fn ffmpeg_version() -> Result<String> {
    let version = env::var("CARGO_PKG_VERSION").expect("missing CARGO_PKG_VERSION env variable");
    version
        .split("ffmpeg")
        .last().map(String::from).ok_or(format!("missing ffmpeg version from package version: {version}").into())
}

type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

fn main() -> Result<()> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?).join("ffmpeg");

    if !out_dir.exists() {
        fs::create_dir(&out_dir)?;
        run_build(&out_dir)?;
    }

    println!(
        "cargo::rustc-env=FFMPEG_PKG_CONFIG_PATH={}",
        out_dir
            .join("install")
            .join("lib")
            .join("pkgconfig")
            .to_str()
            .unwrap()
    );

    Ok(())
}
