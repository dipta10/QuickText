# QuickText

QuickText turns short spoken dictation into text the speaker can review and use elsewhere. Its existing vocabulary is recorded in the [project glossary](docs/glossary.md); the terms below sharpen the distinction between transcription and cleanup.

## Language

**Transcription**:
Recognition of spoken words as text. It is distinct from editing the wording after recognition.

**Transcript cleanup**:
A concise rewrite of a finalized transcript that removes fillers and repetition and corrects grammar while preserving distinct ideas, intent, uncertainty, names, numbers, and language.
_Avoid_: Summary, translation, transcription.

**Original transcript**:
The finalized speech-recognition result before transcript cleanup.
_Avoid_: Raw audio, partial transcript.

**Cleaned transcript**:
An edited version of the original transcript produced by transcript cleanup.
_Avoid_: Correct transcript, new transcription.

**Clean & Copy**:
The speaker's explicit request to clean the transcript and copy the cleaned text to the clipboard.
_Avoid_: Auto-clean, paste.

**Restore original**:
The speaker's choice to display the original transcript again after viewing a cleaned version.
_Avoid_: Delete recording, retranscribe.

**Cleanup provider**:
The service that rewrites transcript text. It is distinct from the speech provider that recognizes audio.
