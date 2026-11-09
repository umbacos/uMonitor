# Build deliverables

The user requires a portable installation ZIP on every completed build, including
the entire `config/` directory and all examples. Always run `powershell -NoProfile
-File .\build.ps1` from this directory after the final source or documentation
changes (add `-Offline` when dependencies are cached). This builds the release and
refreshes `uMonitor-<version>-windows-x64-portable.zip` and its `.sha256` in the repository-root `dist/` (or local `dist/` for extracted source), together with the Windows MSI and source ZIP. A direct `cargo build` or Craft `rebuild_check` alone does not finish the task.

Run `python scripts/verify-portable.py ../dist/uMonitor-<version>-windows-x64-portable.zip`
to check the package, including every source config file. Run Craft
`rebuild_check` before reporting completion, and fix any failures. Preserve existing
workspace changes and personal session files.
