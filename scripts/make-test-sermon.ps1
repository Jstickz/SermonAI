<#
.SYNOPSIS
  Build a ten-minute spoken sermon as 16 kHz mono WAV, for M1's Definition of
  Done.

.DESCRIPTION
  Uses Windows' built-in speech synthesiser, so nothing needs installing.

  A real sermon recording is a better test and should replace this before M4:
  synthesised speech articulates "Thessalonians" and "propitiation" far more
  clearly than a preacher at pace in a room, which is exactly why the FR-09
  vocabulary A/B came back a null result. What this file *is* good for is the
  mechanical half of the DoD -- ten minutes of continuous speech, lag
  percentiles, memory under load, and whether anything drops over a long run.

  Written at 16 kHz mono to match the capture format, so the loopback path
  resamples 48 kHz to 16 kHz exactly as a live microphone would.

.EXAMPLE
  .\scripts\make-test-sermon.ps1 -Minutes 10
#>
param(
  [int]$Minutes = 10,
  [string]$OutFile = "$env:TEMP\sermonai-test-sermon.wav"
)

Add-Type -AssemblyName System.Speech

# Paragraphs a preacher might actually say, with the references and archaic
# forms the detection engine will care about in M2. Repeated to length rather
# than generated, so successive runs transcribe the same words and two runs
# can be compared.
$paragraphs = @(
  "Good morning church. Thank you for being here today. I want us to open our Bibles this morning to the book of Jeremiah, chapter twenty nine, verse eleven. And I want you to receive this word not as history, but as a promise God is speaking over your life today.",
  "For I know the thoughts that I think toward you, saith the Lord, thoughts of peace, and not of evil, to give you an expected end. Beloved, hear that again. Thoughts of peace, and not of evil.",
  "Because before we can walk into what God has for us, we have to first trust the One who wrote our story. Paul understood this. He said in Romans eight twenty eight that all things work together for good to them that love God, to them who are the called according to his purpose.",
  "Turn with me if you will to First Thessalonians chapter five. Rejoice evermore. Pray without ceasing. In every thing give thanks, for this is the will of God in Christ Jesus concerning you.",
  "And I know some of us are in a season where it does not look good. It does not feel good. But hear this: God is not surprised by your situation. He said before you were formed in your mother's womb, he knew you, he set you apart.",
  "The prophet Habakkuk asked the same question we ask. How long, O Lord, shall I cry, and thou wilt not hear? And the answer came: the just shall live by his faith.",
  "Look at what John writes in his first epistle. Herein is love, not that we loved God, but that he loved us, and sent his Son to be the propitiation for our sins.",
  "So I want to leave you with this. Whatsoever things are true, whatsoever things are honest, whatsoever things are just, whatsoever things are pure, think on these things. Let us pray."
)

$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer
$format = New-Object System.Speech.AudioFormat.SpeechAudioFormatInfo(
  16000,
  [System.Speech.AudioFormat.AudioBitsPerSample]::Sixteen,
  [System.Speech.AudioFormat.AudioChannel]::Mono)
$synth.SetOutputToWaveFile($OutFile, $format)
# Slightly slow, which is closer to preaching pace than the default.
$synth.Rate = -2

$target = $Minutes * 60
$builder = New-Object System.Text.StringBuilder
# About 150 words a minute at this rate; the loop below just repeats until the
# written file is long enough, so the estimate only affects how many passes.
$wordsNeeded = $target * 2.5
$words = 0
$i = 0
while ($words -lt $wordsNeeded) {
  $p = $paragraphs[$i % $paragraphs.Count]
  [void]$builder.AppendLine($p)
  $words += ($p -split '\s+').Count
  $i++
}

Write-Host "Synthesising about $Minutes minutes of speech to $OutFile ..."
$synth.Speak($builder.ToString())
$synth.Dispose()

$bytes = (Get-Item $OutFile).Length
$seconds = ($bytes - 44) / 32000
Write-Host ("wrote {0}" -f $OutFile)
Write-Host ("{0:N1} MB, {1:N1} minutes of 16 kHz mono" -f ($bytes / 1MB), ($seconds / 60))
