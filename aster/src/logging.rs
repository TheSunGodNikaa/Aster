use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

pub(crate) fn log_message(level: &str, message: &str) {
    let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") else {
        eprintln!("{level}: {message}");
        return;
    };
    let log_path = PathBuf::from(local_app_data)
        .join("Aster")
        .join("logs")
        .join("aster.log");
    if let Some(parent) = log_path.parent() {
        if let Err(error) = fs::create_dir_all(parent) {
            eprintln!("Could not create ASTER log directory: {error}");
            return;
        }
    }
    match OpenOptions::new().create(true).append(true).open(log_path) {
        Ok(mut file) => {
            if let Err(error) = writeln!(file, "{level}: {message}") {
                eprintln!("Could not write ASTER log: {error}");
            }
        }
        Err(error) => eprintln!("Could not open ASTER log: {error}"),
    }
}
