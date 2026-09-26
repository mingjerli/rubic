# Flow is a pure state machine, not Bevy `States`

The app's Flow (Method picker, Editing, Scan, Solve) is one enum advanced by a pure `step(flow, Action) -> (flow, Effects)`; Bevy systems only translate input into Actions and apply the returned Effects. We chose this over Bevy `States` with `OnEnter`/`OnExit` schedules because the transition rules (e.g. "every way out of a Scan closes the camera", "Confirm only when Ready") then live in one place and are tested as a plain table without building an `App`. Scattered `OnExit` systems would recreate the problem this replaced, where `AppMode`/`InputStage` were written from six sites across three files and Shuffle mid-Scan left the camera running.

## Consequences

- Run conditions are derived from `Flow`; there is no `AppMode` resource.
- Any new side effect of a transition must be added to the closed `Effect` set, which is where reviewers should look for leaks.
