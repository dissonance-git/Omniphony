# Headphone Rendering and Machine Hearing Research

> Current research and engineering direction for headphone playback, auditory scene understanding, and perceptual evaluation in `dissonancehelix/Omniphony`.
>
> **Primary product goal:** make standard Omniphony headphone playback present ordinary audio as a stable, externalized, spatially coherent acoustic scene while preserving musical identity, clarity, timbre, transients, dynamics, and mix hierarchy.
>
> **Broader research goal:** use the same project to build progressively stronger machine hearing: a system that organizes sound more like a human listener does, then uses what it learns about hearing to improve rendering, while rendering experiments in turn provide new evidence about hearing.

This is not a branded listening mode and is not intended to remain a separate product path. Successful research should graduate into **base Omniphony**. Experimental names, presets, parallel renderers, and research-only switches should disappear when the underlying method is proven.

The long-term listener-facing experience should be simple:

```text
application / player / game
          ↓
       Omniphony
          ↓
standard headphone rendering
          ↓
       headphones
```

The user should not need to understand HRTFs, Ambisonics, early-reflection geometry, auditory scene analysis, machine-learning models, source separation, headphone compensation, codec plumbing, or Windows routing. Complexity belongs inside the engine.

---

## 1. Current project thesis

Conventional stereo headphone playback is a severe representational bottleneck. A recording may contain evidence about foreground and background, auditory grouping, source width, direct and diffuse energy, reverberant structure, motion, mix hierarchy, temporal identity, and apparent distance, but ordinary playback presents that organization primarily as two ear-coupled signals.

The immediate audio question is:

> **How much perceptually meaningful acoustic structure can Omniphony recover or construct from the evidence that survives in an ordinary recording, without damaging the recording itself?**

The broader machine-hearing question is:

> **Can the system progressively learn to organize, track, compare, and interpret sound using perceptual structures that correspond more closely to human hearing than clip labels, isolated FFT bins, or disposable per-frame estimates?**

These questions reinforce each other:

```text
better understanding of hearing
          ↓
better scene inference and rendering
          ↓
better controlled listening experiments
          ↓
better evidence about what hearing requires
          ↓
better understanding of hearing
```

This feedback loop is now a central project purpose.

Music remains the primary practical target because most daily listening is stereo music and because music is an unusually demanding hearing test: tiny errors in grouping, timing, transients, pitch structure, timbre, source identity, hierarchy, phase, and spatial continuity become audible quickly. The hearing architecture, however, should not be music-exclusive. A mature system should be able to organize speech, environmental sound, games, films, broadcasts, machinery, animals, weather, and other acoustic scenes through the same lower-level perceptual principles.

---

## 2. The object, the model, and the render must remain distinct

A governing law is:

> **The map must not replace the thing. It should make more of the thing reachable.**

For audio this becomes:

```text
SOURCE AUDIO
= the exact acoustic object available to us

MACHINE-HEARING MODEL
= an interpretation of what appears to be happening in that audio

SCENE MODEL
= a confidence-aware representation of sources, fields, room, relations, and time

RENDER
= a transformation that presents that model to two human ears
```

Therefore:

```text
evidence ≠ model ≠ render
```

The original decoded waveform remains the musical/acoustic authority whenever practical. Machine-learning systems may estimate masks, source likelihoods, object identities, spatial descriptors, room descriptors, embeddings, or control metadata without automatically replacing the corresponding waveform with a synthesized stem.

This is especially important for music. Source separation is useful evidence, but a separated estimate is not automatically a fidelity-safe substitute for material in the master. Phase errors, contamination, musical bleed, transient damage, and model hallucination remain possible.

A preferred long-term pattern is:

```text
original audio ───────────────────────────────► final transformation
      │                                               ▲
      └► machine hearing ► scene hypothesis ► control ┘
```

Use generated/reconstructed audio only where the evidence shows that waveform replacement is necessary and perceptually safe.

---

## 3. Successful research graduates into core

The research path exists to change Omniphony itself.

```text
current Omniphony
      ↓
controlled renderer / hearing experiments
      ↓
objective + listening validation
      ↓
surviving method
      ↓
base Omniphony behavior
```

