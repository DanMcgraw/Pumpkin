# Matrix Result Template

Copy this file for each cell: `results/original-original.md`, `results/original-runner.md`, `results/current-original.md`, `results/current-runner.md`.

## Cell

- **Binary**: (original / current)
- **Runtime**: (original-clean / runner-copy)
- **Date/time**:
- **Tester**:

## Environment

- Bedrock client version:
- OS:
- Render distance on client:
- Server config changes from default:

## Outcome

- [ ] Client reached the world
- [ ] Server received `SetLocalPlayerAsInitialized`
- [ ] Client stayed connected for 60 s
- [ ] Client process crashed (`0xc0000005`)

## Timing

- Time from join to initialization: ___ s
- Time to first substantial NACK: ___ s
- Time to disconnect/crash: ___ s

## Log observations

- First large NACK range:
- `UDP socket error 10054` count:
- `Failed to lock network writer for try_enqueue_packet` count:
- Reliable-frame resend count:
- Loaded entity count near join:
- Loaded chunk count near join:

## Notes

Paste any relevant log excerpts here.
