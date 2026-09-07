# Presentation recording evidence

`representative-recording.json` is the current input for the live recording
harness. Its seed digest follows the current authored identity-proof seed.

`identity-proof-observer-frame.json` and its receipt are historical capture
evidence from the retired client. `identity-proof-recording-source.json` preserves
the exact original fixture bytes identified by that receipt. It is immutable
historical input, not a second current recording definition. The tests bind the
historical receipt to this snapshot while testing the current harness against
`representative-recording.json`.
