# Someday: a local-first chat app

An idea worth keeping around, not a spec or roadmap commitment.

Build a chat app that feels private, fast, and more trustworthy than the
"ask the web" experience that made Perplexity frustrating.

- Run a small Qwen 2B-class language model entirely on-device.
- Run an embedding model locally too, preferably using the device NPU where it
  is genuinely faster and more energy-efficient.
- Use Qenlo as the durable local memory layer: store embeddings plus user/time
  metadata, retrieve relevant memories, and use that retrieval to extend useful
  context beyond the model's native context window.
- Keep search optional and explicit. When the user asks for current web
  information, call a web-search provider (evaluate Exa, Firecrawl, or another
  strong search API later), then show the sources rather than pretending the
  model already knew the answer.

The interesting system split is deliberate:

```text
NPU: embedding inference
CPU/GPU: Qenlo filtered retrieval
local LLM: response generation
web API: fresh, attributable information when requested
```

The point is not to claim every operation belongs on an NPU. It is to keep data
local by default, use accelerators for the work they suit, and make retrieval
and web evidence visible to the person using the chat.
