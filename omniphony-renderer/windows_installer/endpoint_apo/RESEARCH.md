# Omniphony endpoint APO research note

The native endpoint APO bootstrap is intentionally split into two stages.

**Stage 1, current:** prove that Windows can host a project-owned, identity-only APO on the physical render endpoint with no virtual playback device and no sound change.

**Stage 2, after physical-machine proof:** connect the existing Omniphony Current renderer through a bounded realtime bridge. The callback must remain allocation-free and nonblocking. The existing asynchronous Current ABI is a transitional boundary, not permission to accumulate avoidable internal buffering.

Research inputs for this implementation included Microsoft's SysVAD APO examples and mature endpoint attachment behavior in Equalizer APO, plus peer-reviewed work on realtime binaural rendering and partitioned convolution. The literature consistently favors bounded/precomputed realtime work, partitioned convolution for long FIR/HRTF filters, and avoiding internal buffering that adds one or more audio blocks of latency.

No Current-model DSP constants are changed by this bootstrap.
