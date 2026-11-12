# Text-to-speech

Open the **Text to speech** tab, enter or paste text, and choose **Read aloud**.
Select part of the editor text to read just that passage. Ctrl+Enter (Cmd+Enter
on macOS) starts reading from the editor. The playback button stops reading
immediately, including while preparing audio. Closing the window leaves reading
running in the tray. Text drafts and generated audio are not saved to disk.

Settings → Text-to-speech contains the shared Soniox key, voice name or cloned
voice ID, language, speed (0.7–1.3×), and two independent shortcut assignments:

- **Read selection / stop** reads the current selection in the focused application; pressing it again stops reading.
- **Stop reading** stops an active reading without starting one.

Soniox's key needs text-to-speech permission. The initial voice is Adrian,
language is English, and speed is 1×. Preferences apply to the next reading.
Changing the dictation provider does not change the speech synthesis service.
Speech-to-text language hints and reading language are independent preferences.

A reading accepts up to 5,000 characters and Soniox generates up to two minutes
of audio. Split longer passages into shorter readings. A provider limit,
connection failure, or disconnected output produces a recovery message and
preserves the text draft. Starting dictation stops playback first; finish a
recording before starting a reading.

On Linux, install `wl-clipboard` for Wayland selection capture or `xclip` for X11.
The selection must be exposed through the primary selection protocol. Apps that
do not support it can use Paste in QuickText. Wayland global keys are configured
in the compositor, as with recording:

```text
quicktext read-selection
quicktext stop-speech
```

For example, Hyprland bindings can execute those commands. They toggle or stop
the same backend reading session used by the UI and never raise the window.
On macOS, selection capture may require Accessibility permission. macOS and
Windows temporarily use Copy and restore plain text clipboard contents; rich
clipboard formats cannot be preserved by this capture fallback. If the clipboard
is not text, copy the desired passage and paste it into QuickText instead.

Native playback uses the system default audio output chosen at reading start.
A new reading picks up a changed default output. There is no microphone capture
or audio export in this flow.

## Validation

Automated checks cover provider streaming before completion, malformed and
truncated responses, timeouts, audio resampling, Unicode limits, settings
round-trips, shortcut restoration/conflicts/clearing, stale frontend events,
stop during preparation, credential setup, and preservation of drafts on errors.
Live Soniox synthesis and installed-app selection/playback still need checks on
each operating system using a key with text-to-speech permission.

See [ADR 0022](adrs/0022-streaming-text-to-speech.md) for the implementation boundary.
