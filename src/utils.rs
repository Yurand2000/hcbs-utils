/// Execute the given shell command
pub fn __shell(cmd: &str) -> anyhow::Result<std::process::Output> {
    use std::process::Command;

    Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .map_err(|err| anyhow::format_err!("Error in executing \"sh -c {cmd}\": {err}").into())
}

pub fn __read_file<P>(file: P) -> anyhow::Result<String>
    where P: AsRef<std::path::Path> + std::fmt::Display
{
    std::fs::read_to_string(file.as_ref())
        .map_err(|err| anyhow::format_err!("Error in reading file {file}: {err}"))
}

pub fn __read_file_parse<P, F, U, E>(file: P, mut parse: F) -> anyhow::Result<U>
    where
        P: AsRef<std::path::Path> + std::fmt::Display,
        F: FnMut(String) -> Result<U, E>,
        E: std::fmt::Display
{
    std::fs::read_to_string(file.as_ref())
        .map_err(|err| anyhow::format_err!("Error in reading file {file}: {err}"))
        .and_then(|data| parse(data).map_err(|err| anyhow::format_err!("Error in parsing file {file}: {err}")))
}

pub fn __write_file<P, C>(file: P, data: C) -> anyhow::Result<()>
    where P: AsRef<std::path::Path> + std::fmt::Display, C: AsRef<[u8]>
{
    std::fs::write(file.as_ref(), data)
        .map_err(|err| anyhow::format_err!("Error in writing file {file}: {err}"))
}