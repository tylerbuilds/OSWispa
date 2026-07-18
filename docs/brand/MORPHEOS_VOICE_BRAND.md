# MorpheOS Voice brand source of truth

## Identity

| Field | Approved value |
|---|---|
| Product | **MorpheOS Voice** |
| Company | **MorpheOS** |
| Legacy product name | OS Whisper / OSWispa |
| Repository/package slug where safe | `morpheos-voice` |
| Canonical public URL | `https://morpheos.net/voice` |
| Primary headline | **Talk instead of type — in any app.** |
| Primary descriptor | **Free, open-source voice typing for your computer.** |
| Technical/category line | **The open voice layer for your computer.** |
| Short repository description | **Free, open-source voice typing for any app.** |

## Long description

MorpheOS Voice lets you speak naturally and put the resulting text wherever you are working. It is free, open source and designed to make voice typing understandable, dependable and straightforward to install. Local speech processing is the default; an optional remote, OpenAI-compatible endpoint can be configured by advanced users. Linux is the primary alpha experience. The current macOS and Windows packages provide the dictation loop with additional setup and signing limitations.

Do not shorten this in a way that removes the remote-processing or platform qualification when those details are material to the surface.

## Product role in the MorpheOS business

MorpheOS Voice is the genuinely free, open-source entry product for MorpheOS. It should demonstrate the business through usefulness, restraint, transparent engineering and trustworthy release evidence.

This role does **not** introduce:

- a trial period;
- a forced account;
- a freemium product name;
- locked core dictation features;
- artificial usage limits;
- a lead-capture gate before download;
- misleading prompts that imply the user must buy another MorpheOS product.

The public page may include one quiet route back to MorpheOS and its other products after the download and open-source information. The main journey remains download → first dictation → reliable daily use.

## Core message hierarchy

1. Talk instead of type.
2. Works wherever the user is already writing, subject to documented text-field and permission limits.
3. Hold a shortcut, speak and release.
4. Free and open source.
5. Clear control over local or remote processing.
6. Advanced model, provider and personalisation controls are available without leading the first-use experience.

## Central problem and promise

**User problem:** “I can express the thought faster by speaking than by typing it.”

**Product promise:** “Hold a key, speak naturally, and put useful text wherever your cursor is.”

Where a surface cannot guarantee insertion into every field, use:

> Hold a key, speak naturally, and put useful text into the app where you are working. If insertion cannot be confirmed, the text stays available on your clipboard.

## Audience priorities

1. People who regularly write prompts in ChatGPT, Claude, Codex or other AI tools.
2. Writers, marketers, founders, developers, researchers and consultants.
3. People who write many emails, messages, documents, prompts or notes.
4. People for whom typing creates friction, including users with RSI, dyslexia, ADHD, mobility limitations or temporary injuries.
5. Privacy-conscious users looking for an open-source dictation tool.

Accessibility users must be addressed respectfully. Do not position the product as software for people who are “bad at computers”, and do not make medical outcome claims.

## Product boundary

MorpheOS Voice is a short-form voice-typing product. It is not a meeting recorder, notes platform, chatbot, knowledge base or autonomous desktop agent.

The protected core loop is:

> Shortcut pressed → audio captured → transcription processed → text produced → text inserted or made recoverable → result clearly confirmed.

The current product implements **Write mode** only:

- **Write mode:** produces text and cannot execute an external action.
- **Act mode:** future direction only. It would pass an instruction to a connected tool or agent with preview, explicit approval and proof for consequential actions.

Do not publish “One key to write. One key to act.” until Act mode exists and passes its approval/proof gates.

## Approved calls to action

- Download MorpheOS Voice
- Get MorpheOS Voice
- View on GitHub
- See how it works
- Hold **Ctrl + Super** and speak — Linux default
- Hold **Ctrl + Windows** and speak — Windows default

Do not use “Get started free”. There is no paid trial to contrast with.

## Evidence-backed feature headings

- Speak in the app you already use
- Start with one shortcut
- Choose how speech is processed
- Keep control of your words
- Built in the open
- Recover text from the clipboard
- Teach it the spelling you choose

Do not present the current Tauri Settings or History previews as released features. Linux has a real settings/tray experience; macOS and Windows remain alpha packages with configuration-file and console/Terminal edges.

## Processing and privacy language

Approved labels:

- **Local — Processed on this computer.**
- **Remote — Sent to the endpoint you selected for processing.**

Use “local” and “private” only with a stated mode or verified boundary. Local mode keeps microphone audio and transcripts on the computer running MorpheOS Voice, apart from the separate model download. Remote mode sends recorded audio and request metadata to the configured endpoint; that provider's terms apply.

Never claim:

