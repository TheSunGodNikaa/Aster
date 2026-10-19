# Development progress

## Phase 1 — Foundation and launch flow

- [x] Inspect the existing project and reference assets.
- [x] Verify Rust and Cargo. Windows GNU Rust is installed; the required `gcc.exe` C compiler is missing.
- [x] Establish the eframe application window and ASTER title.
- [x] Apply a white visual theme and use the requested `resource/image (4).jpg` as an embedded app background.
- [x] Add a compact financial-year and working-date selection window using the April 1 boundary.
- [x] Add a compact first-run administrator setup and login window.
- [x] Store account passwords as Argon2 hashes, with no hard-coded credentials.
- [x] Open the main workspace maximized after successful login.
- [x] Add a new per-user SQLite database and initial versioned schema.
- [x] Add startup logging under the per-user Local AppData directory.
- [x] Add build and development notes.
- [x] Run `cargo fmt --check`.
- [x] Run `cargo fmt --check` and `cargo check` after splitting the UI into modules.
- [ ] Launch the app and visually review the updated compact screens and maximized workspace.

**Acceptance:** The code formats and type-checks. Visual launch verification is still pending.

## Source layout

- `aster/src/main.rs` — application entry point and startup window options.
- `aster/src/app.rs` — app state, launch stages, and native window transitions.
- `aster/src/screens/` — financial-year/date selection, administrator/login, and workspace UI.
- `aster/src/database.rs` — existing SQLite initialization and user/year access.
- `aster/src/date.rs`, `theme.rs`, and `logging.rs` — shared helpers.

## Next phases

- Phase 2: Extend the database schema and add repositories/services for business records.
- Phase 3: Replace the starter workspace navigation with working tabs and status messages.
- Phase 4: Add account management, password changes, and role/permission checks.
