use anyhow::{bail, Context, Result};
use std::{
    env, fs,
    process::{Command, Output},
};

const GROTH16_PROVER_IMAGE: &str = "risczero/risc0-groth16-prover:v2025-04-03.1";

pub fn prepare() -> Result<()> {
    ensure_docker_cli()?;
    ensure_docker_arch()?;
    ensure_work_dir()?;
    ensure_image_present()?;
    ensure_image_runtime()?;
    Ok(())
}

pub fn image_name() -> &'static str {
    GROTH16_PROVER_IMAGE
}

fn ensure_docker_cli() -> Result<()> {
    let output = docker_output(&["version"])?;
    if output.status.success() {
        return Ok(());
    }

    bail!(
        "Docker is not ready.\n{}",
        format_output("docker version", &output)
    );
}

fn ensure_docker_arch() -> Result<()> {
    let output = docker_output(&["version", "--format", "{{.Server.Os}}/{{.Server.Arch}}"])?;
    if !output.status.success() {
        bail!(
            "Cannot read Docker server architecture.\n{}",
            format_output(
                "docker version --format {{.Server.Os}}/{{.Server.Arch}}",
                &output
            )
        );
    }

    let arch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if arch == "linux/amd64" {
        return Ok(());
    }

    bail!("RISC Zero Groth16 Docker prover requires linux/amd64, but Docker server is {arch}");
}

fn ensure_work_dir() -> Result<()> {
    if env::var_os("RISC0_WORK_DIR").is_some() {
        return Ok(());
    }

    let work_dir = env::current_dir()
        .context("Cannot read current directory for RISC0_WORK_DIR")?
        .join("target")
        .join("risc0-groth16-work");
    fs::create_dir_all(&work_dir).with_context(|| {
        format!(
            "Cannot create RISC0_WORK_DIR at {}",
            work_dir.to_string_lossy()
        )
    })?;
    env::set_var("RISC0_WORK_DIR", &work_dir);
    Ok(())
}

fn ensure_image_present() -> Result<()> {
    let inspect = docker_output(&["image", "inspect", GROTH16_PROVER_IMAGE])?;
    if inspect.status.success() {
        return Ok(());
    }

    pull_image()
}

fn ensure_image_runtime() -> Result<()> {
    match validate_image_runtime() {
        Ok(()) => Ok(()),
        Err(first_err) => {
            pull_image().with_context(|| {
                format!(
                    "Groth16 prover image failed validation, then docker pull also failed. \
                     First validation error:\n{first_err}"
                )
            })?;

            validate_image_runtime().with_context(|| {
                format!(
                    "Groth16 prover image is present but does not expose the expected runtime. \
                     Image: {GROTH16_PROVER_IMAGE}"
                )
            })
        }
    }
}

fn validate_image_runtime() -> Result<()> {
    let output = docker_output(&[
        "run",
        "--rm",
        "--entrypoint",
        "/bin/sh",
        GROTH16_PROVER_IMAGE,
        "-c",
        "test -x /app/prover.sh && command -v bash && command -v stark_verify && command -v prover",
    ])?;

    if output.status.success() {
        return Ok(());
    }

    bail!(
        "Docker Groth16 prover image validation failed.\n{}",
        format_output(
            "docker run --entrypoint /bin/sh <groth16-image> ...",
            &output
        )
    );
}

fn pull_image() -> Result<()> {
    let output = docker_output(&["pull", GROTH16_PROVER_IMAGE])?;
    if output.status.success() {
        return Ok(());
    }

    if should_retry_with_clean_docker_config(&output) {
        install_clean_docker_config()?;
        let retry = docker_output(&["pull", GROTH16_PROVER_IMAGE])?;
        if retry.status.success() {
            return Ok(());
        }

        bail!(
            "Cannot pull Groth16 prover image after retrying with a clean DOCKER_CONFIG.\n{}",
            format_output(&format!("docker pull {GROTH16_PROVER_IMAGE}"), &retry)
        );
    }

    bail!(
        "Cannot pull Groth16 prover image.\n{}",
        format_output(&format!("docker pull {GROTH16_PROVER_IMAGE}"), &output)
    );
}

fn should_retry_with_clean_docker_config(output: &Output) -> bool {
    if env::var_os("DOCKER_CONFIG").is_some() {
        return false;
    }

    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
    .to_lowercase();

    text.contains("docker-credential") || text.contains("error getting credentials")
}

fn install_clean_docker_config() -> Result<()> {
    let dir = env::current_dir()
        .context("Cannot read current directory for Docker config fallback")?
        .join("target")
        .join("risc0-docker-config");
    fs::create_dir_all(&dir).with_context(|| {
        format!(
            "Cannot create Docker config fallback at {}",
            dir.to_string_lossy()
        )
    })?;

    let config_path = dir.join("config.json");
    if !config_path.exists() {
        fs::write(&config_path, "{}\n").with_context(|| {
            format!(
                "Cannot write Docker config fallback at {}",
                config_path.to_string_lossy()
            )
        })?;
    }

    env::set_var("DOCKER_CONFIG", &dir);
    Ok(())
}

fn docker_output(args: &[&str]) -> Result<Output> {
    Command::new("docker")
        .args(args)
        .output()
        .context("Cannot start docker CLI")
}

fn format_output(command: &str, output: &Output) -> String {
    let stdout = trim_output(&output.stdout);
    let stderr = trim_output(&output.stderr);

    format!(
        "command: {command}\nexit: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        if stdout.is_empty() {
            "<empty>"
        } else {
            &stdout
        },
        if stderr.is_empty() {
            "<empty>"
        } else {
            &stderr
        },
    )
}

fn trim_output(bytes: &[u8]) -> String {
    const MAX_LEN: usize = 4096;

    let text = String::from_utf8_lossy(bytes).trim().to_string();
    if text.len() <= MAX_LEN {
        return text;
    }

    let truncated: String = text.chars().take(MAX_LEN).collect();
    format!("{truncated}...\n<truncated>")
}