Do not preserve an experimental premium mode merely because a technique began as an experiment. If directional early reflections are demonstrably better, they should become the normal early-reflection implementation. If a scene representation is demonstrably better, it should become the normal scene representation. If an auditory model consistently improves inference, it should become normal upstream intelligence.

The reverse also applies: a candidate that does not survive matched, repeatable tests should be removed or retained only as a diagnostic control.

---

## 4. Listener contract and transition law

The project is being developed for a listener who already has a preferred foobar2000 DSP + HeSuVi playback chain. That working setup is not to be dismantled merely to create a test environment.

The migration must be earned.

```text
CURRENT PLAYBACK
foobar DSPs + HeSuVi
        │
        │ remains available and unchanged
        ▼
OMNIPHONY DEVELOPMENT
        │
        ├► deterministic/offline tests
        ├► artificial-listener tests
        ├► known-scene tests
        └► optional listening comparison
        │
        ▼
Omniphony becomes independently stable and clearly better
        │
        ▼
redundant pieces of the old chain are bypassed one by one
        │
        ▼
Omniphony becomes the normal playback path
```

Do not require a cold-turkey migration.

Do not make the listener act as the build engineer. The user's high-value role is:

```text
guide
→ choose direction
→ recognize important perceptual differences
→ reject bad results
→ listen when a human perceptual judgment is needed
```

The project should automate installation, codec handling, fixture rendering, measurement, candidate bookkeeping, and reproducibility as far as practical.

### 4.1 First-install bar

The first time the user installs an experimental Omniphony listening build, it should already have a reasonable expectation of producing a **large audible improvement** over stock/basic playback. Do not ask the user to replace or reconfigure the working chain merely to hear an internal engineering baseline.

Internal controls can remain available to automation and developers. User-facing builds should bundle enough surviving improvements to make the installation worthwhile.

### 4.2 Normal playback, not drag-and-drop ritual

Drag-and-drop rendering may exist as a developer/debug path, but it is not the intended listening workflow.

Near-term preferred integration:

```text
foobar / normal playback
        ↓
Omniphony optional parallel or switchable path
        ↓
headphones
```

Long-term preferred integration:

```text
Windows audio
→ Omniphony
→ physical output
```

The mature experience should approach:

```text
install once
select headphones/output once
play audio normally
```

---

## 5. Input representation law

All source types should ultimately enter a canonical scene representation with provenance describing what is known versus inferred.

```text
explicit authored objects / ADM / S-ADM / PMD / similar metadata
              ↓
native Ambisonics / HOA
              ↓
discrete surround channels
              ↓
structured source layers / stems / sequencer information
              ↓
ordinary stereo
              ↓
mono
```

Available source structure increases upward. Required inference increases downward.

The rule is:

> **Preserve truth; infer only absence.**

Do not collapse a rich source to stereo and then attempt to rediscover structure that was already provided.

Ordinary stereo is not a compatibility afterthought. It is the primary everyday machine-hearing problem because it is where the most important structure is missing.

---

## 6. Codec and container boundary

The user should be able to play the existing music library directly. Most of that library may be Opus, including high-quality ~192 kbps stereo Opus.

The architecture should treat codec/container handling as a boundary concern rather than a hearing concern.

Preferred division:

```text
FFmpeg / ffprobe
→ identify container, codec, sample rate, channel count, channel layout
→ decode to PCM without user intervention

Omniphony hearing / scene layer
→ understand the decoded acoustic content

Omniphony renderer
→ present the scene to the ears
```

User-facing input should eventually include whatever normal audio formats the bundled decoder supports, for example Opus/Ogg/WebM, FLAC, AAC/M4A, MP3, WAV, and other common formats.

For controlled comparisons, decode a source once and feed every candidate the same PCM. This makes any codec loss upstream common to all variants.

Generated test outputs should preferably be lossless (for example FLAC or float PCM) so evaluation does not add another lossy encode when listening for small changes in timbre, transients, phase, spatial stability, or coloration.

Do not require the user to manually convert Opus to WAV.

---

## 7. Non-negotiable perceptual constraints

### 7.1 Clarity-preserving dimensionality

Increase spatial dimensionality while keeping blur near zero.

