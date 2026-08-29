# Lorem Ipsum for Tezbar

Generate placeholder text for design and development directly in Tezbar on Windows or macOS.

## Commands

- **Generate Paragraphs** generates one or more lorem ipsum paragraphs and copies or pastes the result.
- **Generate Sentences** generates sentences.
- **Generate Words** generates individual words.
- **Generate with AI** uses Raycast AI (`OpenAI_GPT-4o_mini`) to generate 2-5 short paragraphs on a topic, or a random topic when none is supplied.

All commands except AI share the same flow: optional numeric argument (e.g. `5`) capped at 1000, validated with a toast on error, and an action preference that controls whether the result is copied, pasted, or both.

## Compatibility

- Windows: built and exercised in Tezbar on Windows 11, including clipboard and paste round-trips.
- macOS: uses only cross-platform `@raycast/api` and the `lorem-ipsum` JS library and retains the upstream macOS-compatible design.
- Linux: not currently declared.

## Local verification

```powershell
pnpm install
pnpm typecheck
pnpm test
pnpm build
```

## Credits

Adapted for Tezbar from Raycast's MIT-licensed `lorem-ipsum` extension at upstream commit `6734d85992290066fd31a7f5bee3cca3d3545a1a`. Original author `AntonNiklasson` and contributors remain credited in `package.json`.
