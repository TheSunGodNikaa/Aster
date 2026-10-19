# ASTER Billing

ASTER is a Windows desktop billing and inventory application for medical wholesale. It is being built in Rust with eframe/egui and SQLite.

## Development status

The project is implementing the foundation and launch flow. Startup opens a compact financial-year/date chooser, then either first-run administrator setup or login. After successful login the workspace maximizes. The first-run password is stored as an Argon2 hash; ASTER does not create a default account or password.

## Build and run

Install the Rust stable toolchain and the Windows build tools required by eframe, then run from the `aster` directory:

```powershell
cargo run
```

Check formatting with:

```powershell
cargo fmt --check
```

The project uses the bundled SQLite library through `rusqlite`. `cargo fmt --check` and `cargo check` pass after the current UI refactor. The app has not been relaunched since these UI changes; Cargo still prints a non-fatal path canonicalization warning in this environment.

## Application data

The SQLite database is created at `%LOCALAPPDATA%\Aster\aster.sqlite3`. Financial-year metadata and user credentials are stored there; business records remain empty. Startup logs are appended under `%LOCALAPPDATA%\Aster\logs\aster.log`.

## Legacy references

Files in `resource/` are reference material only. The old Access databases are not imported, copied, or migrated; ASTER will start with a new SQLite database and empty business data.