```text
maximize:
  externalization
  front / side / rear discrimination
  radial depth
  elevation
  apparent source extent
  apparent source width
  listener envelopment
  room scale
  source separation
  spatial persistence
  ambient continuity

subject to bounded change in:
  timbre
  transient shape
  direct-source coherence
  bass stability
  integrated loudness
  spectral balance
  phase / group delay
  source identity
  musical hierarchy
  fatigue
```

No amount of spatial scale compensates for softened attacks, watery cymbals, wandering vocals, unstable bass, collapsed center energy, or obvious comb coloration.

A central formulation is:

> **A source can become more physically present without becoming less precise, while the environment can become vastly more enveloping without swallowing the source.**

### 7.2 Cue agreement

A believable source is not just a coordinate. Direction, distance, apparent width, direct/reverberant ratio, early reflections, spectral cues, room response, extent, and temporal behavior should describe the same acoustic event.

Contradictory cues create an effect. Consistent cues create a scene.

### 7.3 Spatial specificity follows confidence

```text
high confidence   → precise persistent direct object
medium confidence → bounded region / larger extent
low confidence    → diffuse or ambient field
no evidence       → do not invent unnecessary precision
```

### 7.4 Spatial persistence

Object identity and position should not be recomputed as a new universe every analysis frame. Maintain hypotheses and change them only when new evidence is strong enough.

### 7.5 Dimensional independence

Do not collapse all spatial quality into one `spaciousness` variable. Direction, distance, width, height, source extent, room scale, envelopment, externalization, and stability can change independently.

### 7.6 One binaural transform in controlled tests

For scientific renderer comparisons, terminate at Omniphony's binaural stereo output. Do not feed that controlled result through HeSuVi, Windows Spatial Audio, another HRTF virtualizer, or a game-side binaural renderer.

However, **Omniphony + HeSuVi is a valid reference condition when the explicit research question is what additional perceptual cue HeSuVi is contributing.** Do not confuse that comparison with a clean renderer validation.

---

## 8. Apparent source width and listener envelopment are separate

Room-acoustics and precedence-effect research distinguishes at least two important spatial-impression components.

**Apparent source width (ASW)** is the perceived width/body of a source image that remains fused with the direct source.

**Listener envelopment (LEV)** is the sense of surrounding room or field energy that is perceptually distinct from the direct source image.

```text
direct source
+ fused early directional structure
          ↓
source body / apparent source width

later + distributed field energy
          ↓
listener envelopment / room
```

Do not enlarge every source in order to make the room larger. Do not collapse the room onto the source in order to make the source wider.

### 8.1 Precedence-effect boundary

Early reflections can strengthen source body, room evidence, and externalization while the first arrival retains localization dominance. Fusion/echo behavior depends on signal content, level, direction, repetition, adaptation, and delay, so it must not be encoded as one universal millisecond threshold.

### 8.2 Preserve temporal envelopes

Directional reflected energy should increase useful spatial evidence while preserving onset and amplitude-envelope integrity.

Reject candidates that create:

- audible doubling;
- softened attacks;
- obvious comb coloration;
- unstable source identity;
- reverberant haze masquerading as size.

### 8.3 Interaural correlation is a diagnostic, not a target knob

Measure interaural coherence/correlation by band where useful, but do not blindly maximize decorrelation. ASW and LEV depend on different combinations of interaural and spatial energy structure.

---

## 9. Canonical hearing and scene architecture

The exact Rust API remains intentionally unfrozen. Conceptually, the machine-hearing system should progress from acoustic evidence toward increasingly semantic hypotheses without forcing semantic labels too early.

```text
                           SOUND
                             │
                             ▼
                   PERCEPTUAL FRONT END
       spectral / temporal / binaural / envelope cues
                             │
                             ▼
                    AUDITORY ORGANIZATION
             grouping, boundaries, continuity
                             │
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
           OBJECTS         FIELDS         EVENTS
              │              │              │
              └──────────────┼──────────────┘
                             ▼
                        PERSISTENCE
                 what remains the same?
                             │
                             ▼
                         RELATIONS
            foreground / background / room / group
                             │
                             ▼
                          CONTEXT
              semantic priors when confidence allows
                             │
                             ▼
                   CONFIDENCE-AWARE SCENE
```

