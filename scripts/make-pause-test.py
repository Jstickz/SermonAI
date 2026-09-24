"""Build a WAV with a deliberate 30-second silence in the middle.

The test for loopback silence synthesis. Windows loopback delivers *nothing*
while nothing is playing, so a quiet passage disappears from the audio timeline
entirely — which breaks the 2.5 s rule that opens paragraphs, and drifts every
timestamp after it.

The silence here is **digital zero**. It turned out not to be the trigger on
the endpoint that fails: Speakers (Realtek) keeps delivering through zeros
*inside* a playing stream and only stops when the stream itself ends. Measured
on 24 Sept — the whole 90 s file arrived, and the fix synthesised 9 s only
after playback finished. That matches the DoD run exactly, where the sermon's
own pauses survived and the restart gap did not.

So to exercise the bug, **stop playback and start it again** rather than relying
on the gap in the file. The gap is kept because other endpoints may behave
differently, and because it is what a real room does.

    python scripts/make-pause-test.py

Writes two files to %TEMP%, 16 kHz mono 16-bit, matching capture:

  sermonai-pause-tone.wav    30 s tone, 30 s silence, 30 s tone.
                             For `capture_rate`: needs no Deepgram, gives the
                             timeline ratio directly.

  sermonai-pause-speech.wav  60 s of the test sermon, 30 s silence, 60 s more.
                             For the app: real speech, so the transcript has
                             words either side of the gap and a paragraph break
                             can be looked for at it. Needs
                             %TEMP%/sermonai-test-sermon.wav to exist
                             (scripts/make-test-sermon.ps1).
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

    Not real speech: this file is about whether audio *arrives*, not about
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


def digital_silence(seconds: int) -> bytes:
    """Exact zeros. Built with struct rather than a byte-string literal, so
    the source file never contains a NUL and no editor can mangle it."""
    return struct.pack("<h", 0) * (seconds * RATE)


def read_sermon(seconds_from: int, seconds_to: int) -> bytes | None:
    """A slice of the test sermon, or None if it has not been generated."""
    path = os.path.join(os.environ.get("TEMP", "."), "sermonai-test-sermon.wav")
    if not os.path.exists(path):
        return None
    with wave.open(path, "rb") as wav:
        if (wav.getnchannels(), wav.getframerate(), wav.getsampwidth()) != (1, RATE, 2):
            raise SystemExit(f"{path} is not 16 kHz mono 16-bit")
        wav.setpos(seconds_from * RATE)
        return wav.readframes((seconds_to - seconds_from) * RATE)


def write(path: str, parts: list[bytes]) -> None:
    with wave.open(path, "wb") as wav:
        wav.setnchannels(1)
        wav.setsampwidth(2)
        wav.setframerate(RATE)
        for part in parts:
            wav.writeframes(part)


def main() -> None:
    temp = os.environ.get("TEMP", ".")
    silence = digital_silence(SILENCE_SECONDS)

    tone_path = os.path.join(temp, "sermonai-pause-tone.wav")
    write(tone_path, [tone(TONE_SECONDS), silence, tone(TONE_SECONDS)])
    total = TONE_SECONDS * 2 + SILENCE_SECONDS
    print(f"wrote {tone_path}")
    print(f"  {TONE_SECONDS}s tone, {SILENCE_SECONDS}s digital silence, {TONE_SECONDS}s tone ({total}s)")
    print(f"  capture_rate: expect ratio ~1.000 with the fix, ~{TONE_SECONDS * 2 / total:.3f} without")

    before = read_sermon(0, 60)
    after = read_sermon(60, 120)
    if before is None or after is None:
        print()
        print("sermonai-test-sermon.wav not found; skipped the speech version.")
        print("  Generate it with: .\\scripts\\make-test-sermon.ps1 -Minutes 10")
        return

    speech_path = os.path.join(temp, "sermonai-pause-speech.wav")
    write(speech_path, [before, silence, after])
    print()
    print(f"wrote {speech_path}")
    print(f"  60s of sermon, {SILENCE_SECONDS}s digital silence, 60s more (150s)")
    print("  in the app: expect synthesised_silence_seconds ~30, and a paragraph")
    print("  break in the transcript where the gap was")


if __name__ == "__main__":
    main()
