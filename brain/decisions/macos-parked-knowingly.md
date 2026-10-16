# macOS is parked, knowingly, and the debt now spans two milestones

**Decided:** 23 September 2026 (M1 close) · **Reaffirmed:** 15 October 2026 (M2 close)

M1 and M2 each closed on Windows only, although each milestone's own gate says "on both platforms". The operator took this call knowingly: there is no Mac to hand, and waiting would have stopped Phase 1.

What has never executed on a Mac outside a CI compile:

- Device enumeration (`audio/devices.rs` branches heavily on platform; output endpoints are not offered on macOS, virtual cables are recognised by name).
- The silence watchdog, whose premise (cpal delivers buffers continuously) was found **false** for two of three loopback endpoints on Windows. CoreAudio may differ again, in either direction.
- The capture path end to end. DoD 1 needs BlackHole there, because CoreAudio cannot capture a render endpoint.
- Detection cards rendering (M2 DoD 4).

The cost accepted: a macOS fault surfaces later, with detection, staging and the summariser built on top of it. Needs a Mac, M1's four DoD lines, M2's card check and a 60-minute run, before M4's live service. The pilot (M8) requires at least three churches on macOS.

Related: [[open-questions]], [[sessions/2026-09-23]], [[sessions/2026-10-15]]
