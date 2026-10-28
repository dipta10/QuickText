# ADR 0021: Persist Shortcuts In Native Settings

## Status

Accepted and implemented.

## Context

A Windows user reports that a window-visibility shortcut works for the current
session but disappears after quitting and restarting QuickText. Previously the
backend retained registered shortcuts only in memory. The frontend saved them
to webview `localStorage` and re-registered them during frontend initialization.
There was no native settings file or backend startup restoration for either
shortcut. Frontend tests that reused the same storage object across module
reloads did not test persistence across native process exit or webview profile
loss. The precise cause of the reported WebView2 storage loss has not been
reproduced on Windows in the Linux development environment.

## Decision

- Store both independent shortcut assignments in `shortcuts.json` under Tauri's
  application configuration directory, using the existing atomic settings writer.
- Register a replacement before removing the old binding. Persist before
  reporting success; roll back registration if saving fails.
- Restore native assignments in backend setup, including hidden autostart.
  Keep registration failures separate from persistence: retain the saved
  assignment, show its error, and retry registration on the next launch.
- Import legacy webview assignments once for each missing native slot. An empty
  native string represents an explicitly unassigned slot and takes precedence
  over stale legacy values. Do not overwrite an unreadable native settings file.
- Load the resulting assignments and per-binding errors through an allowed
  Tauri command. Disable editing until initialization succeeds so a late startup
  response cannot overwrite an interactive change.
- Test persistence through separate native test processes and run the shortcut
  and atomic file replacement tests on Windows CI.

## Consequences

Shortcut persistence no longer depends on the webview profile, origin, storage
flush timing, or frontend restoration. Existing assignments migrate if they are
still available; assignments already lost by an old build must be entered once
in the new build. The backend remains responsible for both registration and
persistence, while the UI displays the saved choice even when the OS rejects it.

## Windows Manual Verification

1. Build and install this branch on Windows. Set different recording and
   window-visibility shortcuts and check that both work.
2. Quit through the tray menu, rather than only closing the window. Launch the
   installed app again. Check both labels in Settings and trigger show/hide
   while another app is focused.
3. Change the window shortcut, quit, and relaunch. Verify that only the new key
   toggles visibility and the recording shortcut remains unchanged.
4. Clear the window shortcut, quit, and relaunch. It must remain unassigned,
   including when an older webview assignment still exists.
5. Enable launch on startup and sign out/in. Verify the window shortcut works
   from the initial hidden state.
6. If another app claims the saved key before launch, verify Settings retains
   the assignment and shows its registration error. Release the conflicting
   key and restart QuickText to verify that restoration retries.

Automated restart tests use real files and fresh native processes with an
in-memory OS registration adapter. They verify durable storage and startup
registration requests; they do not replace the installed-app checks above.