- everything is always processed locally;
- nothing ever leaves the device;
- MorpheOS Voice learns from corrections;
- recordings can never be lost;
- every text field accepts automated insertion;
- the product controls the whole computer;
- affiliation with OpenAI;
- Whisper is the product rather than a supported engine/model family.

## Tone and writing rules

Use British English. Write in a plain, practical, human, calm and confident voice. Prefer short concrete sentences. State limitations close to the claim they qualify.

Avoid corporate AI language and unsupported superlatives. In particular, do not use “revolutionary”, “magical”, “game-changing”, “effortless”, “groundbreaking”, “seamless”, “supercharge”, “transformative” or “unleash” unless a specific evidenced context requires the word.

Do not promise “instant” transcription. Processing time depends on the model, recording length and hardware.

## Public message examples

### One sentence

MorpheOS Voice is a free, open-source push-to-talk dictation app for Linux, macOS and Windows, with local processing by default and an optional remote endpoint for advanced users.

### Short page introduction

Hold a shortcut, speak and release. MorpheOS Voice turns the recording into text, copies it safely and inserts it into the app where you are working when the operating system allows it.

### Open-source statement

The original MorpheOS Voice source code is MIT-licensed. Third-party dependencies and optional speech models retain their respective licences.

### Official-project statement

Official MorpheOS Voice source and releases are published by MorpheOS through the repository linked from `morpheos.net/voice`. Forks are welcome under the MIT licence but should use their own name and visual identity.

## Terminology

| Term | Meaning in MorpheOS Voice |
|---|---|
| Voice typing | Speaking text into the application where the cursor is active. Preferred public category term. |
| Dictation | The shortcut-to-text workflow. Suitable as a secondary, widely understood term. |
| Transcription | Converting recorded speech into text. It does not include text insertion. |
| Local processing | Speech recognition runs on the same computer using an installed model. The initial model download still needs a network connection. |
| Remote processing | Recorded audio and request fields are sent to the endpoint selected by the user. |
| Provider | The operator of a remote transcription endpoint. MorpheOS Voice does not currently provide a hosted transcription service. |
| Model | The speech-recognition data used by a local or remote engine. Whisper-family models are supported; they are not the product name. |
| Write mode | Produces text only. It cannot execute an external action. This is the current product. |
| Act mode | A possible future approved instruction path to a tool or agent. It does not exist in the current product. |
| Recovery history | Bounded, locally stored text from completed transcriptions. The current Tauri history screen is a preview; Linux runtime history is real. Audio is not recovery history. |
| Copied | Text was verified on the clipboard but insertion was not confirmed. |
| Inserted | Text was copied and automated insertion completed without a reported error. It is not semantic proof that every target app accepted the text exactly as intended. |

## Search and discovery language

Use these phrases naturally where implementation evidence supports them:

- open-source voice typing;
- open-source dictation app;
- speech to text for Linux, macOS and Windows;
- voice typing in existing apps;
- local speech to text;
- push-to-talk dictation;
- voice typing for AI prompts;
- open-source alternative to commercial dictation software.

Do not use “private dictation” without an immediate local-mode qualification. Do not keyword-stuff.

Approved metadata:

- **Title:** `MorpheOS Voice — Free, Open-Source Voice Typing`
- **Description:** `Talk instead of type with MorpheOS Voice, a free, open-source voice typing tool for Linux, macOS and Windows.`
- **Canonical URL:** `https://morpheos.net/voice`

## Naming and compatibility

“MorpheOS Voice” is the public name. “MorpheOS Voice Free”, “OS Whisper”, “OSWispa”, “Talk to PC”, “Chat to PC”, “MorpheOS Whisper”, “Whisper Voice”, “Voice AI” and “AI Voice Assistant” are not current product names.

The legacy name may appear only in:

- migration and compatibility explanations;
- old release names, tags, assets and changelog entries;
- retained internal identifiers required for existing users;
- troubleshooting that needs the exact legacy command, path or service name.

When a compatibility identifier is shown to users, label it plainly, for example: “The command remains `oswispa` in this transition release so existing scripts and installations continue to work.”

## Decisions still requiring Tyler

- `[NEEDS TYLER]` Approve the final registered/trademark treatment for the MorpheOS and MorpheOS Voice names after appropriate professional advice.
- `[NEEDS TYLER]` Approve whether the official GitHub repository should later move from `tylerbuilds/OSWispa` to a MorpheOS-owned `morpheos-voice` URL.
- `[NEEDS TYLER]` Approve the stable signing publisher/team identities before bundle IDs, installer identities or update channels change.
- `[NEEDS TYLER]` Choose the quiet cross-sell destination on the free product page: MorpheOS home, IntelligenceOS or a future product index.
