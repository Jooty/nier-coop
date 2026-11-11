use std::fs;
use std::io;
use std::path::Path;

pub fn write_atomically(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut temporary_name = path.as_os_str().to_owned();
    temporary_name.push(".tmp");
    let temporary = Path::new(&temporary_name);
    fs::write(temporary, bytes)?;
    fs::rename(temporary, path).inspect_err(|_| {
        let _ = fs::remove_file(temporary);
    })
}