Semantic naming should not be required before perceptual grouping. A system should be able to hear one coherent auditory object without first knowing whether it is an oboe, voice, engine, bird, or synthesizer.

A mature scene model should distinguish at least:

```text
                     SCENE
                       │
          ┌────────────┼────────────┐
          │            │            │
   DIRECT SOURCES   AMBIENT FIELD   ROOM FIELD
          │            │            │
    precise HRTF      HOA / SH     early + late
    stable position   diffuse      source-linked
    radial depth      broad        externalization
    sharp attacks     enveloping   environment
          │            │            │
          └────────────┼────────────┘
                       ▼
                 binaural output
```

The three roles are perceptually different and do not need to share one rendering algorithm.

---

## 10. Human-like hearing requires time at several scales

Do not make important scene decisions at audio-block or FFT-frame cadence merely because those are convenient computational units.

A useful hierarchy is:

```text
sample / frame time
→ waveform and acoustic cues

auditory-event time
→ onsets, attacks, decays, local grouping

musical / behavioral time
→ beats, phrases, words, gestures, repeated events

scene time
→ persistent identities, sections, environments, relationships
```

For music, structure analysis can provide meaningful boundaries such as beat, downbeat, phrase, verse, chorus, bridge, solo, breakdown, and return.

Spatial persistence should respect musical form. A guitar or vocal that returns after a section should not randomly reappear in a different location without evidence. This creates **spatial musical memory**.

For non-music scenes, analogous persistence can track speakers, moving vehicles, repeating machines, room ambience, footsteps, weather, animals, and other sources across time.

---

## 11. Music-specific machine hearing

Music is the primary proving ground, but the hearing representation should not equate a musical object with a source-separation stem.

```text
stem
= a model category such as vocals / drums / bass / other

auditory object
= a persistent perceptual entity or group that the listener can track
```

One instrument can create several perceptual objects. Several instruments can fuse into one perceptual object or field.

Useful evidence includes:

- source-separation probabilities/masks;
- harmonicity;
- onset synchrony;
- common fate;
- pitch/timbre similarity;
- stereo level, phase, delay, and coherence;
- transient ownership;
- decay/directness;
- beat/bar/section structure;
- production/mix hierarchy;
- temporal continuity;
- learned audio embeddings.

### 11.1 The mix itself is evidence

A stereo master contains production decisions about prominence, panning, width, masking, reverb, compression, grouping, and section-by-section hierarchy.

The hearing system should attempt to preserve relationships such as:

```text
lead vocal intentionally dominant
pad intentionally diffuse
guitar intentionally broad
percussion transient-critical
reverb shared by several sources
chorus intentionally expands
verse intentionally contracts
```

Do not optimize toward maximum spatial separation if the music intentionally blends.

### 11.2 Separation as evidence, not automatic replacement

Demucs/Open-Unmix-style systems, Wiener filtering, and related tools are valuable because they can estimate where different source classes or components exist. Their reconstructed stems should not automatically replace the master.

Prefer:

```text
source evidence
+ original mixture
→ scene-control metadata
→ conservative transformation
```

until a replacement waveform proves perceptually superior.

---

## 12. General machine-hearing direction

The project should be capable of growing beyond music without creating a separate hearing architecture for each domain.

Representative future acoustic scenes include:

```text
conversation
street / city
forest
storm
football broadcast
movie
video game
machinery
animals
crowded room
home environment
```

The shared questions are:

```text
what coherent things are present?
what belongs together?
what is foreground versus background?
what persists?
what moved?
what changed?
what is direct versus reverberant?
what is a point-like source versus a field?
what is the room/environment doing?
what is uncertain?
```

Modern general audio representations such as BEATs and HEAR-style embedding benchmarks are useful sources of semantic/contextual evidence. Two!Ears is a particularly relevant architectural precedent because it combines bottom-up auditory processing with higher-level hypotheses and top-down knowledge. Clarity/Cadenza work is useful for perceptual prediction, intelligibility, hearing models, and objective audio-quality methodology.

These systems are references and candidate components, not automatic dependencies.

---

## 13. Why Omniphony is the rendering base

Current Omniphony already provides useful foundations:

