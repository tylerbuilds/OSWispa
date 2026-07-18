# Privacy

OSWispa is local-first. With the default local backend, microphone audio and transcripts stay on the computer running OSWispa.

## Data stored locally

- Microphone audio is written to an owner-only temporary WAV file, processed, and deleted after transcription. Incomplete recordings are deleted automatically.
- Clipboard history is stored in the OSWispa data directory. Configuration, history, and stored API keys use owner-only permissions on Unix systems.
- Whisper models are stored locally and are not uploaded by OSWispa.

## Optional remote transcription

Remote mode is opt-in. When enabled, OSWispa sends the recorded audio, selected language, model name, and optional API credential to the endpoint configured by the user. The privacy and retention policy of that endpoint then applies. HTTPS is required unless the user explicitly enables insecure HTTP.

## Desktop permissions

- Microphone access is required to record speech.
- macOS Accessibility access is required for global hotkeys and text insertion.
- Linux global hotkeys currently require membership in the `input` group, which grants broad access to input devices for that user session.

## Project website

The project website does not load analytics or remote font services. The contact form is processed by FormSubmit and delivered by email. Do not submit secrets, API keys, private transcripts, or recorded audio through the contact form.

Questions can be sent to [tc@tylerbuilds.com](mailto:tc@tylerbuilds.com).
