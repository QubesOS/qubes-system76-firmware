use sha2::{Digest, Sha256};
use std::ffi::OsStr;
use std::io::{Read, Write};
use std::{fs, io, path, process};

pub fn get_efi_mnt() -> Option<String> {
    let bootctl_esp = process::Command::new("bootctl")
        .args(["--print-esp-path"])
        .output()
        .ok()
        .filter(|x| x.status.success())
        .and_then(|x| String::from_utf8(x.stdout).ok())
        .and_then(|x| x.lines().next().map(String::from));

    bootctl_esp.or_else(|| {
        let efi = path::Path::new("EFI");
        [
            path::Path::new("/boot"),
            path::Path::new("/boot/efi"),
            path::Path::new("/efi"),
        ]
        .iter()
        .find(|x| x.join(efi).as_path().is_dir())
        .and_then(|x| x.to_str().map(String::from))
    })
}

pub fn extract<P: AsRef<path::Path>>(data: &[u8], p: P) -> io::Result<()> {
    let mut extract_cli = process::Command::new("tar")
        .args(["-xJv"])
        .stdin(process::Stdio::piped())
        .current_dir(p)
        .spawn()?;
    let mut stdin = extract_cli.stdin.take().expect("stdlib bug");
    stdin.write_all(data)?;
    let output = extract_cli.wait()?;
    if output.success() {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "tar failed"))
    }
}

pub fn extract_file<P: AsRef<path::Path>>(data: &[u8], path: P) -> io::Result<String> {
    let mut extract_cli = process::Command::new("tar")
        .args([
            OsStr::new("-xJvO"),
            OsStr::new("--"),
            path.as_ref().as_os_str(),
        ])
        .stdin(process::Stdio::piped())
        .stdout(process::Stdio::piped())
        .spawn()?;
    let mut stdin = extract_cli.stdin.take().expect("stdlib bug");
    let mut stdout = extract_cli.stdout.take().expect("stdlib bug");
    let output = std::thread::scope(|scope| {
        let res = scope.spawn(move || stdin.write_all(data));

        let mut output = String::new();
        stdout.read_to_string(&mut output)?;
        res.join().expect("thread panicked")?;
        Ok::<_, std::io::Error>(output)
    })?;
    let status = extract_cli.wait()?;
    if status.success() {
        Ok(output)
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "tar failed"))
    }
}

pub fn read_string<P: AsRef<path::Path>>(p: P) -> io::Result<String> {
    let mut string = String::new();
    {
        let mut file = fs::File::open(p)?;
        file.read_to_string(&mut string)?;
    }
    Ok(string)
}

pub fn sha256(input: &[u8]) -> String {
    format!("{:x}", Sha256::digest(input))
}

pub fn retry<T, E>(
    action: impl Fn() -> Result<T, E>,
    cleanup: impl Fn() -> Result<(), E>,
) -> Result<T, E> {
    let mut tries = 0;

    loop {
        let result = action();

        if result.is_err() {
            cleanup()?;

            if tries < 3 {
                tries += 1;
                continue;
            }
        }

        return result;
    }
}