- independent binaural output that bypasses the VBAP speaker path;
- per-channel/object 3D positions;
- separate ITD and HRIR processing;
- embedded measured KEMAR, synthetic/parametric, and SOFA HRTF paths;
- first-order shoebox image-source geometry;
- shared stereo FDN late reverberation;
- distance scaling and air absorption;
- real-time Rust rendering;
- deterministic non-realtime file output;
- decoder bridge ABI;
- OSC state/control and 3D Studio supervision;
- a self-contained multichannel/7.1.4 reference asset;
- Windows ASIO path in the current upstream architecture.

As of this research snapshot, the fork contains upstream commit `c48808f509ab5b56525e1df1765ff81146bc4e4b`, so the current research documentation is based on the recent upstream code rather than an older detached foundation.

The immediate acoustic job remains raising the native binaural-renderer ceiling before allowing complex stereo inference to hide renderer weaknesses.

---

## 14. Stock Omniphony demo as a calibration anchor

The stock/native multichannel demonstration is important evidence because it shows that Omniphony can already create a convincing external spatial bubble when sufficient scene structure is supplied.

That reframes the research problem.

Instead of only asking:

> Can Omniphony make spatial headphone audio?

ask:

> **How much of a convincing known spatial scene can the hearing system recover after that scene is collapsed to stereo?**

A powerful controlled experiment is:

```text
KNOWN NATIVE SCENE
       │
       ├► direct Omniphony binaural render ──────────► reference percept
       │
       └► controlled stereo downmix
                ↓
          machine hearing
                ↓
        reconstructed scene
                ↓
        same Omniphony renderer
                ↓
          reconstructed percept
```

Now the original native scene supplies ground truth for source geometry and structure without requiring the listener to annotate every detail.

This should become a core training/evaluation family for stereo reconstruction.

---

## 15. HeSuVi as a perceptual reference, not the target architecture

A local HeSuVi package was inspected as a reference because the current listening setup uses HeSuVi and a DTS Virtual:X-derived HRIR and because Omniphony's stock binaural demo sounds strongly bubble-like both with HeSuVi disabled and with HeSuVi added afterward.

The uploaded HeSuVi configuration shows a useful distinction: the active spatial effect is not simply headphone EQ or a generic crossfeed.

In the inspected setup:

- `cfact.txt` disables the optional crossfeed block (`usecf=false`);
- channel gain is neutral;
- the per-channel EQ include files are effectively inactive/no-op in this snapshot;
- `master.txt` applies a simple 0.90 output scale;
- stereo input is expanded into virtual center/side/rear channels with signed mixtures;
- those virtual channels are remapped into ear-specific convolution paths;
- the selected 48 kHz convolution file is `DTSVirtualX-for-speakers.wav`;
- that filter bank contains 14 channels and 1024 float samples per channel at 48 kHz.

The stereo matrix in the inspected configuration is approximately:

```text
C  =  0.20 L + 0.20 R
RL =  0.30 L - 0.20 R
RR = -0.20 L + 0.30 R
SL =  0.45 L - 0.25 R
SR = -0.25 L + 0.45 R
```

A following remap keeps most direct L/R energy in front while feeding portions of the synthetic side/rear field into the virtual surround paths before the 14-channel convolution and final binaural sum.

Simple inspection of the selected HRIR bank shows direction-dependent peak timing and post-peak energy across ear paths. This is consistent with the bank carrying distinct ear/direction cues rather than acting as one global stereo effect. It does **not** establish which proprietary internal design choices are responsible for the user's preference.

### 15.1 What to learn from HeSuVi

HeSuVi gives us several useful research hypotheses:

1. A virtual-speaker field plus ear-specific filters can create substantial perceived volume even from stereo.
2. Cross-ear timing/spectral structure can add externalization and side/rear differentiation.
3. Signed/difference components can expose lateral/diffuse information, but they can also create phase artifacts and should not become the permanent scene model by default.
4. The perceptual value of HeSuVi should be decomposed into measurable cues rather than copied as a black box.
5. A large library of virtualizer HRIRs/headphone corrections is useful as an offline comparison corpus where licensing permits, but Omniphony should not depend on redistributing proprietary filters.

### 15.2 Differential HeSuVi experiment

A useful reference comparison is:

