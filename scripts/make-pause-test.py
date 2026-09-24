"""Build a WAV with a deliberate 30-second silence in the middle.

The test for loopback silence synthesis. Windows loopback delivers *nothing*
while nothing is playing, so a quiet passage disappears from the audio timeline
entirely — which breaks the 2.5 s rule that opens paragraphs, and drifts every
timestamp after it.

The silence here is **digital zero**, which is the hard case. A real room is
never silent, and a sermon recording's pauses carry room tone that keeps the
endpoint delivering. Zero is what a stopped media player produces, and it is
what makes WASAPI stop calling back at all.

    python scripts/make-pause-test.py

Writes %TEMP%/sermonai-pause-test.wav: 30 s of speech-shaped tone, 30 s of
digital silence, 30 s of tone. 16 kHz mono 16-bit, matching capture.
"""

import math
import os
import struct
import wave

RATE = 16_000
TONE_SECONDS = 30
SILENCE_SECONDS = 30


def tone(seconds: int) -> bytes:
    """Speech-shaped enough to be audible and to move a level meter.

    Not real speech: this test is about whether audio *arrives*, not about
    whether it transcribes, so a buzz with a couple of harmonics is enough.
    """
    out = bytearray()
    for i in range(seconds * RATE):
        t = i / RATE
        v = (
            math.sin(2 * math.pi * 130 * t) * 0.45
            + math.sin(2 * math.pi * 390 * t) * 0.2
            + math.sin(2 * math.pi * 650 * t) * 0.1
        )
        # Syllable-rate envelope, so it is not a single flat drone.
        v *= 0.55 + 0.45 * math.sin(2 * math.pi * 3.5 * t)
        out += struct.pack("<h", int(v * 9000))
    return bytes(out)


def main() -> None:
    path = os.path.join(os.environ.get("TEMP", "."), "sermonai-pause-test.wav")
    with wave.open(path, "wb") as wav:
        wav.setnchannels(1)
        wav.setsampwidth(2)
        wav.setframerate(RATE)
        wav.writeframes(tone(TONE_SECONDS))
        wav.writeframes(b"\x00\x00" * (SILENCE_SECONDS * RATE))
        wav.writeframes(tone(TONE_SECONDS))

    total = TONE_SECONDS * 2 + SILENCE_SECONDS
    print(f"wrote {path}")
    print(f"  {TONE_SECONDS}s tone, {SILENCE_SECONDS}s digital silence, {TONE_SECONDS}s tone")
    print(f"  {total}s total, 16 kHz mono 16-bit")
    print()
    print("Expected with the fix, capturing the System audio loopback:")
    print(f"  ratio ~1.000  (without it, ~{TONE_SECONDS * 2 / total:.3f} — the silence is missing)")


if __name__ == "__main__":
    main()
