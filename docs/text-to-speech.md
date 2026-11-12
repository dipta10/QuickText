# Text-to-speech

Open the **Text to speech** tab, enter or paste text, and choose **Read aloud**.
Select part of the editor text to read just that passage. Ctrl+Enter (Cmd+Enter
on macOS) starts reading from the editor. The playback button stops reading
immediately, including while preparing audio. Closing the window leaves reading
running in the tray. Text drafts and generated audio are not saved to disk.

Settings → Text-to-speech contains the shared Soniox key, a voice picker,
language, playback speed (0.5–2×), and two independent shortcut assignments:

- **Read selection / stop** reads the current selection in the focused application; pressing it again stops reading.
- **Stop reading** stops an active reading without starting one.

The Voice dropdown loads the voices supported by the current Soniox speech
model using your saved key. Each option includes its gender, and the selected
voice's description appears below the picker. Choose a voice and save voice
preferences to use it on your next reading, including readings started through
a shortcut or the CLI. Refresh voices retries a failed request or updates the
list. Your saved choice remains available when offline. The expandable custom
voice ID field supports cloned voices from the Soniox Console. The catalog
request stays in the backend; API keys never reach the voice picker.

Soniox's key needs text-to-speech permission. The initial voice is Adrian,
language is English, and speed is 1×. Voice and language apply to the next reading.
The Speed control beside Play changes the current playback immediately, preserves
voice pitch, and saves automatically for future readings, including shortcut
readings. Speed changes do not restart synthesis or resend the text. Settings
also provides a speed slider that applies when saved.
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
A new reading picks up a changed default output. A small output buffer lets speed
changes take effect promptly; Soniox generates at its natural speed and native
playback adjusts tempo without shifting pitch. There is no microphone capture
or audio export in this flow.

## Validation

Automated checks cover provider streaming before completion, malformed and
truncated responses, timeouts, audio resampling, Unicode limits, settings
round-trips, shortcut restoration/conflicts/clearing, stale frontend events,
stop during preparation, credential setup, and preservation of drafts on errors.
Live Soniox synthesis and installed-app selection/playback still need checks on
each operating system using a key with text-to-speech permission.

See [ADR 0022](adrs/0022-streaming-text-to-speech.md) for the implementation boundary.