```text
Omniphony
vs
Omniphony + current HeSuVi chain
```

Measure what changes in:

- interaural coherence by band;
- cross-ear delay/group delay;
- spectral balance;
- side/rear energy;
- apparent width;
- externalization;
- room/envelopment proxies;
- transient integrity.

Then ask what audible benefit remains after loudness matching.

If a cue consistently improves the experience, reproduce that cue **inside Omniphony through the cleanest perceptually correct mechanism available**, rather than requiring permanent double virtualization.

The desired endpoint is:

```text
Omniphony
→ complete scene + binaural + headphone translation
→ headphones
```

with HeSuVi no longer needed because its useful perceptual contribution has been understood and absorbed.

---

## 16. Reproducible controls

Stable binaural controls live in:

```text
omniphony-renderer/assets/binaural-baselines/
```

The baseline configs must continue to exercise already-established renderer behavior only. Experimental algorithms receive explicit flags/configs and must not silently modify the control.

For every meaningful candidate retain:

- exact source asset/hash;
- source codec/container metadata where relevant;
- exact decoded PCM conditions;
- exact config;
- sample rate / block size;
- renderer commit;
- hearing-model version;
- HRTF source;
- output gain;
- objective measurements;
- listening result;
- failure reason if rejected.

Offline and real-time paths should execute the same DSP whenever practical.

---

## 17. Immediate renderer research frontier

### R1. Directional early reflections

Current Omniphony computes first-order image-source positions and relative delays, then renders each reflection with a much simpler directional treatment than the direct binaural source path. The geometry contains more directional information than the ear signals currently receive.

Test:

1. important early images rendered through interpolated HRIRs;
2. early images encoded into an HOA field and binauralized once;
3. a hybrid path using precise HRTF for strongest/earliest images and field rendering for weaker/dense images.

Measure and listen for:

- externalization;
- source width/body without lost identity;
- room geometry;
- front/back stability;
- transient-envelope error;
- comb coloration;
- interaural coherence by band;
- CPU cost.

This remains the first bounded DSP experiment.

### R2. HRTF geometry and interpolation

Compare current grid behavior against:

- denser grids;
- full lower-hemisphere coverage;
- spherical-neighbor interpolation;
- SOFA native-grid interpolation;
- normalization/equalization alternatives.

No grid-resolution control belongs in the normal UI.

### R3. Hybrid direct-source / field binauralization

```text
high-confidence direct source → precise direct HRTF
broad / diffuse material      → HOA / field representation
early room events             → directional or hybrid
late room energy              → diffuse field
```

Do not force one renderer type onto every perceptual role.

### R4. HOA perceptual optimization

Use conventional ACN/SN3D semantics at internal boundaries. Compare basic decoding against psychoacoustically motivated methods such as max-rE, diffuse-field equalization, and MagLS-style approaches where implementation and licensing permit.

### R5. Radial depth and near field

Distance must remain independent from simple gain reduction. Evaluate direct/reverberant ratio, source-linked early reflections, air absorption, near-field HRTF behavior, source extent, bass anchoring, and proximity filtering together.

### R6. Late field

Retain the current FDN as a baseline. Candidate improvements include:

- frequency-dependent decay;
- room-size-dependent mixing;
- controlled interaural coherence;
- explicit transition from directional early events to diffuse late energy;
- spatially distributed late energy rather than stereo decorrelation alone;
- measured BRIR/BinauralSDM comparison;
- expensive learned or simulated room models as offline oracles.

### R7. ASW / LEV independence

Add experiments that vary apparent source width and listener envelopment independently.

A candidate that increases LEV by smearing direct sources fails.

---

## 18. Artificial listener and QESTRAL-style evaluation

Objective models are not a replacement for listening. They are a way to reject obvious regressions, compare thousands of variants, and make perceptual changes inspectable before asking for human attention.

QESTRAL is especially relevant because it explicitly models spatial-reproduction attributes such as location, width, and envelopment through an artificial-listener approach.

Build the evaluator as a family of models rather than one magic score:

