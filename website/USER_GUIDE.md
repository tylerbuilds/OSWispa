# OSWispa User Guide

OSWispa is built for short dictation bursts, not hour-long live captioning. Hold the hotkey, speak, release, and it inserts the transcript into the active app.

## How recording works

1. Press and hold your hotkey. The default is **Ctrl + Super**.
2. Speak naturally.
3. Release the hotkey.

OSWispa transcribes locally by default, inserts the text into the focused app, and also copies the transcript to the clipboard.

On Windows the default shortcut is **Ctrl + Windows**. Keep the OSWispa console window open while dictating.

## What happens on first launch

OSWispa now runs a short device test before it downloads a model. It looks at:
- whether the current app build can use Metal, CUDA, HIPBLAS, or CPU only
- available memory
- a small local CPU speed probe

Then it picks a model that tries to stay responsive instead of chasing the biggest model.

## What model you should expect

- Older Intel Macs and other CPU-only machines usually get `base.en`. Expect fast short dictation, but names, accents, and noisy rooms will be less reliable.
- Faster CPU-only desktops and Intel workstations may get `small.en`. Expect better wording than `base.en`, but still slower than a GPU-backed setup.
- Apple Silicon Macs with lighter memory budgets usually land on `small.en`. This is the safe choice for keeping the Mac responsive.
- Apple Silicon Macs with more unified memory can move up to `medium.en`.
- High-headroom GPU systems can use `distil-large-v3` for the best English speed/accuracy balance.
- `large-v3` is accuracy-first, not speed-first. It is not the default auto-pick.

## What “fast” actually means

- `base.en`: best for quick replies, messages, and short prompts
- `small.en`: still practical for day-to-day dictation, but a little heavier
- `medium.en`: better accuracy, but you should expect more delay on CPU-only systems
- `distil-large-v3`: strong English dictation on capable hardware
- `large-v3`: use it when accuracy matters more than latency

## Tips for better results

- Speak in short phrases if you are on a CPU-only machine.
- Use a quiet room if you want better punctuation and fewer word misses.
- If a model makes your machine feel heavy, step down one size.
- If you care about speed more than absolute accuracy, do not force `large-v3`.

## Settings on Linux

The current graphical settings and tray are Linux-only. Right-click the OSWispa icon in the Linux tray to:
- change the hotkey
- choose a Linux PipeWire/PulseAudio microphone source, or leave it blank to follow the system default
- choose or import a different model
- enable or disable audio feedback
- enable or disable automatic text insertion
- configure the optional remote backend

The macOS and Windows alpha packages do not have a tray or graphical settings yet. They use configuration files in each operating system's application configuration directory; do not assume the Linux `~/.config/oswispa/` path applies to them.

## Linux microphone troubleshooting

If OSWispa inserts `[BLANK_AUDIO]` or reports no speech, confirm which input PipeWire/PulseAudio is using:

```bash
pactl get-default-source
pactl list short sources
```

Set the working microphone system-wide with `pactl set-default-source SOURCE_NAME`, or copy the source name into **Settings → General → Linux microphone source** to override it only for OSWispa.

For reliable login startup, use the installed user service:

```bash
systemctl --user enable --now oswispa
systemctl --user status oswispa
```

## Windows microphone troubleshooting

If recording does not start, open **Settings > Privacy & security > Microphone** and enable microphone access for desktop apps. The unsigned alpha may also require **More info > Run anyway** at the first SmartScreen prompt.
