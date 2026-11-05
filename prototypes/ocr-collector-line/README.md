# OCR collector line prototype (#646, #656)

Throwaway app: opens the iPhone camera, runs Vision text recognition (accurate level, no language
correction) on every frame, and shows the raw recognised text and the time to a stable reading.
Never merged into `main`; nothing here touches `ios-app/`.

## Run

1. `xcodegen generate` in this directory (the `.xcodeproj` is git-ignored).
2. Replace `YOUR_TEAM_ID` in `project.yml` (both targets) with your Apple development team, regenerate.
3. Open `OCRPrototype.xcodeproj`, pick your iPhone, run.

The camera does not exist in the simulator: the simulator only proves it builds and that the
tests pass. Real verification needs an iPhone and physical cards.

## Timing measure

Frame timestamps come from the capture buffers (`PresentationTimer`).

- Card presented: first frame where Vision returns at least 3 text lines after the lens was
  empty (no such frame) for 1 s.
- Recognised: first later frame whose set of lines equals the previous frame's (one-frame
  stabilisation).
- Latency = recognised time - presented time. It includes the OCR time of each frame in between
  (`ms / frame` is shown too). Frames are dropped while Vision is busy.

Caveat: "presented" is when text first becomes readable, not when the card physically enters the
frame, so the figure is a lower bound.

## Collector line reader (#657)

`CollectorLineReader.read(lines:)` is a pure function: Vision's lines of one frame in, a
`CollectorReading {set, number, language, isToken}` or nil out. Understood formats (2014-2022 then since 2023), the two lines being separate or merged:

- `0123/0280 R` then `DMU • FR  Artist`
- `R 0123` then `MOM • EN`

Rarity `T` = token (the Scryfall set is then `t` + `set`). The "•" may come as `*`, `·`, `-`;
O/I/l in the number are read as 0/1; leading zeros are stripped; printed languages map to Scryfall
codes (SP -> es, JP -> ja...). A reading needs set, number and a known language together. No
bounding boxes are used.

Stabilisation: a reading is retained only when identical on two consecutive frames
(`PresentationTimer.observe(lines:reading:at:)`), so the latency shown includes that extra frame.

The fixtures in `OCRPrototypeTests/CollectorLineFixtures.swift` are SYNTHETIC (written from the
known printed formats, not captured): replace/extend them with real captures in #658.

## Fixtures

`Copy reading` / `Export` give the raw lines frozen at the measure, one Vision line per row.
Paste them into the test target as fixtures for the collector line reader (#657).

## Tests

From this directory (outside `mise`, the prototype has no task): `xcodebuild test -project
OCRPrototype.xcodeproj -scheme OCRPrototype -destination 'platform=iOS Simulator,id=<simulator id>'
CODE_SIGNING_ALLOWED=NO`. Use the id (`xcrun simctl list devices`): a device name matches several
runtimes.