```text
                 RENDERED BINAURAL AUDIO
                          │
          ┌───────────────┼────────────────┐
          ▼               ▼                ▼
     PHYSICAL CUES     AUDITORY MODEL    SCENE METRICS
     ITD / ILD         filterbanks       localization
     coherence         masking           source width
     spectrum          binaural cues     envelopment
     DRR               loudness          stability
     transients        coloration        depth
     early/late                          room structure
```

Candidate diagnostics include:

- short-time ILD;
- ITD/group-delay behavior;
- interaural coherence by band;
- direct/reverberant ratio;
- early-reflection timing/energy;
- transient-envelope error;
- magnitude-response deviation;
- left/right energy bias;
- RMS/integrated loudness;
- true peak/headroom;
- estimated localization;
- ASW proxies/models;
- LEV/envelopment proxies/models;
- source stability;
- deterministic fixture hashes;
- real-time CPU/callback timing.

A future prefilter loop:

```text
candidate renderer / hearing model
          ↓
known fixtures
          ↓
artificial-listener metrics
          ↓
reject obvious regressions
          ↓
retain informative candidates
          ↓
human matched-loudness listening
```

The final perceptual tribunal remains human listening.

---

## 19. Human listening should be sparse, blinded, and high-value

The user should not maintain spreadsheets of ratings for every build.

When human judgment is needed, automate the experiment and ask only high-information questions.

Useful protocol:

- loudness-match candidates;
- randomize labels/order;
- include occasional duplicate/identity controls;
- preserve source and build metadata automatically;
- ask which presentation the listener would keep using;
- ask for optional short perceptual tags/comments;
- use acclimation + removal for major candidates.

Score dimensions separately when needed:

```text
front externalization
rear discrimination
side precision
elevation
radial distance
apparent source width
listener envelopment
source extent
source separation
source stability
room presence / scale
ambient continuity
transient clarity
vocal/direct clarity
timbral fidelity
bass stability
fatigue
bypass-collapse strength
```

Desired removal result:

> acoustic volume, depth, height, externalization, or envelopment collapses while the dry/reference source does not suddenly become obviously cleaner or more stable.

---

## 20. Research references and their roles

Use external projects to extract methods, controls, and independent hypotheses, not to assemble a dependency collage.

### Spatial / binaural / room

- Omniphony upstream architecture;
- Spatial Audio Framework / SPARTA;
- IEM Plug-in Suite;
- VISR / Binaural Synthesis Toolkit;
- BinauralSDM;
- OpenAL Soft;
- Google Open Binaural Renderer;
- libspatialaudio;
- libmysofa;
- pyfar / sofar / spharpy;
- pyroomacoustics;
- measured HRTF/BRIR databases.

### Auditory modeling / general hearing

- QESTRAL/artificial-listener work;
- Two!Ears;
- BEATs/acoustic-token representations;
- HEAR benchmark/evaluation approach;
- Clarity/Cadenza perceptual and hearing-model tooling;
- established binaural auditory models.

### Music understanding

- Essentia;
- madmom;
- All-In-One music structure analysis;
- Open-Unmix;
- Demucs as separation evidence/reference;
- Norbert/generalized Wiener filtering;
- learned stereo-to-spatial models such as Ambisonizer-style systems;
- research on reverse-engineering mix graphs and direct remixing.

A useful algorithm does not imply that its code/data belongs in the production binary. Check license compatibility before importing anything.

---

## 21. Anti-patterns and boundaries

Reject or heavily penalize candidates that depend primarily on:

- loudness increase;
- bass boost;
- generic reverb as a substitute for scene structure;
- blind anti-phase/synthetic rear generation as the final architecture;
- hard per-bin speaker assignment;
- rapid scene churn;
- indiscriminate decorrelation;
- duplicating content into rear/height channels with EQ/reverb only;
- mandatory per-track manual tuning;
- proprietary HRIR redistribution;
- permanent double binauralization;
- resurrecting the abandoned Spatial DSP renderer wholesale;
- AI-generated replacement audio when metadata/control would preserve the original more faithfully;
- semantic labels treated as if they were auditory objects;
- optimizing one aggregate spatial score at the expense of independent perceptual dimensions.

Do not delete upstream head-tracking support, but static listening remains the primary target for this research.

Do not claim that the system already hears exactly like a human. The research target is progressively stronger **human-perceptually aligned machine hearing**, with explicit tests showing which parts are and are not human-like.

