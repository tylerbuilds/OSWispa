# OSWispa desktop UI foundation

This directory is an original, framework-free desktop UI contract for the next OSWispa host. It is bundled HTML, CSS and JavaScript with no remote assets, telemetry or network calls.

The three surfaces are:

- `index.html`: Ready Check onboarding and the eight-section Settings shell.
- `signal.html`: compact lifecycle feedback for `ready`, `arming`, `listening`, `processing`, `inserted`, `copied` and `needs_attention`.
- `history.html`: bounded, text-only recovery with collapsed transcript fixtures.

## Development boundary

Every page uses `data-bridge="development"`. `bridge.js` installs a narrow in-memory adapter with three methods: `invoke`, `listen` and `dispose`. The interface is deliberately small so a future desktop host can install a production adapter without changing surface code.

The development adapter cannot open a microphone, register a shortcut, read or write the clipboard, download a model, persist settings, touch files or perform insertion. Preview actions are labelled and return synthetic receipts. They must not be treated as host proof.

The Signal document has no interactive controls and does not display transcript content. Making its native window non-activating, always appropriately positioned and excluded from task switching remains the responsibility of the future host; HTML alone cannot guarantee those behaviours.

No desktop-host dependency, configuration, installer, signing flow or update mechanism is introduced here.

## Bridge contract

Commands are exposed as `OSWispaDesktopBridge.COMMANDS`:

- `read_bootstrap`
- `save_settings`
- `run_ready_check`
- `copy_history_entry`
- `clear_history`

Events are exposed as `OSWispaDesktopBridge.EVENTS`:

- `lifecycle`
- `ready_check_changed`
- `settings_changed`
- `history_changed`

A production adapter must implement `invoke(command, payload)`, `listen(eventName, handler)` and `dispose()`. `listen` returns an unlisten function. Host results should preserve the receipt-shaped objects used by the development adapter and add fields only when the UI can ignore them safely.

Signal states can be previewed with a local query, for example `signal.html?state=processing`.

## Validation

Run the standard-library contract suite from the repository root:

```sh
python3 -m unittest discover -s desktop/ui/tests -p 'test_*.py'
```

The suite blocks remote resources and networking primitives, checks document and form semantics, verifies expected settings, readiness and lifecycle identifiers, and scans the bundled UI for competitor leakage.
