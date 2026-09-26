# rubic

A Rubik's cube app: the user enters a physical cube (by hand, by camera, or by shuffling), then plays it or watches a solver solve it.

## Language

### Entering a cube

**Flow**:
Where the user is in the app: the Method picker, Editing, a Scan, or Solve. Exactly one at a time.
_Avoid_: mode, screen, stage

**Setup method**:
How the user enters their cube: Shuffle, Manual, or Camera.
_Avoid_: input mode, entry mode

**Shuffle**:
Replace the cube with a random scrambled one and go straight to Solve. Available from anywhere.
_Avoid_: New Game, scramble

**Method picker**:
The opening screen where the user chooses a Setup method.
_Avoid_: menu, start screen

**Editing**:
Painting stickers onto the Net, either from blank (Manual) or reviewing a finished Scan.
_Avoid_: paint mode, review mode

**Entry**:
The cube being entered while Editing: its painted stickers, the Brush, and its Completion.
_Avoid_: input, input state

**Start over**:
Abandon the current cube and return to the Method picker.
_Avoid_: reset, cancel

**Net**:
The flat 2D unfolding of the cube's six faces, used to show and paint stickers.
_Avoid_: map, grid

**Palette**:
The six face colors the user paints with; the selected one is the **Brush**.
_Avoid_: swatches, color picker

**Completion**:
Whether the stickers entered so far determine a cube: Unique, NeedMore, or Impossible.
_Avoid_: validity

**Ready**:
The cube entered so far has a Unique Completion and can be solved.
_Avoid_: valid, complete

### Camera

**Scan**:
Entering a cube by capturing its six faces, one at a time, with the camera.
_Avoid_: camera input, detection

**Face capture**:
Committing the face currently in view as one of the Scan's six faces. It can be retaken.
_Avoid_: snapshot, grab

**Handoff**:
The end of a Scan, when the captured faces become the cube being Edited.

### Solving

**Solve**:
Playing the entered cube by hand, or following a Solver's Solution.
_Avoid_: play mode

**Solution**:
A Solver's answer: a sequence of Steps, each a run of moves within a Stage.

**Playback**:
Stepping or animating through a Solution on the 3D cube.
_Avoid_: replay, animation

### Controls

**Action**:
Something the user asks the app to do, whatever the source: a key, a button tap, a Net click or a drag. The same Action means the same thing from every source.
_Avoid_: command, key, event
