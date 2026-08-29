# ADR 0016: Use Native Linux Packages Alongside AppImage

## Status

Accepted for the Linux distribution-hardening slice.

## Context

QuickText's CD workflow currently builds Linux artifacts on Ubuntu 22.04 and publishes an AppImage, a Debian package, and an RPM package. The AppImage is intended to cover distributions without a matching native package.

That portability boundary is not reliable for a Tauri/WebKitGTK application. On a current Manjaro Hyprland system with Mesa 26 and Wayland 1.26, the Ubuntu-built AppImage loads an older bundled `libwayland-client` alongside host graphics libraries. WebKit's render process then aborts with `EGL_BAD_PARAMETER` and leaves a blank window. Tauri tracks the same default-bundler failure in [tauri-apps/tauri#15665](https://github.com/tauri-apps/tauri/issues/15665).

Launching with the host Wayland library preloaded is a useful recovery command for an already-published artifact, but it is not an acceptable installation contract. Library paths vary across distributions, environment flags are easy to lose in desktop launchers and autostart entries, and QuickText cannot repair libraries that were packaged incorrectly by changing capture or UI behavior.

## Decision

Treat this as a distribution problem, not an application-code problem.

- Keep the Debian package for Debian-family systems and the RPM package for RPM-family systems.
- Add an x86_64 Arch package (`.pkg.tar.zst`) to every CD release. Build it in a pinned Arch Linux environment so it links against and declares Arch's system GTK, WebKitGTK, Wayland, audio, tray, and input dependencies instead of carrying Ubuntu's desktop stack.
- Publish the Arch package directly on the same GitHub Release as the other installers. An AUR entry is deferred until QuickText has a stable versioned release process; rolling per-main releases and bounded retention make a continuously updated AUR checksum a poor first distribution channel.
- Keep the AppImage as a fallback for distributions without a native package, but describe it as experimental on newer Wayland/Mesa systems until it passes the compatibility checks below.
- Keep the current release-note recovery command for already-published and fallback AppImages. Once the Arch package exists, Arch and Manjaro users should be directed to that package first rather than to the environment-variable workaround.
- Do not hardcode `LD_PRELOAD`, distribution-specific library paths, or renderer overrides in the QuickText Rust binary. Such process-wide behavior would affect native packages too and would hide a packaging defect behind an incomplete runtime workaround.
- Track the upstream Tauri AppImage fix. Adopt a released bundler fix, rebuild the AppImage, and remove the experimental warning only after testing it on both an older glibc distribution and a current rolling Wayland distribution. Do not permanently fork or broadly delete AppImage libraries without equivalent compatibility coverage.

Linux release coverage is therefore:

| Artifact | Primary audience | Support position |
|---|---|---|
| `.deb` | Debian and Ubuntu family, x86_64. | Native package. |
| `.rpm` | Fedora and compatible RPM family, x86_64. | Native package. |
| `.pkg.tar.zst` | Arch, Manjaro, and compatible Arch family, x86_64. | Native package and preferred Wayland path. |
| `.AppImage` | Other x86_64 Linux distributions. | Compatibility fallback; experimental on newer Wayland/Mesa until gated. |

Linux ARM64/aarch64 remains unsupported by this decision. Adding it requires a separate native build and test matrix rather than renaming an x86_64 artifact.

## Compatibility Checks

An artifact existing is not sufficient evidence of support. Before the Arch package is advertised:

- Install the produced package with pacman in a clean Arch environment and verify its declared runtime dependencies resolve.
- Verify package contents and ownership for the QuickText binary, desktop entry, and icons; the package must not own user configuration, credentials, logs, or transcript data.
- Launch the installed package on a current Manjaro or Arch Hyprland session without `LD_PRELOAD` or WebKit renderer overrides.
- Exercise window rendering, tray residency, close-to-hide, autostart-hidden launch, microphone enumeration, one recording, transcription finalization, clipboard copy, and companion CLI access.
- Confirm the package reports the same release `build_id` and `source_revision` as the other artifacts from that CD run.

The AppImage warning can be removed only after the unmodified artifact launches and renders on:

- The oldest supported glibc baseline represented by the Linux build strategy.
- A current Arch-family Wayland system with current Mesa.
- A current Fedora-family Wayland system.

## Consequences

- Arch-family users receive a normal package that follows their system desktop stack instead of a shell command they must remember.
- CD gains another Linux build environment and packaging definition that must stay aligned with the app version, release tag, icons, desktop entry, runtime dependencies, `build_id`, and source revision.
- QuickText does not attempt to produce a unique package for every distribution. Three native package families cover the main audiences, with AppImage retained as a clearly qualified fallback.
- The temporary AppImage note remains useful for old releases and unsupported distributions, but it is no longer presented as the proper Arch installation path.
- Upstream AppImage repair and native Arch distribution can progress independently; an upstream delay does not block dependable Arch testing.

## Rejected Alternatives

- **Release-note workaround only:** Rejected because a blank first launch is a broken artifact, and environment flags do not survive normal desktop and autostart usage reliably.
- **Hardcode the workaround in Rust:** Rejected because the failure originates in AppImage library composition, paths differ by distribution, and native packages should not inherit AppImage-specific rendering changes.
- **Build one AppImage on Arch for everyone:** Rejected because a rolling-distribution build raises the glibc baseline and reverses the compatibility problem for older distributions.
- **Publish only an AUR package:** Deferred because AUR metadata and checksums do not fit the current release-on-every-main-push channel cleanly. The GitHub-hosted pacman package provides a testable first step.

