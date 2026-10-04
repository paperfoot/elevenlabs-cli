# Prompting Eleven v4

Checked against ElevenLabs documentation on 4 October 2026.

## Choose the voice before directing it

Use `elevenlabs voices list` or `elevenlabs voices search NAME`, then pin a
voice ID for repeatable comparisons. Try a representative passage containing
the hardest name, a normal sentence and one emotional change before producing
a full script.

V4 supports instant and professional clones. It reproduces recording flaws
as well as voice characteristics, so use clean source audio. In another
language, it generally adopts the target language's pronunciation instead of
carrying the reference voice's original accent. Audition that behaviour when
accent identity matters. [Model and cloning guidance](https://elevenlabs.io/docs/overview/capabilities/text-to-speech/eleven-v4).

## Write what should be heard

The `tts` text is the spoken script. Put the brief, audience and editing
instructions in the LLM prompt that prepares that script. Do not prepend
“read the following in a friendly voice” to the speech input.

Use complete thoughts and readable punctuation. Ellipses can suggest
hesitation; capitals can add emphasis. V4 does not support SSML breaks.
Bracketed delivery directions are flexible prompts, not fixed commands or
precise duration controls. [V4 prompting](https://elevenlabs.io/docs/overview/capabilities/text-to-speech/best-practices#prompting-eleven-v4).

For predictable timing, measure the generated audio and edit silence in
post-production. Do not promise a two-second pause from a prose tag.

## Tags

Put a direction immediately before the phrase it should affect. Give a new
direction when delivery changes. These examples are documented by ElevenLabs;
they are not an exhaustive vocabulary. [Audio tags](https://elevenlabs.io/docs/overview/capabilities/text-to-speech/best-practices#audio-tags).

| Intention | Examples |
|---|---|
| Delivery or emotion | `[whispers]`, `[curious]`, `[excited]`, `[sarcastic]`, `[crying]` |
| Vocal reactions | `[laughs]`, `[sighs]`, `[exhales]` |
| Sound events | `[applause]`, `[clapping]` |
| Experimental performance | `[sings]`, `[strong French accent]` |

Try a descriptive direction such as `[gentle voice, holding back a laugh]`.
Specify a vocal quality when you want voice acting: an ambiguous effect cue may produce a sound instead. Delivery already
present in the voice's training material tends to be easier to elicit.

As a working method, start untagged, then add one direction at the point that
needs it. Avoid stacking contradictory directions. Add laughter or sound
events only when the script calls for them; they change the audible content.

## Settings

Start comparisons at `--stability 0.5 --similarity 0.75`, the dialogue API's
default values. Lower stability permits more variation; higher stability
favours consistency. Higher similarity follows the reference voice more
closely, sometimes at the expense of naturalness. V4 has no style or speed
slider, and this CLI rejects those options for v4. These values are a baseline,
not a universal best preset. [Model settings](https://elevenlabs.io/docs/overview/capabilities/text-to-speech/eleven-v4),
[dialogue request schema](https://elevenlabs.io/docs/api-reference/text-to-dialogue/convert).

Change one variable at a time: wording, tag, voice, stability, then similarity.
Keep the chosen voice, script and settings with the accepted output. A seed
helps comparisons but does not guarantee identical audio.

## CLI examples

These examples use the saved voice. Add `--voice-id VOICE_ID` to choose a
specific one.

### Plain narration

```sh
elevenlabs tts 'The sample remained stable for six hours. The next measurement is at noon.' \
  --model eleven_v4 --stability 0.5 --similarity 0.75 -o narration.mp3
```

### A delivery change

```sh
elevenlabs tts '[curious] Is that light coming from the cellar? [whispers] Stay here. I will check.' \
  --model eleven_v4 --stability 0.5 --similarity 0.75 -o scene.mp3
```

### A longer passage with alignment

```sh
cat > passage.txt <<'TEXT'
[gentle voice, with quiet wonder] Beneath the bridge, the river carried a single red leaf.
Mara watched it turn once, twice... then disappear around the bend.
[brighter, ready to move] There. That is the way home.
TEXT

elevenlabs tts - < passage.txt --model eleven_v4 \
  --stream --with-timestamps --save-timestamps passage.timings.jsonl \
  -o passage.mp3
```

### Two speakers

Replace `VOICE_A` and `VOICE_B` with IDs from your library. Each turn has its
own voice; a speaker name written inside `text` may be spoken aloud.

```sh
cat > dialogue.json <<'JSON'
[
  {"voice_id":"VOICE_A","text":"[curious] Did you move the telescope?"},
  {"voice_id":"VOICE_B","text":"[holding back a laugh] No. I moved the whole table."},
  {"voice_id":"VOICE_A","text":"[sighs] That would explain the ceiling."}
]
JSON

elevenlabs dialogue dialogue.json --model eleven_v4 \
  --stability 0.5 --similarity 0.75 -o dialogue.mp3
```

Generate connected turns together when testing a scene: the dialogue model
uses context across inputs. It can produce interruptions and overlap, but
these examples do not specify exact speaker start times.
[Dialogue capabilities](https://elevenlabs.io/docs/overview/capabilities/text-to-dialogue).

## Pronunciation and text preparation

V4 documents native IPA written between forward slashes. Apply it selectively
and check the pronunciation with the chosen voice. For example, test
`The word is "/kæt/".` Do not substitute older models' XML phoneme syntax.
[IPA guidance](https://elevenlabs.io/docs/overview/capabilities/text-to-speech/best-practices#ipa-with-eleven-v4).

Before generation, decide how ambiguous items should sound: a year, a decimal,
an initialism, a URL or a unit. Keep the source unchanged and make a separate
spoken version. Preserve the value of every number and unit. Do not expand
an abbreviation whose intended meaning is unknown; resolve it first.

Use this preparation prompt with an LLM, then send only its resulting script
to the CLI:

```text
Prepare this script for Eleven v4.
Audience: <who is listening>
Voice and delivery: <selected voice; intended performance>
Mode: TAGS ONLY or SPOKEN ADAPTATION

In TAGS ONLY mode, preserve every original word and its order. Add square-
bracket directions only where the delivery needs a change. Do not add sounds,
reactions, claims or speaker names that are not part of the intended audio.

In SPOKEN ADAPTATION mode, make punctuation and awkward written forms easier
to say. Preserve meaning, names, numerical values, units, quotations and the
speaker order. Do not invent expansions or resolve ambiguous dates by guessing.

Choose compatible, concrete voice directions. Do not add a tag to every line.
Do not use SSML or request a precise pause duration. Keep editing instructions
out of the spoken text. Return only the finished script. If an ambiguity would
change meaning, ask one specific question instead of producing a guessed script.

Script:
<source text>
```

For multiple speakers, request a JSON array of `{voice_id, text}` entries and
provide the exact allowed voice IDs. Require the same speaker mapping and
turn order throughout.

## Long scripts and review

This CLI sends v4 through the HTTP dialogue API: keep the combined text in a
request at or below 2,000 characters and use at most 10 distinct voice IDs.
The same limit applies to `tts --model eleven_v4` here. Split at sentence or
scene boundaries. `dialogue` still defaults to v3, so explicitly set v4.

For neighbouring chunks, `--previous-text` and `--next-text` accept 100
characters each; the latter maps to the API's `future_text`. The corresponding
request-ID flags accept up to three IDs. Use IDs from real dialogue responses,
not filenames. These fields guide continuity rather than guaranteeing a
seamless edit. [Dialogue API](https://elevenlabs.io/docs/api-reference/text-to-dialogue/convert).

```sh
elevenlabs tts 'The door opened. Nobody was there.' --model eleven_v4 \
  --previous-text 'Someone knocked three times.' \
  --next-text 'Then the telephone rang.' -o middle.mp3
```

Audition a short representative sample first. When a choice matters, compare
an untagged take with a directed take. Check spoken-word accuracy, name
pronunciation, voice identity, emotional intent, unintended sounds and the
joins between chunks. Keep the better take based on listening, not the
presence of a tag in the request. The dialogue API is nondeterministic;
request acceptance alone does not establish acting quality.
[Generation guidance](https://elevenlabs.io/docs/overview/capabilities/text-to-dialogue).

| Problem heard | Next experiment |
|---|---|
| Direction has little effect | Simplify it, move it next to the phrase, then try a better-matched voice. |
| Delivery is exaggerated | Remove competing tags; test a more restrained direction. |
| Wrong word or number | Correct the spoken script or test a targeted pronunciation. |
| An unwanted laugh or sound appears | Remove reaction/effect cues and ambiguous descriptions. |
| Adjacent clips sound disconnected | Revisit the split, use short context and compare the join. |

V4 Turbo is the realtime variant. Use it for agent configuration or a
WebSocket client; this CLI's direct speech commands use HTTP v4. A
conversational agent's system prompt should define its role, task, tool rules
and turn-taking separately from any speech-delivery directions.
