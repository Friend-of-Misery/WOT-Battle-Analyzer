# UI translations

Supported language codes: `de`, `en`, `es`, `pl`, `fr`, `cs`, `uk`, `ru`, `it`, `tr`, `pt`, `hu`, `ro`.

Choose the language under Settings / Sprache. The selection is stored in `settings.json` and changes the interface immediately. The embedded catalog requires no internet connection. Language names are shown in their native spelling.

`catalog.json` uses the original German UI string as its key, and language codes as translation keys. The main navigation, search/import actions, filters and major statistics labels have translations for all supported languages. More detailed explanatory texts currently fall back to English. Some legacy technical replay-field explanations still originate in German; technical field names, SQL identifiers, player names and original replay values are intentionally not translated.

Keep every Rust-format placeholder exactly as in the source, including formatting specifications (`{name}`, `{}`, `{:.1}`, etc.). The tests check these placeholders. Runtime formatting preserves the original values and number formatting. `tr_format!` performs translation of formatted UI text without translating the underlying data.

When adding a control, add all language variants. Prefer a proper translation to English fallback. Test with `cargo test --offline`; the UI test renders every tab for each supported language at 1000×620 and 1280×820.