---

## 22. Development order

```text
0. preserve reproducible stock/native controls
1. raise native binaural renderer ceiling
   - directional early reflections
   - HRTF interpolation / coverage
   - radial depth / near field
   - early→late room transition
   - direct vs field rendering
2. build automated artificial-listener / regression harness
3. package codec/container handling so normal Opus and other files require no manual conversion
4. provide optional low-friction listening integration while current foobar + HeSuVi remains intact
5. conservative stereo direct / ambient / reverberant decomposition
6. persistent auditory-object / field scene model
7. music-time structure and mix-hierarchy integration
8. learned scene-intelligence experiments
9. general-machine-hearing expansion and benchmarks
10. automatic standard playback behavior
11. surround/game precision and latency validation
12. Windows-wide capture/output integration
13. headphone translation / personalization
14. optional structured-source expansions such as VGM
```

This is not a waterfall. Evidence can force the project backward whenever a weaker lower layer is discovered.

---

## 23. Current first falsifiable acoustic experiment

**Directional early reflections.**

Keep current image-source positions, relative delays, room dimensions, HRTF source, late field, source content, and overall gain as controlled as practical. Change only how reflected directions reach the ears.

Compare the current simpler early-reflection directional treatment against direction-dependent binaural alternatives.

Advance only if the candidate produces a repeatable increase in externalization, room geometry, source body, or distance **without** measurable/listenable damage to localization, transient envelope, timbre, bass stability, headroom, or musical hierarchy.

This remains first because it improves the renderer itself before stereo inference can obscure the result.

---

## 24. Current first user-facing milestone

Do not ship the listener a laboratory.

The first meaningful personal listening build should:

- coexist with the current foobar + HeSuVi chain;
- not require removing current DSP configuration;
- support normal high-quality Opus library input without manual conversion;
- use bundled/automatic codec handling where practical;
- expose a very small enable/bypass or route switch;
- contain enough proven renderer improvements to sound meaningfully better than a trivial baseline on first setup;
- log its own exact version/configuration for later comparison;
- fail safely back to the current playback route;
- avoid requiring ASIO SDK/compiler/toolchain installation by the listener;
- eventually be buildable as a downloadable Windows artifact rather than a local Rust development ritual.

The target experience is not:

```text
convert file
edit YAML
run terminal
label renders
maintain spreadsheet
```

It is closer to:

```text
install
select Omniphony when ready
listen normally
```

The user should be able to wean off the existing stack only after Omniphony proves stable and superior.

---

## 25. Current durable project laws

1. **Research graduates into core.**
2. **Preserve the acoustic object; the model should expose it, not replace it unnecessarily.**
3. **Preserve truth; infer only absence.**
4. **Clarity-preserving dimensionality.**
5. **Object integrity.**
6. **Dimensional independence.**
7. **Spatial persistence.**
8. **Cue agreement.**
9. **Spatial specificity follows confidence.**
10. **ASW/source body and LEV/environment are independent perceptual axes.**
11. **Directional early structure must preserve temporal-envelope fidelity.**
12. **Direct sources, ambient fields, and room fields are different perceptual jobs.**
13. **Offline and real-time paths should share DSP.**
14. **Objective/artificial-listener models assist triage; human listening remains final.**
15. **Music is the primary everyday proving ground, not the boundary of the hearing architecture.**
16. **A stem is evidence; an auditory object is a perceptual hypothesis.**
17. **Semantic intelligence comes after lower-level auditory organization, not before it.**
18. **The mix itself is evidence and should not be casually remastered away.**
19. **Native multichannel/object structure should be preserved rather than rediscovered.**
20. **Codec/container complexity belongs below the user interface.**
21. **HeSuVi is a useful perceptual reference and transition aid, not the permanent target architecture.**
22. **The existing listener setup remains intact until Omniphony earns replacement.**
23. **The first user-facing install should produce a meaningful improvement, not merely expose a developer baseline.**
24. **Better hearing should improve rendering; better rendering experiments should improve the hearing model.**

The project should ultimately become a single coherent system in which machine hearing, scene representation, binaural acoustics, headphone translation, perceptual measurement, and human correction continually make one another better.